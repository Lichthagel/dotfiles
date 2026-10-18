mod support;

use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use dotfiles::error::Result;
use dotfiles::model::{PackageManager, Platform};
use dotfiles::packages::{CommandOutput, CommandRunner, ManagerAdapter, PackageManagerAdapter};
use dotfiles::setup::providers::{install_homebrew, install_scoop};
use dotfiles::setup::{HttpClient, SetupContext};
use support::mappings::MappingFixture;

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

struct NoHttp;

impl HttpClient for NoHttp {
    fn get_json(&self, _url: &str) -> Result<serde_json::Value> {
        unreachable!("provider bootstraps do not use HTTP")
    }

    fn get_bytes(&self, _url: &str) -> Result<Vec<u8>> {
        unreachable!("provider bootstraps do not use HTTP")
    }
}

#[derive(Default)]
struct CloneRunner {
    calls: Mutex<Vec<(String, Vec<String>)>>,
    available: std::sync::Arc<Mutex<std::collections::BTreeSet<String>>>,
}

impl CloneRunner {
    fn new(available: &[&str]) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            available: std::sync::Arc::new(Mutex::new(
                available.iter().map(|value| (*value).into()).collect(),
            )),
        }
    }

    fn calls(&self) -> Vec<(String, Vec<String>)> {
        self.calls.lock().unwrap().clone()
    }
}

impl CommandRunner for CloneRunner {
    fn available(&self, program: &str) -> bool {
        self.available.lock().unwrap().contains(program)
            || (program == "brew"
                && std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                    .any(|entry| entry.join("brew").is_file()))
    }

    fn run(
        &self,
        program: &str,
        args: &[String],
        _env: &[(String, String)],
        _stdin: Option<&[u8]>,
    ) -> Result<CommandOutput> {
        self.calls
            .lock()
            .unwrap()
            .push((program.to_owned(), args.to_vec()));
        if program == "run0" && args.iter().any(|arg| arg == "git") {
            self.available.lock().unwrap().insert("git".into());
        }
        if program == "git" && args.first().map(String::as_str) == Some("clone") {
            let destination = PathBuf::from(args.last().expect("clone destination"));
            fs::create_dir_all(destination.join("bin")).unwrap();
            let brew = destination.join("bin/brew");
            fs::write(&brew, b"#!/bin/sh\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&brew, fs::Permissions::from_mode(0o755)).unwrap();
            }
            fs::write(destination.join("bin/scoop.ps1"), b"Write-Output scoop\n").unwrap();
            if destination.join("bin/brew").is_file() {
                self.available.lock().unwrap().insert("brew".into());
            }
        }
        Ok(CommandOutput {
            status: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }
}

fn context<'a>(
    fixture: &'a MappingFixture,
    runner: &'a dyn CommandRunner,
    http: &'a NoHttp,
) -> SetupContext<'a> {
    SetupContext {
        platform: &fixture.ops,
        runner,
        http,
        secrets: None,
    }
}

#[test]
fn existing_homebrew_needs_no_bootstrap() {
    let runner = CloneRunner::new(&["brew"]);
    let fixture = MappingFixture::with_platform(Platform::Linux);
    let http = NoHttp;

    install_homebrew(&context(&fixture, &runner, &http)).unwrap();
    assert!(runner.calls().is_empty());
}

#[test]
fn existing_homebrew_install_is_rediscovered_and_path_is_not_duplicated() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let prefix_root = tempfile::tempdir().unwrap();
    let prefix = prefix_root.path().join("linuxbrew");
    fs::create_dir_all(prefix.join("bin")).unwrap();
    fs::create_dir_all(prefix.join("sbin")).unwrap();
    fs::write(prefix.join("bin/brew"), b"brew").unwrap();
    let old_prefix = std::env::var_os("DOTFILES_HOMEBREW_PREFIX");
    let old_path = std::env::var_os("PATH");
    std::env::set_var("DOTFILES_HOMEBREW_PREFIX", &prefix);
    std::env::set_var("PATH", "/usr/bin:/usr/bin");

    let runner = CloneRunner::new(&["git"]);
    let fixture = MappingFixture::with_platform(Platform::Linux);
    let http = NoHttp;
    install_homebrew(&context(&fixture, &runner, &http)).unwrap();
    let first_path = std::env::split_paths(&std::env::var_os("PATH").unwrap()).collect::<Vec<_>>();
    install_homebrew(&context(&fixture, &runner, &http)).unwrap();
    let second_path = std::env::split_paths(&std::env::var_os("PATH").unwrap()).collect::<Vec<_>>();

    assert_eq!(first_path, second_path);
    assert_eq!(first_path[0], prefix.join("bin"));
    assert_eq!(first_path[1], prefix.join("sbin"));
    assert_eq!(
        first_path[2..],
        [PathBuf::from("/usr/bin"), PathBuf::from("/usr/bin")]
    );
    assert_eq!(runner.calls().len(), 1);
    assert_eq!(runner.calls()[0], ("brew".into(), vec!["--version".into()]));

    restore_env("DOTFILES_HOMEBREW_PREFIX", old_prefix);
    restore_env("PATH", old_path);
}

