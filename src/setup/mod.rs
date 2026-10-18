pub mod http;
pub mod jsonc;
pub mod providers;

use crate::error::Result;
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

pub type SetupHandler = for<'a> fn(&SetupContext<'a>) -> Result<()>;
pub type SecretRequestHandler = for<'a> fn(&SetupProbeContext<'a>) -> Result<Option<SecretMode>>;
