use crate::model::{AssetId, PackageManager, Platform, SecretBundle, SetupKind, Target};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageSpec {
    pub logical: &'static str,
    pub manager: PackageManager,
    pub name: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingBehavior {
    Replace,
    ManagedSection {
        start: &'static str,
        end: &'static str,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MappingSpec {
    pub platform: Platform,
    pub asset: AssetId,
    pub target: Target,
    pub requires: &'static [&'static str],
    pub behavior: MappingBehavior,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleSpec {
    pub id: &'static str,
    pub description: &'static str,
    pub platforms: &'static [Platform],
    pub default_selected: bool,
    pub provides: Option<PackageManager>,
    pub requires: &'static [&'static str],
    pub packages: &'static [PackageSpec],
    pub mappings: &'static [MappingSpec],
    pub setup: SetupKind,
    pub secret_bundle: Option<SecretBundle>,
}

impl ModuleSpec {
    pub fn supports(&self, platform: Platform) -> bool {
        self.platforms.contains(&platform)
    }
}

impl PackageManager {
    pub const fn priority(platform: Platform) -> &'static [Self] {
        match platform {
            Platform::Linux => &[Self::Apt, Self::Dnf, Self::Pacman, Self::Brew, Self::Mise],
            Platform::Windows => &[Self::Winget, Self::Scoop, Self::Brew, Self::Mise],
        }
    }
}

const LINUX: &[Platform] = &[Platform::Linux];
const WINDOWS: &[Platform] = &[Platform::Windows];
const CROSS_PLATFORM: &[Platform] = &[Platform::Linux, Platform::Windows];
const NO_REQUIREMENTS: &[&str] = &[];
const REQUIRES_BASH: &[&str] = &["bash"];
const REQUIRES_ZSH: &[&str] = &["zsh"];
const REQUIRES_POWERSHELL: &[&str] = &["powershell"];
const REPLACE_MAPPING: MappingBehavior = MappingBehavior::Replace;
const BASH_PROFILE_SECTION: MappingBehavior = MappingBehavior::ManagedSection {
    start: "# >>> dotfiles managed bash section >>>",
    end: "# <<< dotfiles managed bash section <<<",
};
const ZSH_PROFILE_SECTION: MappingBehavior = MappingBehavior::ManagedSection {
    start: "# >>> dotfiles managed zsh section >>>",
    end: "# <<< dotfiles managed zsh section <<<",
};
const POWERSHELL_PROFILE_SECTION: MappingBehavior = MappingBehavior::ManagedSection {
    start: "# >>> dotfiles managed powershell section >>>",
    end: "# <<< dotfiles managed powershell section <<<",
};

const GIT_PACKAGES: &[PackageSpec] = &[
    PackageSpec {
        logical: "git",
        manager: PackageManager::Apt,
        name: "git",
    },
    PackageSpec {
        logical: "git",
        manager: PackageManager::Dnf,
        name: "git",
    },
    PackageSpec {
        logical: "git",
        manager: PackageManager::Pacman,
        name: "git",
    },
    PackageSpec {
        logical: "git",
        manager: PackageManager::Brew,
        name: "git",
    },
    PackageSpec {
        logical: "git",
        manager: PackageManager::Mise,
        name: "git",
    },
    PackageSpec {
        logical: "git",
        manager: PackageManager::Winget,
        name: "Git.Git",
    },
    PackageSpec {
        logical: "git",
        manager: PackageManager::Scoop,
        name: "git",
    },
];

const MISE_PACKAGES: &[PackageSpec] = &[
    PackageSpec {
        logical: "mise",
        manager: PackageManager::Apt,
        name: "mise",
    },
    PackageSpec {
        logical: "mise",
        manager: PackageManager::Dnf,
        name: "mise",
    },
    PackageSpec {
        logical: "mise",
        manager: PackageManager::Pacman,
        name: "mise",
    },
    PackageSpec {
        logical: "mise",
        manager: PackageManager::Brew,
        name: "mise",
    },
    PackageSpec {
        logical: "mise",
        manager: PackageManager::Winget,
        name: "jdx.mise",
    },
    PackageSpec {
        logical: "mise",
        manager: PackageManager::Scoop,
        name: "mise",
    },
];

