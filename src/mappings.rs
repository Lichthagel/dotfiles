use std::path::PathBuf;

use crate::assets::asset_bytes;
use crate::error::{InstallError, Result};
use crate::model::{Platform, Target};
use crate::platform::{BackupRecord, PlatformOps};
use crate::registry::MappingSpec;

pub fn resolve_target(ops: &dyn PlatformOps, target: Target) -> Result<PathBuf> {
    ops.resolve_target(target)
}

pub fn backup_root(ops: &dyn PlatformOps) -> Result<PathBuf> {
    ops.backup_root()
}

pub fn apply_mapping(
    ops: &dyn PlatformOps,
    mapping: &MappingSpec,
    bytes: &[u8],
) -> Result<BackupRecord> {
    if ops.platform() != mapping.platform {
        return Err(InstallError::Mapping(format!(
            "mapping targets {:?}, but the active platform is {:?}",
            mapping.platform,
            ops.platform()
        )));
    }
    if bytes.is_empty() {
        return Err(InstallError::Asset(format!(
            "embedded asset {:?} is empty",
            mapping.asset
        )));
    }
    let target = resolve_target(ops, mapping.target)?;
    ops.replace_file(&target, bytes)
}

pub fn apply_embedded_mapping(
    ops: &dyn PlatformOps,
    mapping: &MappingSpec,
) -> Result<BackupRecord> {
    apply_mapping(ops, mapping, asset_bytes(mapping.asset))
}

pub fn mapping_supported(ops: &dyn PlatformOps, mapping: &MappingSpec) -> bool {
    ops.platform() == mapping.platform
}

pub fn current_platform() -> Platform {
    crate::platform::platform_from_cfg()
}
