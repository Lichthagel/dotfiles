use std::collections::{BTreeSet, HashSet};
use std::sync::{Arc, Mutex};

use dotfiles::packages::{CommandOutput, CommandRunner};

#[derive(Clone, Debug)]
pub struct Invocation {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub stdin: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct FakeRunner {
    available: Arc<Mutex<BTreeSet<String>>>,
    installed: HashSet<(String, String)>,
    fail_programs: BTreeSet<String>,
    atuin_status: Option<bool>,
    calls: Arc<Mutex<Vec<Invocation>>>,
}

impl FakeRunner {
    pub fn available<I, S>(programs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            available: Arc::new(Mutex::new(programs.into_iter().map(Into::into).collect())),
            installed: HashSet::new(),
            fail_programs: BTreeSet::new(),
            atuin_status: None,
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn available_and_installed<I, S, J, M>(programs: I, installed: J) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
        J: IntoIterator<Item = (M, M)>,
        M: Into<String>,
    {
        Self {
            available: Arc::new(Mutex::new(programs.into_iter().map(Into::into).collect())),
            installed: installed
                .into_iter()
                .map(|(manager, package)| (manager.into(), package.into()))
                .collect(),
            fail_programs: BTreeSet::new(),
            atuin_status: None,
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn failing_program(mut self, program: &str) -> Self {
        self.fail_programs.insert(program.to_owned());
        self
    }

    pub fn atuin_not_logged_in() -> Self {
        let mut runner = Self::available(["atuin"]);
        runner.atuin_status = Some(false);
        runner
    }

    pub fn atuin_logged_in() -> Self {
        let mut runner = Self::available(["atuin"]);
        runner.atuin_status = Some(true);
        runner
    }

    pub fn add_available(&self, program: &str) {
        self.available.lock().unwrap().insert(program.to_owned());
    }

    pub fn calls(&self) -> Vec<Invocation> {
        self.calls.lock().unwrap().clone()
    }

    pub fn last_program(&self) -> String {
        self.calls()
            .last()
            .expect("recorded command")
            .program
            .clone()
    }

    pub fn last_args(&self) -> Vec<String> {
        self.calls().last().expect("recorded command").args.clone()
    }

    pub fn last_stdin(&self) -> Option<Vec<u8>> {
        self.calls().last().expect("recorded command").stdin.clone()
    }

    pub fn log_contains(&self, value: &str) -> bool {
        self.calls().iter().any(|call| {
            call.program.contains(value)
                || call.args.iter().any(|arg| arg.contains(value))
                || call
                    .env
                    .iter()
                    .any(|(name, env_value)| name.contains(value) || env_value.contains(value))
        })
    }

    fn installed_for(&self, manager: &str, package: &str) -> bool {
        self.installed
            .contains(&(manager.to_owned(), package.to_owned()))
    }

    fn installed_names(&self, manager: &str) -> Vec<String> {
        self.installed
            .iter()
            .filter(|(installed_manager, _)| installed_manager == manager)
            .map(|(_, package)| package.clone())
            .collect()
    }

    fn output_for_query(&self, program: &str, args: &[String]) -> CommandOutput {
        let manager = match program {
            "dpkg-query" => "apt",
            "rpm" => "dnf",
            "pacman" => "pacman",
            "brew" => "brew",
            "mise" => "mise",
            "winget" => "winget",
            "scoop" => "scoop",
            _ => "",
        };
        let is_list_query = matches!(program, "scoop" | "mise")
            || (program == "brew" && args.first().is_some_and(|arg| arg == "list"));
        let package = if program == "winget" {
            args.iter()
                .position(|arg| arg == "--id")
                .and_then(|index| args.get(index + 1))
                .map(String::as_str)
                .unwrap_or_default()
        } else {
            args.last().map(String::as_str).unwrap_or_default()
        };
        let installed = if is_list_query && program != "winget" {
            self.installed_names(manager)
        } else if self.installed_for(manager, package) {
            vec![package.to_owned()]
        } else {
            Vec::new()
        };
        if installed.is_empty() {
            return CommandOutput {
                status: 1,
                stdout: Vec::new(),
                stderr: Vec::new(),
            };
        }
        CommandOutput {
            status: 0,
            stdout: installed
                .into_iter()
                .map(|name| format!("{name} install ok installed\n"))
                .collect::<String>()
                .into_bytes(),
            stderr: Vec::new(),
        }
    }
}

impl CommandRunner for FakeRunner {
    fn available(&self, program: &str) -> bool {
        self.available.lock().unwrap().contains(program)
    }

    fn run(
        &self,
        program: &str,
        args: &[String],
        env: &[(String, String)],
        stdin: Option<&[u8]>,
    ) -> dotfiles::error::Result<CommandOutput> {
        self.calls.lock().unwrap().push(Invocation {
            program: program.to_owned(),
            args: args.to_vec(),
            env: env.to_vec(),
            stdin: stdin.map(ToOwned::to_owned),
        });

        let is_query = matches!(program, "dpkg-query" | "rpm" | "pacman")
            || (program == "winget" && args.first().is_some_and(|arg| arg == "list"))
            || (program == "scoop" && args.first().is_some_and(|arg| arg == "list"))
            || (program == "brew" && args.first().is_some_and(|arg| arg == "list"))
            || (program == "mise" && args.first().is_some_and(|arg| arg == "list"));
        if program == "atuin" && args.first().map(String::as_str) == Some("status") {
            return Ok(CommandOutput {
                status: if self.atuin_status.unwrap_or(false) {
                    0
                } else {
                    1
                },
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        }
        if is_query {
            return Ok(self.output_for_query(program, args));
        }
        if self.fail_programs.contains(program) {
            return Ok(CommandOutput {
                status: 1,
                stdout: Vec::new(),
                stderr: b"fake command failure".to_vec(),
            });
        }
        Ok(CommandOutput {
            status: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }
}
