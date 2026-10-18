use super::{CROSS_PLATFORM, NO_REQUIREMENTS, REPLACE_MAPPING};
use crate::model::{EmbeddedAsset, PackageManager, Platform, Target};
use crate::registry::{MappingSpec, ModuleSpec, Package, PackageSource};

const GIT_PACKAGES: &[Package] = &[Package {
    id: "git",
    command: Some("git"),
    sources: &[
        PackageSource {
            manager: PackageManager::Apt,
            name: "git",
        },
        PackageSource {
            manager: PackageManager::Dnf,
            name: "git",
        },
        PackageSource {
            manager: PackageManager::Pacman,
            name: "git",
        },
        PackageSource {
            manager: PackageManager::Brew,
            name: "git",
        },
        PackageSource {
            manager: PackageManager::Mise,
            name: "git",
        },
        PackageSource {
            manager: PackageManager::Winget,
            name: "Git.Git",
        },
        PackageSource {
            manager: PackageManager::Scoop,
            name: "git",
        },
    ],
}];

const GIT_CONFIG_ASSET: EmbeddedAsset = EmbeddedAsset::new(
    "git/gitconfig",
    include_bytes!("../../modules/git/gitconfig"),
);

const GIT_MAPPINGS: &[MappingSpec] = &[
    MappingSpec {
        platform: Platform::Linux,
        asset: GIT_CONFIG_ASSET,
        target: Target::Home(".gitconfig"),
        requires: NO_REQUIREMENTS,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Windows,
        asset: GIT_CONFIG_ASSET,
        target: Target::Home(".gitconfig"),
        requires: NO_REQUIREMENTS,
        behavior: REPLACE_MAPPING,
    },
];

pub const SPEC: ModuleSpec = ModuleSpec {
    id: "git",
    description: "Git configuration",
    platforms: CROSS_PLATFORM,
    default_selected: true,
    provides: None,
    requires: NO_REQUIREMENTS,
    packages: GIT_PACKAGES,
    mappings: GIT_MAPPINGS,
    setup: None,
    secret_request: None,
    secret_bundle: None,
};
