mod support;

use std::fs;

use dotfiles::cli::InstallRequest;
use dotfiles::engine::{install, resolve_modules};
use dotfiles::model::Platform;
use dotfiles::run;
use support::installer::{test_context, InstallFixture};
use support::runner::FakeRunner;

#[test]
fn git_installs_before_mapping() {
    let runner = FakeRunner::apt_without_mise_then_mise_after_install();
    let mut fixture = InstallFixture::new(Platform::Linux);
    install(
        InstallRequest::explicit(["git"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
    )
    .unwrap();

    assert_eq!(runner.package_log(), ["git"]);
    assert_eq!(
        fs::read(fixture.target(".gitconfig")).unwrap(),
        include_bytes!("../modules/git/gitconfig")
    );
}

#[test]
fn provider_phase_precedes_dependent_package_phase() {
    let runner = FakeRunner::apt_without_mise_then_mise_after_install();
    let mut fixture = InstallFixture::new(Platform::Linux);
    install(
        InstallRequest::explicit(["mise", "git"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
    )
    .unwrap();
    assert_eq!(runner.package_log(), ["mise", "git"]);
}

#[test]
fn required_shell_module_is_resolved_for_prompt_integration() {
    let runner = FakeRunner::available(["apt", "dpkg-query", "run0", "apt-get"]);
    let resolved = resolve_modules(&["oh-my-posh".into()], Platform::Linux, &runner).unwrap();
    assert!(resolved.iter().any(|module| module.id == "bash"));
    assert!(resolved.iter().any(|module| module.id == "oh-my-posh"));
}

#[test]
fn bash_and_prompt_assets_are_materialized_as_regular_files() {
    let runner = FakeRunner::available(["apt", "dpkg-query", "run0", "apt-get"]);
    let mut fixture = InstallFixture::new(Platform::Linux);
    install(
        InstallRequest::explicit(["bash", "oh-my-posh"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
    )
    .unwrap();

    let bashrc = fixture.target(".bashrc");
    let prompt = fixture.target(".config/oh-my-posh/config.json");
    assert!(bashrc.is_file() && !bashrc.is_symlink());
    assert!(prompt.is_file() && !prompt.is_symlink());
}

#[test]
fn unavailable_managers_fail_before_mapping() {
    let runner = FakeRunner::available(std::iter::empty::<&str>());
    let mut fixture = InstallFixture::new(Platform::Linux);
    let error = install(
        InstallRequest::explicit(["git"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
    )
    .unwrap_err();
    assert!(error.to_string().contains("git"));
    assert!(!fixture.target(".gitconfig").exists());
}

#[test]
fn package_failure_prevents_setup_and_mapping() {
    let runner = FakeRunner::apt_install_failure();
    let mut fixture = InstallFixture::new(Platform::Linux);
    assert!(install(
        InstallRequest::explicit(["git"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
    )
    .is_err());
    assert!(!fixture.target(".gitconfig").exists());
}

#[test]
fn empty_selection_is_successful_noop() {
    let runner = FakeRunner::available(std::iter::empty::<&str>());
    let mut fixture = InstallFixture::new(Platform::Linux);
    install(
        InstallRequest::explicit(std::iter::empty::<&str>(), true, Platform::Linux),
        test_context(&mut fixture, &runner),
    )
    .unwrap();
    assert!(runner.calls().is_empty());
}

#[test]
fn list_does_not_probe_or_install_packages() {
    let mut output = Vec::new();
    run(["dotfiles", "--list"], &mut output).unwrap();
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("git - Git configuration"));
    if cfg!(windows) {
        assert!(text.contains("powershell - PowerShell profile"));
    } else {
        assert!(text.contains("bash - Bash profile"));
    }
}

#[test]
fn optional_opencode_secrets_leave_provider_configuration_unchanged() {
    let runner = FakeRunner::available(["brew"]);
    let mut fixture = InstallFixture::new(Platform::Linux);
    let previous_identity = std::env::var_os("AGE_IDENTITIES");
    std::env::set_var(
        "AGE_IDENTITIES",
        fixture.temp.path().join("missing-age-identity"),
    );
    let result = install(
        InstallRequest::explicit(["opencode"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
    );
    match previous_identity {
        Some(value) => std::env::set_var("AGE_IDENTITIES", value),
        None => std::env::remove_var("AGE_IDENTITIES"),
    }
    result.unwrap();
    let config = fixture.target(".config/opencode/opencode.json");
    let text = fs::read_to_string(config).unwrap();
    assert!(!text.contains("provider"));
}
