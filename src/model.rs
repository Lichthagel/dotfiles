#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Platform {
    Linux,
    Windows,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PackageManager {
    Apt,
    Dnf,
    Pacman,
    Brew,
    Mise,
    Winget,
    Scoop,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AssetId {
    GitConfig,
    Bashrc,
    Zshrc,
    PowerShellProfile,
    MiseBash,
    MiseZsh,
    MisePowerShell,
    BrewBash,
    BrewZsh,
    ScoopPowerShell,
    AtuinBash,
    AtuinZsh,
    AtuinPowerShell,
    OhMyPoshConfig,
    OhMyPoshBash,
    OhMyPoshZsh,
    OhMyPoshPowerShell,
}

impl AssetId {
    pub const ALL: [Self; 17] = [
        Self::GitConfig,
        Self::Bashrc,
        Self::Zshrc,
        Self::PowerShellProfile,
        Self::MiseBash,
        Self::MiseZsh,
        Self::MisePowerShell,
        Self::BrewBash,
        Self::BrewZsh,
        Self::ScoopPowerShell,
        Self::AtuinBash,
        Self::AtuinZsh,
        Self::AtuinPowerShell,
        Self::OhMyPoshConfig,
        Self::OhMyPoshBash,
        Self::OhMyPoshZsh,
        Self::OhMyPoshPowerShell,
    ];

    pub const fn all() -> &'static [Self] {
        &Self::ALL
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SecretBundle {
    Atuin,
    OpenCode,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SetupKind {
    None,
    Homebrew,
    Scoop,
    Atuin,
    OpenCode,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Target {
    Home(&'static str),
    ProfileFile,
    ProfileSibling(&'static str),
}
