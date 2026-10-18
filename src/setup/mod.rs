pub mod http;
pub mod jsonc;
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

pub struct SetupProbeContext<'a> {
    pub platform: &'a dyn PlatformOps,
    pub runner: &'a dyn CommandRunner,
}

pub fn secret_mode(kind: SetupKind) -> Option<SecretMode> {
    match kind {
        SetupKind::Atuin => Some(SecretMode::Required),
        SetupKind::OpenCode => Some(SecretMode::Optional),
        SetupKind::None | SetupKind::Homebrew | SetupKind::Scoop => None,
    }
}

pub fn secret_request(
    kind: SetupKind,
    probe: &SetupProbeContext<'_>,
) -> Result<Option<SecretMode>> {
    match kind {
        SetupKind::Atuin => crate::modules::atuin::secret_request(probe),
        SetupKind::OpenCode => crate::modules::opencode::secret_request(probe),
        SetupKind::None | SetupKind::Homebrew | SetupKind::Scoop => Ok(None),
    }
}

pub fn run_setup(kind: SetupKind, context: &SetupContext<'_>) -> Result<()> {
    match kind {
        SetupKind::None => Ok(()),
        SetupKind::Homebrew => crate::modules::brew::install_homebrew(context),
        SetupKind::Scoop => crate::modules::scoop::install_scoop(context),
        SetupKind::Atuin => crate::modules::atuin::setup_atuin(context),
        SetupKind::OpenCode => crate::modules::opencode::configure_opencode(context),
    }
}
