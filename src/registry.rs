use std::fmt;

use crate::model::{EmbeddedAsset, EmbeddedSecretBundle, PackageManager, Platform, Target};
use crate::setup::{SecretRequestHandler, SetupHandler};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageSpec {
    pub logical: &'static str,
    pub manager: PackageManager,
    pub name: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingBehavior {
    Replace,
    ManagedSection {
        start: &'static str,
        end: &'static str,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MappingSpec {
    pub platform: Platform,
    pub asset: EmbeddedAsset,
    pub target: Target,
    pub requires: &'static [&'static str],
    pub behavior: MappingBehavior,
}

#[derive(Clone, Copy)]
pub struct ModuleSpec {
    pub id: &'static str,
    pub description: &'static str,
    pub platforms: &'static [Platform],
    pub default_selected: bool,
    pub provides: Option<PackageManager>,
    pub requires: &'static [&'static str],
    pub packages: &'static [PackageSpec],
    pub mappings: &'static [MappingSpec],
    pub setup: Option<SetupHandler>,
    pub secret_request: Option<SecretRequestHandler>,
    pub secret_bundle: Option<EmbeddedSecretBundle>,
}

impl fmt::Debug for ModuleSpec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModuleSpec")
            .field("id", &self.id)
            .field("description", &self.description)
            .field("platforms", &self.platforms)
            .field("default_selected", &self.default_selected)
            .field("provides", &self.provides)
            .field("requires", &self.requires)
            .field("packages", &self.packages)
            .field("mappings", &self.mappings)
            .field("has_setup", &self.setup.is_some())
            .field("has_secret_request", &self.secret_request.is_some())
            .field("secret_bundle", &self.secret_bundle)
            .finish()
    }
}

impl ModuleSpec {
    pub fn supports(&self, platform: Platform) -> bool {
        self.platforms.contains(&platform)
    }
}

impl PackageManager {
    pub const fn priority(platform: Platform) -> &'static [Self] {
        match platform {
            Platform::Linux => &[Self::Apt, Self::Dnf, Self::Pacman, Self::Brew, Self::Mise],
            Platform::Windows => &[Self::Winget, Self::Scoop, Self::Brew, Self::Mise],
        }
    }
}

pub static MODULES: &[ModuleSpec] = &[
    crate::modules::git::SPEC,
    crate::modules::bash::SPEC,
    crate::modules::zsh::SPEC,
    crate::modules::powershell::SPEC,
    crate::modules::mise::SPEC,
    crate::modules::brew::SPEC,
    crate::modules::scoop::SPEC,
    crate::modules::atuin::SPEC,
    crate::modules::oh_my_posh::SPEC,
    crate::modules::opencode::SPEC,
];

pub fn modules() -> &'static [ModuleSpec] {
    MODULES
}

pub fn find_module(id: &str) -> Option<&'static ModuleSpec> {
    MODULES.iter().find(|module| module.id == id)
}
