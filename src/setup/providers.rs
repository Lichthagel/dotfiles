use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{InstallError, Result};
use crate::model::PackageManager;
use crate::packages::{ManagerAdapter, PackageManagerAdapter};

use super::SetupContext;

pub(crate) fn clone_repository(
    context: &SetupContext<'_>,
    provider: &str,
    repository: &str,
    destination: &Path,
    expected_entrypoint: &Path,
) -> Result<()> {
    if destination.exists() {
        if expected_entrypoint.is_file() {
            return Ok(());
        }
        return Err(InstallError::Setup(format!(
            "{provider} installation exists but is missing its expected entry point: {}",
            expected_entrypoint.display()
        )));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| provider_error(provider, "create clone parent", error))?;
    }
    let args = vec![
        "clone".into(),
        "--depth".into(),
        "1".into(),
        repository.into(),
        destination.to_string_lossy().into_owned(),
    ];
    let output = context
        .runner
        .run("git", &args, &[], None)
        .map_err(|error| provider_error(provider, "run git clone", error))?;
    if output.status != 0 {
        return Err(InstallError::Setup(format!(
            "{provider} git clone failed for {repository}"
        )));
    }
    if !expected_entrypoint.is_file() {
        return Err(InstallError::Setup(format!(
            "{provider} bootstrap did not produce its expected entry point: {}",
            expected_entrypoint.display()
        )));
    }
    Ok(())
}

pub(crate) fn ensure_git(context: &SetupContext<'_>) -> Result<()> {
    if context.runner.available("git") {
        return Ok(());
    }

    let platform = context.platform.platform();
    let manager = PackageManager::priority(platform)
        .iter()
        .copied()
        .find(|manager| PackageManagerAdapter::new(platform, *manager).available(context.runner))
        .ok_or_else(|| {
            InstallError::Setup(format!(
                "provider bootstrap requires git and no supported package manager is available; supported managers: {}",
                PackageManager::priority(platform)
                    .iter()
                    .map(|manager| manager.label())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;
    let adapter = PackageManagerAdapter::new(platform, manager);
    if !adapter.installed(context.runner, "git")? {
        adapter.install(context.runner, "git").map_err(|error| {
            InstallError::Setup(format!("install git for provider bootstrap: {error}"))
        })?;
        context.runner.refresh().map_err(|error| {
            InstallError::Setup(format!("refresh PATH after installing git: {error}"))
        })?;
    }
    if !context.runner.available("git") {
        return Err(InstallError::Setup(
            "git installation did not make git discoverable".into(),
        ));
    }
    Ok(())
}

pub(crate) fn create_directory(provider: &str, path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|error| provider_error(provider, "create directory", error))
}

pub(crate) fn prepend_path(provider: &str, entries: &[PathBuf]) -> Result<bool> {
    let existing = env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut prefix: Vec<PathBuf> = Vec::new();
    for candidate in entries {
        if !existing
            .iter()
            .chain(prefix.iter())
            .any(|current| same_path(current, candidate))
        {
            prefix.push(candidate.clone());
        }
    }
    if prefix.is_empty() {
        return Ok(false);
    }
    prefix.extend(existing);
    let path =
        env::join_paths(prefix).map_err(|error| provider_error(provider, "update PATH", error))?;
    env::set_var("PATH", path);
    Ok(true)
}

#[cfg(windows)]
fn same_path(left: &Path, right: &Path) -> bool {
    left.as_os_str()
        .to_string_lossy()
        .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
}

#[cfg(not(windows))]
fn same_path(left: &Path, right: &Path) -> bool {
    left == right
}

pub(crate) fn verify_manager(
    context: &SetupContext<'_>,
    provider: &str,
    program: &str,
    args: &[String],
) -> Result<()> {
    if !context.runner.available(program) {
        return Err(InstallError::Setup(format!(
            "{provider} bootstrap did not make {program} discoverable"
        )));
    }
    let output = context
        .runner
        .run(program, args, &[], None)
        .map_err(|error| provider_error(provider, "verify manager", error))?;
    if output.status != 0 {
        return Err(InstallError::Setup(format!(
            "{provider} manager verification failed for {program}"
        )));
    }
    Ok(())
}

pub(crate) fn provider_error(
    provider: &str,
    operation: &str,
    error: impl std::fmt::Display,
) -> InstallError {
    InstallError::Setup(format!("{provider} {operation} failed: {error}"))
}
