use super::{
    CROSS_PLATFORM, NO_REQUIREMENTS, REPLACE_MAPPING, REQUIRES_BASH, REQUIRES_POWERSHELL,
    REQUIRES_ZSH,
};
use crate::model::{EmbeddedAsset, PackageManager, Platform, Target};
use crate::registry::{MappingSpec, ModuleSpec, PackageSpec};

const MISE_PACKAGES: &[PackageSpec] = &[
    PackageSpec {
        logical: "mise",
        manager: PackageManager::Apt,
        name: "mise",
    },
    PackageSpec {
        logical: "mise",
        manager: PackageManager::Dnf,
        name: "mise",
    },
    PackageSpec {
        logical: "mise",
        manager: PackageManager::Pacman,
        name: "mise",
    },
    PackageSpec {
        logical: "mise",
        manager: PackageManager::Brew,
        name: "mise",
    },
    PackageSpec {
        logical: "mise",
        manager: PackageManager::Winget,
        name: "jdx.mise",
    },
    PackageSpec {
        logical: "mise",
        manager: PackageManager::Scoop,
        name: "mise",
    },
];

const MISE_BASH_ASSET: EmbeddedAsset = EmbeddedAsset::new(
    "mise/bash/10-mise.bash",
    include_bytes!("../../modules/mise/bash/10-mise.bash"),
);
const MISE_ZSH_ASSET: EmbeddedAsset = EmbeddedAsset::new(
    "mise/zsh/10-mise.zsh",
    include_bytes!("../../modules/mise/zsh/10-mise.zsh"),
);
const MISE_POWERSHELL_ASSET: EmbeddedAsset = EmbeddedAsset::new(
    "mise/powershell/10-mise.ps1",
    include_bytes!("../../modules/mise/powershell/10-mise.ps1"),
);

const MISE_MAPPINGS: &[MappingSpec] = &[
    MappingSpec {
        platform: Platform::Linux,
        asset: MISE_BASH_ASSET,
        target: Target::Home(".config/bashrc.d/10-mise.bash"),
        requires: REQUIRES_BASH,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Linux,
        asset: MISE_ZSH_ASSET,
        target: Target::Home(".config/zshrc.d/10-mise.zsh"),
        requires: REQUIRES_ZSH,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Windows,
        asset: MISE_POWERSHELL_ASSET,
        target: Target::ProfileSibling("Profile.d/10-mise.ps1"),
        requires: REQUIRES_POWERSHELL,
        behavior: REPLACE_MAPPING,
    },
];

pub const SPEC: ModuleSpec = ModuleSpec {
    id: "mise",
    description: "mise tool version manager",
    platforms: CROSS_PLATFORM,
    default_selected: true,
    provides: Some(PackageManager::Mise),
    requires: NO_REQUIREMENTS,
    packages: MISE_PACKAGES,
    mappings: MISE_MAPPINGS,
    setup: None,
    secret_request: None,
    secret_bundle: None,
};
