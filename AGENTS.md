# Agent Instructions

## Repository Shape

- `src/main.rs` and the `dotfiles` Cargo binary are the cross-platform entry point.
- `src/registry.rs` is the single typed module registry; it assembles the
  explicit `SPEC` catalog from `src/modules/*.rs` and replaces the old manifest
  parser. Every supported module must be present in that catalog.
- Keep `docs/modules.md` up to date whenever modules are added, removed, or
  changed, including supported platforms, default selections, and behavior.
- Each `src/modules/*.rs` file explicitly embeds its static configuration,
  profile, and encrypted secret assets beside its module definition. Encrypted
  bundle source files remain at `secrets/atuin.env.age` and
  `secrets/opencode.env.age`; decrypt them in memory and never write plaintext.
- `src/modules/` owns each module's typed definition and module-specific setup;
  `src/setup/` contains shared native setup utilities. Do not add arbitrary Bash
  or PowerShell setup hooks.
- `bootstrap/install.sh` and `bootstrap/install.ps1` are ephemeral Cloudflare R2
  download helpers only; they are not installer engines.
- `--modules` selects modules without prompting; `--apps` is its compatibility
  alias. `--yes` bypasses package-plan confirmation, but explicit module
  selection is still required for unattended setup.

## Commands

- Rust formatting: `cargo fmt --check`
- Rust tests: `cargo test --locked`
- Rust lint: `cargo clippy --locked -- -D warnings`
- Linux bootstrap syntax/tests: `bash -n bootstrap/install.sh && bash tests/bootstrap.sh`
- Windows bootstrap tests:
  `pwsh -NoProfile -File .\tests\bootstrap.ps1`
- Linux release build: `cargo build --release --locked --target x86_64-unknown-linux-gnu`
- Windows release build:
  `cargo build --release --locked --target x86_64-pc-windows-msvc`

Run the focused Rust integration test after changing its subsystem, then run
all available Rust and bootstrap checks before completion. Native Linux and
Windows CI are required for platform-specific filesystem, ACL, package, and
release behavior.

## Operational Constraints

- Interactive module selection uses Up/Down, Space, Enter, and Esc/q. Package
  selection additionally uses Left/Right and `b` to return to module selection.
- Package-manager priority is Linux: apt, dnf, pacman, brew, mise; Windows:
  winget, scoop, brew, mise. Linux system managers prefer `run0`, then `sudo`.
- Provider modules must be resolved before dependent package phases. Command
  lookup is refreshed after every successful package installation.
- Content-changing target replacements are backed up; unchanged regular files
  are not rewritten or backed up. Embedded mappings are materialized as regular
  files and must not symlink to the temporary binary or a source checkout.
- The binary decrypts embedded age bundles with the Rust age library in memory;
  do not add an external age package dependency or write plaintext bundle files.
- Atuin values are `ATUIN_USERNAME`, `ATUIN_PASSWORD`, and `ATUIN_KEY`, with
  optional `ATUIN_SYNC_ADDRESS`.
- Never commit plaintext credentials, age private keys, decrypted secret files,
  or real credentials. Encrypted bundles belong at
  `secrets/atuin.env.age` and `secrets/opencode.env.age`.
- OpenCode provider setup is optional as a group, writes protected user-local
  credential files, leaves existing provider configuration unchanged when
  absent, and keeps secret values out of JSON configuration and diagnostics.
- Dynamic OpenCode theme fetching remains network-dependent and must validate
  downloaded JSON before replacing managed outputs.
- Do not commit installation state, plaintext secret files, or OS/editor noise
  covered by `.gitignore`.
