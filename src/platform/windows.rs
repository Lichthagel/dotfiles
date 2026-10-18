use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::model::{PackageManager, Platform};

use super::{replace_file_at, set_private_permissions, BackupRecord, PathContext, PlatformOps};

pub(crate) const fn package_manager_program(
    platform: Platform,
    manager: PackageManager,
) -> Option<&'static str> {
    match (platform, manager) {
        (Platform::Linux, PackageManager::Apt) => Some("apt"),
        (Platform::Linux, PackageManager::Dnf) => Some("dnf"),
        (Platform::Linux, PackageManager::Pacman) => Some("pacman"),
        (Platform::Linux, PackageManager::Brew) => Some("brew"),
        (Platform::Linux, PackageManager::Mise) => Some("mise"),
        (Platform::Windows, PackageManager::Winget) => Some("winget"),
        (Platform::Windows, PackageManager::Scoop) => Some("scoop"),
        (Platform::Windows, PackageManager::Brew) => Some("brew"),
        (Platform::Windows, PackageManager::Mise) => Some("mise"),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub struct WindowsPlatform {
    context: PathContext,
}

impl WindowsPlatform {
    pub fn new(context: PathContext) -> Self {
        Self { context }
    }

    pub fn context(&self) -> &PathContext {
        &self.context
    }
}

impl PlatformOps for WindowsPlatform {
    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn path_context(&self) -> &PathContext {
        &self.context
    }

    fn replace_file(&self, target: &Path, bytes: &[u8]) -> Result<BackupRecord> {
        replace_file_at(&self.context, target, bytes, false)
    }

    fn set_private_file_permissions(&self, path: &Path) -> Result<()> {
        set_private_permissions(path)
    }
}

pub fn from_environment() -> Result<WindowsPlatform> {
    let home = super::current_home()?;
    let profile = super::current_profile(&home);
    let backup_root = super::current_backup_root(&home);
    Ok(WindowsPlatform::new(PathContext::new(
        home,
        Some(profile),
        backup_root,
    )))
}

pub fn default_profile_path(home: &Path) -> PathBuf {
    super::current_profile(home)
}

pub(crate) fn profile_path_from_documents(home: &Path, documents: Option<&Path>) -> PathBuf {
    documents
        .map(Path::to_path_buf)
        .unwrap_or_else(|| home.join("Documents"))
        .join("PowerShell")
        .join("Microsoft.PowerShell_profile.ps1")
}

pub(crate) fn documents_dir() -> Option<PathBuf> {
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{FOLDERID_Documents, SHGetKnownFolderPath};

    let mut raw_path = std::ptr::null_mut();
    let status = unsafe {
        SHGetKnownFolderPath(&FOLDERID_Documents, 0, std::ptr::null_mut(), &mut raw_path)
    };
    if status < 0 || raw_path.is_null() {
        if !raw_path.is_null() {
            unsafe { CoTaskMemFree(raw_path.cast()) };
        }
        return None;
    }

    let documents = unsafe {
        let mut length = 0;
        while *raw_path.add(length) != 0 {
            length += 1;
        }
        let wide_path = std::slice::from_raw_parts(raw_path, length);
        let documents = PathBuf::from(OsString::from_wide(wide_path));
        CoTaskMemFree(raw_path.cast());
        documents
    };
    Some(documents)
}

#[cfg(test)]
mod tests {
    use super::profile_path_from_documents;
    use std::path::{Path, PathBuf};

    #[test]
    fn profile_fallback_respects_a_custom_documents_directory() {
        let home = Path::new(r"C:\Users\Example");
        let documents = PathBuf::from(r"D:\Documents");

        assert_eq!(
            profile_path_from_documents(home, Some(&documents)),
            documents.join("PowerShell/Microsoft.PowerShell_profile.ps1")
        );
    }
}

pub fn default_identity_path(home: &Path) -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("AppData").join("Roaming"))
        .join("age")
        .join("keys.txt")
}
