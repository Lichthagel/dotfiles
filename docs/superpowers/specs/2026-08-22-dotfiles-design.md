# Dotfiles Design

## Goal

Provide a minimal dotfiles repository usable on Linux and Windows, with native one-liner entry points, interactive application selection, and deterministic application selection for automation.

## Architecture

The repository uses native Bash and PowerShell installers backed by the same restricted module manifest format. Modules contain metadata and source files; installers handle selection, validation, backups, symlink creation, and copy fallback without requiring Python, Node, or administrator privileges.

## Repository Flow

The installer accepts `--apps git,shell` on Linux and `-Apps git,shell` on PowerShell. When omitted, it presents a numbered multi-select prompt. `--list` and `--help` are available on both platforms. Unknown modules are rejected before filesystem changes.

Existing targets are moved to timestamped backup directories before replacement. Symlinks are preferred. Windows falls back to copying when link creation is unavailable and reports that result explicitly.

## Initial Modules

The initial catalog contains `git`, `bash`, and `powershell`. Git installs `.gitconfig`. Bash installs `.bashrc` on Linux, and PowerShell installs the PowerShell profile on Windows. No terminal module is included without a concrete configuration target.

## Safety and Verification

Installers do not execute configuration files, require administrator privileges, or silently install every module. Tests cover parsing, selection, unknown modules, backups, and successful installation on Bash and PowerShell. Syntax checks and isolated temporary-home tests are part of verification.
