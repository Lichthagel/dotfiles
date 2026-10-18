use super::{
    CROSS_PLATFORM, NO_REQUIREMENTS, REPLACE_MAPPING, REQUIRES_BASH, REQUIRES_POWERSHELL,
    REQUIRES_ZSH,
};
use crate::model::{EmbeddedAsset, PackageManager, Platform, Target};
use crate::registry::{MappingSpec, ModuleSpec, PackageSpec};

const OH_MY_POSH_PACKAGES: &[PackageSpec] = &[
    PackageSpec {
        logical: "oh-my-posh",
        manager: PackageManager::Apt,
        name: "oh-my-posh",
    },
    PackageSpec {
        logical: "oh-my-posh",
        manager: PackageManager::Dnf,
        name: "oh-my-posh",
    },
    PackageSpec {
        logical: "oh-my-posh",
        manager: PackageManager::Pacman,
        name: "oh-my-posh",
    },
    PackageSpec {
        logical: "oh-my-posh",
        manager: PackageManager::Brew,
        name: "oh-my-posh",
    },
    PackageSpec {
        logical: "oh-my-posh",
        manager: PackageManager::Mise,
        name: "oh-my-posh",
    },
    PackageSpec {
        logical: "oh-my-posh",
        manager: PackageManager::Winget,
        name: "JanDeDobbeleer.OhMyPosh",
    },
    PackageSpec {
        logical: "oh-my-posh",
        manager: PackageManager::Scoop,
        name: "oh-my-posh",
    },
];

const OH_MY_POSH_CONFIG_ASSET: EmbeddedAsset = EmbeddedAsset::new(
    "oh-my-posh/oh-my-posh.config.json",
    include_bytes!("../../modules/oh-my-posh/oh-my-posh.config.json"),
);
const OH_MY_POSH_BASH_ASSET: EmbeddedAsset = EmbeddedAsset::new(
    "oh-my-posh/bash/10-oh-my-posh.bash",
    include_bytes!("../../modules/oh-my-posh/bash/10-oh-my-posh.bash"),
);
const OH_MY_POSH_ZSH_ASSET: EmbeddedAsset = EmbeddedAsset::new(
    "oh-my-posh/zsh/10-oh-my-posh.zsh",
    include_bytes!("../../modules/oh-my-posh/zsh/10-oh-my-posh.zsh"),
);
const OH_MY_POSH_POWERSHELL_ASSET: EmbeddedAsset = EmbeddedAsset::new(
    "oh-my-posh/powershell/10-oh-my-posh.ps1",
    include_bytes!("../../modules/oh-my-posh/powershell/10-oh-my-posh.ps1"),
);

const OH_MY_POSH_MAPPINGS: &[MappingSpec] = &[
    MappingSpec {
        platform: Platform::Linux,
        asset: OH_MY_POSH_CONFIG_ASSET,
        target: Target::Home(".config/oh-my-posh/config.json"),
        requires: NO_REQUIREMENTS,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Linux,
        asset: OH_MY_POSH_BASH_ASSET,
        target: Target::Home(".config/bashrc.d/10-oh-my-posh.bash"),
        requires: REQUIRES_BASH,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Linux,
        asset: OH_MY_POSH_ZSH_ASSET,
        target: Target::Home(".config/zshrc.d/10-oh-my-posh.zsh"),
        requires: REQUIRES_ZSH,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Windows,
        asset: OH_MY_POSH_CONFIG_ASSET,
        target: Target::ProfileSibling("oh-my-posh.config.json"),
        requires: NO_REQUIREMENTS,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Windows,
        asset: OH_MY_POSH_POWERSHELL_ASSET,
        target: Target::ProfileSibling("Profile.d/10-oh-my-posh.ps1"),
        requires: REQUIRES_POWERSHELL,
        behavior: REPLACE_MAPPING,
    },
];

pub const SPEC: ModuleSpec = ModuleSpec {
    id: "oh-my-posh",
    description: "Oh My Posh prompt",
    platforms: CROSS_PLATFORM,
    default_selected: false,
    provides: None,
    requires: NO_REQUIREMENTS,
    packages: OH_MY_POSH_PACKAGES,
    mappings: OH_MY_POSH_MAPPINGS,
    setup: None,
    secret_request: None,
    secret_bundle: None,
};
