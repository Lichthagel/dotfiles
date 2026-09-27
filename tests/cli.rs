use std::collections::BTreeMap;

use clap::Parser;
use dotfiles::cli::{
    parse_modules, select_modules, select_packages, validate_modules, Cli, CrosstermTerminal,
    PackageSelection, TerminalIo, TerminalKey,
};
use dotfiles::error::{ExitCategory, InstallError};
use dotfiles::model::{PackageManager, Platform};
use dotfiles::packages::{PackageAction, PackageCandidate, PackagePlan};
use dotfiles::registry;

#[derive(Default)]
struct FakeTerminal {
    interactive: bool,
    keys: Vec<TerminalKey>,
    output: String,
}

impl FakeTerminal {
    fn interactive(keys: impl IntoIterator<Item = TerminalKey>) -> Self {
        Self {
            interactive: true,
            keys: keys.into_iter().collect(),
            output: String::new(),
        }
    }

    fn noninteractive() -> Self {
        Self::default()
    }
}

impl TerminalIo for FakeTerminal {
    fn is_interactive(&self) -> bool {
        self.interactive
    }

    fn read_key(&mut self) -> dotfiles::error::Result<TerminalKey> {
        self.keys
            .is_empty()
            .then_some(TerminalKey::Escape)
            .or_else(|| self.keys.first().copied())
            .map(|key| {
                if !self.keys.is_empty() {
                    self.keys.remove(0);
                }
                key
            })
            .ok_or_else(|| InstallError::Usage("test terminal input ended".into()))
    }

    fn clear(&mut self) -> dotfiles::error::Result<()> {
        Ok(())
    }

    fn write_line(&mut self, line: &str) -> dotfiles::error::Result<()> {
        self.output.push_str(line);
        self.output.push('\n');
        Ok(())
    }
}

#[test]
fn canonical_and_compatibility_module_flags_parse_identically() {
    let canonical = Cli::try_parse_from(["dotfiles", "--modules", "git,bash", "--yes"]).unwrap();
    let alias = Cli::try_parse_from(["dotfiles", "--apps", "git,bash", "--yes"]).unwrap();
    assert_eq!(canonical.module_list(), alias.module_list());
    assert!(canonical.yes && alias.yes);
}

#[test]
fn invalid_module_lists_return_usage_errors() {
    let error = parse_modules("git,,bash").unwrap_err();
    assert_eq!(error.exit_category(), ExitCategory::Usage);
    assert!(parse_modules("").unwrap().is_empty());
    assert_eq!(parse_modules("git,bash").unwrap(), ["git", "bash"]);
}

#[test]
fn missing_modules_value_is_rejected_by_clap() {
    assert!(Cli::try_parse_from(["dotfiles", "--modules"]).is_err());
}

#[test]
fn duplicate_unknown_and_unsupported_modules_are_rejected() {
    let duplicate = parse_modules("git,git").unwrap_err();
    assert!(matches!(duplicate, InstallError::Usage(message) if message.contains("duplicate")));

    let unknown = validate_modules(&["missing".into()], Platform::Linux).unwrap_err();
    assert!(matches!(unknown, InstallError::Usage(message) if message.contains("unknown")));

    let unsupported = validate_modules(&["bash".into()], Platform::Windows).unwrap_err();
    assert!(
        matches!(unsupported, InstallError::Usage(message) if message.contains("not supported"))
    );
}

#[test]
fn default_module_selection_and_enter_confirm_selected_defaults() {
    let modules = registry::modules()
        .iter()
        .filter(|module| module.supports(Platform::Linux))
        .collect::<Vec<_>>();
    let mut terminal = FakeTerminal::interactive([TerminalKey::Enter]);
    let selected = select_modules(&modules, &mut terminal).unwrap();
    assert_eq!(selected, ["git", "bash", "mise"]);
    assert!(terminal.output.contains("git - Git configuration"));
}

#[test]
fn module_space_toggles_and_escape_cancels() {
    let modules = registry::modules()
        .iter()
        .filter(|module| module.supports(Platform::Linux))
        .collect::<Vec<_>>();
    let mut terminal = FakeTerminal::interactive([TerminalKey::Space, TerminalKey::Escape]);
    assert!(matches!(
        select_modules(&modules, &mut terminal),
        Err(InstallError::Cancelled)
    ));
}

