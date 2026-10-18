mod support;

use std::fs;
use std::sync::{Mutex, OnceLock};

use dotfiles::cli::{InstallRequest, TerminalIo, TerminalKey};
use dotfiles::engine::{
    install, install_with_registry, resolve_modules, InstallContext, InstallOutcome,
};
use dotfiles::engine::{ordered_phases, resolve_modules_from_registry};
use dotfiles::error::Result;
use dotfiles::model::{PackageManager, Platform};
use dotfiles::platform::PlatformOps;
use dotfiles::registry::{MappingSpec, ModuleSpec, PackageSpec};
use dotfiles::run;
use dotfiles::state::ModuleState;
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
static UNAVAILABLE_GIT_PACKAGES: &[PackageSpec] = &[PackageSpec {
    logical: "git",
    manager: PackageManager::Apt,
    name: "git",
}];
static TEST_PROVIDER: ModuleSpec = ModuleSpec {
    id: "test-mise-provider",
    description: "test mise provider",
    platforms: &[Platform::Linux],
    default_selected: false,
    provides: Some(PackageManager::Mise),
    requires: &[],
    packages: TEST_PROVIDER_PACKAGES,
    mappings: TEST_NO_MAPPINGS,
    setup: None,
    secret_request: None,
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
    setup: None,
    secret_request: None,
    secret_bundle: None,
};
static TEST_MODULES: &[ModuleSpec] = &[TEST_PROVIDER, TEST_CONSUMER];

fn count_files(root: &std::path::Path) -> usize {
    let Ok(entries) = fs::read_dir(root) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| {
            if entry.path().is_dir() {
                count_files(&entry.path())
            } else {
                1
            }
        })
        .sum()
}

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
fn existing_commands_allow_dotfiles_without_a_package_manager() {
    for platform in [Platform::Linux, Platform::Windows] {
        for yes in [false, true] {
            let runner = FakeRunner::available(["git"]);
            let mut fixture = InstallFixture::new(platform);
            install(
                InstallRequest::explicit(["git"], yes, platform),
                test_context(&mut fixture, &runner),
            )
            .unwrap();

            assert!(runner.calls().is_empty());
            assert_eq!(runner.refresh_count(), 0);
            assert_eq!(
                fs::read(fixture.target(".gitconfig")).unwrap(),
                include_bytes!("../modules/git/gitconfig")
            );
            let state = ModuleState::load(&fixture.ops.state_path().unwrap()).unwrap();
            assert!(state.contains("git"));
            assert!(!state.contains("mise"));
            assert!(!state.contains("scoop"));
        }
    }
}

#[test]
fn existing_commands_do_not_resolve_package_manager_providers() {
    let runner = FakeRunner::available(["consumer"]);
    let resolved = resolve_modules_from_registry(
        &["test-mise-consumer".into()],
        Platform::Linux,
        &runner,
        TEST_MODULES,
    )
    .unwrap();
    assert_eq!(
        resolved.iter().map(|module| module.id).collect::<Vec<_>>(),
        ["test-mise-consumer"]
    );
}

#[test]
fn converged_rerun_does_not_reinstall_or_rewrite_module_outputs() {
    let runner = FakeRunner::available(["git", "apt", "dpkg-query", "run0", "apt-get"]);
    let mut fixture = InstallFixture::new(Platform::Linux);
    let request = InstallRequest::explicit(["git", "bash", "mise"], true, Platform::Linux);
    install(request.clone(), test_context(&mut fixture, &runner)).unwrap();

    let outputs = [
        fixture.target(".gitconfig"),
        fixture.target(".bashrc"),
        fixture.target(".config/bashrc.d/10-mise.bash"),
    ];
    let bytes = outputs
        .iter()
        .map(|path| fs::read(path).unwrap())
        .collect::<Vec<_>>();
    let modified = outputs
        .iter()
        .map(|path| fs::metadata(path).unwrap().modified().unwrap())
        .collect::<Vec<_>>();
    let state_path = fixture.ops.state_path().unwrap();
    let state_bytes = fs::read(&state_path).unwrap();
    let state_modified = fs::metadata(&state_path).unwrap().modified().unwrap();
    let backup_root = fixture.ops.backup_root().unwrap();
    let backup_count = count_files(&backup_root);
    let installed_packages = runner.package_log();
    assert_eq!(installed_packages, ["mise"]);

    runner.mark_installed("apt", "git");
    runner.mark_installed("apt", "mise");
    install(request, test_context(&mut fixture, &runner)).unwrap();

    assert_eq!(runner.package_log(), installed_packages);
    assert_eq!(
        outputs
            .iter()
            .map(|path| fs::read(path).unwrap())
            .collect::<Vec<_>>(),
        bytes
    );
    assert_eq!(
        outputs
            .iter()
            .map(|path| fs::metadata(path).unwrap().modified().unwrap())
            .collect::<Vec<_>>(),
        modified
    );
    assert_eq!(fs::read(&state_path).unwrap(), state_bytes);
    assert_eq!(
        fs::metadata(&state_path).unwrap().modified().unwrap(),
        state_modified
    );
    assert_eq!(count_files(&backup_root), backup_count);
    assert!(fixture
        .terminal
        .lines()
        .iter()
        .any(|line| line.starts_with("unchanged:")));
}

