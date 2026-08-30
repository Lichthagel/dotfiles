pub mod atuin;
pub mod http;
pub mod jsonc;
pub mod opencode;
pub mod providers;

use crate::error::Result;
use crate::model::SetupKind;
use crate::packages::CommandRunner;
use crate::platform::PlatformOps;
use crate::secrets::{SecretMode, SecretValues};

pub use http::HttpClient;

pub struct SetupContext<'a> {
    pub platform: &'a dyn PlatformOps,
    pub runner: &'a dyn CommandRunner,
    pub http: &'a dyn HttpClient,
    pub secrets: Option<&'a SecretValues>,
}

pub fn secret_mode(kind: SetupKind) -> Option<SecretMode> {
    match kind {
        SetupKind::Atuin => Some(SecretMode::Required),
        SetupKind::OpenCode => Some(SecretMode::Optional),
        SetupKind::None | SetupKind::Homebrew | SetupKind::Scoop => None,
    }
}

pub fn run_setup(kind: SetupKind, context: &SetupContext<'_>) -> Result<()> {
    match kind {
        SetupKind::None => Ok(()),
        SetupKind::Homebrew => providers::install_homebrew(context),
        SetupKind::Scoop => providers::install_scoop(context),
        SetupKind::Atuin => atuin::setup_atuin(context),
        SetupKind::OpenCode => opencode::configure_opencode(context),
    }
}