#[test]
fn redirected_module_selection_returns_terminal_error() {
    let modules = registry::modules().iter().collect::<Vec<_>>();
    let mut terminal = FakeTerminal::noninteractive();
    let error = select_modules(&modules, &mut terminal).unwrap_err();
    assert!(matches!(error, InstallError::Usage(message) if message.contains("--modules")));
}

#[test]
fn interactive_terminal_clears_screen_and_scrollback() {
    let mut output = Vec::new();
    let mut terminal = CrosstermTerminal::new(&mut output);
    terminal.clear().unwrap();
    assert_eq!(output, b"\x1b[2J\x1b[3J\x1b[H");
}

#[test]
fn interactive_terminal_starts_each_line_at_column_zero() {
    let mut output = Vec::new();
    let mut terminal = CrosstermTerminal::new(&mut output);
    terminal.write_line("first").unwrap();
    terminal.write_line("second").unwrap();
    assert_eq!(output, b"first\r\nsecond\r\n");
}

fn package_plan() -> PackagePlan {
    PackagePlan {
        actions: vec![PackageAction {
            logical: "git".into(),
            module: "git".into(),
            manager: PackageManager::Apt,
            package_name: "git".into(),
            selected: true,
        }],
        alternatives: BTreeMap::from([(
            "git".into(),
            vec![
                PackageCandidate {
                    manager: PackageManager::Apt,
                    package_name: "git".into(),
                },
                PackageCandidate {
                    manager: PackageManager::Brew,
                    package_name: "git".into(),
                },
            ],
        )]),
    }
}

#[test]
fn package_selector_changes_manager_toggles_and_confirms() {
    let mut plan = package_plan();
    let mut terminal =
        FakeTerminal::interactive([TerminalKey::Right, TerminalKey::Space, TerminalKey::Enter]);
    assert_eq!(
        select_packages(&mut plan, &mut terminal).unwrap(),
        PackageSelection::Continue
    );
    assert_eq!(plan.actions[0].manager, PackageManager::Brew);
    assert!(!plan.actions[0].selected);
}

#[test]
fn package_selector_supports_back_and_cancel() {
    let mut plan = package_plan();
    let mut back = FakeTerminal::interactive([TerminalKey::Char('b')]);
    assert_eq!(
        select_packages(&mut plan, &mut back).unwrap(),
        PackageSelection::Back
    );

    let mut cancel = FakeTerminal::interactive([TerminalKey::Escape]);
    assert_eq!(
        select_packages(&mut plan, &mut cancel).unwrap(),
        PackageSelection::Cancel
    );
}

#[test]
fn run_lists_supported_modules_and_empty_selection_is_a_noop() {
    let mut output = Vec::new();
    dotfiles::run(["dotfiles", "--list"], &mut output).unwrap();
    let listed = String::from_utf8(output).unwrap();
    assert!(listed.contains("git - Git configuration"));
    if cfg!(windows) {
        assert!(listed.contains("powershell - PowerShell profile"));
    } else {
        assert!(listed.contains("bash - Bash profile"));
    }

    let mut output = Vec::new();
    dotfiles::run(["dotfiles", "--modules", ""], &mut output).unwrap();
    assert_eq!(
        String::from_utf8(output).unwrap(),
        "No applications selected.\n"
    );
}

#[test]
fn run_rejects_invalid_explicit_modules_before_listing() {
    let mut output = Vec::new();
    let error =
        dotfiles::run(["dotfiles", "--list", "--modules", "unknown"], &mut output).unwrap_err();
    assert_eq!(error.exit_category(), ExitCategory::Usage);
    assert!(output.is_empty());
}

#[test]
fn run_redirected_without_modules_requires_a_terminal() {
    let mut output = Vec::new();
    let error = dotfiles::run(["dotfiles"], &mut output).unwrap_err();
    assert!(matches!(error, InstallError::Usage(message) if message.contains("--modules")));
}

#[test]
fn run_help_is_successful_and_unknown_option_is_usage_error() {
    let mut output = Vec::new();
    dotfiles::run(["dotfiles", "--help"], &mut output).unwrap();
    assert!(String::from_utf8(output).unwrap().contains("--modules"));

    let mut output = Vec::new();
    let error = dotfiles::run(["dotfiles", "--unknown"], &mut output).unwrap_err();
    assert_eq!(error.exit_category(), ExitCategory::Usage);
}
