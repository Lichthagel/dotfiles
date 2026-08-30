use std::io::{self, IsTerminal, Write};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::error::{InstallError, Result};
use crate::model::{PackageManager, Platform};
use crate::packages::PackagePlan;
use crate::registry::{self, ModuleSpec};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalKey {
    Up,
    Down,
    Left,
    Right,
    Space,
    Enter,
    Escape,
    Char(char),
}

pub trait TerminalIo {
    fn is_interactive(&self) -> bool;
    fn read_key(&mut self) -> Result<TerminalKey>;
    fn clear(&mut self) -> Result<()>;
    fn write_line(&mut self, line: &str) -> Result<()>;
}

pub struct CrosstermTerminal<'a> {
    output: &'a mut dyn Write,
    interactive: bool,
}

impl<'a> CrosstermTerminal<'a> {
    pub fn new(output: &'a mut dyn Write) -> Self {
        Self {
            output,
            interactive: io::stdin().is_terminal() && io::stdout().is_terminal(),
        }
    }
}

impl TerminalIo for CrosstermTerminal<'_> {
    fn is_interactive(&self) -> bool {
        self.interactive
    }

    fn read_key(&mut self) -> Result<TerminalKey> {
        loop {
            let event = event::read().map_err(InstallError::Io)?;
            let Event::Key(key) = event else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            return Ok(match key.code {
                KeyCode::Up => TerminalKey::Up,
                KeyCode::Down => TerminalKey::Down,
                KeyCode::Left => TerminalKey::Left,
                KeyCode::Right => TerminalKey::Right,
                KeyCode::Enter => TerminalKey::Enter,
                KeyCode::Esc => TerminalKey::Escape,
                KeyCode::Char(' ') => TerminalKey::Space,
                KeyCode::Char(value) => TerminalKey::Char(value),
                _ => continue,
            });
        }
    }

    fn clear(&mut self) -> Result<()> {
        self.output
            .write_all(b"\x1b[2J\x1b[H")
            .map_err(InstallError::Io)
    }

    fn write_line(&mut self, line: &str) -> Result<()> {
        writeln!(self.output, "{line}").map_err(InstallError::Io)?;
        self.output.flush().map_err(InstallError::Io)
    }
}

#[derive(Clone, Debug, Parser)]
#[command(name = "dotfiles", about = "Install selected dotfiles modules")]
pub struct Cli {
    #[arg(long = "modules", visible_alias = "apps", value_name = "a,b,c")]
    pub modules: Option<String>,
    #[arg(long)]
    pub yes: bool,
    #[arg(long)]
    pub list: bool,
}

impl Cli {
    pub fn module_list(&self) -> Option<&str> {
        self.modules.as_deref()
    }

