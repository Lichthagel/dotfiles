# Dotfiles Implementation Plan

> **For agentic workers:** Use `superpowers:subagent-driven-development` or `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Build a barebones cross-platform dotfiles repository with interactive and noninteractive setup.

**Architecture:** Native Bash and PowerShell installers consume the same restricted module metadata format. Modules provide source files and platform-qualified mappings.

**Tech Stack:** POSIX shell, PowerShell, Git, plain text configuration files.

**Spec:** `docs/superpowers/specs/2026-08-22-dotfiles-design.md`

## Tasks

- [x] Initialize the Git repository and documentation.
- [x] Add the shared module manifest and Git/shell modules.
- [x] Implement Linux selection, validation, backup, linking, and copy fallback.
- [x] Implement Windows selection, validation, backup, linking, and copy fallback.
- [x] Add Bash and PowerShell integration tests.
- [x] Document local and piped setup commands.
- [x] Run syntax, integration, and whitespace verification.
- [x] Split the platform-specific shell profiles into `bash` and `powershell` modules.
