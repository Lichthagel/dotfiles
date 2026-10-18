use std::fs;
use std::io::ErrorKind;
use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::error::{InstallError, Result};
use crate::model::{Platform, Target};
use crate::platform::{FileApplyResult, PlatformOps, TimestampProvider};
use crate::registry::{MappingBehavior, MappingSpec, ModuleSpec};

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
) -> Result<FileApplyResult> {
    if ops.platform() != mapping.platform {
        return Err(InstallError::Mapping(format!(
            "mapping targets {:?}, but the active platform is {:?}",
            mapping.platform,
            ops.platform()
        )));
    }
    if bytes.is_empty() {
        return Err(InstallError::Asset(format!(
            "embedded asset {} is empty",
            mapping.asset.name
        )));
    }
    let target = resolve_target(ops, mapping.target)?;
    match mapping.behavior {
        MappingBehavior::Replace => ops.ensure_file_with_timestamp(&target, bytes, timestamp),
        MappingBehavior::ManagedSection { start, end } => {
            let merged = merge_managed_section(&target, bytes, start, end)?;
            ops.ensure_file_with_timestamp(&target, &merged, timestamp)
        }
    }
}

fn merge_managed_section(
    target: &Path,
    template: &[u8],
    start_marker: &str,
    end_marker: &str,
) -> Result<Vec<u8>> {
    let managed = section_range(template, start_marker, end_marker)?.ok_or_else(|| {
        InstallError::Asset(format!(
            "managed-section asset for {} has no boundary comments",
            target.display()
        ))
    })?;
    if managed.start != 0 || managed.end != template.len() {
        return Err(InstallError::Asset(format!(
            "managed-section asset for {} must be bounded by its comments",
            target.display()
        )));
    }

    let metadata = match fs::symlink_metadata(target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(template.to_vec()),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(InstallError::Mapping(format!(
            "managed-section target must be a regular file: {}",
            target.display()
        )));
    }
    let existing = fs::read(target)?;

    match section_range(&existing, start_marker, end_marker)? {
        Some(range) => {
            let mut merged = Vec::with_capacity(existing.len() - range.len() + template.len());
            merged.extend_from_slice(&existing[..range.start]);
            merged.extend_from_slice(template);
            merged.extend_from_slice(&existing[range.end..]);
            Ok(merged)
        }
        None => {
            let mut merged = Vec::with_capacity(existing.len() + template.len() + 1);
            merged.extend_from_slice(&existing);
            if !existing.is_empty() && !existing.ends_with(b"\n") {
                merged.push(b'\n');
            }
            merged.extend_from_slice(template);
            Ok(merged)
        }
    }
}

fn section_range(
    bytes: &[u8],
    start_marker: &str,
    end_marker: &str,
) -> Result<Option<Range<usize>>> {
    let starts = marker_lines(bytes, start_marker);
    let ends = marker_lines(bytes, end_marker);
    if starts.is_empty() && ends.is_empty() {
        return Ok(None);
    }
    if starts.len() != 1 || ends.len() != 1 || starts[0].start >= ends[0].start {
        return Err(InstallError::Mapping(format!(
            "invalid managed-section boundary comments: {start_marker} / {end_marker}"
        )));
    }
    Ok(Some(starts[0].start..ends[0].end))
}

fn marker_lines(bytes: &[u8], marker: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut line_start = 0;
    while line_start < bytes.len() {
        let line_end = bytes[line_start..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(bytes.len(), |offset| line_start + offset + 1);
        let mut content_end = line_end;
        if content_end > line_start && bytes[content_end - 1] == b'\n' {
            content_end -= 1;
        }
        if content_end > line_start && bytes[content_end - 1] == b'\r' {
            content_end -= 1;
        }
        if &bytes[line_start..content_end] == marker.as_bytes() {
            ranges.push(line_start..line_end);
        }
        line_start = line_end;
    }
    ranges
}

pub fn apply_embedded_mapping(
    ops: &dyn PlatformOps,
    mapping: &MappingSpec,
    timestamp: &dyn TimestampProvider,
) -> Result<FileApplyResult> {
    apply_mapping(ops, mapping, mapping.asset.bytes, timestamp)
}

pub fn apply_module_mappings(
    ops: &dyn PlatformOps,
    module: &ModuleSpec,
    available_modules: &[String],
    timestamp: &dyn TimestampProvider,
) -> Result<Vec<FileApplyResult>> {
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
