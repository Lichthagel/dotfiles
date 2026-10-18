use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileChange {
    Unchanged,
    Content { backup: Option<PathBuf> },
    Permissions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileApplyResult {
    pub target: PathBuf,
    pub change: FileChange,
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

pub trait TimestampProvider {
    fn timestamp(&self) -> String;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemTimestamp;

impl TimestampProvider for SystemTimestamp {
    fn timestamp(&self) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}-{:09}", now.as_secs(), now.subsec_nanos())
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
    fn state_path(&self) -> Result<PathBuf> {
        let backup_root = self.backup_root()?;
        let root = backup_root.parent().ok_or_else(|| {
            InstallError::State("backup directory has no parent for module state".into())
        })?;
        Ok(root.join("state.json"))
    }
    fn resolve_target(&self, target: Target) -> Result<PathBuf> {
        resolve_target_in_context(self.path_context(), target)
    }
    fn replace_file(&self, target: &Path, bytes: &[u8]) -> Result<BackupRecord> {
        self.replace_file_with_timestamp(target, bytes, &SystemTimestamp)
    }
    fn replace_file_with_timestamp(
        &self,
        target: &Path,
        bytes: &[u8],
        timestamp: &dyn TimestampProvider,
    ) -> Result<BackupRecord> {
        replace_file_at_with_timestamp(self.path_context(), target, bytes, false, timestamp)
    }
    fn replace_private_file(&self, target: &Path, bytes: &[u8]) -> Result<BackupRecord> {
        self.replace_private_file_with_timestamp(target, bytes, &SystemTimestamp)
    }
    fn replace_private_file_with_timestamp(
        &self,
        target: &Path,
        bytes: &[u8],
        timestamp: &dyn TimestampProvider,
    ) -> Result<BackupRecord> {
        replace_private_file_at_with_timestamp(
            self.path_context(),
            target,
            bytes,
            false,
            timestamp,
            &|path| self.set_private_file_permissions(path),
        )
    }
    fn ensure_file(&self, target: &Path, bytes: &[u8]) -> Result<FileApplyResult> {
        self.ensure_file_with_timestamp(target, bytes, &SystemTimestamp)
    }
    fn ensure_file_with_timestamp(
        &self,
        target: &Path,
        bytes: &[u8],
        timestamp: &dyn TimestampProvider,
    ) -> Result<FileApplyResult> {
        let replace = |path: &Path, content: &[u8], time: &dyn TimestampProvider| {
            self.replace_file_with_timestamp(path, content, time)
        };
        ensure_file_at_with_timestamp(target, bytes, timestamp, &replace, None, None)
    }
    fn ensure_private_file(&self, target: &Path, bytes: &[u8]) -> Result<FileApplyResult> {
        let replace = |path: &Path, content: &[u8], time: &dyn TimestampProvider| {
            self.replace_private_file_with_timestamp(path, content, time)
        };
        let protect = |path: &Path| self.set_private_file_permissions(path);
        let inspect = |path: &Path| self.private_file_permissions_are_correct(path);
        ensure_file_at_with_timestamp(
            target,
            bytes,
            &SystemTimestamp,
            &replace,
            Some(&protect),
            Some(&inspect),
        )
    }
    fn private_file_permissions_are_correct(&self, path: &Path) -> Result<bool> {
        private_permissions_are_correct(path)
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

type FileReplacement<'a> =
    dyn Fn(&Path, &[u8], &dyn TimestampProvider) -> Result<BackupRecord> + 'a;
type FileProtector<'a> = dyn Fn(&Path) -> Result<()> + 'a;
type PermissionInspector<'a> = dyn Fn(&Path) -> Result<bool> + 'a;

fn ensure_file_at_with_timestamp(
    target: &Path,
    bytes: &[u8],
    timestamp: &dyn TimestampProvider,
    replace: &FileReplacement<'_>,
    protect: Option<&FileProtector<'_>>,
    inspect_private: Option<&PermissionInspector<'_>>,
) -> Result<FileApplyResult> {
    if protect.is_some() != inspect_private.is_some() {
        return Err(InstallError::Mapping(
            "private replacement requires both permission operations".into(),
        ));
    }
    let presence = fs::symlink_metadata(target);
    match presence {
        Ok(metadata) if metadata.file_type().is_file() => {
            let contents_match = if inspect_private.is_some() {
                Zeroizing::new(fs::read(target)?).as_slice() == bytes
            } else {
                fs::read(target)? == bytes
            };
            if contents_match {
                if let Some(inspect) = inspect_private {
                    if !inspect(target)? {
                        protect.expect("private inspection requires a protector")(target)?;
                        if !inspect(target)? {
                            return Err(InstallError::Mapping(format!(
                                "private permissions remain unsafe: {}",
                                target.display()
                            )));
                        }
                        return Ok(FileApplyResult {
                            target: target.to_path_buf(),
                            change: FileChange::Permissions,
                        });
                    }
                }
                return Ok(FileApplyResult {
                    target: target.to_path_buf(),
                    change: FileChange::Unchanged,
                });
            }
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    let record = replace(target, bytes, timestamp)?;
    Ok(FileApplyResult {
        target: record.target,
        change: FileChange::Content {
            backup: record.backup,
        },
    })
}

pub fn replace_file_at(
    context: &PathContext,
    target: &Path,
    bytes: &[u8],
    fail_rename: bool,
) -> Result<BackupRecord> {
    replace_file_at_with_timestamp(context, target, bytes, fail_rename, &SystemTimestamp)
}

pub fn replace_file_at_with_timestamp(
    context: &PathContext,
    target: &Path,
    bytes: &[u8],
    fail_rename: bool,
    timestamp: &dyn TimestampProvider,
) -> Result<BackupRecord> {
    replace_file_at_with_timestamp_internal(context, target, bytes, fail_rename, timestamp, None)
}

type PrivateProtector<'a> = Option<&'a dyn Fn(&Path) -> Result<()>>;

pub fn replace_private_file_at_with_timestamp(
    context: &PathContext,
    target: &Path,
    bytes: &[u8],
    fail_rename: bool,
    timestamp: &dyn TimestampProvider,
    protect: &dyn Fn(&Path) -> Result<()>,
) -> Result<BackupRecord> {
    replace_file_at_with_timestamp_internal(
        context,
        target,
        bytes,
        fail_rename,
        timestamp,
        Some(protect),
    )
}

fn replace_file_at_with_timestamp_internal(
    context: &PathContext,
    target: &Path,
    bytes: &[u8],
    fail_rename: bool,
    timestamp: &dyn TimestampProvider,
    protect: PrivateProtector<'_>,
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
            timestamp.timestamp(),
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
    if let Some(protect) = protect {
        if let Err(error) = protect(&stage_path) {
            let _ = fs::remove_file(&stage_path);
            return Err(error);
        }
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
            .join(timestamp.timestamp())
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
        if let Some(protect) = protect {
            if let Err(error) = protect(&backup) {
                let _ = fs::remove_file(&stage_path);
                if let Err(restore_error) = fs::rename(&backup, target) {
                    return Err(InstallError::Mapping(format!(
                        "private backup permission setup failed: {error}; restoring backup failed: {restore_error}"
                    )));
                }
                return Err(error);
            }
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
fn private_permissions_are_correct(path: &Path) -> Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    Ok(fs::metadata(path)?.permissions().mode() & 0o7777 == 0o600)
}

#[cfg(windows)]
fn private_permissions_are_correct(path: &Path) -> Result<bool> {
    windows_private_permissions_are_correct(path)
}

#[cfg(not(any(unix, windows)))]
fn private_permissions_are_correct(_path: &Path) -> Result<bool> {
    Err(InstallError::Mapping(
        "private permissions are unsupported on this platform".into(),
    ))
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
fn windows_private_permissions_are_correct(path: &Path) -> Result<bool> {
    use std::mem::zeroed;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{
        GetLastError, LocalFree, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS,
    };
    use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
    use windows_sys::Win32::Security::{
        AclSizeInformation, EqualSid, GetAce, GetAclInformation, GetSecurityDescriptorControl,
        GetSecurityDescriptorDacl, LookupAccountNameW, ACCESS_ALLOWED_ACE, ACE_HEADER,
        ACL_SIZE_INFORMATION, DACL_SECURITY_INFORMATION, SE_DACL_PROTECTED, SID_NAME_USE,
    };
    use windows_sys::Win32::Storage::FileSystem::FILE_ALL_ACCESS;
    use windows_sys::Win32::System::WindowsProgramming::GetUserNameW;

    let mut username = [0u16; 256];
    let mut username_len = username.len() as u32;
    if unsafe { GetUserNameW(username.as_mut_ptr(), &mut username_len) } == 0 {
        return Err(InstallError::Mapping(format!(
            "could not determine current Windows user: {}",
            io::Error::last_os_error()
        )));
    }
    let mut sid_len = 0u32;
    let mut domain_len = 0u32;
    let mut use_type: SID_NAME_USE = unsafe { zeroed() };
    let found = unsafe {
        LookupAccountNameW(
            null(),
            username.as_ptr(),
            null_mut(),
            &mut sid_len,
            null_mut(),
            &mut domain_len,
            &mut use_type,
        )
    };
    if found != 0 || unsafe { GetLastError() } != ERROR_INSUFFICIENT_BUFFER {
        return Err(InstallError::Mapping(format!(
            "could not resolve current Windows user SID: {}",
            io::Error::last_os_error()
        )));
    }
    let mut sid_storage = vec![0u32; sid_len.div_ceil(4) as usize];
    let mut domain = vec![0u16; domain_len as usize];
    if unsafe {
        LookupAccountNameW(
            null(),
            username.as_ptr(),
            sid_storage.as_mut_ptr().cast(),
            &mut sid_len,
            domain.as_mut_ptr(),
            &mut domain_len,
            &mut use_type,
        )
    } == 0
    {
        return Err(InstallError::Mapping(format!(
            "could not resolve current Windows user SID: {}",
            io::Error::last_os_error()
        )));
    }

    let mut wide_path: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide_path.push(0);
    let mut descriptor = null_mut();
    let status = unsafe {
        GetNamedSecurityInfoW(
            wide_path.as_mut_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            &mut descriptor,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(InstallError::Mapping(format!(
            "could not inspect private Windows ACL: {status}"
        )));
    }

    let result = (|| {
        let mut control = 0u16;
        let mut revision = 0u32;
        if unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) } == 0 {
            return Err(InstallError::Mapping(format!(
                "could not inspect private Windows ACL control: {}",
                io::Error::last_os_error()
            )));
        }
        if control & SE_DACL_PROTECTED == 0 {
            return Ok(false);
        }
        let mut present = 0;
        let mut acl = null_mut();
        let mut defaulted = 0;
        if unsafe { GetSecurityDescriptorDacl(descriptor, &mut present, &mut acl, &mut defaulted) }
            == 0
        {
            return Err(InstallError::Mapping(format!(
                "could not inspect private Windows DACL: {}",
                io::Error::last_os_error()
            )));
        }
        if present == 0 || acl.is_null() {
            return Ok(false);
        }
        let mut information: ACL_SIZE_INFORMATION = unsafe { zeroed() };
        if unsafe {
            GetAclInformation(
                acl,
                (&mut information as *mut ACL_SIZE_INFORMATION).cast(),
                std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                AclSizeInformation,
            )
        } == 0
        {
            return Err(InstallError::Mapping(format!(
                "could not inspect private Windows ACL entries: {}",
                io::Error::last_os_error()
            )));
        }
        if information.AceCount != 1 {
            return Ok(false);
        }
        let mut ace_pointer = null_mut();
        if unsafe { GetAce(acl, 0, &mut ace_pointer) } == 0 {
            return Err(InstallError::Mapping(format!(
                "could not inspect private Windows ACL entry: {}",
                io::Error::last_os_error()
            )));
        }
        let header = unsafe { &*ace_pointer.cast::<ACE_HEADER>() };
        if header.AceType != 0 || header.AceFlags != 0 {
            return Ok(false);
        }
        let ace = unsafe { &*ace_pointer.cast::<ACCESS_ALLOWED_ACE>() };
        if ace.Mask != FILE_ALL_ACCESS {
            return Ok(false);
        }
        let ace_sid = (&ace.SidStart as *const u32).cast_mut().cast();
        Ok(unsafe { EqualSid(ace_sid, sid_storage.as_mut_ptr().cast()) != 0 })
    })();
    unsafe {
        LocalFree(descriptor.cast());
    }
    result
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

pub(crate) fn current_scoop_root(home: &Path) -> PathBuf {
    if let Some(root) =
        std::env::var_os("DOTFILES_SCOOP_ROOT").or_else(|| std::env::var_os("SCOOP"))
    {
        return PathBuf::from(root);
    }
    std::env::var_os("DOTFILES_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.to_path_buf())
        .join("scoop")
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
                let documents = windows::documents_dir();
                windows::profile_path_from_documents(home, documents.as_deref())
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
    fn private_permission_inspection_errors_propagate_without_replacement() {
        use super::{
            ensure_file_at_with_timestamp, BackupRecord, Path, Result, SystemTimestamp,
            TimestampProvider,
        };
        use std::fs;
        use tempfile::tempdir;

        let temp = tempdir().unwrap();
        let target = temp.path().join("credential");
        fs::write(&target, b"secret").unwrap();
        let replace =
            |_path: &Path,
             _bytes: &[u8],
             _timestamp: &dyn TimestampProvider|
             -> Result<BackupRecord> { panic!("equal content must not be replaced") };
        let protect = |_path: &Path| Ok(());
        let inspect = |_path: &Path| {
            Err(super::InstallError::Mapping(
                "permission inspection denied".into(),
            ))
        };

        let error = ensure_file_at_with_timestamp(
            &target,
            b"secret",
            &SystemTimestamp,
            &replace,
            Some(&protect),
            Some(&inspect),
        )
        .unwrap_err();

        assert!(error.to_string().contains("permission inspection denied"));
        assert_eq!(fs::read(target).unwrap(), b"secret");
    }

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
