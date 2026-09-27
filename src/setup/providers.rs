use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{InstallError, Result};
use crate::model::{PackageManager, Platform};
use crate::packages::{ManagerAdapter, PackageManagerAdapter};
use crate::platform::current_scoop_root;

use super::SetupContext;

const HOMEBREW_REPOSITORY: &str = "https://github.com/Homebrew/brew";
const SCOOP_REPOSITORY: &str = "https://github.com/ScoopInstaller/Scoop";

pub fn install_homebrew(context: &SetupContext<'_>) -> Result<()> {
    if context.platform.platform() != Platform::Linux {
        return Err(InstallError::Setup(
            "Homebrew provider is supported only on Linux".into(),
        ));
    }
    if context.runner.available("brew") {
        return Ok(());
    }
    ensure_git(context)?;

    let prefix = env::var_os("DOTFILES_HOMEBREW_PREFIX")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/linuxbrew/.linuxbrew"));
    clone_repository(context, "Homebrew", HOMEBREW_REPOSITORY, &prefix)?;
    create_directory("Homebrew", &prefix.join("bin"))?;
    create_directory("Homebrew", &prefix.join("sbin"))?;
    let brew = prefix.join("bin").join("brew");
    if !brew.is_file() {
        return Err(InstallError::Setup(format!(
            "Homebrew bootstrap did not produce brew: {}",
            brew.display()
        )));
    }
    prepend_path("Homebrew", &[prefix.join("bin"), prefix.join("sbin")])?;
    verify_manager(context, "Homebrew", "brew", &["--version".into()])?;
    Ok(())
}

pub fn install_scoop(context: &SetupContext<'_>) -> Result<()> {
    if context.platform.platform() != Platform::Windows {
        return Err(InstallError::Setup(
            "Scoop provider is supported only on Windows".into(),
        ));
    }
    if context.runner.available("scoop") {
        return Ok(());
    }
    ensure_git(context)?;

    let home = context
        .platform
        .home_dir()
        .map_err(|error| provider_error("Scoop", "resolve home directory", error))?;
    let root = current_scoop_root(&home);
    let current = root.join("apps").join("scoop").join("current");
    clone_repository(context, "Scoop", SCOOP_REPOSITORY, &current)?;
    for directory in [
        root.join("shims"),
        root.join("apps"),
        root.join("buckets"),
        root.join("cache"),
        root.join("persist"),
    ] {
        create_directory("Scoop", &directory)?;
    }
    let scoop_script = current.join("bin").join("scoop.ps1");
    if !scoop_script.is_file() {
        return Err(InstallError::Setup(format!(
            "Scoop bootstrap did not produce scoop.ps1: {}",
            scoop_script.display()
        )));
    }
    prepend_path("Scoop", &[root.join("shims")])?;
    if !context.runner.available("pwsh") {
        return Err(InstallError::Setup(
            "Scoop bootstrap requires pwsh to run scoop.ps1".into(),
        ));
    }
    verify_manager(
        context,
        "Scoop",
        "pwsh",
        &[
            "-NoProfile".into(),
            "-File".into(),
            scoop_script.to_string_lossy().into_owned(),
            "--version".into(),
        ],
    )?;
    Ok(())
}

fn clone_repository(
    context: &SetupContext<'_>,
    provider: &str,
    repository: &str,
    destination: &Path,
) -> Result<()> {
    if destination.exists() {
        return Ok(());
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
    Ok(())
}

fn ensure_git(context: &SetupContext<'_>) -> Result<()> {
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

fn create_directory(provider: &str, path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|error| provider_error(provider, "create directory", error))
}

fn prepend_path(provider: &str, entries: &[PathBuf]) -> Result<()> {
    let mut paths = entries.to_vec();
    if let Some(existing) = env::var_os("PATH") {
        paths.extend(env::split_paths(&existing));
    }
    let path =
        env::join_paths(paths).map_err(|error| provider_error(provider, "update PATH", error))?;
    env::set_var("PATH", path);
    Ok(())
}

fn verify_manager(
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

fn provider_error(provider: &str, operation: &str, error: impl std::fmt::Display) -> InstallError {
    InstallError::Setup(format!("{provider} {operation} failed: {error}"))
}
