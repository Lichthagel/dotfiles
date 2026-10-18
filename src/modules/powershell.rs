use super::{NO_REQUIREMENTS, POWERSHELL_PROFILE_SECTION, WINDOWS};
use crate::model::{EmbeddedAsset, Platform, SetupKind, Target};
use crate::registry::{MappingSpec, ModuleSpec};

const POWERSHELL_PROFILE_ASSET: EmbeddedAsset = EmbeddedAsset::new(
    "powershell/profile.ps1",
    include_bytes!("../../modules/powershell/profile.ps1"),
);

const POWERSHELL_MAPPINGS: &[MappingSpec] = &[MappingSpec {
    platform: Platform::Windows,
    asset: POWERSHELL_PROFILE_ASSET,
    target: Target::ProfileFile,
    requires: NO_REQUIREMENTS,
    behavior: POWERSHELL_PROFILE_SECTION,
}];

pub const SPEC: ModuleSpec = ModuleSpec {
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
};
