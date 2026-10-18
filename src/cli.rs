use std::io::{self, IsTerminal, Write};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::style::Stylize;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::error::{InstallError, Result};
use crate::model::{PackageManager, Platform};
use crate::packages::PackagePlan;
use crate::registry::{self, ModuleSpec};
use crate::state::ModuleState;
use zeroize::Zeroizing;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalKey {
    Up,
    Down,
    Left,
    Right,
    Space,
    Enter,
    Backspace,
    Escape,
    Char(char),
}

pub struct RawModeGuard {
    active: bool,
}

impl RawModeGuard {
    fn inactive() -> Self {
        Self { active: false }
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = disable_raw_mode();
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuStyle {
    Pointer,
    Heading,
    Selected,
    FocusedSelected,
    Normal,
}

pub trait TerminalIo {
    fn is_interactive(&self) -> bool;
    fn enter_raw_mode(&mut self) -> Result<RawModeGuard> {
        Ok(RawModeGuard::inactive())
    }
    fn read_key(&mut self) -> Result<TerminalKey>;
    fn read_line(&mut self, prompt: &str) -> Result<Option<String>> {
        self.write_line(prompt)?;
        let mut value = String::new();
        let read = io::stdin()
            .read_line(&mut value)
            .map_err(InstallError::Io)?;
        if read == 0 {
            Ok(None)
        } else {
            Ok(Some(value.trim_end_matches(['\r', '\n']).to_owned()))
        }
    }
    fn read_secret(&mut self, prompt: &str) -> Result<Option<Zeroizing<String>>> {
        self.read_line(prompt)
            .map(|value| value.map(Zeroizing::new))
    }
    fn clear(&mut self) -> Result<()>;
    fn write_line(&mut self, line: &str) -> Result<()>;
    fn write_menu_line(&mut self, line: &str, _style: MenuStyle) -> Result<()> {
        self.write_line(line)
    }
    fn write_menu_item(&mut self, pointer: char, line: &str, _style: MenuStyle) -> Result<()> {
        self.write_line(&format!("{pointer}{line}"))
    }
}

pub struct CrosstermTerminal<'a> {
    output: &'a mut dyn Write,
    interactive: bool,
    color_enabled: bool,
}

impl<'a> CrosstermTerminal<'a> {
    pub fn new(output: &'a mut dyn Write) -> Self {
        let interactive = io::stdin().is_terminal() && io::stdout().is_terminal();
        Self {
            output,
            interactive,
            color_enabled: interactive && std::env::var_os("NO_COLOR").is_none(),
        }
    }
}

impl TerminalIo for CrosstermTerminal<'_> {
    fn is_interactive(&self) -> bool {
        self.interactive
    }

    fn enter_raw_mode(&mut self) -> Result<RawModeGuard> {
        enable_raw_mode().map_err(InstallError::Io)?;
        Ok(RawModeGuard { active: true })
    }

    fn read_secret(&mut self, prompt: &str) -> Result<Option<Zeroizing<String>>> {
        if !self.interactive {
            return self
                .read_line(prompt)
                .map(|value| value.map(Zeroizing::new));
        }
        self.output
            .write_all(prompt.as_bytes())
            .map_err(InstallError::Io)?;
        self.output.flush().map_err(InstallError::Io)?;
        let _raw_mode = self.enter_raw_mode()?;
        let mut value = String::new();
        loop {
            let event = event::read().map_err(InstallError::Io)?;
            let Event::Key(key) = event else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Enter => break,
                KeyCode::Esc => return Ok(None),
                KeyCode::Backspace => {
                    value.pop();
                }
                KeyCode::Char(character) => value.push(character),
                _ => {}
            }
        }
        self.output.write_all(b"\n").map_err(InstallError::Io)?;
        self.output.flush().map_err(InstallError::Io)?;
        Ok(Some(Zeroizing::new(value)))
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
                KeyCode::Backspace => TerminalKey::Backspace,
                KeyCode::Esc => TerminalKey::Escape,
                KeyCode::Char(' ') => TerminalKey::Space,
                KeyCode::Char(value) => TerminalKey::Char(value),
                _ => continue,
            });
        }
    }

    fn clear(&mut self) -> Result<()> {
        self.output
            .write_all(b"\x1b[2J\x1b[3J\x1b[H")
            .map_err(InstallError::Io)
    }

    fn write_line(&mut self, line: &str) -> Result<()> {
        write!(self.output, "{line}\r\n").map_err(InstallError::Io)?;
        self.output.flush().map_err(InstallError::Io)
    }

    fn write_menu_line(&mut self, line: &str, style: MenuStyle) -> Result<()> {
        if !self.color_enabled {
            return self.write_line(line);
        }

        write_styled_menu_text(self.output, line, style).map_err(InstallError::Io)?;
        write!(self.output, "\r\n").map_err(InstallError::Io)?;
        self.output.flush().map_err(InstallError::Io)
    }

    fn write_menu_item(&mut self, pointer: char, line: &str, style: MenuStyle) -> Result<()> {
        if !self.color_enabled {
            return self.write_line(&format!("{pointer}{line}"));
        }

        write_styled_menu_text(self.output, &pointer.to_string(), MenuStyle::Pointer)
            .map_err(InstallError::Io)?;
        write_styled_menu_text(self.output, line, style).map_err(InstallError::Io)?;
        write!(self.output, "\r\n").map_err(InstallError::Io)?;
        self.output.flush().map_err(InstallError::Io)
    }
}

