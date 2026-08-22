# Dotfiles Design

## Goal

Provide a minimal dotfiles repository usable on Linux and Windows, with native one-liner entry points, interactive application selection, and deterministic application selection for automation.

## Architecture

The repository uses native Bash and PowerShell installers backed by the same restricted module manifest format. Modules contain metadata and source files; installers handle selection, validation, backups, symlink creation, and copy fallback without requiring Python, Node, or administrator privileges.

Modules may declare manager-specific packages using `package=<logical>|<manager>:<name>`. Linux manager priority is `apt`, `dnf`, `pacman`, `brew`, then `mise`. Windows manager priority is `winget`, `scoop`, `brew`, then `mise`. System managers prefer `run0` and fall back to `sudo`; user-scoped managers are never elevated.

## Repository Flow

The installer accepts `--apps git,shell` on Linux and `-Apps git,shell` on PowerShell. When omitted, it presents a numbered multi-select prompt. `--list` and `--help` are available on both platforms. Unknown modules are rejected before filesystem changes.

After module selection, missing packages are shown in a second checklist. Up/Down moves, Left/Right selects a manager for the current package, Space toggles package installation, Enter confirms, `b` returns to module selection, and Esc/q cancels. `--yes` / `-Yes` skips package-plan confirmation. Package-install failures prevent dotfile changes; explicitly deselected missing packages produce a warning and do not prevent module setup.

Existing targets are moved to timestamped backup directories before replacement. Symlinks are preferred. Windows falls back to copying when link creation is unavailable and reports that result explicitly.

## Initial Modules

The initial catalog contains `git`, `bash`, and `powershell`. Git installs `.gitconfig`. Bash installs `.bashrc` on Linux, and PowerShell installs the PowerShell profile on Windows. No terminal module is included without a concrete configuration target.

## Safety and Verification

Installers do not execute configuration files, require administrator privileges, or silently install every module. Tests cover parsing, selection, unknown modules, backups, and successful installation on Bash and PowerShell. Syntax checks and isolated temporary-home tests are part of verification.
