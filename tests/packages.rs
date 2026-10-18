mod support;

use dotfiles::model::{PackageManager, Platform};
use dotfiles::packages::{build_package_plan, install_package, CommandRunner, PackageAction};
use dotfiles::registry::find_module;
use std::sync::Mutex;
use support::runner::FakeRunner;

struct RepositoryRunner {
    calls: Mutex<Vec<(String, Vec<String>)>>,
    rhel_release: Option<String>,
    repository_enabled: Mutex<bool>,
}

impl RepositoryRunner {
    fn new(rhel_release: Option<&str>) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            rhel_release: rhel_release.map(str::to_owned),
            repository_enabled: Mutex::new(false),
        }
    }

    fn calls(&self) -> Vec<(String, Vec<String>)> {
        self.calls.lock().unwrap().clone()
    }
}

impl CommandRunner for RepositoryRunner {
    fn available(&self, program: &str) -> bool {
        matches!(program, "apt" | "dnf" | "run0" | "rpm")
            || (program == "dnf5" && self.rhel_release.is_none())
    }

    fn run(
        &self,
        program: &str,
        args: &[String],
        _env: &[(String, String)],
        _stdin: Option<&[u8]>,
    ) -> dotfiles::error::Result<dotfiles::packages::CommandOutput> {
        self.calls
            .lock()
            .unwrap()
            .push((program.to_owned(), args.to_vec()));
        if program == "rpm" {
            return Ok(dotfiles::packages::CommandOutput {
                status: 0,
                stdout: self.rhel_release.clone().unwrap_or_default().into_bytes(),
                stderr: Vec::new(),
            });
        }

        let (command, command_args) = if program == "run0" {
            (args[0].as_str(), &args[1..])
        } else {
            (program, args)
        };
        let is_mise_install =
            command_args == ["install", "-y", "mise"] || command_args == ["install", "mise", "-y"];
        if matches!(command, "apt-get" | "dnf")
            && is_mise_install
            && !*self.repository_enabled.lock().unwrap()
        {
            return Ok(dotfiles::packages::CommandOutput {
                status: 1,
                stdout: Vec::new(),
                stderr: if command == "apt-get" {
                    b"E: Unable to locate package mise".to_vec()
                } else {
                    b"No match for argument: mise".to_vec()
                },
            });
        }
        if (command == "extrepo" && command_args == ["enable", "mise"])
            || (command == "dnf"
                && command_args
                    .first()
                    .is_some_and(|argument| argument == "copr")
                && command_args
                    .get(1)
                    .is_some_and(|argument| argument == "enable"))
            || command == "yum-config-manager"
        {
            *self.repository_enabled.lock().unwrap() = true;
        }
        Ok(dotfiles::packages::CommandOutput {
            status: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }
}

fn args_are(actual: &[String], expected: &[&str]) -> bool {
    actual
        .iter()
        .map(String::as_str)
        .eq(expected.iter().copied())
}

fn module(id: &str) -> &'static dotfiles::registry::ModuleSpec {
    find_module(id).expect("registry module")
}

fn action(manager: PackageManager, package_name: &str) -> PackageAction {
    PackageAction {
        logical: "test".to_owned(),
        module: "test".to_owned(),
        manager,
        package_name: package_name.to_owned(),
        selected: true,
    }
}

#[test]
fn linux_prefers_apt_and_run0_over_other_managers() {
    let runner = FakeRunner::available(["apt", "dnf", "brew", "run0"]);
    let plan = build_package_plan(&runner, Platform::Linux, &[module("git")], true).unwrap();
    assert_eq!(plan.actions()[0].manager, PackageManager::Apt);
    install_package(&runner, Platform::Linux, &plan.actions()[0]).unwrap();
    assert_eq!(runner.last_program(), "run0");
    assert_eq!(runner.last_args(), ["apt-get", "install", "-y", "git"]);
}

#[test]
fn linux_falls_back_to_sudo_when_run0_is_unavailable() {
    let runner = FakeRunner::available(["apt", "sudo"]);
    install_package(
        &runner,
        Platform::Linux,
        &action(PackageManager::Apt, "git"),
    )
    .unwrap();
    assert_eq!(runner.last_program(), "sudo");
    assert_eq!(runner.last_args(), ["apt-get", "install", "-y", "git"]);
}

#[test]
fn apt_enables_the_mise_extrepo_after_a_missing_package() {
    let runner = RepositoryRunner::new(None);
    install_package(
        &runner,
        Platform::Linux,
        &action(PackageManager::Apt, "mise"),
    )
    .unwrap();

    let calls = runner.calls();
    assert!(calls.iter().any(|(program, args)| {
        program == "run0" && args_are(args, &["apt-get", "install", "-y", "extrepo"])
    }));
    assert!(calls.iter().any(|(program, args)| {
        program == "run0" && args_are(args, &["extrepo", "enable", "mise"])
    }));
    assert!(calls
        .iter()
        .any(|(program, args)| program == "run0" && args_are(args, &["apt-get", "update"])));
    assert_eq!(
        calls
            .iter()
            .filter(|(program, args)| {
                program == "run0" && args_are(args, &["apt-get", "install", "-y", "mise"])
            })
            .count(),
        2
    );
}

#[test]
fn dnf_enables_the_mise_copr_for_fedora_and_rhel_nine() {
    for (release, expected) in [
        (None, vec!["dnf", "copr", "enable", "-y", "jdxcode/mise"]),
        (
            Some("9"),
            vec![
                "dnf",
                "copr",
                "enable",
                "-y",
                "jdxcode/mise",
                "centos-stream+epel-next-9",
            ],
        ),
    ] {
        let runner = RepositoryRunner::new(release);
        install_package(
            &runner,
            Platform::Linux,
            &action(PackageManager::Dnf, "mise"),
        )
        .unwrap();
        let calls = runner.calls();
        assert!(calls
            .iter()
            .any(|(program, args)| program == "run0" && args_are(args, &expected)));
        let plugins = if release.is_none() {
            "dnf5-plugins"
        } else {
            "dnf-plugins-core"
        };
        assert!(calls.iter().any(|(program, args)| {
            program == "run0" && args_are(args, &["dnf", "install", "-y", plugins])
        }));
    }
}

#[test]
fn dnf_uses_the_mise_rpm_repo_on_rhel_eight() {
    let runner = RepositoryRunner::new(Some("8"));
    install_package(
        &runner,
        Platform::Linux,
        &action(PackageManager::Dnf, "mise"),
    )
    .unwrap();

    assert!(runner.calls().iter().any(|(program, args)| {
        program == "run0" && args_are(args, &["dnf", "install", "-y", "yum-utils"])
    }));
    assert!(runner.calls().iter().any(|(program, args)| {
        program == "run0"
            && args_are(
                args,
                &[
                    "yum-config-manager",
                    "--add-repo",
                    "https://mise.jdx.dev/rpm/mise.repo",
                ],
            )
    }));
}

#[test]
fn installed_logical_packages_are_not_planned() {
    let runner = FakeRunner::available_and_installed(["winget"], [("winget", "Git.Git")]);
    let plan = build_package_plan(&runner, Platform::Windows, &[module("git")], true).unwrap();
    assert!(plan.actions().is_empty());
}

#[test]
fn no_available_candidate_lists_supported_managers() {
    let runner = FakeRunner::available(std::iter::empty::<&str>());
    let error = build_package_plan(&runner, Platform::Linux, &[module("git")], true)
        .unwrap_err()
        .to_string();
    assert!(error.contains("supported managers: apt, dnf, pacman, brew, mise"));
}

#[test]
fn near_matches_do_not_suppress_mise_or_scoop_packages() {
    for (platform, manager) in [
        (Platform::Linux, PackageManager::Mise),
        (Platform::Windows, PackageManager::Scoop),
    ] {
        let runner =
            FakeRunner::available_and_installed([manager.label()], [(manager.label(), "git-lfs")]);
        let plan = build_package_plan(&runner, platform, &[module("git")], true).unwrap();
        assert_eq!(plan.actions()[0].manager, manager);
        assert_eq!(plan.actions()[0].package_name, "git");
    }
}

#[test]
fn unavailable_platform_managers_are_ignored() {
    let runner = FakeRunner::available(["brew"]);
    let plan = build_package_plan(&runner, Platform::Linux, &[module("git")], true).unwrap();
    assert_eq!(plan.actions()[0].manager, PackageManager::Brew);
    assert_eq!(
        plan.alternatives["git"]
            .iter()
            .map(|candidate| candidate.manager)
            .collect::<Vec<_>>(),
        vec![PackageManager::Brew]
    );
}

#[test]
fn noninteractive_plans_select_every_action() {
    let runner = FakeRunner::available(["apt"]);
    let plan = build_package_plan(&runner, Platform::Linux, &[module("git")], true).unwrap();
    assert!(plan.actions().iter().all(|action| action.selected));
}

#[test]
fn each_manager_uses_its_native_install_command() {
    let cases = [
        (
            Platform::Linux,
            PackageManager::Dnf,
            "dnf",
            vec!["dnf", "install", "-y", "pkg"],
        ),
        (
            Platform::Linux,
            PackageManager::Pacman,
            "pacman",
            vec!["pacman", "-S", "--needed", "--noconfirm", "pkg"],
        ),
        (
            Platform::Linux,
            PackageManager::Brew,
            "brew",
            vec!["install", "pkg"],
        ),
        (
            Platform::Linux,
            PackageManager::Mise,
            "mise",
            vec!["use", "--global", "pkg"],
        ),
        (
            Platform::Windows,
            PackageManager::Winget,
            "winget",
            vec![
                "install",
                "--id",
                "pkg",
                "--exact",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ],
        ),
        (
            Platform::Windows,
            PackageManager::Scoop,
            "scoop",
            vec!["install", "pkg"],
        ),
    ];

    for (platform, manager, executable, expected) in cases {
        let runner = if matches!(manager, PackageManager::Dnf | PackageManager::Pacman) {
            FakeRunner::available([executable, "run0"])
        } else {
            FakeRunner::available([executable])
        };
        install_package(&runner, platform, &action(manager, "pkg")).unwrap();
        let expected_program = if matches!(manager, PackageManager::Dnf | PackageManager::Pacman) {
            "run0"
        } else {
            executable
        };
        assert_eq!(runner.last_program(), expected_program);
        assert_eq!(runner.last_args(), expected);
    }
}

#[test]
fn unsupported_manager_returns_an_error() {
    let runner = FakeRunner::available(["apt"]);
    let error = install_package(
        &runner,
        Platform::Linux,
        &action(PackageManager::Winget, "pkg"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("not supported"));
}

#[cfg(windows)]
#[test]
fn system_runner_invokes_cmd_scripts_for_scoop() {
    use std::sync::{Mutex, OnceLock};

    static PATH_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = PATH_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let fixture = tempfile::tempdir().unwrap();
    let log = fixture.path().join("scoop.log");
    let script = fixture.path().join("scoop.cmd");
    std::fs::write(
        &script,
        format!("@echo off\r\necho %*>\"{}\"\r\n", log.display()),
    )
    .unwrap();
    let old_path = std::env::var_os("PATH");
    let mut paths = vec![fixture.path().to_path_buf()];
    if let Some(old_path) = &old_path {
        paths.extend(std::env::split_paths(old_path));
    }
    std::env::set_var("PATH", std::env::join_paths(paths).unwrap());

    let result = install_package(
        &dotfiles::packages::SystemCommandRunner,
        Platform::Windows,
        &action(PackageManager::Scoop, "pkg"),
    );

    if let Some(old_path) = old_path {
        std::env::set_var("PATH", old_path);
    } else {
        std::env::remove_var("PATH");
    }
    result.unwrap();
    assert_eq!(std::fs::read_to_string(log).unwrap().trim(), "install pkg");
}

#[cfg(unix)]
#[test]
fn system_runner_rejects_non_executable_direct_paths_on_unix() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = tempfile::tempdir().unwrap();
    let path = fixture.path().join("manager");
    std::fs::write(&path, b"#!/bin/sh\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(!dotfiles::packages::SystemCommandRunner.available(path.to_str().unwrap()));
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(dotfiles::packages::SystemCommandRunner.available(path.to_str().unwrap()));
}

#[cfg(windows)]
#[test]
fn system_runner_rejects_non_executable_direct_paths_on_windows() {
    let fixture = tempfile::tempdir().unwrap();
    let text_path = fixture.path().join("manager.txt");
    let cmd_path = fixture.path().join("manager.cmd");
    std::fs::write(&text_path, b"not executable").unwrap();
    std::fs::write(&cmd_path, b"@echo off\n").unwrap();
    assert!(!dotfiles::packages::SystemCommandRunner.available(text_path.to_str().unwrap()));
    assert!(dotfiles::packages::SystemCommandRunner.available(cmd_path.to_str().unwrap()));
}

#[test]
fn package_manager_failure_includes_package_name() {
    let runner = FakeRunner::available(["apt", "run0"]).failing_program("run0");
    let error = install_package(
        &runner,
        Platform::Linux,
        &action(PackageManager::Apt, "pkg"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("pkg"));
}
