use std::fmt;

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

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct EmbeddedAsset {
    pub name: &'static str,
    pub bytes: &'static [u8],
}

impl EmbeddedAsset {
    pub const fn new(name: &'static str, bytes: &'static [u8]) -> Self {
        Self { name, bytes }
    }
}

impl fmt::Debug for EmbeddedAsset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EmbeddedAsset")
            .field("name", &self.name)
            .finish()
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct EmbeddedSecretBundle {
    pub name: &'static str,
    pub bytes: &'static [u8],
}

impl EmbeddedSecretBundle {
    pub const fn new(name: &'static str, bytes: &'static [u8]) -> Self {
        Self { name, bytes }
    }
}

impl fmt::Debug for EmbeddedSecretBundle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EmbeddedSecretBundle")
            .field("name", &self.name)
            .finish()
    }
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
