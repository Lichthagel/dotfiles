use super::{LINUX, NO_REQUIREMENTS, ZSH_PROFILE_SECTION};
use crate::model::{EmbeddedAsset, Platform, SetupKind, Target};
use crate::registry::{MappingSpec, ModuleSpec};

const ZSHRC_ASSET: EmbeddedAsset =
    EmbeddedAsset::new("zsh/zshrc", include_bytes!("../../modules/zsh/zshrc"));

const ZSH_MAPPINGS: &[MappingSpec] = &[MappingSpec {
    platform: Platform::Linux,
    asset: ZSHRC_ASSET,
    target: Target::Home(".zshrc"),
    requires: NO_REQUIREMENTS,
    behavior: ZSH_PROFILE_SECTION,
}];

pub const SPEC: ModuleSpec = ModuleSpec {
    id: "zsh",
    description: "Zsh profile",
    platforms: LINUX,
    default_selected: true,
    provides: None,
    requires: NO_REQUIREMENTS,
    packages: &[],
    mappings: ZSH_MAPPINGS,
    setup: SetupKind::None,
    secret_bundle: None,
};