const ATUIN_PACKAGES: &[PackageSpec] = &[
    PackageSpec {
        logical: "atuin",
        manager: PackageManager::Apt,
        name: "atuin",
    },
    PackageSpec {
        logical: "atuin",
        manager: PackageManager::Dnf,
        name: "atuin",
    },
    PackageSpec {
        logical: "atuin",
        manager: PackageManager::Pacman,
        name: "atuin",
    },
    PackageSpec {
        logical: "atuin",
        manager: PackageManager::Brew,
        name: "atuin",
    },
    PackageSpec {
        logical: "atuin",
        manager: PackageManager::Mise,
        name: "atuin",
    },
    PackageSpec {
        logical: "atuin",
        manager: PackageManager::Winget,
        name: "Atuinsh.Atuin",
    },
    PackageSpec {
        logical: "atuin",
        manager: PackageManager::Scoop,
        name: "atuin",
    },
];

const OH_MY_POSH_PACKAGES: &[PackageSpec] = &[
    PackageSpec {
        logical: "oh-my-posh",
        manager: PackageManager::Apt,
        name: "oh-my-posh",
    },
    PackageSpec {
        logical: "oh-my-posh",
        manager: PackageManager::Dnf,
        name: "oh-my-posh",
    },
    PackageSpec {
        logical: "oh-my-posh",
        manager: PackageManager::Pacman,
        name: "oh-my-posh",
    },
    PackageSpec {
        logical: "oh-my-posh",
        manager: PackageManager::Brew,
        name: "oh-my-posh",
    },
    PackageSpec {
        logical: "oh-my-posh",
        manager: PackageManager::Mise,
        name: "oh-my-posh",
    },
    PackageSpec {
        logical: "oh-my-posh",
        manager: PackageManager::Winget,
        name: "JanDeDobbeleer.OhMyPosh",
    },
    PackageSpec {
        logical: "oh-my-posh",
        manager: PackageManager::Scoop,
        name: "oh-my-posh",
    },
];

const OPENCODE_PACKAGES: &[PackageSpec] = &[
    PackageSpec {
        logical: "opencode",
        manager: PackageManager::Brew,
        name: "anomalyco/tap/opencode",
    },
    PackageSpec {
        logical: "opencode",
        manager: PackageManager::Mise,
        name: "github:anomalyco/opencode",
    },
    PackageSpec {
        logical: "opencode",
        manager: PackageManager::Scoop,
        name: "opencode",
    },
];

const GIT_MAPPINGS: &[MappingSpec] = &[
    MappingSpec {
        platform: Platform::Linux,
        asset: AssetId::GitConfig,
        target: Target::Home(".gitconfig"),
        requires: NO_REQUIREMENTS,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Windows,
        asset: AssetId::GitConfig,
        target: Target::Home(".gitconfig"),
        requires: NO_REQUIREMENTS,
        behavior: REPLACE_MAPPING,
    },
];

const BASH_MAPPINGS: &[MappingSpec] = &[MappingSpec {
    platform: Platform::Linux,
    asset: AssetId::Bashrc,
    target: Target::Home(".bashrc"),
    requires: NO_REQUIREMENTS,
    behavior: BASH_PROFILE_SECTION,
}];

const ZSH_MAPPINGS: &[MappingSpec] = &[MappingSpec {
    platform: Platform::Linux,
    asset: AssetId::Zshrc,
    target: Target::Home(".zshrc"),
    requires: NO_REQUIREMENTS,
    behavior: ZSH_PROFILE_SECTION,
}];

