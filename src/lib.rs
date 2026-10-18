pub mod cli;
pub mod engine;
pub mod error;
pub mod mappings;
pub mod model;
pub mod modules;
pub mod packages;
pub mod platform;
pub mod registry;
pub mod secrets;
pub mod setup;
pub mod state;

use std::ffi::OsString;
use std::io::Write;

use clap::error::ErrorKind;
use clap::Parser;

use cli::{
    print_module_list, select_modules_with_state, supported_modules, Cli, CrosstermTerminal,
    InstallRequest, TerminalIo,
};
use engine::{install, InstallContext};
use error::{InstallError, Result};
use packages::SystemCommandRunner;
use platform::{default_platform_ops, platform_from_cfg, SystemTimestamp};
use setup::http::CurlHttpClient;

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

    let interactive_modules = explicit_request.is_none();
    let explicit_modules = explicit_request.map(|request| request.modules);
    let mut terminal = CrosstermTerminal::new(io);
    let platform_ops = default_platform_ops()?;
    let runner = SystemCommandRunner;
    let http = CurlHttpClient::new();
    let timestamp = SystemTimestamp;

    loop {
        let modules = match &explicit_modules {
            Some(modules) => modules.clone(),
            None => {
                let available = supported_modules(platform);
                let state = if terminal.is_interactive() {
                    state::ModuleState::load(&platform_ops.state_path()?)?
                } else {
                    state::ModuleState::default()
                };
                select_modules_with_state(&available, &state, &mut terminal)?
            }
        };
        if modules.is_empty() {
            writeln!(io, "No applications selected.").map_err(InstallError::Io)?;
            return Ok(());
        }

        let request = if interactive_modules {
            InstallRequest::interactive(modules, cli.yes, platform)
        } else {
            InstallRequest::explicit(modules, cli.yes, platform)
        };
        let outcome = install(
            request,
            InstallContext {
                platform: platform_ops.as_ref(),
                runner: &runner,
                http: &http,
                terminal: &mut terminal,
                timestamp: &timestamp,
            },
        )?;
        match outcome {
            engine::InstallOutcome::Completed => return Ok(()),
            engine::InstallOutcome::BackToModuleSelection if interactive_modules => continue,
            engine::InstallOutcome::BackToModuleSelection => {
                return Err(InstallError::Usage(
                    "module selection is fixed by --modules".into(),
                ))
            }
        }
    }
}
