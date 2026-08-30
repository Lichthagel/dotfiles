mod support;

use std::fs;
use std::sync::{Mutex, OnceLock};

use dotfiles::cli::{InstallRequest, TerminalIo, TerminalKey};
use dotfiles::engine::{
    install, install_with_registry, resolve_modules, InstallContext, InstallOutcome,
};
use dotfiles::engine::{ordered_phases, resolve_modules_from_registry};
use dotfiles::error::Result;
use dotfiles::model::{PackageManager, Platform, SetupKind};
use dotfiles::registry::{MappingSpec, ModuleSpec, PackageSpec};
use dotfiles::run;
use support::installer::{test_context, InstallFixture};
use support::runner::FakeRunner;

static TEST_PROVIDER_PACKAGES: &[PackageSpec] = &[PackageSpec {
    logical: "mise",
    manager: PackageManager::Apt,
    name: "mise",
}];
static TEST_CONSUMER_PACKAGES: &[PackageSpec] = &[PackageSpec {
    logical: "consumer",
    manager: PackageManager::Mise,
    name: "consumer",
}];
static TEST_NO_MAPPINGS: &[MappingSpec] = &[];
static TEST_PROVIDER: ModuleSpec = ModuleSpec {
    id: "test-mise-provider",
    description: "test mise provider",
    platforms: &[Platform::Linux],
    default_selected: false,
    provides: Some(PackageManager::Mise),
    requires: &[],
    packages: TEST_PROVIDER_PACKAGES,
    mappings: TEST_NO_MAPPINGS,
    setup: SetupKind::None,
    secret_bundle: None,
};
static TEST_CONSUMER: ModuleSpec = ModuleSpec {
    id: "test-mise-consumer",
    description: "test mise consumer",
    platforms: &[Platform::Linux],
    default_selected: false,
    provides: None,
    requires: &[],
    packages: TEST_CONSUMER_PACKAGES,
    mappings: TEST_NO_MAPPINGS,
    setup: SetupKind::None,
    secret_bundle: None,
};
static TEST_MODULES: &[ModuleSpec] = &[TEST_PROVIDER, TEST_CONSUMER];

fn environment_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

struct BackTerminal;

impl TerminalIo for BackTerminal {
    fn is_interactive(&self) -> bool {
        true
    }

    fn read_key(&mut self) -> Result<TerminalKey> {
        Ok(TerminalKey::Char('b'))
    }

    fn clear(&mut self) -> Result<()> {
        Ok(())
    }

    fn write_line(&mut self, _line: &str) -> Result<()> {
        Ok(())
    }
}

