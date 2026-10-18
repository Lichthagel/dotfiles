use super::{NO_REQUIREMENTS, REPLACE_MAPPING, REQUIRES_POWERSHELL, WINDOWS};
use crate::error::{InstallError, Result};
use crate::model::{EmbeddedAsset, PackageManager, Platform, Target};
use crate::platform::current_scoop_root;
use crate::registry::{MappingSpec, ModuleSpec};
use crate::setup::providers::{
    clone_repository, create_directory, ensure_git, prepend_path, provider_error, verify_manager,
};
use crate::setup::SetupContext;

const SCOOP_POWERSHELL_ASSET: EmbeddedAsset = EmbeddedAsset::new(
    "scoop/powershell/05-scoop.ps1",
    include_bytes!("../../modules/scoop/powershell/05-scoop.ps1"),
);

const SCOOP_MAPPINGS: &[MappingSpec] = &[MappingSpec {
    platform: Platform::Windows,
    asset: SCOOP_POWERSHELL_ASSET,
    target: Target::ProfileSibling("Profile.d/05-scoop.ps1"),
    requires: REQUIRES_POWERSHELL,
    behavior: REPLACE_MAPPING,
}];

pub const SPEC: ModuleSpec = ModuleSpec {
    id: "scoop",
    description: "Scoop package manager",
    platforms: WINDOWS,
    default_selected: false,
    provides: Some(PackageManager::Scoop),
    requires: NO_REQUIREMENTS,
    packages: &[],
    mappings: SCOOP_MAPPINGS,
    setup: Some(install_scoop),
    secret_request: None,
    secret_bundle: None,
};

const SCOOP_REPOSITORY: &str = "https://github.com/ScoopInstaller/Scoop";

pub fn install_scoop(context: &SetupContext<'_>) -> Result<()> {
    if context.platform.platform() != Platform::Windows {
        return Err(InstallError::Setup(
            "Scoop provider is supported only on Windows".into(),
        ));
    }
    let home = context
        .platform
        .home_dir()
        .map_err(|error| provider_error("Scoop", "resolve home directory", error))?;
    let root = current_scoop_root(&home);
    let current = root.join("apps").join("scoop").join("current");
    let scoop_script = current.join("bin").join("scoop.ps1");
    if context.runner.available("scoop") {
        return Ok(());
    }
    if !scoop_script.is_file() {
        ensure_git(context)?;
        clone_repository(context, "Scoop", SCOOP_REPOSITORY, &current, &scoop_script)?;
    }
    for directory in [
        root.join("shims"),
        root.join("apps"),
        root.join("buckets"),
        root.join("cache"),
        root.join("persist"),
    ] {
        create_directory("Scoop", &directory)?;
    }
    let changed = prepend_path("Scoop", &[root.join("shims")])?;
    if changed {
        context
            .runner
            .refresh()
            .map_err(|error| provider_error("Scoop", "refresh PATH", error))?;
    }
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
