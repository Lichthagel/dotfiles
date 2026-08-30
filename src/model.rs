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
    PowerShellProfile,
    MiseBash,
    MisePowerShell,
    BrewBash,
    ScoopPowerShell,
    AtuinBash,
    AtuinPowerShell,
    OhMyPoshConfig,
    OhMyPoshBash,
    OhMyPoshPowerShell,
}

impl AssetId {
    pub const ALL: [Self; 12] = [
        Self::GitConfig,
        Self::Bashrc,
        Self::PowerShellProfile,
        Self::MiseBash,
        Self::MisePowerShell,
        Self::BrewBash,
        Self::ScoopPowerShell,
        Self::AtuinBash,
        Self::AtuinPowerShell,
        Self::OhMyPoshConfig,
        Self::OhMyPoshBash,
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
