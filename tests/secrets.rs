mod support;

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use dotfiles::cli::{TerminalIo, TerminalKey};
use dotfiles::error::Result;
use dotfiles::model::SecretBundle;
use dotfiles::secrets::{
    decrypt_bundle, decrypt_bundle_with_prompt, required_values, IdentityPrompt, IdentitySource,
    SecretMode, TerminalIdentityPrompt,
};
use support::secrets::{EncryptedFixture, TempRootGuard};
use zeroize::Zeroizing;

fn environment_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn decrypts_embedded_style_age_payload_without_creating_plaintext_files() {
    let _lock = environment_lock();
    let temp_root = TempRootGuard::new();
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
    assert!(temp_root.temp_plaintext_files().is_empty());
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
    let _lock = environment_lock();
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
        &IdentitySource::Direct(Zeroizing::new(fixture.identity_text().to_owned())),
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
fn empty_identity_file_prompts_for_a_usable_identity() {
    let _lock = environment_lock();
    let fixture = EncryptedFixture::new("KEY=value\n");
    let temp = tempfile::tempdir().unwrap();
    let identity_path = temp.path().join("empty-keys.txt");
    std::fs::write(&identity_path, b"\n# no identities\n").unwrap();
    let mut prompt = Prompt::new(fixture.identity_text().to_owned());

    let resolved = dotfiles::secrets::resolve_identity(
        IdentitySource::File(identity_path),
        SecretMode::Optional,
        Some(&mut prompt),
    )
    .unwrap()
    .unwrap();

    assert_eq!(resolved.as_identity_count(), 1);
}

#[test]
fn invalid_identity_file_prompts_for_a_usable_identity() {
    let _lock = environment_lock();
    let fixture = EncryptedFixture::new("KEY=value\n");
    let temp = tempfile::tempdir().unwrap();
    let identity_path = temp.path().join("invalid-keys.txt");
    std::fs::write(&identity_path, b"not-an-age-identity\n").unwrap();
    let mut prompt = Prompt::new(fixture.identity_text().to_owned());

    let resolved = dotfiles::secrets::resolve_identity(
        IdentitySource::File(identity_path),
        SecretMode::Optional,
        Some(&mut prompt),
    )
    .unwrap()
    .unwrap();

    assert_eq!(resolved.as_identity_count(), 1);
}

#[test]
fn nonmatching_identity_prompts_for_a_decrypting_identity() {
    let _lock = environment_lock();
    let payload_fixture = EncryptedFixture::new("KEY=value\n");
    let wrong_fixture = EncryptedFixture::new("WRONG=value\n");
    let temp = tempfile::tempdir().unwrap();
    let identity_path = temp.path().join("wrong-keys.txt");
    wrong_fixture.write_identity(&identity_path);
    let mut prompt = Prompt::new(payload_fixture.identity_text().to_owned());

    let scope = decrypt_bundle_with_prompt(
        SecretBundle::Atuin,
        payload_fixture.payload(),
        &IdentitySource::File(identity_path),
        SecretMode::Optional,
        Some(&mut prompt),
    )
    .unwrap()
    .unwrap();

    assert_eq!(scope.values().get("KEY"), Some("value"));
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
    let _lock = environment_lock();
    for plaintext in ["not-an-assignment\n", "bad-name=value\n"] {
        let temp_root = TempRootGuard::new();
        let fixture = EncryptedFixture::new(plaintext);
        assert!(decrypt_bundle(
            SecretBundle::Atuin,
            fixture.payload(),
            &fixture.identity(),
            SecretMode::Required,
        )
        .is_err());
        assert!(temp_root.temp_plaintext_files().is_empty());
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
fn malformed_secret_errors_do_not_include_values() {
    let fixture = EncryptedFixture::new("LEAK=do-not-print\nnot-an-assignment\n");
    let error = decrypt_bundle(
        SecretBundle::Atuin,
        fixture.payload(),
        &fixture.identity(),
        SecretMode::Required,
    )
    .unwrap_err();
    assert!(!error.to_string().contains("do-not-print"));
}

#[test]
fn terminal_identity_prompt_uses_terminal_line_input() {
    let fixture = EncryptedFixture::new("KEY=value\n");
    let mut terminal = LineTerminal::new([fixture.identity_text().to_owned()]);
    let mut prompt = TerminalIdentityPrompt::new(&mut terminal);
    let resolved = dotfiles::secrets::resolve_identity(
        IdentitySource::File(PathBuf::from("missing-identity-file")),
        SecretMode::Required,
        Some(&mut prompt),
    )
    .unwrap()
    .unwrap();
    assert_eq!(resolved.as_identity_count(), 1);
    assert!(terminal
        .prompts
        .iter()
        .any(|prompt| prompt.contains("Age identity")));
    assert_eq!(terminal.secret_reads, 1);
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
    fn prompt_identity(&mut self) -> Result<Option<Zeroizing<String>>> {
        Ok(self.value.take().map(Zeroizing::new))
    }
}

struct LineTerminal {
    lines: Vec<String>,
    prompts: Vec<String>,
    secret_reads: usize,
}

impl LineTerminal {
    fn new(lines: impl IntoIterator<Item = String>) -> Self {
        Self {
            lines: lines.into_iter().collect(),
            prompts: Vec::new(),
            secret_reads: 0,
        }
    }
}

impl TerminalIo for LineTerminal {
    fn is_interactive(&self) -> bool {
        true
    }

    fn read_key(&mut self) -> Result<TerminalKey> {
        Err(dotfiles::error::InstallError::Usage(
            "line-only terminal".into(),
        ))
    }

    fn read_line(&mut self, prompt: &str) -> Result<Option<String>> {
        self.prompts.push(prompt.to_owned());
        Ok(self.lines.pop())
    }

    fn read_secret(&mut self, prompt: &str) -> Result<Option<Zeroizing<String>>> {
        self.secret_reads += 1;
        self.read_line(prompt)
            .map(|value| value.map(Zeroizing::new))
    }

    fn clear(&mut self) -> Result<()> {
        Ok(())
    }

    fn write_line(&mut self, line: &str) -> Result<()> {
        self.prompts.push(line.to_owned());
        Ok(())
    }
}

fn restore_env(name: &str, value: Option<std::ffi::OsString>) {
    match value {
        Some(value) => std::env::set_var(name, value),
        None => std::env::remove_var(name),
    }
}
