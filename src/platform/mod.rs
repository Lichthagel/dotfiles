use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{InstallError, Result};
use crate::model::{PackageManager, Platform, Target};

#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathContext {
    pub home: PathBuf,
    pub profile: Option<PathBuf>,
    pub backup_root: PathBuf,
}

impl PathContext {
    pub fn new(home: PathBuf, profile: Option<PathBuf>, backup_root: PathBuf) -> Self {
        Self {
            home,
            profile,
            backup_root,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupRecord {
    pub target: PathBuf,
    pub backup: Option<PathBuf>,
}

pub fn package_manager_program(
    platform: Platform,
    manager: PackageManager,
) -> Option<&'static str> {
    match platform {
        #[cfg(unix)]
        _ => unix::package_manager_program(platform, manager),
        #[cfg(windows)]
        _ => windows::package_manager_program(platform, manager),
        #[cfg(not(any(unix, windows)))]
        _ => None,
    }
}

pub trait PlatformOps {
    fn platform(&self) -> Platform;
    fn path_context(&self) -> &PathContext;
    fn home_dir(&self) -> Result<PathBuf> {
        Ok(self.path_context().home.clone())
    }
    fn profile_path(&self) -> Result<PathBuf> {
        self.path_context()
            .profile
            .clone()
            .ok_or_else(|| InstallError::Mapping("PowerShell profile is unavailable".into()))
    }
    fn backup_root(&self) -> Result<PathBuf> {
        Ok(self.path_context().backup_root.clone())
    }
    fn resolve_target(&self, target: Target) -> Result<PathBuf> {
        resolve_target_in_context(self.path_context(), target)
    }
    fn replace_file(&self, target: &Path, bytes: &[u8]) -> Result<BackupRecord> {
        replace_file_at(self.path_context(), target, bytes, false)
    }
    fn replace_private_file(&self, target: &Path, bytes: &[u8]) -> Result<BackupRecord> {
        let record = self.replace_file(target, bytes)?;
        if let Err(error) = self.set_private_file_permissions(target) {
            let _ = fs::remove_file(target);
            if let Some(backup) = &record.backup {
                if let Err(restore_error) = fs::rename(backup, target) {
                    return Err(InstallError::Mapping(format!(
                        "private permission setup failed: {error}; restoring backup failed: {restore_error}"
                    )));
                }
            }
            return Err(error);
        }
        Ok(record)
    }
    fn set_private_file_permissions(&self, path: &Path) -> Result<()>;
}

pub(crate) fn resolve_target_in_context(context: &PathContext, target: Target) -> Result<PathBuf> {
    match target {
        Target::Home(relative) => join_relative(&context.home, relative),
        Target::ProfileFile => context
            .profile
            .clone()
            .ok_or_else(|| InstallError::Mapping("PowerShell profile is unavailable".into())),
        Target::ProfileSibling(relative) => {
            let profile = context
                .profile
                .as_ref()
                .ok_or_else(|| InstallError::Mapping("PowerShell profile is unavailable".into()))?;
            let parent = profile.parent().ok_or_else(|| {
                InstallError::Mapping(format!(
                    "profile has no parent directory: {}",
                    profile.display()
                ))
            })?;
            join_relative(parent, relative)
        }
    }
}

fn join_relative(root: &Path, relative: &str) -> Result<PathBuf> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(component, Component::ParentDir)
                || (cfg!(windows) && matches!(component, Component::Prefix(_) | Component::RootDir))
        })
    {
        return Err(InstallError::Mapping(format!(
            "mapping target must stay below {}: {relative}",
            root.display()
        )));
    }
    Ok(root.join(relative_path))
}

fn timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}-{:09}", now.as_secs(), now.subsec_nanos())
}

fn relative_backup_path(context: &PathContext, target: &Path) -> PathBuf {
    if let Ok(relative) = target.strip_prefix(&context.home) {
        return relative.to_path_buf();
    }
    let mut relative = PathBuf::new();
    for component in target.components() {
        if let Component::Normal(value) = component {
            relative.push(value);
        }
    }
    relative
}

