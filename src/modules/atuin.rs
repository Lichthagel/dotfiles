use super::{
    CROSS_PLATFORM, NO_REQUIREMENTS, REPLACE_MAPPING, REQUIRES_BASH, REQUIRES_POWERSHELL,
    REQUIRES_ZSH,
};
use crate::error::{InstallError, Result};
use crate::model::{EmbeddedAsset, EmbeddedSecretBundle, PackageManager, Platform, Target};
use crate::packages::CommandRunner;
use crate::registry::{MappingSpec, ModuleSpec, Package, PackageSource};
use crate::secrets::{required_values, SecretMode, SecretValues};
use crate::setup::{SetupContext, SetupProbeContext};
use zeroize::Zeroizing;

const ATUIN_PACKAGES: &[Package] = &[Package {
    id: "atuin",
    command: Some("atuin"),
    sources: &[
        PackageSource {
            manager: PackageManager::Apt,
            name: "atuin",
        },
        PackageSource {
            manager: PackageManager::Dnf,
            name: "atuin",
        },
        PackageSource {
            manager: PackageManager::Pacman,
            name: "atuin",
        },
        PackageSource {
            manager: PackageManager::Brew,
            name: "atuin",
        },
        PackageSource {
            manager: PackageManager::Mise,
            name: "atuin",
        },
        PackageSource {
            manager: PackageManager::Winget,
            name: "Atuinsh.Atuin",
        },
        PackageSource {
            manager: PackageManager::Scoop,
            name: "atuin",
        },
    ],
}];

const ATUIN_SECRET_BUNDLE: EmbeddedSecretBundle =
    EmbeddedSecretBundle::new("Atuin", include_bytes!("../../secrets/atuin.env.age"));

const ATUIN_BASH_ASSET: EmbeddedAsset = EmbeddedAsset::new(
    "atuin/bash/50-atuin.bash",
    include_bytes!("../../modules/atuin/bash/50-atuin.bash"),
);
const ATUIN_ZSH_ASSET: EmbeddedAsset = EmbeddedAsset::new(
    "atuin/zsh/50-atuin.zsh",
    include_bytes!("../../modules/atuin/zsh/50-atuin.zsh"),
);
const ATUIN_POWERSHELL_ASSET: EmbeddedAsset = EmbeddedAsset::new(
    "atuin/powershell/50-atuin.ps1",
    include_bytes!("../../modules/atuin/powershell/50-atuin.ps1"),
);

const ATUIN_MAPPINGS: &[MappingSpec] = &[
    MappingSpec {
        platform: Platform::Linux,
        asset: ATUIN_BASH_ASSET,
        target: Target::Home(".config/bashrc.d/50-atuin.bash"),
        requires: REQUIRES_BASH,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Linux,
        asset: ATUIN_ZSH_ASSET,
        target: Target::Home(".config/zshrc.d/50-atuin.zsh"),
        requires: REQUIRES_ZSH,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Windows,
        asset: ATUIN_POWERSHELL_ASSET,
        target: Target::ProfileSibling("Profile.d/50-atuin.ps1"),
        requires: REQUIRES_POWERSHELL,
        behavior: REPLACE_MAPPING,
    },
];

pub const SPEC: ModuleSpec = ModuleSpec {
    id: "atuin",
    description: "Atuin shell history",
    platforms: CROSS_PLATFORM,
    default_selected: false,
    provides: None,
    requires: NO_REQUIREMENTS,
    packages: ATUIN_PACKAGES,
    mappings: ATUIN_MAPPINGS,
    setup: Some(setup_atuin),
    secret_request: Some(secret_request),
    secret_bundle: Some(ATUIN_SECRET_BUNDLE),
};

const REQUIRED_ATUIN_SECRETS: [&str; 3] = ["ATUIN_USERNAME", "ATUIN_PASSWORD", "ATUIN_KEY"];

pub fn atuin_is_logged_in(runner: &dyn CommandRunner) -> Result<bool> {
    let status = runner
        .run("atuin", &["status".into()], &[], None)
        .map_err(|error| InstallError::Setup(format!("could not check Atuin status: {error}")))?;
    Ok(status.status == 0)
}

pub fn secret_request(probe: &SetupProbeContext<'_>) -> Result<Option<SecretMode>> {
    if atuin_is_logged_in(probe.runner)? {
        Ok(None)
    } else {
        Ok(Some(SecretMode::Required))
    }
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

    let mut environment = vec![("ATUIN_ENCRYPTION_KEY".into(), key.to_owned())];
    if let Some(address) = sync_address.filter(|address| !address.is_empty()) {
        environment.push(("ATUIN_SYNC_ADDRESS".into(), address.to_owned()));
    }
    let mut stdin = Zeroizing::new(Vec::with_capacity(password.len() + 1));
    stdin.extend_from_slice(password.as_bytes());
    stdin.push(b'\n');
    let output = runner.run(
        "atuin",
        &[
            "login".into(),
            "-u".into(),
            username.to_owned(),
            "--password".into(),
            "-".into(),
        ],
        &environment,
        Some(&stdin),
    )?;
    if output.status != 0 {
        return Err(InstallError::Setup("Atuin login failed".into()));
    }
    Ok(())
}
