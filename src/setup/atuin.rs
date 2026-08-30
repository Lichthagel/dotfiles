use crate::error::{InstallError, Result};
use crate::packages::CommandRunner;
use crate::secrets::{required_values, SecretValues};
use zeroize::Zeroizing;

use super::SetupContext;

const REQUIRED_ATUIN_SECRETS: [&str; 3] = ["ATUIN_USERNAME", "ATUIN_PASSWORD", "ATUIN_KEY"];

pub fn atuin_is_logged_in(runner: &dyn CommandRunner) -> Result<bool> {
    let status = runner
        .run("atuin", &["status".into()], &[], None)
        .map_err(|error| InstallError::Setup(format!("could not check Atuin status: {error}")))?;
    Ok(status.status == 0)
}

pub fn setup_atuin(context: &SetupContext<'_>) -> Result<()> {
    if atuin_is_logged_in(context.runner)? {
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
    let mut stdin = Zeroizing::new(Vec::with_capacity(password.len() + key.len() + 2));
    stdin.extend_from_slice(password.as_bytes());
    stdin.push(b'\n');
    stdin.extend_from_slice(key.as_bytes());
    stdin.push(b'\n');
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
