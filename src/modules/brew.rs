use super::{LINUX, NO_REQUIREMENTS, REPLACE_MAPPING, REQUIRES_BASH, REQUIRES_ZSH};
use crate::error::{InstallError, Result};
use crate::model::{EmbeddedAsset, PackageManager, Platform, Target};
use crate::registry::{MappingSpec, ModuleSpec};
use crate::setup::providers::{
    clone_repository, create_directory, ensure_git, prepend_path, verify_manager,
};
use crate::setup::SetupContext;
use std::env;
use std::path::PathBuf;

const BREW_BASH_ASSET: EmbeddedAsset = EmbeddedAsset::new(
    "brew/bash/05-brew.bash",
    include_bytes!("../../modules/brew/bash/05-brew.bash"),
);
const BREW_ZSH_ASSET: EmbeddedAsset = EmbeddedAsset::new(
    "brew/zsh/05-brew.zsh",
    include_bytes!("../../modules/brew/zsh/05-brew.zsh"),
);

const BREW_MAPPINGS: &[MappingSpec] = &[
    MappingSpec {
        platform: Platform::Linux,
        asset: BREW_BASH_ASSET,
        target: Target::Home(".config/bashrc.d/05-brew.bash"),
        requires: REQUIRES_BASH,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Linux,
        asset: BREW_ZSH_ASSET,
        target: Target::Home(".config/zshrc.d/05-brew.zsh"),
        requires: REQUIRES_ZSH,
        behavior: REPLACE_MAPPING,
    },
];

pub const SPEC: ModuleSpec = ModuleSpec {
    id: "brew",
    description: "Homebrew package manager",
    platforms: LINUX,
    default_selected: false,
    provides: Some(PackageManager::Brew),
    requires: NO_REQUIREMENTS,
    packages: &[],
    mappings: BREW_MAPPINGS,
    setup: Some(install_homebrew),
    secret_request: None,
    secret_bundle: None,
};

const HOMEBREW_REPOSITORY: &str = "https://github.com/Homebrew/brew";

pub fn install_homebrew(context: &SetupContext<'_>) -> Result<()> {
    if context.platform.platform() != Platform::Linux {
        return Err(InstallError::Setup(
            "Homebrew provider is supported only on Linux".into(),
        ));
    }
    let prefix = env::var_os("DOTFILES_HOMEBREW_PREFIX")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/linuxbrew/.linuxbrew"));
    let brew = prefix.join("bin").join("brew");
    if context.runner.available("brew") {
        return Ok(());
    }
    if !brew.is_file() {
        ensure_git(context)?;
        clone_repository(context, "Homebrew", HOMEBREW_REPOSITORY, &prefix, &brew)?;
    }
    create_directory("Homebrew", &prefix.join("bin"))?;
    create_directory("Homebrew", &prefix.join("sbin"))?;
    let changed = prepend_path("Homebrew", &[prefix.join("bin"), prefix.join("sbin")])?;
    if changed {
        context.runner.refresh().map_err(|error| {
            crate::setup::providers::provider_error("Homebrew", "refresh PATH", error)
        })?;
    }
    verify_manager(context, "Homebrew", "brew", &["--version".into()])?;
    Ok(())
}