fn write_styled_menu_text(output: &mut dyn Write, line: &str, style: MenuStyle) -> io::Result<()> {
    match style {
        MenuStyle::Pointer => write!(output, "{}", line.magenta().bold()),
        MenuStyle::Heading => write!(output, "{}", line.cyan().bold()),
        MenuStyle::Selected => write!(output, "{}", line.green()),
        MenuStyle::FocusedSelected => write!(output, "{}", line.green().bold()),
        MenuStyle::Normal => write!(output, "{line}"),
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
            allow_module_back: false,
        }))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallRequest {
    pub modules: Vec<String>,
    pub yes: bool,
    pub list: bool,
    pub platform: Platform,
    pub allow_module_back: bool,
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
            allow_module_back: false,
        }
    }

    pub fn interactive<I, S>(modules: I, yes: bool, platform: Platform) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            modules: modules.into_iter().map(Into::into).collect(),
            yes,
            list: false,
            platform,
            allow_module_back: true,
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
    validate_modules_against(modules, platform, registry::modules())
}

pub fn validate_modules_against(
    modules: &[String],
    platform: Platform,
    registry: &[ModuleSpec],
) -> Result<()> {
    for module_id in modules {
        let module = registry
            .iter()
            .find(|module| module.id == module_id)
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
    select_modules_with_state(modules, &ModuleState::default(), terminal)
}

pub fn select_modules_with_state(
    modules: &[&'static ModuleSpec],
    state: &ModuleState,
    terminal: &mut dyn TerminalIo,
) -> Result<Vec<String>> {
    require_interactive(terminal)?;
    let _raw_mode = terminal.enter_raw_mode()?;
    if modules.is_empty() {
        return Ok(Vec::new());
    }

    let mut selected = modules
        .iter()
        .map(|module| module.default_selected)
        .collect::<Vec<_>>();
    let mut cursor = 0usize;
    loop {
        render_modules(modules, state, &selected, cursor, terminal)?;
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
            TerminalKey::Left | TerminalKey::Right | TerminalKey::Backspace => {}
            TerminalKey::Char(_) => {}
        }
    }
}

fn render_modules(
    modules: &[&'static ModuleSpec],
    state: &ModuleState,
    selected: &[bool],
    cursor: usize,
    terminal: &mut dyn TerminalIo,
) -> Result<()> {
    terminal.clear()?;
    terminal.write_menu_line(
        "Select applications (Up/Down to move, Space to toggle, Enter to confirm):",
        MenuStyle::Heading,
    )?;
    for (index, module) in modules.iter().enumerate() {
        let pointer = if index == cursor { '>' } else { ' ' };
        let marker = if selected[index] { 'x' } else { ' ' };
        let status = if state.contains(module.id) {
            " (previously applied)"
        } else {
            ""
        };
        let style = menu_item_style(index == cursor, selected[index]);
        terminal.write_menu_item(
            pointer,
            &format!(" [{marker}] {} - {}{status}", module.id, module.description),
            style,
        )?;
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
    let _raw_mode = terminal.enter_raw_mode()?;

    let mut cursor = 0usize;
    loop {
        render_packages(plan, cursor, terminal)?;
        match terminal.read_key()? {
            TerminalKey::Up => cursor = cursor.saturating_sub(1),
            TerminalKey::Down => cursor = (cursor + 1).min(plan.actions.len() - 1),
            TerminalKey::Space => plan.actions[cursor].selected = !plan.actions[cursor].selected,
            key @ (TerminalKey::Left | TerminalKey::Right) => {
                let action = &mut plan.actions[cursor];
                let candidates = plan.alternatives.get(&action.package_id).ok_or_else(|| {
                    InstallError::Package(format!(
                        "package plan has no alternatives for {}",
                        action.package_id
                    ))
                })?;
                let current = candidates
                    .iter()
                    .position(|candidate| candidate.manager == action.manager)
                    .ok_or_else(|| {
                        InstallError::Package(format!(
                            "package plan has no selected manager for {}",
                            action.package_id
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
            TerminalKey::Backspace | TerminalKey::Char(_) => {}
        }
    }
}

fn render_packages(plan: &PackagePlan, cursor: usize, terminal: &mut dyn TerminalIo) -> Result<()> {
    terminal.clear()?;
    terminal.write_menu_line(
        "Package plan (Up/Down move, Left/Right manager, Space toggle, Enter install, b back):",
        MenuStyle::Heading,
    )?;
    for (index, action) in plan.actions.iter().enumerate() {
        let pointer = if index == cursor { '>' } else { ' ' };
        let marker = if action.selected { 'x' } else { ' ' };
        let style = menu_item_style(index == cursor, action.selected);
        terminal.write_menu_item(
            pointer,
            &format!(
                " [{marker}] {} -> {}",
                action.package_id,
                action.manager.label()
            ),
            style,
        )?;
    }
    Ok(())
}

fn menu_item_style(focused: bool, selected: bool) -> MenuStyle {
    match (focused, selected) {
        (true, true) => MenuStyle::FocusedSelected,
        (false, true) => MenuStyle::Selected,
        (_, false) => MenuStyle::Normal,
    }
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

pub fn package_manager_options(plan: &PackagePlan, package_id: &str) -> Vec<PackageManager> {
    plan.alternatives
        .get(package_id)
        .map(|items| items.iter().map(|item| item.manager).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{menu_item_style, parse_modules, write_styled_menu_text, MenuStyle};

    #[test]
    fn blank_module_list_is_a_successful_noop() {
        assert!(parse_modules(" ").unwrap().is_empty());
    }

    #[test]
    fn focused_unselected_rows_are_plain_and_the_pointer_is_purple() {
        assert_eq!(menu_item_style(true, false), MenuStyle::Normal);

        let mut output = Vec::new();
        write_styled_menu_text(&mut output, ">", MenuStyle::Pointer).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(
            output.contains("\x1b[38;5;13m"),
            "unexpected styling: {output:?}"
        );
    }
}
