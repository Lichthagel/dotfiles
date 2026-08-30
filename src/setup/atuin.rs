use crate::error::{InstallError, Result};
use crate::packages::CommandRunner;
use crate::secrets::{required_values, SecretValues};

use super::SetupContext;

const REQUIRED_ATUIN_SECRETS: [&str; 3] = ["ATUIN_USERNAME", "ATUIN_PASSWORD", "ATUIN_KEY"];

pub fn setup_atuin(context: &SetupContext<'_>) -> Result<()> {
    let status = context
        .runner
        .run("atuin", &["status".into()], &[], None)
        .map_err(|error| InstallError::Setup(format!("could not check Atuin status: {error}")))?;
    if status.status == 0 {
        return Ok(());
    }
    let secrets = context
        .secrets
        .ok_or_else(|| InstallError::Secret("Atuin secrets are required for login".into()))?;
    login_atuin(context.runner, secrets, secrets.get("ATUIN_SYNC_ADDRESS"))
}

pub fn login_atuin(
    runner: &dyn CommandRunner,
    secrets: &SecretValues,
    sync_address: Option<&str>,
) -> Result<()> {
    required_values(secrets, &REQUIRED_ATUIN_SECRETS)?;
    let username = secrets
        .get("ATUIN_USERNAME")
        .expect("required Atuin username was validated");
    let password = secrets
        .get("ATUIN_PASSWORD")
        .expect("required Atuin password was validated");
    let key = secrets
        .get("ATUIN_KEY")
        .expect("required Atuin key was validated");

    let mut environment = Vec::new();
    if let Some(address) = sync_address.filter(|address| !address.is_empty()) {
        environment.push(("ATUIN_SYNC_ADDRESS".into(), address.to_owned()));
    }
    let stdin = format!("{password}\n{key}\n").into_bytes();
    let output = runner.run(
        "atuin",
        &["login".into(), "-u".into(), username.to_owned()],
        &environment,
        Some(&stdin),
    )?;
    if output.status != 0 {
        return Err(InstallError::Setup("Atuin login failed".into()));
    }
    Ok(())
}