#[test]
fn git_installs_before_mapping() {
    let runner = FakeRunner::apt_without_mise_then_mise_after_install();
    let mut fixture = InstallFixture::new(Platform::Linux);
    fs::write(fixture.target(".gitconfig"), b"old").unwrap();
    install(
        InstallRequest::explicit(["git"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
    )
    .unwrap();

    assert_eq!(runner.package_log(), ["git"]);
    assert_eq!(runner.refresh_count(), 1);
    assert_eq!(
        fs::read(
            fixture
                .ops
                .context()
                .backup_root
                .join("fixed-timestamp/.gitconfig")
        )
        .unwrap(),
        b"old"
    );
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
fn refresh_makes_provider_available_to_dependent_phase() {
    let runner =
        FakeRunner::available(["apt", "dpkg-query", "run0", "apt-get"]).refresh_adds("mise");
    let mut fixture = InstallFixture::new(Platform::Linux);
    install_with_registry(
        InstallRequest::explicit(["test-mise-consumer"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
        TEST_MODULES,
    )
    .unwrap();

    assert_eq!(runner.package_log(), ["mise"]);
    assert!(runner
        .calls()
        .iter()
        .any(|call| { call.program == "mise" && call.args == ["use", "--global", "consumer"] }));
}

#[test]
fn refresh_failure_identifies_the_current_package_and_module() {
    let runner = FakeRunner::apt_without_mise_then_mise_after_install().failing_refresh();
    let mut fixture = InstallFixture::new(Platform::Linux);
    let error = install(
        InstallRequest::explicit(["git"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
    )
    .unwrap_err();
    let text = error.to_string();
    assert!(
        text.contains("git package git"),
        "missing package context: {text}"
    );
    assert!(text.contains("refresh"), "missing refresh context: {text}");
}

#[test]
fn required_shell_module_is_resolved_before_prompt_integration() {
    let runner = FakeRunner::available(["apt", "dpkg-query", "run0", "apt-get"]);
    let resolved = resolve_modules(&["oh-my-posh".into()], Platform::Linux, &runner).unwrap();
    let phases = ordered_phases(&resolved).unwrap();
    let ids: Vec<_> = phases.iter().map(|phase| phase[0].id).collect();
    assert_eq!(ids, ["bash", "oh-my-posh"]);
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
fn synthetic_dependency_cycles_are_rejected() {
    use dotfiles::registry::MappingSpec;

    static A_REQUIRES: &[&str] = &["cycle-b"];
    static B_REQUIRES: &[&str] = &["cycle-a"];
    static EMPTY_PACKAGES: &[dotfiles::registry::PackageSpec] = &[];
    static EMPTY_MAPPINGS: &[MappingSpec] = &[];
    static A: ModuleSpec = ModuleSpec {
        id: "cycle-a",
        description: "cycle a",
        platforms: &[Platform::Linux],
        default_selected: false,
        provides: None,
        requires: A_REQUIRES,
        packages: EMPTY_PACKAGES,
        mappings: EMPTY_MAPPINGS,
        setup: SetupKind::None,
        secret_bundle: None,
    };
    static B: ModuleSpec = ModuleSpec {
        id: "cycle-b",
        description: "cycle b",
        platforms: &[Platform::Linux],
        default_selected: false,
        provides: None,
        requires: B_REQUIRES,
        packages: EMPTY_PACKAGES,
        mappings: EMPTY_MAPPINGS,
        setup: SetupKind::None,
        secret_bundle: None,
    };
    static CYCLE: &[ModuleSpec] = &[A, B];

    let runner = FakeRunner::available(std::iter::empty::<&str>());
    let error = resolve_modules_from_registry(&["cycle-a".into()], Platform::Linux, &runner, CYCLE)
        .unwrap_err();
    assert!(error.to_string().contains("cycle-a -> cycle-b -> cycle-a"));
}

#[test]
fn package_selector_back_returns_to_module_selection_before_side_effects() {
    let runner = FakeRunner::available(["apt", "dpkg-query", "run0", "apt-get"]);
    let fixture = InstallFixture::new(Platform::Linux);
    let mut terminal = BackTerminal;
    let request = InstallRequest::interactive(["git"], false, Platform::Linux);
    let outcome = install(
        request,
        InstallContext {
            platform: &fixture.ops,
            runner: &runner,
            http: &fixture.http,
            terminal: &mut terminal,
            timestamp: &fixture.timestamp,
        },
    )
    .unwrap();
    assert_eq!(outcome, InstallOutcome::BackToModuleSelection);
    assert!(runner.package_log().is_empty());
    assert!(!fixture.target(".gitconfig").exists());
}

#[test]
fn logged_in_atuin_skips_required_secret_decryption() {
    let _lock = environment_lock();
    let old_identity = std::env::var_os("AGE_IDENTITY");
    let old_identities = std::env::var_os("AGE_IDENTITIES");
    std::env::set_var("AGE_IDENTITY", "not-an-age-identity");
    std::env::remove_var("AGE_IDENTITIES");

    let runner = FakeRunner::atuin_logged_in();
    for program in ["apt", "dpkg-query", "run0", "apt-get"] {
        runner.add_available(program);
    }
    let mut fixture = InstallFixture::new(Platform::Linux);
    let result = install(
        InstallRequest::explicit(["atuin"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
    );

    match old_identity {
        Some(value) => std::env::set_var("AGE_IDENTITY", value),
        None => std::env::remove_var("AGE_IDENTITY"),
    }
    match old_identities {
        Some(value) => std::env::set_var("AGE_IDENTITIES", value),
        None => std::env::remove_var("AGE_IDENTITIES"),
    }
    result.unwrap();
}

#[test]
fn optional_opencode_secrets_leave_provider_configuration_unchanged() {
    let _lock = environment_lock();
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
