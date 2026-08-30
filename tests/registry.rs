use dotfiles::model::{AssetId, PackageManager, Platform, SecretBundle, SetupKind, Target};
use dotfiles::registry;

#[test]
fn registry_contains_only_supported_modules() {
    let ids: Vec<_> = registry::modules().iter().map(|module| module.id).collect();
    assert_eq!(
        ids,
        [
            "git",
            "bash",
            "powershell",
            "mise",
            "brew",
            "scoop",
            "atuin",
            "oh-my-posh",
            "opencode",
        ]
    );
}

#[test]
fn provider_and_secret_contracts_match_existing_modules() {
    assert_eq!(
        registry::find_module("brew").unwrap().provides,
        Some(PackageManager::Brew)
    );
    assert_eq!(
        registry::find_module("scoop").unwrap().provides,
        Some(PackageManager::Scoop)
    );
    assert_eq!(
        registry::find_module("mise").unwrap().provides,
        Some(PackageManager::Mise)
    );
    assert_eq!(
        registry::find_module("atuin").unwrap().secret_bundle,
        Some(SecretBundle::Atuin)
    );
    assert_eq!(
        registry::find_module("opencode").unwrap().secret_bundle,
        Some(SecretBundle::OpenCode)
    );
    assert_eq!(
        registry::find_module("atuin").unwrap().setup,
        SetupKind::Atuin
    );
    assert_eq!(
        registry::find_module("opencode").unwrap().setup,
        SetupKind::OpenCode
    );
}

#[test]
fn platform_filtering_excludes_shell_specific_modules() {
    assert!(registry::find_module("bash")
        .unwrap()
        .supports(Platform::Linux));
    assert!(!registry::find_module("bash")
        .unwrap()
        .supports(Platform::Windows));
    assert!(registry::find_module("powershell")
        .unwrap()
        .supports(Platform::Windows));
    assert!(!registry::find_module("powershell")
        .unwrap()
        .supports(Platform::Linux));
}

#[test]
fn defaults_and_descriptions_match_current_modules() {
    for module in ["git", "bash", "powershell", "mise"] {
        assert!(registry::find_module(module).unwrap().default_selected);
    }
    for module in ["brew", "scoop", "atuin", "oh-my-posh", "opencode"] {
        assert!(!registry::find_module(module).unwrap().default_selected);
    }
    assert_eq!(
        registry::find_module("git").unwrap().description,
        "Git configuration"
    );
    assert_eq!(
        registry::find_module("opencode").unwrap().description,
        "OpenCode AI coding agent"
    );
}

#[test]
fn package_alternatives_are_preserved_without_jq_or_age() {
    let git = registry::find_module("git").unwrap();
    for (manager, name) in [
        (PackageManager::Apt, "git"),
        (PackageManager::Dnf, "git"),
        (PackageManager::Pacman, "git"),
        (PackageManager::Brew, "git"),
        (PackageManager::Mise, "git"),
        (PackageManager::Winget, "Git.Git"),
        (PackageManager::Scoop, "git"),
    ] {
        assert!(git.packages.iter().any(|package| {
            package.manager == manager && package.name == name && package.logical == "git"
        }));
    }

    let atuin = registry::find_module("atuin").unwrap();
    for (manager, name) in [
        (PackageManager::Apt, "atuin"),
        (PackageManager::Dnf, "atuin"),
        (PackageManager::Pacman, "atuin"),
        (PackageManager::Brew, "atuin"),
        (PackageManager::Mise, "atuin"),
        (PackageManager::Winget, "Atuinsh.Atuin"),
        (PackageManager::Scoop, "atuin"),
    ] {
        assert!(atuin.packages.iter().any(|package| {
            package.manager == manager && package.name == name && package.logical == "atuin"
        }));
    }

    let oh_my_posh = registry::find_module("oh-my-posh").unwrap();
    for (manager, name) in [
        (PackageManager::Apt, "oh-my-posh"),
        (PackageManager::Dnf, "oh-my-posh"),
        (PackageManager::Pacman, "oh-my-posh"),
        (PackageManager::Brew, "oh-my-posh"),
        (PackageManager::Mise, "oh-my-posh"),
        (PackageManager::Winget, "JanDeDobbeleer.OhMyPosh"),
        (PackageManager::Scoop, "oh-my-posh"),
    ] {
        assert!(oh_my_posh.packages.iter().any(|package| {
            package.manager == manager && package.name == name && package.logical == "oh-my-posh"
        }));
    }

    let mise = registry::find_module("mise").unwrap();
    assert!(mise
        .packages
        .iter()
        .any(|package| package.manager == PackageManager::Winget && package.name == "jdx.mise"));

    let opencode = registry::find_module("opencode").unwrap();
    assert!(opencode.packages.iter().any(|package| {
        package.manager == PackageManager::Brew && package.name == "anomalyco/tap/opencode"
    }));
    assert!(opencode.packages.iter().any(|package| {
        package.manager == PackageManager::Mise && package.name == "github:anomalyco/opencode"
    }));
    assert!(opencode
        .packages
        .iter()
        .any(|package| package.manager == PackageManager::Scoop && package.name == "opencode"));
    assert!(!registry::modules()
        .iter()
        .flat_map(|module| module.packages)
        .any(|package| {
            package.name == "jq" || package.name == "FiloSottile.age" || package.logical == "age"
        }));
}

