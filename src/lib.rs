pub mod assets;
pub mod cli;
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

use cli::{print_module_list, select_modules, supported_modules, Cli, CrosstermTerminal};
use error::{InstallError, Result};
use platform::platform_from_cfg;

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

    let modules = match explicit_request {
        Some(request) => request.modules,
        None => {
            let modules = supported_modules(platform);
            let mut terminal = CrosstermTerminal::new(io);
            select_modules(&modules, &mut terminal)?
        }
    };
    if modules.is_empty() {
        writeln!(io, "No applications selected.").map_err(InstallError::Io)?;
    }
    Ok(())
}