const POWERSHELL_MAPPINGS: &[MappingSpec] = &[MappingSpec {
    platform: Platform::Windows,
    asset: AssetId::PowerShellProfile,
    target: Target::ProfileFile,
    requires: NO_REQUIREMENTS,
    behavior: POWERSHELL_PROFILE_SECTION,
}];

const MISE_MAPPINGS: &[MappingSpec] = &[
    MappingSpec {
        platform: Platform::Linux,
        asset: AssetId::MiseBash,
        target: Target::Home(".config/bashrc.d/10-mise.bash"),
        requires: REQUIRES_BASH,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Linux,
        asset: AssetId::MiseZsh,
        target: Target::Home(".config/zshrc.d/10-mise.zsh"),
        requires: REQUIRES_ZSH,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Windows,
        asset: AssetId::MisePowerShell,
        target: Target::ProfileSibling("Profile.d/10-mise.ps1"),
        requires: REQUIRES_POWERSHELL,
        behavior: REPLACE_MAPPING,
    },
];

const BREW_MAPPINGS: &[MappingSpec] = &[
    MappingSpec {
        platform: Platform::Linux,
        asset: AssetId::BrewBash,
        target: Target::Home(".config/bashrc.d/05-brew.bash"),
        requires: REQUIRES_BASH,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Linux,
        asset: AssetId::BrewZsh,
        target: Target::Home(".config/zshrc.d/05-brew.zsh"),
        requires: REQUIRES_ZSH,
        behavior: REPLACE_MAPPING,
    },
];

const SCOOP_MAPPINGS: &[MappingSpec] = &[MappingSpec {
    platform: Platform::Windows,
    asset: AssetId::ScoopPowerShell,
    target: Target::ProfileSibling("Profile.d/05-scoop.ps1"),
    requires: REQUIRES_POWERSHELL,
    behavior: REPLACE_MAPPING,
}];

const ATUIN_MAPPINGS: &[MappingSpec] = &[
    MappingSpec {
        platform: Platform::Linux,
        asset: AssetId::AtuinBash,
        target: Target::Home(".config/bashrc.d/50-atuin.bash"),
        requires: REQUIRES_BASH,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Linux,
        asset: AssetId::AtuinZsh,
        target: Target::Home(".config/zshrc.d/50-atuin.zsh"),
        requires: REQUIRES_ZSH,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Windows,
        asset: AssetId::AtuinPowerShell,
        target: Target::ProfileSibling("Profile.d/50-atuin.ps1"),
        requires: REQUIRES_POWERSHELL,
        behavior: REPLACE_MAPPING,
    },
];

const OH_MY_POSH_MAPPINGS: &[MappingSpec] = &[
    MappingSpec {
        platform: Platform::Linux,
        asset: AssetId::OhMyPoshConfig,
        target: Target::Home(".config/oh-my-posh/config.json"),
        requires: NO_REQUIREMENTS,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Linux,
        asset: AssetId::OhMyPoshBash,
        target: Target::Home(".config/bashrc.d/10-oh-my-posh.bash"),
        requires: REQUIRES_BASH,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Linux,
        asset: AssetId::OhMyPoshZsh,
        target: Target::Home(".config/zshrc.d/10-oh-my-posh.zsh"),
        requires: REQUIRES_ZSH,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Windows,
        asset: AssetId::OhMyPoshConfig,
        target: Target::ProfileSibling("oh-my-posh.config.json"),
        requires: NO_REQUIREMENTS,
        behavior: REPLACE_MAPPING,
    },
    MappingSpec {
        platform: Platform::Windows,
        asset: AssetId::OhMyPoshPowerShell,
        target: Target::ProfileSibling("Profile.d/10-oh-my-posh.ps1"),
        requires: REQUIRES_POWERSHELL,
        behavior: REPLACE_MAPPING,
    },
];