#[test]
fn invalid_existing_homebrew_root_is_not_overwritten_or_cloned_into() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let prefix_root = tempfile::tempdir().unwrap();
    let prefix = prefix_root.path().join("linuxbrew");
    fs::create_dir_all(&prefix).unwrap();
    fs::write(prefix.join("keep"), b"user data").unwrap();
    let old_prefix = std::env::var_os("DOTFILES_HOMEBREW_PREFIX");
    std::env::set_var("DOTFILES_HOMEBREW_PREFIX", &prefix);

    let runner = CloneRunner::new(&["git"]);
    let fixture = MappingFixture::with_platform(Platform::Linux);
    let http = NoHttp;
    assert!(install_homebrew(&context(&fixture, &runner, &http)).is_err());
    assert!(runner.calls().is_empty());
    assert_eq!(fs::read(prefix.join("keep")).unwrap(), b"user data");

    restore_env("DOTFILES_HOMEBREW_PREFIX", old_prefix);
}

#[test]
fn homebrew_bootstrap_clones_repository_and_updates_path() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let prefix_root = tempfile::tempdir().unwrap();
    let prefix = prefix_root.path().join("linuxbrew");
    let old_prefix = std::env::var_os("DOTFILES_HOMEBREW_PREFIX");
    let old_path = std::env::var_os("PATH");
    std::env::set_var("DOTFILES_HOMEBREW_PREFIX", &prefix);
    std::env::set_var("PATH", "/usr/bin");

    let runner = CloneRunner::new(&["git"]);
    let fixture = MappingFixture::with_platform(Platform::Linux);
    let http = NoHttp;
    install_homebrew(&context(&fixture, &runner, &http)).unwrap();

    let calls = runner.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, "git");
    assert_eq!(
        calls[0].1[..4],
        ["clone", "--depth", "1", "https://github.com/Homebrew/brew"]
    );
    assert_eq!(PathBuf::from(&calls[0].1[4]), prefix);
    assert_eq!(calls[1].0, "brew");
    assert_eq!(calls[1].1, vec!["--version"]);
    let path = std::env::var_os("PATH").unwrap();
    let paths = std::env::split_paths(&path).collect::<Vec<_>>();
    assert_eq!(paths[0], prefix.join("bin"));
    assert_eq!(paths[1], prefix.join("sbin"));

    restore_env("DOTFILES_HOMEBREW_PREFIX", old_prefix);
    restore_env("PATH", old_path);
}

#[test]
fn homebrew_bootstrap_installs_git_with_an_available_system_manager() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let prefix_root = tempfile::tempdir().unwrap();
    let prefix = prefix_root.path().join("linuxbrew");
    let old_prefix = std::env::var_os("DOTFILES_HOMEBREW_PREFIX");
    let old_path = std::env::var_os("PATH");
    std::env::set_var("DOTFILES_HOMEBREW_PREFIX", &prefix);
    std::env::set_var("PATH", "/usr/bin");

    let runner = CloneRunner::new(&["apt", "dpkg-query", "run0", "apt-get"]);
    let fixture = MappingFixture::with_platform(Platform::Linux);
    let http = NoHttp;
    install_homebrew(&context(&fixture, &runner, &http)).unwrap();

    let calls = runner.calls();
    assert_eq!(calls[0].0, "dpkg-query");
    assert_eq!(calls[1].0, "run0");
    assert!(calls[1].1.iter().any(|arg| arg == "git"));
    assert_eq!(calls[2].0, "git");

    restore_env("DOTFILES_HOMEBREW_PREFIX", old_prefix);
    restore_env("PATH", old_path);
}

#[test]
fn existing_scoop_needs_no_bootstrap() {
    let runner = CloneRunner::new(&["scoop"]);
    let fixture = MappingFixture::with_platform(Platform::Windows);
    let http = NoHttp;

    install_scoop(&context(&fixture, &runner, &http)).unwrap();
    assert!(runner.calls().is_empty());
}

#[test]
fn scoop_bootstrap_and_package_adapter_share_dotfiles_home_root() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let fixture_root = tempfile::tempdir().unwrap();
    let home = fixture_root.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let old_home = std::env::var_os("DOTFILES_HOME");
    let old_scoop_root = std::env::var_os("DOTFILES_SCOOP_ROOT");
    let old_scoop = std::env::var_os("SCOOP");
    let old_path = std::env::var_os("PATH");
    std::env::set_var("DOTFILES_HOME", &home);
    std::env::remove_var("DOTFILES_SCOOP_ROOT");
    std::env::remove_var("SCOOP");
    std::env::set_var("PATH", r"C:\\Windows\\System32");

    let runner = CloneRunner::new(&["git", "pwsh"]);
    let fixture = MappingFixture::with_platform(Platform::Windows);
    let http = NoHttp;
    install_scoop(&context(&fixture, &runner, &http)).unwrap();

    let expected_root = home.join("scoop");
    assert!(expected_root
        .join("apps/scoop/current/bin/scoop.ps1")
        .is_file());
    assert!(
        PackageManagerAdapter::new(Platform::Windows, PackageManager::Scoop).available(&runner)
    );

    restore_env("DOTFILES_HOME", old_home);
    restore_env("DOTFILES_SCOOP_ROOT", old_scoop_root);
    restore_env("SCOOP", old_scoop);
    restore_env("PATH", old_path);
}