    pub fn explicit_request(&self, platform: Platform) -> Result<Option<InstallRequest>> {
        let Some(value) = self.module_list() else {
            return Ok(None);
        };
        let modules = parse_modules(value)?;
        validate_modules(&modules, platform)?;
        Ok(Some(InstallRequest {
            modules,
            yes: self.yes,
            list: self.list,
            platform,
        }))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallRequest {
    pub modules: Vec<String>,
    pub yes: bool,
    pub list: bool,
    pub platform: Platform,
}

impl InstallRequest {
    pub fn explicit<I, S>(modules: I, yes: bool, platform: Platform) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            modules: modules.into_iter().map(Into::into).collect(),
            yes,
            list: false,
            platform,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageSelection {
    Continue,
    Back,
    Cancel,
}

pub fn parse_modules(value: &str) -> Result<Vec<String>> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut modules = Vec::new();
    for (index, item) in value.split(',').enumerate() {
        let module = item.trim();
        if module.is_empty() {
            return Err(InstallError::Usage(format!(
                "module list contains an empty name at position {}",
                index + 1
            )));
        }
        if modules.iter().any(|known| known == module) {
            return Err(InstallError::Usage(format!(
                "module list contains duplicate module: {module}"
            )));
        }
        modules.push(module.to_owned());
    }
    Ok(modules)
}

pub fn validate_modules(modules: &[String], platform: Platform) -> Result<()> {
    for module_id in modules {
        let module = registry::find_module(module_id)
            .ok_or_else(|| InstallError::Usage(format!("unknown module: {module_id}")))?;
        if !module.supports(platform) {
            return Err(InstallError::Usage(format!(
                "module is not supported on {platform:?}: {module_id}"
            )));
        }
    }
    Ok(())
}

pub fn select_modules(
    modules: &[&'static ModuleSpec],
    terminal: &mut dyn TerminalIo,
) -> Result<Vec<String>> {
    require_interactive(terminal)?;
    if modules.is_empty() {
        return Ok(Vec::new());
    }

    let mut selected = modules
        .iter()
        .map(|module| module.default_selected)
        .collect::<Vec<_>>();
    let mut cursor = 0usize;
    loop {
        render_modules(modules, &selected, cursor, terminal)?;
        match terminal.read_key()? {
            TerminalKey::Up => cursor = cursor.saturating_sub(1),
            TerminalKey::Down => cursor = (cursor + 1).min(modules.len() - 1),
            TerminalKey::Space => selected[cursor] = !selected[cursor],
            TerminalKey::Enter => {
                return Ok(modules
                    .iter()
                    .zip(selected)
                    .filter_map(|(module, selected)| selected.then_some(module.id.to_owned()))
                    .collect())
            }
            TerminalKey::Escape | TerminalKey::Char('q') | TerminalKey::Char('Q') => {
                return Err(InstallError::Cancelled)
            }
            TerminalKey::Left | TerminalKey::Right => {}
            TerminalKey::Char(_) => {}
        }
    }
}

fn render_modules(
    modules: &[&'static ModuleSpec],
    selected: &[bool],
    cursor: usize,
    terminal: &mut dyn TerminalIo,
) -> Result<()> {
    terminal.clear()?;
    terminal
        .write_line("Select applications (Up/Down to move, Space to toggle, Enter to confirm):")?;
    for (index, module) in modules.iter().enumerate() {
        let pointer = if index == cursor { '>' } else { ' ' };
        let marker = if selected[index] { 'x' } else { ' ' };
        terminal.write_line(&format!(
            "{pointer} [{marker}] {} - {}",
            module.id, module.description
        ))?;
    }
    Ok(())
}

pub fn select_packages(
    plan: &mut PackagePlan,
    terminal: &mut dyn TerminalIo,
) -> Result<PackageSelection> {
    if plan.actions.is_empty() {
        return Ok(PackageSelection::Continue);
    }
    require_interactive(terminal)?;

    let mut cursor = 0usize;
    loop {
        render_packages(plan, cursor, terminal)?;
        match terminal.read_key()? {
            TerminalKey::Up => cursor = cursor.saturating_sub(1),
            TerminalKey::Down => cursor = (cursor + 1).min(plan.actions.len() - 1),
            TerminalKey::Space => plan.actions[cursor].selected = !plan.actions[cursor].selected,
            key @ (TerminalKey::Left | TerminalKey::Right) => {
                let action = &mut plan.actions[cursor];
                let candidates = plan.alternatives.get(&action.logical).ok_or_else(|| {
                    InstallError::Package(format!(
                        "package plan has no alternatives for {}",
                        action.logical
                    ))
                })?;
                let current = candidates
                    .iter()
                    .position(|candidate| candidate.manager == action.manager)
                    .ok_or_else(|| {
                        InstallError::Package(format!(
                            "package plan has no selected manager for {}",
                            action.logical
                        ))
                    })?;
                let next = if key == TerminalKey::Right {
                    (current + 1) % candidates.len()
                } else {
                    (current + candidates.len() - 1) % candidates.len()
                };
                action.manager = candidates[next].manager;
                action.package_name = candidates[next].package_name.clone();
            }
            TerminalKey::Enter => return Ok(PackageSelection::Continue),
            TerminalKey::Char('b') | TerminalKey::Char('B') => return Ok(PackageSelection::Back),
            TerminalKey::Escape | TerminalKey::Char('q') | TerminalKey::Char('Q') => {
                return Ok(PackageSelection::Cancel)
            }
            TerminalKey::Char(_) => {}
        }
    }
}

fn render_packages(plan: &PackagePlan, cursor: usize, terminal: &mut dyn TerminalIo) -> Result<()> {
    terminal.clear()?;
    terminal.write_line(
        "Package plan (Up/Down move, Left/Right manager, Space toggle, Enter install, b back):",
    )?;
    for (index, action) in plan.actions.iter().enumerate() {
        let pointer = if index == cursor { '>' } else { ' ' };
        let marker = if action.selected { 'x' } else { ' ' };
        terminal.write_line(&format!(
            "{pointer} [{marker}] {} -> {}",
            action.logical,
            action.manager.label()
        ))?;
    }
    Ok(())
}

fn require_interactive(terminal: &dyn TerminalIo) -> Result<()> {
    if terminal.is_interactive() {
        Ok(())
    } else {
        Err(InstallError::Usage(
            "interactive selection requires a terminal; use --modules for noninteractive setup"
                .into(),
        ))
    }
}

pub fn supported_modules(platform: Platform) -> Vec<&'static ModuleSpec> {
    registry::modules()
        .iter()
        .filter(|module| module.supports(platform))
        .collect()
}

pub fn print_module_list(platform: Platform, output: &mut dyn Write) -> Result<()> {
    for module in supported_modules(platform) {
        writeln!(output, "{} - {}", module.id, module.description).map_err(InstallError::Io)?;
    }
    Ok(())
}

pub fn package_manager_options(plan: &PackagePlan, logical: &str) -> Vec<PackageManager> {
    plan.alternatives
        .get(logical)
        .map(|items| items.iter().map(|item| item.manager).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::parse_modules;

    #[test]
    fn blank_module_list_is_a_successful_noop() {
        assert!(parse_modules(" ").unwrap().is_empty());
    }
}
