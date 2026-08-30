pub mod atuin;
pub mod http;
pub mod providers;

use crate::error::{InstallError, Result};
use crate::model::SetupKind;
use crate::packages::CommandRunner;
use crate::platform::PlatformOps;
use crate::secrets::SecretValues;

pub use http::HttpClient;

pub struct SetupContext<'a> {
    pub platform: &'a dyn PlatformOps,
    pub runner: &'a dyn CommandRunner,
    pub http: &'a dyn HttpClient,
    pub secrets: Option<&'a SecretValues>,
}

pub fn run_setup(kind: SetupKind, context: &SetupContext<'_>) -> Result<()> {
    match kind {
        SetupKind::None => Ok(()),
        SetupKind::Homebrew => providers::install_homebrew(context),
        SetupKind::Scoop => providers::install_scoop(context),
        SetupKind::Atuin => atuin::setup_atuin(context),
        SetupKind::OpenCode => Err(InstallError::Setup(
            "OpenCode setup is not implemented yet".into(),
        )),
    }
}