pub static MODULES: &[ModuleSpec] = &[
    ModuleSpec {
        id: "git",
        description: "Git configuration",
        platforms: CROSS_PLATFORM,
        default_selected: true,
        provides: None,
        requires: NO_REQUIREMENTS,
        packages: GIT_PACKAGES,
        mappings: GIT_MAPPINGS,
        setup: SetupKind::None,
        secret_bundle: None,
    },
    ModuleSpec {
        id: "bash",
        description: "Bash profile",
        platforms: LINUX,
        default_selected: true,
        provides: None,
        requires: NO_REQUIREMENTS,
        packages: &[],
        mappings: BASH_MAPPINGS,
        setup: SetupKind::None,
        secret_bundle: None,
    },
    ModuleSpec {
        id: "zsh",
        description: "Zsh profile",
        platforms: LINUX,
        default_selected: true,
        provides: None,
        requires: NO_REQUIREMENTS,
        packages: &[],
        mappings: ZSH_MAPPINGS,
        setup: SetupKind::None,
        secret_bundle: None,
    },
    ModuleSpec {
        id: "powershell",
        description: "PowerShell profile",
        platforms: WINDOWS,
        default_selected: true,
        provides: None,
        requires: NO_REQUIREMENTS,
        packages: &[],
        mappings: POWERSHELL_MAPPINGS,
        setup: SetupKind::None,
        secret_bundle: None,
    },
    ModuleSpec {
        id: "mise",
        description: "mise tool version manager",
        platforms: CROSS_PLATFORM,
        default_selected: true,
        provides: Some(PackageManager::Mise),
        requires: NO_REQUIREMENTS,
        packages: MISE_PACKAGES,
        mappings: MISE_MAPPINGS,
        setup: SetupKind::None,
        secret_bundle: None,
    },
    ModuleSpec {
        id: "brew",
        description: "Homebrew package manager",
        platforms: LINUX,
        default_selected: false,
        provides: Some(PackageManager::Brew),
        requires: NO_REQUIREMENTS,
        packages: &[],
        mappings: BREW_MAPPINGS,
        setup: SetupKind::Homebrew,
        secret_bundle: None,
    },
    ModuleSpec {
        id: "scoop",
        description: "Scoop package manager",
        platforms: WINDOWS,
        default_selected: false,
        provides: Some(PackageManager::Scoop),
        requires: NO_REQUIREMENTS,
        packages: &[],
        mappings: SCOOP_MAPPINGS,
        setup: SetupKind::Scoop,
        secret_bundle: None,
    },
    ModuleSpec {
        id: "atuin",
        description: "Atuin shell history",
        platforms: CROSS_PLATFORM,
        default_selected: false,
        provides: None,
        requires: NO_REQUIREMENTS,
        packages: ATUIN_PACKAGES,
        mappings: ATUIN_MAPPINGS,
        setup: SetupKind::Atuin,
        secret_bundle: Some(SecretBundle::Atuin),
    },
    ModuleSpec {
        id: "oh-my-posh",
        description: "Oh My Posh prompt",
        platforms: CROSS_PLATFORM,
        default_selected: false,
        provides: None,
        requires: NO_REQUIREMENTS,
        packages: OH_MY_POSH_PACKAGES,
        mappings: OH_MY_POSH_MAPPINGS,
        setup: SetupKind::None,
        secret_bundle: None,
    },
    ModuleSpec {
        id: "opencode",
        description: "OpenCode AI coding agent",
        platforms: CROSS_PLATFORM,
        default_selected: false,
        provides: None,
        requires: NO_REQUIREMENTS,
        packages: OPENCODE_PACKAGES,
        mappings: &[],
        setup: SetupKind::OpenCode,
        secret_bundle: Some(SecretBundle::OpenCode),
    },
];

pub fn modules() -> &'static [ModuleSpec] {
    MODULES
}

pub fn find_module(id: &str) -> Option<&'static ModuleSpec> {
    MODULES.iter().find(|module| module.id == id)
}
