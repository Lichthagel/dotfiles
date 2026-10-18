pub mod atuin;
pub mod bash;
pub mod brew;
pub mod git;
pub mod mise;
pub mod oh_my_posh;
pub mod opencode;
pub mod powershell;
pub mod scoop;
pub mod zsh;

use crate::model::Platform;
use crate::registry::MappingBehavior;

pub(super) const LINUX: &[Platform] = &[Platform::Linux];
pub(super) const WINDOWS: &[Platform] = &[Platform::Windows];
pub(super) const CROSS_PLATFORM: &[Platform] = &[Platform::Linux, Platform::Windows];
pub(super) const NO_REQUIREMENTS: &[&str] = &[];
pub(super) const REQUIRES_BASH: &[&str] = &["bash"];
pub(super) const REQUIRES_ZSH: &[&str] = &["zsh"];
pub(super) const REQUIRES_POWERSHELL: &[&str] = &["powershell"];
pub(super) const REPLACE_MAPPING: MappingBehavior = MappingBehavior::Replace;
pub(super) const BASH_PROFILE_SECTION: MappingBehavior = MappingBehavior::ManagedSection {
    start: "# >>> dotfiles managed bash section >>>",
    end: "# <<< dotfiles managed bash section <<<",
};
pub(super) const ZSH_PROFILE_SECTION: MappingBehavior = MappingBehavior::ManagedSection {
    start: "# >>> dotfiles managed zsh section >>>",
    end: "# <<< dotfiles managed zsh section <<<",
};
pub(super) const POWERSHELL_PROFILE_SECTION: MappingBehavior = MappingBehavior::ManagedSection {
    start: "# >>> dotfiles managed powershell section >>>",
    end: "# <<< dotfiles managed powershell section <<<",
};
