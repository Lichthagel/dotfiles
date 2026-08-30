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
