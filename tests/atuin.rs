mod support;

use dotfiles::error::InstallError;
use dotfiles::model::Platform;
use dotfiles::modules::atuin::{login_atuin, setup_atuin};
use dotfiles::secrets::SecretValues;
use dotfiles::setup::{HttpClient, SetupContext};
use support::mappings::MappingFixture;
use support::runner::FakeRunner;

struct NoHttp;

impl HttpClient for NoHttp {
    fn get_json(&self, _url: &str) -> dotfiles::error::Result<serde_json::Value> {
        unreachable!("Atuin does not use HTTP")
    }

    fn get_bytes(&self, _url: &str) -> dotfiles::error::Result<Vec<u8>> {
        unreachable!("Atuin does not use HTTP")
    }
}

#[test]
fn atuin_login_sends_only_password_and_key_to_stdin() {
    let runner = FakeRunner::atuin_not_logged_in();
    let secrets = SecretValues::from_pairs([
        ("ATUIN_USERNAME", "user"),
        ("ATUIN_PASSWORD", "password"),
        ("ATUIN_KEY", "encryption-key"),
    ])
    .unwrap();

    login_atuin(&runner, &secrets, Some("https://sync.example")).unwrap();

    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].args, ["login", "-u", "user"]);
    assert_eq!(
        calls[0].stdin.as_deref(),
        Some(b"password\nencryption-key\n".as_slice())
    );
    assert_eq!(
        calls[0].env,
        [("ATUIN_SYNC_ADDRESS".into(), "https://sync.example".into())]
    );
    assert!(!runner.log_contains("password"));
    assert!(!runner.log_contains("encryption-key"));
}

#[test]
fn logged_in_atuin_skips_secret_login() {
    let runner = FakeRunner::atuin_logged_in();
    let fixture = MappingFixture::with_platform(Platform::Linux);
    let http = NoHttp;
    let context = SetupContext {
        platform: &fixture.ops,
        runner: &runner,
        http: &http,
        secrets: None,
    };

    setup_atuin(&context).unwrap();
    assert_eq!(runner.calls().len(), 1);
    assert_eq!(runner.calls()[0].args, ["status"]);
}

#[test]
fn repeated_atuin_setup_logs_in_once_then_skips_without_secrets() {
    let runner = FakeRunner::atuin_not_logged_in();
    let fixture = MappingFixture::with_platform(Platform::Linux);
    let http = NoHttp;
    let secrets = SecretValues::from_pairs([
        ("ATUIN_USERNAME", "user"),
        ("ATUIN_PASSWORD", "password"),
        ("ATUIN_KEY", "encryption-key"),
    ])
    .unwrap();
    let context = SetupContext {
        platform: &fixture.ops,
        runner: &runner,
        http: &http,
        secrets: Some(&secrets),
    };
    setup_atuin(&context).unwrap();

    runner.set_atuin_logged_in(true);
    let logged_in_context = SetupContext {
        platform: &fixture.ops,
        runner: &runner,
        http: &http,
        secrets: None,
    };
    setup_atuin(&logged_in_context).unwrap();

    assert_eq!(
        runner
            .calls()
            .iter()
            .filter(|call| call.args.first().map(String::as_str) == Some("login"))
            .count(),
        1
    );
}

#[test]
fn missing_required_atuin_secrets_fails_before_login() {
    let runner = FakeRunner::atuin_not_logged_in();
    let secrets = SecretValues::from_pairs([("ATUIN_USERNAME", "user")]).unwrap();

    let error = login_atuin(&runner, &secrets, None).unwrap_err();
    assert!(matches!(error, InstallError::Secret(message) if message.contains("ATUIN_PASSWORD")));
    assert_eq!(runner.calls().len(), 0);
}

#[test]
fn failed_atuin_login_returns_setup_error_without_secret_output() {
    let runner = FakeRunner::atuin_not_logged_in().failing_program("atuin");
    let secrets = SecretValues::from_pairs([
        ("ATUIN_USERNAME", "user"),
        ("ATUIN_PASSWORD", "password"),
        ("ATUIN_KEY", "encryption-key"),
    ])
    .unwrap();

    let error = login_atuin(&runner, &secrets, None).unwrap_err();
    assert!(matches!(error, InstallError::Setup(ref message) if message == "Atuin login failed"));
    assert!(!error.to_string().contains("password"));
    assert!(!error.to_string().contains("encryption-key"));
}