#[test]
fn successful_modules_are_recorded_and_reruns_still_apply_mappings() {
    let runner = FakeRunner::available(["git", "apt", "dpkg-query", "run0", "apt-get"]);
    let mut fixture = InstallFixture::new(Platform::Linux);
    let path = fixture.ops.state_path().unwrap();
    for _ in 0..2 {
        fs::write(fixture.target(".gitconfig"), b"needs repair").unwrap();
        install(
            InstallRequest::explicit(["git", "bash"], true, Platform::Linux),
            test_context(&mut fixture, &runner),
        )
        .unwrap();
        assert_eq!(
            fs::read(fixture.target(".gitconfig")).unwrap(),
            include_bytes!("../modules/git/gitconfig")
        );
        let json: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"schema_version": 1, "modules": ["bash", "git"]})
        );
    }
}

#[test]
fn completed_modules_remain_recorded_when_a_later_package_fails() {
    let runner = FakeRunner::apt_install_failure();
    let mut fixture = InstallFixture::new(Platform::Linux);
    assert!(install(
        InstallRequest::explicit(["bash", "git"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
    )
    .is_err());
    let state = ModuleState::load(&fixture.ops.state_path().unwrap()).unwrap();
    assert!(state.contains("bash"));
    assert!(!state.contains("git"));
}

#[test]
fn mapping_failure_does_not_record_the_module() {
    let runner = FakeRunner::available(["git", "apt", "dpkg-query", "run0", "apt-get"]);
    let mut fixture = InstallFixture::new(Platform::Linux);
    fixture.ops = fixture.ops.clone().with_failing_rename();
    assert!(install(
        InstallRequest::explicit(["git"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
    )
    .is_err());
    assert!(!fixture.ops.state_path().unwrap().exists());
}

#[test]
fn setup_failure_does_not_record_the_module() {
    let runner = FakeRunner::available(std::iter::empty::<&str>());
    let mut fixture = InstallFixture::new(Platform::Linux);
    let modules = [ModuleSpec {
        id: "failing-setup",
        setup: Some(dotfiles::modules::opencode::configure_opencode),
        packages: &[],
        ..TEST_CONSUMER
    }];
    struct FailingHttp;
    impl dotfiles::setup::HttpClient for FailingHttp {
        fn get_json(&self, _url: &str) -> Result<serde_json::Value> {
            Err(dotfiles::error::InstallError::Setup(
                "injected HTTP failure".into(),
            ))
        }
        fn get_bytes(&self, _url: &str) -> Result<Vec<u8>> {
            Err(dotfiles::error::InstallError::Setup(
                "injected HTTP failure".into(),
            ))
        }
    }
    let _lock = environment_lock();
    let _config_home = support::ScopedXdgConfigHome::new(fixture.target(".config"));
    let mut context = test_context(&mut fixture, &runner);
    context.http = &FailingHttp;
    assert!(install_with_registry(
        InstallRequest::explicit(["failing-setup"], true, Platform::Linux),
        context,
        &modules,
    )
    .is_err());
    assert!(!fixture.ops.state_path().unwrap().exists());
}

#[test]
fn invalid_state_fails_before_package_or_mapping_changes() {
    let runner = FakeRunner::apt_without_mise_then_mise_after_install();
    let mut fixture = InstallFixture::new(Platform::Linux);
    let path = fixture.ops.state_path().unwrap();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"invalid").unwrap();
    assert!(install(
        InstallRequest::explicit(["git"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
    )
    .is_err());
    assert!(runner.calls().is_empty());
    assert!(!fixture.target(".gitconfig").exists());
    assert_eq!(fs::read(&path).unwrap(), b"invalid");
}

#[test]
fn intentionally_skipped_packages_do_not_prevent_recording_applied_dotfiles() {
    // Use an interactive terminal that deselects the only package and confirms.
    struct ToggleTerminal(std::collections::VecDeque<TerminalKey>);
    impl TerminalIo for ToggleTerminal {
        fn is_interactive(&self) -> bool {
            true
        }
        fn read_key(&mut self) -> Result<TerminalKey> {
            Ok(self.0.pop_front().unwrap())
        }
        fn clear(&mut self) -> Result<()> {
            Ok(())
        }
        fn write_line(&mut self, _line: &str) -> Result<()> {
            Ok(())
        }
    }
    let runner = FakeRunner::apt_without_mise_then_mise_after_install();
    let fixture = InstallFixture::new(Platform::Linux);
    let mut terminal = ToggleTerminal([TerminalKey::Space, TerminalKey::Enter].into());
    install(
        InstallRequest::explicit(["git"], false, Platform::Linux),
        InstallContext {
            platform: &fixture.ops,
            runner: &runner,
            http: &fixture.http,
            terminal: &mut terminal,
            timestamp: &fixture.timestamp,
        },
    )
    .unwrap();
    assert!(runner.package_log().is_empty());
    assert!(fixture.target(".gitconfig").is_file());
    assert!(ModuleState::load(&fixture.ops.state_path().unwrap())
        .unwrap()
        .contains("git"));
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
fn mise_does_not_implicitly_enable_powershell() {
    let runner = FakeRunner::available(["winget"]);
    let mut fixture = InstallFixture::new(Platform::Windows);
    install(
        InstallRequest::explicit(["mise"], true, Platform::Windows),
        test_context(&mut fixture, &runner),
    )
    .unwrap();

    assert!(runner.calls().iter().any(|call| {
        call.program == "winget" && call.args.iter().any(|argument| argument == "jdx.mise")
    }));
    assert!(!fixture
        .target("Documents/PowerShell/Microsoft.PowerShell_profile.ps1")
        .exists());
    assert!(!fixture
        .target("Documents/PowerShell/Profile.d/10-mise.ps1")
        .exists());
}

#[test]
fn mise_only_installs_activation_for_a_previously_applied_shell_without_reapplying_it() {
    for (platform, shell, profile, activation, expected) in [
        (
            Platform::Linux,
            "bash",
            ".bashrc",
            ".config/bashrc.d/10-mise.bash",
            include_bytes!("../modules/mise/bash/10-mise.bash").as_slice(),
        ),
        (
            Platform::Linux,
            "zsh",
            ".zshrc",
            ".config/zshrc.d/10-mise.zsh",
            include_bytes!("../modules/mise/zsh/10-mise.zsh").as_slice(),
        ),
        (
            Platform::Windows,
            "powershell",
            "Documents/PowerShell/Microsoft.PowerShell_profile.ps1",
            "Documents/PowerShell/Profile.d/10-mise.ps1",
            include_bytes!("../modules/mise/powershell/10-mise.ps1").as_slice(),
        ),
    ] {
        let runner = FakeRunner::available(["apt", "dpkg-query", "run0", "apt-get", "winget"]);
        let mut fixture = InstallFixture::new(platform);
        install(
            InstallRequest::explicit([shell], true, platform),
            test_context(&mut fixture, &runner),
        )
        .unwrap();
        fs::write(fixture.target(profile), b"keep customized shell profile").unwrap();
        assert!(!fixture.target(activation).exists());

        install(
            InstallRequest::explicit(["mise"], true, platform),
            test_context(&mut fixture, &runner),
        )
        .unwrap();

        assert_eq!(fs::read(fixture.target(activation)).unwrap(), expected);
        assert!(!fixture.target(activation).is_symlink());
        assert_eq!(
            fs::read(fixture.target(profile)).unwrap(),
            b"keep customized shell profile"
        );
        let state = ModuleState::load(&fixture.ops.state_path().unwrap()).unwrap();
        assert!(state.contains(shell));
        assert!(state.contains("mise"));
        assert_eq!(
            fixture
                .terminal
                .lines()
                .iter()
                .filter(|line| { **line == format!("completed module: {shell}") })
                .count(),
            1
        );
    }
}

#[test]
fn selecting_zsh_installs_its_profile_and_mise_dropin() {
    let runner = FakeRunner::available(["mise", "apt", "dpkg-query", "run0", "apt-get"]);
    let mut fixture = InstallFixture::new(Platform::Linux);
    install(
        InstallRequest::explicit(["zsh", "mise"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
    )
    .unwrap();

    assert_eq!(
        fs::read(fixture.target(".zshrc")).unwrap(),
        include_bytes!("../modules/zsh/zshrc")
    );
    assert_eq!(
        fs::read(fixture.target(".config/zshrc.d/10-mise.zsh")).unwrap(),
        include_bytes!("../modules/mise/zsh/10-mise.zsh")
    );
    assert!(!fixture.target(".bashrc").exists());
}

#[test]
fn mise_only_does_not_enable_bash_without_a_recorded_shell() {
    let runner = FakeRunner::available(["apt", "dpkg-query", "run0", "apt-get"]);
    let mut fixture = InstallFixture::new(Platform::Linux);
    fs::write(fixture.target(".bashrc"), b"unmanaged bash profile").unwrap();
    for _ in 0..2 {
        install(
            InstallRequest::explicit(["mise"], true, Platform::Linux),
            test_context(&mut fixture, &runner),
        )
        .unwrap();
        assert!(!fixture.target(".config/bashrc.d/10-mise.bash").exists());
        let state = ModuleState::load(&fixture.ops.state_path().unwrap()).unwrap();
        assert!(state.contains("mise"));
        assert!(!state.contains("bash"));
    }
    assert_eq!(
        fs::read(fixture.target(".bashrc")).unwrap(),
        b"unmanaged bash profile"
    );
}

#[test]
fn selecting_mise_with_powershell_installs_its_activation_dropin() {
    let runner = FakeRunner::available(["winget"]);
    let mut fixture = InstallFixture::new(Platform::Windows);
    install(
        InstallRequest::explicit(["mise", "powershell"], true, Platform::Windows),
        test_context(&mut fixture, &runner),
    )
    .unwrap();

    assert!(fixture
        .target("Documents/PowerShell/Microsoft.PowerShell_profile.ps1")
        .is_file());
    assert_eq!(
        fs::read(fixture.target("Documents/PowerShell/Profile.d/10-mise.ps1")).unwrap(),
        include_bytes!("../modules/mise/powershell/10-mise.ps1")
    );
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
fn shell_prerequisites_are_not_implicitly_resolved_but_order_explicit_modules() {
    let runner = FakeRunner::available(["apt", "dpkg-query", "run0", "apt-get"]);
    let resolved = resolve_modules(&["oh-my-posh".into()], Platform::Linux, &runner).unwrap();
    assert_eq!(
        resolved.iter().map(|module| module.id).collect::<Vec<_>>(),
        ["oh-my-posh"]
    );

    let resolved = resolve_modules(
        &["oh-my-posh".into(), "bash".into()],
        Platform::Linux,
        &runner,
    )
    .unwrap();
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
    let git = *dotfiles::registry::find_module("git").unwrap();
    let modules = [ModuleSpec {
        packages: UNAVAILABLE_GIT_PACKAGES,
        ..git
    }];
    let error = install_with_registry(
        InstallRequest::explicit(["git"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
        &modules,
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
    assert!(!fixture.ops.state_path().unwrap().parent().unwrap().exists());
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
        assert!(text.contains("zsh - Zsh profile"));
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
        setup: None,
        secret_request: None,
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
        setup: None,
        secret_request: None,
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
    let request = InstallRequest::explicit(["atuin"], true, Platform::Linux);
    let result = install(request.clone(), test_context(&mut fixture, &runner));

    match old_identity {
        Some(value) => std::env::set_var("AGE_IDENTITY", value),
        None => std::env::remove_var("AGE_IDENTITY"),
    }
    match old_identities {
        Some(value) => std::env::set_var("AGE_IDENTITIES", value),
        None => std::env::remove_var("AGE_IDENTITIES"),
    }
    result.unwrap();
    runner.mark_installed("apt", "atuin");
    install(request, test_context(&mut fixture, &runner)).unwrap();
    assert_eq!(fixture.terminal.secret_reads(), 0);
    assert!(runner
        .calls()
        .iter()
        .all(|call| call.args.first().map(String::as_str) != Some("login")));
}

#[test]
fn atuin_status_failure_stops_before_mapping_or_state_recording() {
    let runner = FakeRunner::available(["atuin", "apt", "dpkg-query", "run0", "apt-get"])
        .failing_program("atuin");
    let mut fixture = InstallFixture::new(Platform::Linux);
    let result = install(
        InstallRequest::explicit(["atuin"], true, Platform::Linux),
        test_context(&mut fixture, &runner),
    );

    assert!(result.is_err());
    assert!(!fixture.ops.state_path().unwrap().exists());
    assert!(!fixture.target(".config/bashrc.d/50-atuin.bash").exists());
    assert_eq!(fixture.terminal.secret_reads(), 0);
}

#[test]
fn optional_opencode_secrets_leave_provider_configuration_unchanged() {
    let _lock = environment_lock();
    let runner = FakeRunner::available(["brew"]);
    let mut fixture = InstallFixture::new(Platform::Linux);
    let _config_home = support::ScopedXdgConfigHome::new(fixture.target(".config"));
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
    assert_eq!(fixture.terminal.secret_reads(), 1);
    let config = fixture.target(".config/opencode/opencode.json");
    let text = fs::read_to_string(config).unwrap();
    assert!(!text.contains("provider"));
}
