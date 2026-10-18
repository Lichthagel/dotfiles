use std::collections::BTreeSet;

use dotfiles::registry;

#[test]
fn all_module_assets_are_embedded_with_their_definitions() {
    let mut names = BTreeSet::new();
    for module in registry::modules() {
        for mapping in module.mappings {
            assert!(
                !mapping.asset.name.is_empty(),
                "unnamed asset in {}",
                module.id
            );
            assert!(
                !mapping.asset.bytes.is_empty(),
                "empty asset {} in module {}",
                mapping.asset.name,
                module.id
            );
            names.insert(mapping.asset.name);
        }
    }
    assert_eq!(names.len(), 17);
}

#[test]
fn encrypted_bundles_are_embedded_as_age_payloads() {
    for (module_id, name) in [("atuin", "Atuin"), ("opencode", "OpenCode")] {
        let bundle = registry::find_module(module_id)
            .unwrap()
            .secret_bundle
            .expect("module should embed a secret bundle");
        assert_eq!(bundle.name, name);
        assert!(bundle.bytes.starts_with(b"age-encryption.org/v1"));
        let debug = format!("{bundle:?}");
        assert!(debug.contains(name));
        assert!(!debug.contains("age-encryption.org/v1"));
    }
}
