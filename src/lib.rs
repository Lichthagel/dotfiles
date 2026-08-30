pub mod assets;
pub mod cli;
pub mod engine;
pub mod error;
pub mod mappings;
pub mod model;
pub mod packages;
pub mod platform;
pub mod registry;
pub mod secrets;
pub mod setup;

use std::ffi::OsString;
use std::io::Write;

use clap::error::ErrorKind;
use clap::Parser;

use cli::{
    print_module_list, select_modules, supported_modules, Cli, CrosstermTerminal, InstallRequest,
};
use engine::{install, InstallContext};
use error::{InstallError, Result};
use packages::SystemCommandRunner;
use platform::{default_platform_ops, platform_from_cfg};
use setup::http::ReqwestHttpClient;

pub fn run<I, T>(args: I, io: &mut dyn Write) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            write!(io, "{error}").map_err(InstallError::Io)?;
            return Ok(());
        }
        Err(error) => return Err(InstallError::Usage(error.to_string())),
    };

    let platform = platform_from_cfg();
    let explicit_request = cli.explicit_request(platform)?;
    if cli.list {
        return print_module_list(platform, io);
    }

    let mut terminal = CrosstermTerminal::new(io);
    let modules = match explicit_request {
        Some(request) => request.modules,
        None => {
            let modules = supported_modules(platform);
            select_modules(&modules, &mut terminal)?
        }
    };
    if modules.is_empty() {
        writeln!(io, "No applications selected.").map_err(InstallError::Io)?;
        return Ok(());
    }

    let request = InstallRequest {
        modules,
        yes: cli.yes,
        list: false,
        platform,
    };
    let platform_ops = default_platform_ops()?;
    let runner = SystemCommandRunner;
    let http = ReqwestHttpClient::new()?;
    install(
        request,
        InstallContext {
            platform: platform_ops.as_ref(),
            runner: &runner,
            http: &http,
            terminal: &mut terminal,
        },
    )
}
