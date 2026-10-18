use super::{BASH_PROFILE_SECTION, LINUX, NO_REQUIREMENTS};
use crate::model::{EmbeddedAsset, Platform, Target};
use crate::registry::{MappingSpec, ModuleSpec};

const BASHRC_ASSET: EmbeddedAsset =
    EmbeddedAsset::new("bash/bashrc", include_bytes!("../../modules/bash/bashrc"));

const BASH_MAPPINGS: &[MappingSpec] = &[MappingSpec {
    platform: Platform::Linux,
    asset: BASHRC_ASSET,
    target: Target::Home(".bashrc"),
    requires: NO_REQUIREMENTS,
    behavior: BASH_PROFILE_SECTION,
}];

pub const SPEC: ModuleSpec = ModuleSpec {
    id: "bash",
    description: "Bash profile",
    platforms: LINUX,
    default_selected: true,
    provides: None,
    requires: NO_REQUIREMENTS,
    packages: &[],
    mappings: BASH_MAPPINGS,
    setup: None,
    secret_request: None,
    secret_bundle: None,
};
