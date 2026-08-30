mod support;

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use dotfiles::error::Result;
use dotfiles::model::SecretBundle;
use dotfiles::secrets::{
    decrypt_bundle, required_values, IdentityPrompt, IdentitySource, SecretMode,
};
use support::secrets::EncryptedFixture;

fn environment_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn decrypts_embedded_style_age_payload_without_creating_plaintext_files() {
    let fixture = EncryptedFixture::new("ATUIN_USERNAME=test-user\nATUIN_PASSWORD=test-pass\n");
    let scope = decrypt_bundle(
        SecretBundle::Atuin,
        fixture.payload(),
        &fixture.identity(),
        SecretMode::Required,
    )
    .unwrap()
    .unwrap();
    assert_eq!(scope.values().get("ATUIN_USERNAME"), Some("test-user"));
    assert_eq!(scope.values().get("ATUIN_PASSWORD"), Some("test-pass"));
    assert!(fixture.temp_plaintext_files().is_empty());
}

#[test]
fn direct_environment_identity_is_resolved_without_writing_it() {
    let _lock = environment_lock();
    let fixture = EncryptedFixture::new("KEY=value\n");
    let old_identity = std::env::var_os("AGE_IDENTITY");
    let old_identities = std::env::var_os("AGE_IDENTITIES");
    std::env::set_var("AGE_IDENTITY", fixture.identity_text());
    std::env::remove_var("AGE_IDENTITIES");
    let source = IdentitySource::from_environment().unwrap();
    assert!(matches!(source, IdentitySource::Direct(_)));
    let scope = decrypt_bundle(
        SecretBundle::Atuin,
        fixture.payload(),
        &source,
        SecretMode::Required,
    )
    .unwrap()
    .unwrap();
    assert_eq!(scope.values().get("KEY"), Some("value"));
    restore_env("AGE_IDENTITY", old_identity);
    restore_env("AGE_IDENTITIES", old_identities);
}

#[test]
fn identity_file_and_default_identity_path_are_supported() {
    let _lock = environment_lock();
    let fixture = EncryptedFixture::new("KEY=value\n");
    let temp = tempfile::tempdir().unwrap();
    let identity_path = temp.path().join("keys.txt");
    fixture.write_identity(&identity_path);
    let source = IdentitySource::File(identity_path.clone());
    let scope = decrypt_bundle(
        SecretBundle::Atuin,
        fixture.payload(),
        &source,
        SecretMode::Required,
    )
    .unwrap()
    .unwrap();
    assert_eq!(scope.values().get("KEY"), Some("value"));

    let old_home = std::env::var_os("DOTFILES_HOME");
    let old_appdata = std::env::var_os("APPDATA");
    let old_identity = std::env::var_os("AGE_IDENTITY");
    let old_identities = std::env::var_os("AGE_IDENTITIES");
    let home = temp.path().join("home");
    std::env::set_var("DOTFILES_HOME", &home);
    #[cfg(windows)]
    std::env::set_var("APPDATA", temp.path().join("appdata"));
    let default_path = dotfiles::platform::default_identity_path().unwrap();
    std::fs::create_dir_all(default_path.parent().unwrap()).unwrap();
    fixture.write_identity(&default_path);
    std::env::remove_var("AGE_IDENTITY");
    std::env::remove_var("AGE_IDENTITIES");
    let source = IdentitySource::from_environment().unwrap();
    assert!(matches!(source, IdentitySource::Default));
    let scope = decrypt_bundle(
        SecretBundle::Atuin,
        fixture.payload(),
        &source,
        SecretMode::Required,
    )
    .unwrap()
    .unwrap();
    assert_eq!(scope.values().get("KEY"), Some("value"));
    restore_env("DOTFILES_HOME", old_home);
    restore_env("APPDATA", old_appdata);
    restore_env("AGE_IDENTITY", old_identity);
    restore_env("AGE_IDENTITIES", old_identities);
}

#[test]
fn interactive_identity_prompt_accepts_a_path_or_direct_key() {
    let fixture = EncryptedFixture::new("KEY=value\n");
    let temp = tempfile::tempdir().unwrap();
    let identity_path = temp.path().join("prompted-keys.txt");
    fixture.write_identity(&identity_path);

    let mut path_prompt = Prompt::new(identity_path.display().to_string());
    let source = IdentitySource::File(temp.path().join("missing.txt"));
    let resolved =
        dotfiles::secrets::resolve_identity(source, SecretMode::Required, Some(&mut path_prompt))
            .unwrap()
            .unwrap();
    let scope = decrypt_bundle(
        SecretBundle::Atuin,
        fixture.payload(),
        &IdentitySource::Direct(fixture.identity_text().to_owned()),
        SecretMode::Required,
    )
    .unwrap()
    .unwrap();
    assert_eq!(scope.values().get("KEY"), Some("value"));
    assert_eq!(resolved.as_identity_count(), 1);

    let mut direct_prompt = Prompt::new(fixture.identity_text().to_owned());
    let resolved = dotfiles::secrets::resolve_identity(
        IdentitySource::File(temp.path().join("missing-direct.txt")),
        SecretMode::Required,
        Some(&mut direct_prompt),
    )
    .unwrap()
    .unwrap();
    assert_eq!(resolved.as_identity_count(), 1);
}

#[test]
fn missing_optional_identity_skips_and_required_identity_fails() {
    let fixture = EncryptedFixture::new("KEY=value\n");
    let missing = IdentitySource::File(PathBuf::from("missing-identity-file"));
    assert!(decrypt_bundle(
        SecretBundle::OpenCode,
        fixture.payload(),
        &missing,
        SecretMode::Optional,
    )
    .unwrap()
    .is_none());
    assert!(decrypt_bundle(
        SecretBundle::Atuin,
        fixture.payload(),
        &missing,
        SecretMode::Required,
    )
    .is_err());
}

#[test]
fn malformed_lines_and_invalid_names_are_rejected() {
    for plaintext in ["not-an-assignment\n", "bad-name=value\n"] {
        let fixture = EncryptedFixture::new(plaintext);
        assert!(decrypt_bundle(
            SecretBundle::Atuin,
            fixture.payload(),
            &fixture.identity(),
            SecretMode::Required,
        )
        .is_err());
    }
}

#[test]
fn empty_required_values_are_rejected_but_comments_and_empty_lines_parse() {
    let fixture = EncryptedFixture::new("\n# comment\nKEY=\n");
    let scope = decrypt_bundle(
        SecretBundle::Atuin,
        fixture.payload(),
        &fixture.identity(),
        SecretMode::Required,
    )
    .unwrap()
    .unwrap();
    assert!(required_values(&scope.values(), &["KEY"]).is_err());
}

#[test]
fn debug_output_redacts_secret_values() {
    let fixture = EncryptedFixture::new("KEY=do-not-print\n");
    let scope = decrypt_bundle(
        SecretBundle::Atuin,
        fixture.payload(),
        &fixture.identity(),
        SecretMode::Required,
    )
    .unwrap()
    .unwrap();
    let debug = format!("{scope:?}");
    assert!(!debug.contains("do-not-print"));
    assert!(debug.contains("redacted"));
}

struct Prompt {
    value: Option<String>,
}

impl Prompt {
    fn new(value: String) -> Self {
        Self { value: Some(value) }
    }
}

impl IdentityPrompt for Prompt {
    fn prompt_identity(&mut self) -> Result<Option<String>> {
        Ok(self.value.take())
    }
}

fn restore_env(name: &str, value: Option<std::ffi::OsString>) {
    match value {
        Some(value) => std::env::set_var(name, value),
        None => std::env::remove_var(name),
    }
}