#[test]
fn existing_scoop_install_is_rediscovered_and_path_is_not_duplicated() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let root = tempfile::tempdir().unwrap();
    let current = root.path().join("apps/scoop/current");
    fs::create_dir_all(current.join("bin")).unwrap();
    fs::write(current.join("bin/scoop.ps1"), b"Write-Output scoop\n").unwrap();
    let old_root = std::env::var_os("DOTFILES_SCOOP_ROOT");
    let old_path = std::env::var_os("PATH");
    std::env::set_var("DOTFILES_SCOOP_ROOT", root.path());
    std::env::set_var("PATH", "/usr/bin");

    let runner = CloneRunner::new(&["git", "pwsh"]);
    let fixture = MappingFixture::with_platform(Platform::Windows);
    let http = NoHttp;
    install_scoop(&context(&fixture, &runner, &http)).unwrap();
    let first_path = std::env::split_paths(&std::env::var_os("PATH").unwrap()).collect::<Vec<_>>();
    install_scoop(&context(&fixture, &runner, &http)).unwrap();
    let second_path = std::env::split_paths(&std::env::var_os("PATH").unwrap()).collect::<Vec<_>>();

    assert_eq!(first_path, second_path);
    assert_eq!(first_path[0], root.path().join("shims"));
    assert_eq!(runner.calls().len(), 2);
    assert!(runner.calls().iter().all(|(program, args)| {
        program == "pwsh" && args.last().is_some_and(|arg| arg == "--version")
    }));

    restore_env("DOTFILES_SCOOP_ROOT", old_root);
    restore_env("PATH", old_path);
}

#[test]
fn invalid_existing_scoop_installation_is_not_overwritten() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let root = tempfile::tempdir().unwrap();
    let current = root.path().join("apps/scoop/current");
    fs::create_dir_all(&current).unwrap();
    fs::write(current.join("keep"), b"user data").unwrap();
    let old_root = std::env::var_os("DOTFILES_SCOOP_ROOT");
    std::env::set_var("DOTFILES_SCOOP_ROOT", root.path());

    let runner = CloneRunner::new(&["git", "pwsh"]);
    let fixture = MappingFixture::with_platform(Platform::Windows);
    let http = NoHttp;
    assert!(install_scoop(&context(&fixture, &runner, &http)).is_err());
    assert!(runner.calls().is_empty());
    assert_eq!(fs::read(current.join("keep")).unwrap(), b"user data");

    restore_env("DOTFILES_SCOOP_ROOT", old_root);
}

#[test]
fn scoop_bootstrap_clones_repository_creates_layout_and_updates_path() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let root = tempfile::tempdir().unwrap();
    let old_root = std::env::var_os("DOTFILES_SCOOP_ROOT");
    let old_path = std::env::var_os("PATH");
    std::env::set_var("DOTFILES_SCOOP_ROOT", root.path());
    std::env::set_var("PATH", r"C:\Windows\System32");

    let runner = CloneRunner::new(&["git", "pwsh"]);
    let fixture = MappingFixture::with_platform(Platform::Windows);
    let http = NoHttp;
    install_scoop(&context(&fixture, &runner, &http)).unwrap();

    let calls = runner.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, "git");
    assert_eq!(
        calls[0].1[..4],
        [
            "clone",
            "--depth",
            "1",
            "https://github.com/ScoopInstaller/Scoop"
        ]
    );
    let current = root.path().join("apps/scoop/current");
    assert_eq!(calls[1].0, "pwsh");
    assert_eq!(calls[1].1[0], "-NoProfile");
    assert_eq!(PathBuf::from(&calls[0].1[4]), current);
    for directory in ["shims", "apps", "buckets", "cache", "persist"] {
        assert!(root.path().join(directory).is_dir(), "missing {directory}");
    }
    assert!(current.join("bin/scoop.ps1").is_file());
    let paths = std::env::split_paths(&std::env::var_os("PATH").unwrap()).collect::<Vec<_>>();
    assert_eq!(paths[0], root.path().join("shims"));
    assert!(
        PackageManagerAdapter::new(Platform::Windows, PackageManager::Scoop).available(&runner)
    );

    restore_env("DOTFILES_SCOOP_ROOT", old_root);
    restore_env("PATH", old_path);
}

fn restore_env(name: &str, value: Option<std::ffi::OsString>) {
    match value {
        Some(value) => std::env::set_var(name, value),
        None => std::env::remove_var(name),
    }
}