pub fn replace_file_at(
    context: &PathContext,
    target: &Path,
    bytes: &[u8],
    fail_rename: bool,
) -> Result<BackupRecord> {
    let parent = target.parent().ok_or_else(|| {
        InstallError::Mapping(format!("target has no parent: {}", target.display()))
    })?;
    fs::create_dir_all(parent)?;

    let mut stage_path = None;
    let mut stage_file = None;
    for attempt in 0..100u32 {
        let candidate = parent.join(format!(
            ".dotfiles-{}-{}-{}.tmp",
            std::process::id(),
            timestamp(),
            attempt
        ));
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&candidate)
        {
            Ok(file) => {
                stage_path = Some(candidate);
                stage_file = Some(file);
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    let stage_path = stage_path.ok_or_else(|| {
        InstallError::Mapping(format!(
            "could not create staging file beside {}",
            target.display()
        ))
    })?;
    let mut stage_file = stage_file.expect("stage path and file are created together");

    let write_result = (|| -> Result<()> {
        stage_file.write_all(bytes)?;
        stage_file.sync_all()?;
        Ok(())
    })();
    drop(stage_file);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&stage_path);
        return Err(error);
    }

    let existed = match target_presence(fs::symlink_metadata(target)) {
        Ok(existed) => existed,
        Err(error) => {
            let _ = fs::remove_file(&stage_path);
            return Err(error);
        }
    };
    let backup = if existed {
        let backup = context
            .backup_root
            .join(timestamp())
            .join(relative_backup_path(context, target));
        if let Some(parent) = backup.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                let _ = fs::remove_file(&stage_path);
                return Err(error.into());
            }
        }
        if let Err(error) = fs::rename(target, &backup) {
            let _ = fs::remove_file(&stage_path);
            return Err(error.into());
        }
        Some(backup)
    } else {
        None
    };

    let replacement_result = if fail_rename {
        Err(io::Error::other("injected replacement failure"))
    } else {
        fs::rename(&stage_path, target)
    };
    if let Err(error) = replacement_result {
        let _ = fs::remove_file(&stage_path);
        if let Some(backup_path) = &backup {
            if let Err(restore_error) = fs::rename(backup_path, target) {
                return Err(InstallError::Mapping(format!(
                    "replacement failed: {error}; restoring backup failed: {restore_error}"
                )));
            }
        }
        return Err(error.into());
    }

    Ok(BackupRecord {
        target: target.to_path_buf(),
        backup,
    })
}

