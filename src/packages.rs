use std::collections::BTreeMap;
use std::env;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::{InstallError, Result};
use crate::model::{PackageManager, Platform};
use crate::platform::package_manager_program;
use crate::registry::ModuleSpec;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandOutput {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub trait CommandRunner {
    fn available(&self, program: &str) -> bool;

    fn run(
        &self,
        program: &str,
        args: &[String],
        env: &[(String, String)],
        stdin: Option<&[u8]>,
    ) -> Result<CommandOutput>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn available(&self, program: &str) -> bool {
        find_in_path(program)
    }

    fn run(
        &self,
        program: &str,
        args: &[String],
        environment: &[(String, String)],
        stdin: Option<&[u8]>,
    ) -> Result<CommandOutput> {
        let mut command = Command::new(program);
        command.args(args).envs(environment.iter().cloned());
        if stdin.is_some() {
            command.stdin(Stdio::piped());
        }
        let mut child = command.spawn().map_err(|error| {
            InstallError::Package(format!("could not start {program}: {error}"))
        })?;
        if let Some(input) = stdin {
            if let Some(mut pipe) = child.stdin.take() {
                use std::io::Write;
                pipe.write_all(input).map_err(|error| {
                    InstallError::Package(format!("could not send input to {program}: {error}"))
                })?;
            }
        }
        let output = child.wait_with_output().map_err(|error| {
            InstallError::Package(format!("could not wait for {program}: {error}"))
        })?;
        Ok(CommandOutput {
            status: output.status.code().unwrap_or(1),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

fn find_in_path(program: &str) -> bool {
    let path = Path::new(program);
    if path.components().count() > 1 {
        return path.is_file();
    }
    let Some(path_value) = env::var_os("PATH") else {
        return false;
    };
    for directory in env::split_paths(&path_value) {
        if directory.join(program).is_file() {
            return true;
        }
        #[cfg(windows)]
        {
            let extensions = env::var_os("PATHEXT")
                .map(|value| {
                    env::split_paths(&value)
                        .map(|path| path.to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| vec![".EXE".into(), ".CMD".into(), ".BAT".into()]);
            if extensions
                .iter()
                .any(|extension| directory.join(format!("{program}{extension}")).is_file())
            {
                return true;
            }
        }
    }
    false
}

pub trait ManagerAdapter {
    fn manager(&self) -> PackageManager;
    fn available(&self, runner: &dyn CommandRunner) -> bool;
    fn installed(&self, runner: &dyn CommandRunner, package_name: &str) -> Result<bool>;
    fn install(&self, runner: &dyn CommandRunner, package_name: &str) -> Result<()>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageManagerAdapter {
    platform: Platform,
    manager: PackageManager,
}

impl PackageManagerAdapter {
    pub const fn new(platform: Platform, manager: PackageManager) -> Self {
        Self { platform, manager }
    }

    fn program(self) -> Result<&'static str> {
        package_manager_program(self.platform, self.manager).ok_or_else(|| {
            InstallError::Package(format!(
                "{} is not supported on {:?}",
                self.manager.label(),
                self.platform
            ))
        })
    }

    fn query(self, runner: &dyn CommandRunner, args: &[String]) -> Result<CommandOutput> {
        let program = match self.manager {
            PackageManager::Apt => "dpkg-query",
            PackageManager::Dnf => "rpm",
            PackageManager::Pacman => "pacman",
            PackageManager::Brew => "brew",
            PackageManager::Mise => "mise",
            PackageManager::Winget => "winget",
            PackageManager::Scoop => "scoop",
        };
        runner.run(program, args, &[], None)
    }
}

impl ManagerAdapter for PackageManagerAdapter {
    fn manager(&self) -> PackageManager {
        self.manager
    }

    fn available(&self, runner: &dyn CommandRunner) -> bool {
        self.program()
            .map(|program| runner.available(program))
            .unwrap_or(false)
    }

    fn installed(&self, runner: &dyn CommandRunner, package_name: &str) -> Result<bool> {
        let args = match self.manager {
            PackageManager::Apt => vec!["-W".into(), "-f=${Status}".into(), package_name.into()],
            PackageManager::Dnf => vec!["-q".into(), package_name.into()],
            PackageManager::Pacman => vec!["-Q".into(), package_name.into()],
            PackageManager::Brew => vec!["list".into(), "--versions".into(), package_name.into()],
            PackageManager::Mise => vec!["list".into()],
            PackageManager::Winget => vec![
                "list".into(),
                "--id".into(),
                package_name.into(),
                "--exact".into(),
                "--accept-source-agreements".into(),
            ],
            PackageManager::Scoop => vec!["list".into()],
        };
        let output = self.query(runner, &args)?;
        if output.status == 0 {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Ok(match self.manager {
                PackageManager::Apt => stdout.contains("install ok installed"),
                PackageManager::Dnf | PackageManager::Pacman => true,
                PackageManager::Brew | PackageManager::Winget => {
                    stdout.lines().any(|line| line.contains(package_name))
                }
                PackageManager::Mise | PackageManager::Scoop => stdout.lines().any(|line| {
                    line.split_whitespace().next() == Some(package_name)
                        || line.contains(package_name)
                }),
            });
        }
        Ok(false)
    }

    fn install(&self, runner: &dyn CommandRunner, package_name: &str) -> Result<()> {
        self.program()?;
        let (program, args) = match self.manager {
            PackageManager::Apt => {
                let privilege = if runner.available("run0") {
                    "run0"
                } else if runner.available("sudo") {
                    "sudo"
                } else {
                    return Err(InstallError::Package(
                        "apt requires run0 or sudo for package installation".into(),
                    ));
                };
                (
                    privilege,
                    vec![
                        "apt-get".into(),
                        "install".into(),
                        "-y".into(),
                        package_name.into(),
                    ],
                )
            }
            PackageManager::Dnf => {
                let privilege = if runner.available("run0") {
                    "run0"
                } else if runner.available("sudo") {
                    "sudo"
                } else {
                    return Err(InstallError::Package(
                        "dnf requires run0 or sudo for package installation".into(),
                    ));
                };
                (
                    privilege,
                    vec![
                        "dnf".into(),
                        "install".into(),
                        "-y".into(),
                        package_name.into(),
                    ],
                )
            }
            PackageManager::Pacman => {
                let privilege = if runner.available("run0") {
                    "run0"
                } else if runner.available("sudo") {
                    "sudo"
                } else {
                    return Err(InstallError::Package(
                        "pacman requires run0 or sudo for package installation".into(),
                    ));
                };
                (
                    privilege,
                    vec![
                        "pacman".into(),
                        "-S".into(),
                        "--needed".into(),
                        "--noconfirm".into(),
                        package_name.into(),
                    ],
                )
            }
            PackageManager::Brew => ("brew", vec!["install".into(), package_name.into()]),
            PackageManager::Mise => (
                "mise",
                vec!["use".into(), "--global".into(), package_name.into()],
            ),
            PackageManager::Winget => (
                "winget",
                vec![
                    "install".into(),
                    "--id".into(),
                    package_name.into(),
                    "--exact".into(),
                    "--accept-source-agreements".into(),
                    "--accept-package-agreements".into(),
                ],
            ),
            PackageManager::Scoop => ("scoop", vec!["install".into(), package_name.into()]),
        };
        let output = runner.run(program, &args, &[], None)?;
        if output.status != 0 {
            let detail = String::from_utf8_lossy(&output.stderr);
            return Err(InstallError::Package(format!(
                "{} failed for {package_name}{}",
                self.manager.label(),
                if detail.is_empty() {
                    String::new()
                } else {
                    format!(": {detail}")
                }
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageAction {
    pub logical: String,
    pub module: String,
    pub manager: PackageManager,
    pub package_name: String,
    pub selected: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageCandidate {
    pub manager: PackageManager,
    pub package_name: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackagePlan {
    pub actions: Vec<PackageAction>,
    pub alternatives: BTreeMap<String, Vec<PackageCandidate>>,
}

impl PackagePlan {
    pub fn actions(&self) -> &[PackageAction] {
        &self.actions
    }

    pub fn actions_mut(&mut self) -> &mut [PackageAction] {
        &mut self.actions
    }
}

pub fn build_package_plan(
    runner: &dyn CommandRunner,
    platform: Platform,
    modules: &[&ModuleSpec],
    _noninteractive: bool,
) -> Result<PackagePlan> {
    let mut candidates_by_logical = BTreeMap::<String, Vec<PackageCandidate>>::new();
    let mut owners = BTreeMap::<String, String>::new();
    for module in modules {
        for package in module
            .packages
            .iter()
            .filter(|package| package.manager.is_supported_on(platform))
        {
            candidates_by_logical
                .entry(package.logical.to_owned())
                .or_default()
                .push(PackageCandidate {
                    manager: package.manager,
                    package_name: package.name.to_owned(),
                });
            owners
                .entry(package.logical.to_owned())
                .or_insert_with(|| module.id.to_owned());
        }
    }

    let mut alternatives = BTreeMap::new();
    let mut actions = Vec::new();
    for (logical, candidates) in candidates_by_logical {
        let available = candidates
            .into_iter()
            .filter(|candidate| {
                PackageManagerAdapter::new(platform, candidate.manager).available(runner)
            })
            .collect::<Vec<_>>();
        if available.is_empty() {
            return Err(InstallError::Package(format!(
                "no supported package manager is available for {logical}"
            )));
        }
        let installed = available.iter().try_fold(false, |installed, candidate| {
            if installed {
                return Ok(true);
            }
            PackageManagerAdapter::new(platform, candidate.manager)
                .installed(runner, &candidate.package_name)
        })?;
        alternatives.insert(logical.clone(), available.clone());
        if installed {
            continue;
        }
        let selected = PackageManager::priority(platform)
            .iter()
            .find_map(|manager| {
                available
                    .iter()
                    .find(|candidate| candidate.manager == *manager)
            })
            .ok_or_else(|| {
                InstallError::Package(format!(
                    "no prioritized package manager is available for {logical}"
                ))
            })?;
        actions.push(PackageAction {
            logical: logical.clone(),
            module: owners
                .get(&logical)
                .cloned()
                .unwrap_or_else(|| logical.clone()),
            manager: selected.manager,
            package_name: selected.package_name.clone(),
            selected: true,
        });
    }
    Ok(PackagePlan {
        actions,
        alternatives,
    })
}

pub fn install_package(
    runner: &dyn CommandRunner,
    platform: Platform,
    action: &PackageAction,
) -> Result<()> {
    let adapter = PackageManagerAdapter::new(platform, action.manager);
    adapter.install(runner, &action.package_name)
}

impl PackageManager {
    pub const fn is_supported_on(self, platform: Platform) -> bool {
        matches!(
            (platform, self),
            (
                Platform::Linux,
                Self::Apt | Self::Dnf | Self::Pacman | Self::Brew | Self::Mise
            ) | (
                Platform::Windows,
                Self::Winget | Self::Scoop | Self::Brew | Self::Mise
            )
        )
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Apt => "apt",
            Self::Dnf => "dnf",
            Self::Pacman => "pacman",
            Self::Brew => "brew",
            Self::Mise => "mise",
            Self::Winget => "winget",
            Self::Scoop => "scoop",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manager_support_matches_platforms() {
        assert!(PackageManager::Apt.is_supported_on(Platform::Linux));
        assert!(!PackageManager::Apt.is_supported_on(Platform::Windows));
        assert!(PackageManager::Winget.is_supported_on(Platform::Windows));
        assert!(!PackageManager::Winget.is_supported_on(Platform::Linux));
    }
}
