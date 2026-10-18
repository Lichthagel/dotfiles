use std::path::PathBuf;

use crate::assets::asset_bytes;
use crate::error::{InstallError, Result};
use crate::model::{Platform, Target};
use crate::platform::{BackupRecord, PlatformOps, TimestampProvider};
use crate::registry::{MappingSpec, ModuleSpec};

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
    timestamp: &dyn TimestampProvider,
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
    ops.replace_file_with_timestamp(&target, bytes, timestamp)
}

pub fn apply_embedded_mapping(
    ops: &dyn PlatformOps,
    mapping: &MappingSpec,
    timestamp: &dyn TimestampProvider,
) -> Result<BackupRecord> {
    apply_mapping(ops, mapping, asset_bytes(mapping.asset), timestamp)
}

pub fn apply_module_mappings(
    ops: &dyn PlatformOps,
    module: &ModuleSpec,
    available_modules: &[String],
    timestamp: &dyn TimestampProvider,
) -> Result<Vec<BackupRecord>> {
    let mut backups = Vec::new();
    for mapping in module.mappings {
        if !mapping_supported(ops, mapping)
            || mapping
                .requires
                .iter()
                .any(|required| !available_modules.iter().any(|id| id == required))
        {
            continue;
        }
        backups.push(apply_embedded_mapping(ops, mapping, timestamp)?);
    }
    Ok(backups)
}

pub fn mapping_supported(ops: &dyn PlatformOps, mapping: &MappingSpec) -> bool {
    ops.platform() == mapping.platform
}

pub fn current_platform() -> Platform {
    crate::platform::platform_from_cfg()
}