#[test]
fn mappings_use_typed_embedded_assets_and_require_shell_modules() {
    let git = registry::find_module("git").unwrap();
    assert!(git.mappings.iter().any(|mapping| {
        mapping.platform == Platform::Linux
            && mapping.asset == AssetId::GitConfig
            && mapping.target == Target::Home(".gitconfig")
            && mapping.requires.is_empty()
    }));
    assert!(git.mappings.iter().any(|mapping| {
        mapping.platform == Platform::Windows
            && mapping.asset == AssetId::GitConfig
            && mapping.target == Target::Home(".gitconfig")
            && mapping.requires.is_empty()
    }));

    let bash = registry::find_module("bash").unwrap();
    assert!(bash.mappings.iter().any(|mapping| {
        mapping.platform == Platform::Linux
            && mapping.asset == AssetId::Bashrc
            && mapping.target == Target::Home(".bashrc")
    }));

    let powershell = registry::find_module("powershell").unwrap();
    assert!(powershell.mappings.iter().any(|mapping| {
        mapping.platform == Platform::Windows
            && mapping.asset == AssetId::PowerShellProfile
            && mapping.target == Target::ProfileFile
    }));

    let mise = registry::find_module("mise").unwrap();
    assert!(mise.mappings.iter().any(|mapping| {
        mapping.platform == Platform::Linux
            && mapping.asset == AssetId::MiseBash
            && mapping.target == Target::ProfileSibling("10-mise.bash")
            && mapping.requires == ["bash"]
    }));
    assert!(mise.mappings.iter().any(|mapping| {
        mapping.platform == Platform::Windows
            && mapping.asset == AssetId::MisePowerShell
            && mapping.target == Target::ProfileSibling("10-mise.ps1")
            && mapping.requires == ["powershell"]
    }));

    let brew = registry::find_module("brew").unwrap();
    assert!(brew.mappings.iter().any(|mapping| {
        mapping.platform == Platform::Linux
            && mapping.asset == AssetId::BrewBash
            && mapping.target == Target::ProfileSibling("05-brew.bash")
            && mapping.requires == ["bash"]
    }));
    let scoop = registry::find_module("scoop").unwrap();
    assert!(scoop.mappings.iter().any(|mapping| {
        mapping.platform == Platform::Windows
            && mapping.asset == AssetId::ScoopPowerShell
            && mapping.target == Target::ProfileSibling("05-scoop.ps1")
            && mapping.requires == ["powershell"]
    }));
}

#[test]
fn package_manager_priority_matches_platforms() {
    assert_eq!(
        PackageManager::priority(Platform::Linux),
        [
            PackageManager::Apt,
            PackageManager::Dnf,
            PackageManager::Pacman,
            PackageManager::Brew,
            PackageManager::Mise,
        ]
    );
    assert_eq!(
        PackageManager::priority(Platform::Windows),
        [
            PackageManager::Winget,
            PackageManager::Scoop,
            PackageManager::Brew,
            PackageManager::Mise,
        ]
    );
}

#[test]
fn unknown_modules_are_not_found() {
    assert!(registry::find_module("unknown").is_none());
}