#[cfg(unix)]
pub(crate) fn set_private_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o600);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(windows)]
pub(crate) fn set_private_permissions(path: &Path) -> Result<()> {
    use std::mem::zeroed;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{LocalFree, ERROR_SUCCESS};
    use windows_sys::Win32::Security::Authorization::{
        BuildExplicitAccessWithNameW, SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W,
        GRANT_ACCESS, SE_FILE_OBJECT,
    };
    use windows_sys::Win32::Security::{
        DACL_SECURITY_INFORMATION, NO_INHERITANCE, PROTECTED_DACL_SECURITY_INFORMATION,
    };
    use windows_sys::Win32::Storage::FileSystem::FILE_ALL_ACCESS;
    use windows_sys::Win32::System::WindowsProgramming::GetUserNameW;

    let mut username = [0u16; 256];
    let mut username_len = username.len() as u32;
    let ok = unsafe { GetUserNameW(username.as_mut_ptr(), &mut username_len) };
    if ok == 0 || username_len == 0 {
        return Err(InstallError::Mapping(format!(
            "could not determine current Windows user: {}",
            io::Error::last_os_error()
        )));
    }

    let mut explicit: EXPLICIT_ACCESS_W = unsafe { zeroed() };
    unsafe {
        BuildExplicitAccessWithNameW(
            &mut explicit,
            username.as_mut_ptr(),
            FILE_ALL_ACCESS,
            GRANT_ACCESS,
            NO_INHERITANCE,
        );
    }
    let mut acl = null_mut();
    let status = unsafe { SetEntriesInAclW(1, &explicit, null(), &mut acl) };
    if status != ERROR_SUCCESS {
        return Err(InstallError::Mapping(format!(
            "could not build private Windows ACL: {status}"
        )));
    }

    let mut wide_path: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide_path.push(0);
    let status = unsafe {
        SetNamedSecurityInfoW(
            wide_path.as_mut_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            acl,
            null(),
        )
    };
    unsafe {
        LocalFree(acl.cast());
    }
    if status != ERROR_SUCCESS {
        return Err(InstallError::Mapping(format!(
            "could not set private Windows ACL: {status}"
        )));
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn set_private_permissions(_path: &Path) -> Result<()> {
    Err(InstallError::Mapping(
        "private permissions are unsupported on this platform".into(),
    ))
}

pub(crate) fn platform_from_cfg() -> Platform {
    #[cfg(unix)]
    {
        Platform::Linux
    }
    #[cfg(windows)]
    {
        Platform::Windows
    }
}

pub(crate) fn current_home() -> Result<PathBuf> {
    let home = std::env::var_os("DOTFILES_HOME");
    #[cfg(unix)]
    let home = home.or_else(|| std::env::var_os("HOME"));
    #[cfg(windows)]
    let home = home.or_else(|| std::env::var_os("USERPROFILE"));
    home.map(PathBuf::from)
        .ok_or_else(|| InstallError::Mapping("home directory is unavailable".into()))
}

pub(crate) fn current_backup_root(home: &Path) -> PathBuf {
    #[cfg(unix)]
    {
        let state = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local").join("state"));
        state.join("dotfiles").join("backups")
    }
    #[cfg(windows)]
    {
        let local = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData").join("Local"));
        local.join("dotfiles").join("backups")
    }
}

pub fn default_identity_path() -> Result<PathBuf> {
    let home = current_home()?;
    Ok(default_identity_path_for_home(&home))
}

fn default_identity_path_for_home(home: &Path) -> PathBuf {
    #[cfg(unix)]
    {
        unix::default_identity_path(home)
    }
    #[cfg(windows)]
    {
        windows::default_identity_path(home)
    }
    #[cfg(not(any(unix, windows)))]
    {
        home.join(".config").join("age").join("keys.txt")
    }
}

pub(crate) fn current_profile(home: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("DOTFILES_PROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                home.join("Documents")
                    .join("PowerShell")
                    .join("Microsoft.PowerShell_profile.ps1")
            })
    }
    #[cfg(not(windows))]
    {
        home.join(".config").join("PowerShell").join("profile.ps1")
    }
}

pub fn default_platform_ops() -> Result<Box<dyn PlatformOps>> {
    let home = current_home()?;
    let backup_root = current_backup_root(&home);
    let profile = if platform_from_cfg() == Platform::Windows {
        Some(current_profile(&home))
    } else {
        None
    };
    let context = PathContext::new(home, profile, backup_root);
    #[cfg(unix)]
    {
        Ok(Box::new(unix::UnixPlatform::new(context)))
    }
    #[cfg(windows)]
    {
        Ok(Box::new(windows::WindowsPlatform::new(context)))
    }
}

pub fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn target_presence(result: io::Result<fs::Metadata>) -> Result<bool> {
    match result {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

pub fn file_exists_or_symlink(path: &Path) -> bool {
    target_presence(fs::symlink_metadata(path)).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::target_presence;
    use std::io;

    #[test]
    fn metadata_errors_are_not_treated_as_absence() {
        let error = target_presence(Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "metadata denied",
        )))
        .unwrap_err();
        assert!(error.to_string().contains("metadata denied"));
    }
}
