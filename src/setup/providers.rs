use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{InstallError, Result};
use crate::model::Platform;

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
    if !context.runner.available("git") {
        return Err(InstallError::Setup(
            "Homebrew bootstrap requires git".into(),
        ));
    }

    let prefix = env::var_os("DOTFILES_HOMEBREW_PREFIX")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/linuxbrew/.linuxbrew"));
    clone_repository(context, HOMEBREW_REPOSITORY, &prefix)?;
    fs::create_dir_all(prefix.join("bin"))?;
    fs::create_dir_all(prefix.join("sbin"))?;
    let brew = prefix.join("bin").join("brew");
    if !brew.is_file() {
        return Err(InstallError::Setup(format!(
            "Homebrew repository was cloned but brew is missing: {}",
            brew.display()
        )));
    }
    prepend_path(&[prefix.join("bin"), prefix.join("sbin")])?;
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
    if !context.runner.available("git") {
        return Err(InstallError::Setup("Scoop bootstrap requires git".into()));
    }

    let root = env::var_os("DOTFILES_SCOOP_ROOT")
        .or_else(|| env::var_os("SCOOP"))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            context
                .platform
                .home_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("scoop")
        });
    let current = root.join("apps").join("scoop").join("current");
    clone_repository(context, SCOOP_REPOSITORY, &current)?;
    for directory in [
        root.join("shims"),
        root.join("apps"),
        root.join("buckets"),
        root.join("cache"),
        root.join("persist"),
    ] {
        fs::create_dir_all(directory)?;
    }
    let scoop_script = current.join("bin").join("scoop.ps1");
    if !scoop_script.is_file() {
        return Err(InstallError::Setup(format!(
            "Scoop repository was cloned but scoop.ps1 is missing: {}",
            scoop_script.display()
        )));
    }
    prepend_path(&[root.join("shims")])?;
    Ok(())
}

fn clone_repository(
    context: &SetupContext<'_>,
    repository: &str,
    destination: &Path,
) -> Result<()> {
    if destination.exists() {
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let args = vec![
        "clone".into(),
        "--depth".into(),
        "1".into(),
        repository.into(),
        destination.to_string_lossy().into_owned(),
    ];
    let output = context.runner.run("git", &args, &[], None)?;
    if output.status != 0 {
        return Err(InstallError::Setup(format!(
            "git clone failed for {repository}"
        )));
    }
    Ok(())
}

fn prepend_path(entries: &[PathBuf]) -> Result<()> {
    let mut paths = entries.to_vec();
    if let Some(existing) = env::var_os("PATH") {
        paths.extend(env::split_paths(&existing));
    }
    let path = env::join_paths(paths)
        .map_err(|error| InstallError::Setup(format!("could not update PATH: {error}")))?;
    env::set_var("PATH", path);
    Ok(())
}
