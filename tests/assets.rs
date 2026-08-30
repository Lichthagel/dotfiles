use dotfiles::assets::{asset_bytes, encrypted_bundle_bytes};
use dotfiles::model::{AssetId, SecretBundle};

#[test]
fn all_static_assets_are_embedded() {
    for asset in AssetId::all() {
        assert!(!asset_bytes(*asset).is_empty(), "empty asset: {asset:?}");
    }
}

#[test]
fn encrypted_bundles_are_embedded_as_age_payloads() {
    for bundle in [SecretBundle::Atuin, SecretBundle::OpenCode] {
        assert!(encrypted_bundle_bytes(bundle).starts_with(b"age-encryption.org/v1"));
    }
}
