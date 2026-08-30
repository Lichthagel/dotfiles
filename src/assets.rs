use crate::model::{AssetId, SecretBundle};

pub fn asset_bytes(asset: AssetId) -> &'static [u8] {
    match asset {
        AssetId::GitConfig => include_bytes!("../modules/git/gitconfig"),
        AssetId::Bashrc => include_bytes!("../modules/bash/bashrc"),
        AssetId::PowerShellProfile => include_bytes!("../modules/powershell/profile.ps1"),
        AssetId::MiseBash => include_bytes!("../modules/mise/bash/10-mise.bash"),
        AssetId::MisePowerShell => include_bytes!("../modules/mise/powershell/10-mise.ps1"),
        AssetId::BrewBash => include_bytes!("../modules/brew/bash/05-brew.bash"),
        AssetId::ScoopPowerShell => include_bytes!("../modules/scoop/powershell/05-scoop.ps1"),
        AssetId::AtuinBash => include_bytes!("../modules/atuin/bash/50-atuin.bash"),
        AssetId::AtuinPowerShell => include_bytes!("../modules/atuin/powershell/50-atuin.ps1"),
        AssetId::OhMyPoshConfig => include_bytes!("../modules/oh-my-posh/oh-my-posh.config.json"),
        AssetId::OhMyPoshBash => include_bytes!("../modules/oh-my-posh/bash/10-oh-my-posh.bash"),
        AssetId::OhMyPoshPowerShell => {
            include_bytes!("../modules/oh-my-posh/powershell/10-oh-my-posh.ps1")
        }
    }
}

pub fn encrypted_bundle_bytes(bundle: SecretBundle) -> &'static [u8] {
    match bundle {
        SecretBundle::Atuin => include_bytes!("../secrets/atuin.env.age"),
        SecretBundle::OpenCode => include_bytes!("../secrets/opencode.env.age"),
    }
}
