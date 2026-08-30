use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::model::{PackageManager, Platform};

use super::{replace_file_at, set_private_permissions, BackupRecord, PathContext, PlatformOps};

pub(crate) const fn package_manager_program(
    platform: Platform,
    manager: PackageManager,
) -> Option<&'static str> {
    match (platform, manager) {
        (
            Platform::Linux,
            PackageManager::Apt
            | PackageManager::Dnf
            | PackageManager::Pacman
            | PackageManager::Brew
            | PackageManager::Mise,
        ) => Some(manager.label()),
        (Platform::Windows, PackageManager::Winget) => Some("winget"),
        (Platform::Windows, PackageManager::Scoop) => Some("scoop"),
        (Platform::Windows, PackageManager::Brew) => Some("brew"),
        (Platform::Windows, PackageManager::Mise) => Some("mise"),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub struct UnixPlatform {
    context: PathContext,
}

impl UnixPlatform {
    pub fn new(context: PathContext) -> Self {
        Self { context }
    }

    pub fn context(&self) -> &PathContext {
        &self.context
    }
}

impl PlatformOps for UnixPlatform {
    fn platform(&self) -> Platform {
        Platform::Linux
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

pub fn from_environment() -> Result<UnixPlatform> {
    let home = super::current_home()?;
    let backup_root = super::current_backup_root(&home);
    Ok(UnixPlatform::new(PathContext::new(home, None, backup_root)))
}

pub fn default_profile_path(home: &Path) -> PathBuf {
    super::current_profile(home)
}
