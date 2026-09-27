# Rust Installer Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the duplicated Bash and PowerShell dotfiles installers with one cross-platform Rust binary that embeds static assets and encrypted age bundles, ports setup behavior to Rust, and is delivered through ephemeral verified release bootstraps.

**Architecture:** A typed Rust registry describes modules, package alternatives, requirements, setup handlers, and embedded mappings. A shared engine performs selection, dependency resolution, package planning, native secret/setup handling, and regular-file mapping; small Linux/Windows adapters supply paths, permissions, terminal behavior, and package-manager commands. Two minimal bootstrap scripts download and verify the appropriate release binary, execute it temporarily, and clean it up.

**Tech Stack:** Rust stable, Cargo, `clap`, `crossterm`, `age`, `serde`, `serde_json`, `reqwest` with Rustls, `thiserror`, `zeroize`, `tempfile`, `assert_cmd`, `predicates`, `windows-sys`, Bash/PowerShell only for the ephemeral bootstrap transport, GitHub Actions for Linux and Windows release builds.

**Spec:** `docs/superpowers/specs/2026-08-30-rust-installer-migration-design.md`

## Global Constraints

- The initial release targets are `x86_64-unknown-linux-gnu` and `x86_64-pc-windows-msvc`.
- The executable embeds static configuration assets and encrypted `*.env.age` bundles; plaintext credentials and age private keys are never embedded.
- The Rust age library decrypts embedded bundles in memory; no external `age` executable or automatic `age` package dependency is required.
- The bootstrap downloads a prebuilt binary, verifies its SHA-256 checksum, runs it from a temporary location, forwards arguments, and removes the temporary files.
- Installed mappings are materialized regular files, not symlinks to the temporary binary or repository checkout.
- OpenCode continues fetching and validating the latest Catppuccin themes at setup time.
- There is no arbitrary `setup.sh`/`setup.ps1` execution path in the Rust installer.
- When Scoop is selected, `pwsh` is invoked only as Scoop’s own package-manager runtime through a typed adapter, never to evaluate a downloaded installer hook.
- Canonical CLI options are `--modules <a,b,c>`, `--yes`, `--list`, and `--help`; `--apps` is a compatibility alias for `--modules`.
- Explicit module selection is required for unattended setup; `--yes` selects the highest-priority available manager and includes all planned packages.
- Linux package-manager priority is `apt`, `dnf`, `pacman`, `brew`, `mise`; Windows priority is `winget`, `scoop`, `brew`, `mise`.
- Linux system package commands prefer `run0` and fall back to `sudo`.
- No progress, error, or diagnostic output may contain secret values.
- Package-manager operations are not transactional and are not rolled back after a failure.
- Tests use temporary homes, fake package commands, generated age identities, and synthetic credentials only.

---

## File map

The migration will introduce these focused Rust files:

- `Cargo.toml` and `Cargo.lock` — crate metadata and locked dependencies.
- `src/lib.rs` — public module wiring and reusable `run` entry point.
- `src/main.rs` — binary entry point and top-level error rendering.
- `src/error.rs` — contextual error types and exit-category mapping.
- `src/model.rs` — platform, package, asset, target, module, and setup data types.
- `src/assets.rs` — explicit compile-time references to static assets and encrypted bundles.
- `src/registry.rs` — typed definitions for all current modules and their relationships.
- `src/cli.rs` — `clap` arguments, module selector, and package-plan selector.
- `src/engine.rs` — module resolution and ordered installation phases.
- `src/packages.rs` — package-manager abstractions, discovery, installed checks, and installs.
- `src/mappings.rs` — target resolution, backups, staged writes, and regular-file replacement.
- `src/secrets.rs` — age identity resolution, in-memory decryption, and strict env parsing.
- `src/setup/mod.rs` — setup dispatch and shared setup context.
- `src/setup/providers.rs` — Rust-native Homebrew/Scoop provider bootstrap behavior.
- `src/setup/atuin.rs` — Atuin status/login behavior.
- `src/setup/opencode.rs` — OpenCode configuration, credentials, MCP/plugin changes, and themes.
- `src/setup/jsonc.rs` — JSONC comment/trailing-comma normalization and validation.
- `src/setup/http.rs` — HTTP client abstraction and bounded theme downloads.
- `src/platform/mod.rs` — platform-operation traits and shared path context.
- `src/platform/unix.rs` — Linux paths, permissions, process behavior, and package helpers.
- `src/platform/windows.rs` — Windows paths, ACLs, process behavior, and package helpers.
- `bootstrap/install.sh` — Linux x86_64 release bootstrap.
- `bootstrap/install.ps1` — Windows x86_64 release bootstrap.
- `tests/assets.rs`, `tests/registry.rs`, `tests/packages.rs`, `tests/mappings.rs`, `tests/secrets.rs`, `tests/atuin.rs`, `tests/opencode.rs`, `tests/cli.rs`, `tests/installer.rs` — Rust integration coverage.
- `tests/support/mod.rs`, `tests/support/runner.rs`, `tests/support/mappings.rs`, `tests/support/secrets.rs`, `tests/support/opencode.rs`, `tests/support/installer.rs` — shared fake runners and temporary fixtures used by the integration tests.
- `tests/bootstrap.sh` and `tests/bootstrap.ps1` — mocked bootstrap download/checksum/cleanup checks.

Existing static configuration files remain under `modules/`, and
`secrets/atuin.env.age`, `secrets/opencode.env.age`, and `secrets/README.md`
remain. The old installer engines, parser manifests, setup scripts, secret
libraries, and their shell/PowerShell test harnesses are removed only in the
final cleanup task after Rust parity tests pass.

---

### Task 1: Create the Cargo crate, error model, domain types, and asset boundary

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/main.rs`
- Create: `src/error.rs`
- Create: `src/model.rs`
- Create: `src/assets.rs`
- Create: `tests/assets.rs`
- Create: `tests/support/mod.rs`
- Create: `rust-toolchain.toml`
- Create: `Cargo.lock` (generated by Cargo)

**Interfaces:**
- Produces `dotfiles::model::{AssetId, PackageManager, Platform, SecretBundle, SetupKind, Target}`.
- Produces `dotfiles::assets::{asset_bytes, encrypted_bundle_bytes}`.
- Produces `dotfiles::error::{InstallError, Result, ExitCategory}`.
- Produces a compiling `dotfiles::run(args, io)` entry point for later CLI work.

- [ ] **Step 1: Write the asset-boundary and type tests first.**

Add `tests/assets.rs` with tests that require every release asset to be nonempty and every encrypted bundle to begin with the age file header:

```rust
use dotfiles::assets::{asset_bytes, encrypted_bundle_bytes};
use dotfiles::model::{AssetId, SecretBundle};

#[test]
fn all_static_assets_are_embedded() {
    for asset in AssetId::all() {
        assert!(!asset_bytes(*asset).is_empty(), "empty asset: {asset:?}");
    }
}

#[test]
fn encrypted_bundles_are_embedded_as_age_payloads() {
    for bundle in [SecretBundle::Atuin, SecretBundle::OpenCode] {
        assert!(encrypted_bundle_bytes(bundle).starts_with(b"age-encryption.org/v1"));
    }
}
```

- [ ] **Step 2: Run the focused tests to verify the crate is not implemented yet.**

Run: `cargo test --test assets`

Expected: FAIL because the Cargo crate and asset functions do not exist.

- [ ] **Step 3: Define the dependency manifest and domain types.**

Add these dependency declarations and configure a binary named `dotfiles` plus a library named `dotfiles`:

```toml
[package]
name = "dotfiles"
version = "0.1.0"
edition = "2021"

[lib]
name = "dotfiles"
path = "src/lib.rs"

[[bin]]
name = "dotfiles"
path = "src/main.rs"

[dependencies]
age = "0.11"
clap = { version = "4", features = ["derive"] }
crossterm = "0.28"
reqwest = { version = "0.12", default-features = false, features = ["blocking", "json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
zeroize = { version = "1", features = ["derive"] }
windows-sys = { version = "0.59", features = ["Win32_Foundation", "Win32_Security", "Win32_Security_Authorization", "Win32_Storage_FileSystem", "Win32_System_Threading"] }
[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

Do not add an `anyhow` dependency; use the typed `thiserror` hierarchy for operational errors. Configure `rust-toolchain.toml` with `[toolchain]`, `channel = "stable"`, and `components = ["rustfmt", "clippy"]`.

Define the stable enums and structs used by every later task:

```rust
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Platform { Linux, Windows }

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PackageManager { Apt, Dnf, Pacman, Brew, Mise, Winget, Scoop }

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AssetId { GitConfig, Bashrc, PowerShellProfile, MiseBash, MisePowerShell,
    BrewBash, ScoopPowerShell, AtuinBash, AtuinPowerShell,
    OhMyPoshConfig, OhMyPoshBash, OhMyPoshPowerShell }

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SecretBundle { Atuin, OpenCode }

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SetupKind { None, Homebrew, Scoop, Atuin, OpenCode }

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Target { Home(&'static str), ProfileFile, ProfileSibling(&'static str) }
```

Add `AssetId::all`, `AssetId::Debug`, and `SecretBundle::Debug` implementations so tests and error messages identify each asset without printing secret bytes. Define `InstallError` variants for usage, cancellation, package, secret, setup, mapping, asset, and I/O failures, plus `ExitCategory` mapping usage errors to status `2`, cancellation to status `0`, and all operational failures to a nonzero status.

Implement `asset_bytes` with explicit `include_bytes!`/`include_str!` references to the current files under `modules/`, and `encrypted_bundle_bytes` with explicit references to `secrets/atuin.env.age` and `secrets/opencode.env.age`. Do not use a glob or reference any `*.env` plaintext path. Create `tests/support/mod.rs` with module declarations and shared assertion helpers; later tasks add the typed fixtures under that module.

- [ ] **Step 4: Run formatting, compilation, and the focused tests.**

Run:

```text
cargo fmt --check
cargo test --test assets
cargo test
```

Expected: formatting passes, the asset tests pass, and the crate’s initial empty-library tests pass.

- [ ] **Step 5: Commit the crate foundation.**

```bash
git add Cargo.toml Cargo.lock rust-toolchain.toml src tests/assets.rs
git commit -m "feat: scaffold rust installer crate"
```

---

### Task 2: Encode the complete typed module registry and compile-time mappings

**Files:**
- Modify: `src/model.rs`
- Modify: `src/assets.rs`
- Create: `src/registry.rs`
- Modify: `src/lib.rs`
- Create: `tests/registry.rs`

**Interfaces:**
- Consumes `Platform`, `PackageManager`, `AssetId`, `SecretBundle`, `SetupKind`, and `Target` from Task 1.
- Produces `PackageSpec`, `MappingSpec`, `ModuleSpec`, `registry::modules()`, and `registry::find_module(id)`.
- Produces `ModuleSpec::supports(platform)` and `PackageManager::priority(platform)` for the resolver and package tasks.

- [ ] **Step 1: Write registry parity tests before defining the registry.**

Add tests that assert the complete module set, descriptions/defaults, platform filtering, provider modules, setup kinds, secret bundles, and representative package/mapping declarations:

```rust
#[test]
fn registry_contains_only_supported_modules() {
    let ids: Vec<_> = registry::modules().iter().map(|m| m.id).collect();
    assert_eq!(ids, ["git", "bash", "powershell", "mise", "brew", "scoop",
        "atuin", "oh-my-posh", "opencode"]);
}

#[test]
fn provider_and_secret_contracts_match_the_existing_modules() {
    assert_eq!(registry::find_module("brew").unwrap().provides,
               Some(PackageManager::Brew));
    assert_eq!(registry::find_module("scoop").unwrap().provides,
               Some(PackageManager::Scoop));
    assert_eq!(registry::find_module("atuin").unwrap().secret_bundle,
               Some(SecretBundle::Atuin));
    assert_eq!(registry::find_module("opencode").unwrap().setup,
               SetupKind::OpenCode);
}

#[test]
fn platform_filtering_excludes_shell_specific_modules() {
    assert!(registry::find_module("bash").unwrap().supports(Platform::Linux));
    assert!(!registry::find_module("bash").unwrap().supports(Platform::Windows));
    assert!(registry::find_module("powershell").unwrap().supports(Platform::Windows));
    assert!(!registry::find_module("powershell").unwrap().supports(Platform::Linux));
}
```

Add assertions for all retained package alternatives, including `Git.Git`, `Atuinsh.Atuin`, `JanDeDobbeleer.OhMyPosh`, `jdx.mise`, `anomalyco/tap/opencode`, and `github:anomalyco/opencode`. Assert that no `jq` or age package is registered because native setup and native secret decryption replace those runtime dependencies.

- [ ] **Step 2: Run the registry tests to verify they fail.**

Run: `cargo test --test registry`

Expected: FAIL because `ModuleSpec` and `registry::modules` are not defined.

- [ ] **Step 3: Define the typed registry data.**

Add these structures:

```rust
pub struct PackageSpec {
    pub logical: &'static str,
    pub manager: PackageManager,
    pub name: &'static str,
}

pub struct MappingSpec {
    pub platform: Platform,
    pub asset: AssetId,
    pub target: Target,
    pub requires: &'static [&'static str],
}

pub struct ModuleSpec {
    pub id: &'static str,
    pub description: &'static str,
    pub platforms: &'static [Platform],
    pub default_selected: bool,
    pub provides: Option<PackageManager>,
    pub requires: &'static [&'static str],
    pub packages: &'static [PackageSpec],
    pub mappings: &'static [MappingSpec],
    pub setup: SetupKind,
    pub secret_bundle: Option<SecretBundle>,
}
```

Encode all declarations from the current `module.conf` files as static arrays. Preserve these exact semantics:

- `git` is default-enabled on Linux and Windows, maps `GitConfig` to `.gitconfig`, and has apt/dnf/pacman/brew/mise/winget/scoop alternatives.
- `bash` is Linux-only, default-enabled, and maps `Bashrc` to `.bashrc`.
- `powershell` is Windows-only, default-enabled, and maps `PowerShellProfile` to `ProfileFile`.
- `mise` is default-enabled, provides `Mise`, has apt/dnf/pacman/brew/winget/scoop alternatives, and requires `bash` or `powershell` for its corresponding drop-in mapping.
- `brew` is Linux-only, opt-in, provides `Brew`, uses `SetupKind::Homebrew`, and maps `BrewBash` with a `bash` requirement.
- `scoop` is Windows-only, opt-in, provides `Scoop`, uses `SetupKind::Scoop`, and maps `ScoopPowerShell` with a `powershell` requirement.
- `atuin` is cross-platform and opt-in, uses `SetupKind::Atuin`, has the Atuin package alternatives, and maps both shell integrations with their shell requirements.
- `oh-my-posh` is cross-platform and opt-in, has the current package alternatives, and maps its config and shell integrations with requirements.
- `opencode` is cross-platform and opt-in, uses `SetupKind::OpenCode`, references `SecretBundle::OpenCode`, and has only the OpenCode package alternatives; native setup performs JSON operations in Rust and registers no jq package.

Represent `Target::Home(".config/...")`, `Target::ProfileFile`, and `Target::ProfileSibling("Profile.d/...")` explicitly. `Target::ProfileSibling` must resolve relative to the directory containing the PowerShell profile, matching the existing `$PROFILE\..\...` mappings.

- [ ] **Step 4: Run parity tests and inspect the registry as data.**

Run:

```text
cargo fmt --check
cargo test --test registry
cargo test --test assets
```

Expected: all tests pass, including the negative assertion that no external age package is added.

- [ ] **Step 5: Commit the typed registry.**

```bash
git add src/model.rs src/assets.rs src/registry.rs src/lib.rs tests/registry.rs
git commit -m "feat: encode dotfiles modules in rust"
```

---

### Task 3: Implement Linux/Windows platform paths and safe regular-file mappings

**Files:**
- Create: `src/platform/mod.rs`
- Create: `src/platform/unix.rs`
- Create: `src/platform/windows.rs`
- Create: `src/mappings.rs`
- Modify: `src/model.rs`
- Modify: `src/lib.rs`
- Create: `tests/mappings.rs`
- Create: `tests/support/mappings.rs`

**Interfaces:**
- Consumes `Target`, `MappingSpec`, `AssetId`, and asset bytes from Tasks 1–2.
- Produces `PlatformOps`, `PathContext`, `resolve_target`, `backup_root`, and `apply_mapping`.
- `PathContext` contains the effective home directory, optional PowerShell profile path, and backup root used by target resolution.
- `apply_mapping` returns a `BackupRecord` and guarantees that a failed staged write does not leave a partially written replacement.

- [ ] **Step 1: Write mapping tests against temporary homes.**

Cover Linux home targets, Windows profile-file/profile-sibling targets, `DOTFILES_HOME` test overrides, XDG/local-appdata backup roots, existing regular files, existing directories, existing symlinks, first installation, and failed replacement. Use the `tests/support/mappings.rs` `MappingFixture`, `TestPathContext`, and `git_mapping()` helper returning the registry’s Git `.gitconfig` mapping for path assertions and native platform tests for permission/ACL behavior.

```rust
#[test]
fn existing_target_is_backed_up_and_embedded_asset_is_a_regular_file() {
    let fixture = MappingFixture::new();
    fixture.write_target(".gitconfig", b"old");
    apply_mapping(&fixture.ops, &git_mapping(), b"new").unwrap();

    assert_eq!(fixture.read_target(".gitconfig"), b"new");
    assert!(!fixture.target(".gitconfig").is_symlink());
    assert_eq!(fixture.read_backup(".gitconfig"), b"old");
}

#[test]
fn failed_write_restores_the_previous_target() {
    let fixture = MappingFixture::with_failing_rename();
    fixture.write_target(".gitconfig", b"old");
    assert!(apply_mapping(&fixture.ops, &git_mapping(), b"new").is_err());
    assert_eq!(fixture.read_target(".gitconfig"), b"old");
}
```

- [ ] **Step 2: Run the focused mapping tests to verify they fail.**

Run: `cargo test --test mappings`

Expected: FAIL because the platform and mapping interfaces are not implemented.

- [ ] **Step 3: Define the platform operation interface and path rules.**

Define `PathContext` and `PlatformOps` with methods equivalent to:

```rust
pub struct PathContext {
    pub home: PathBuf,
    pub profile: Option<PathBuf>,
    pub backup_root: PathBuf,
}

pub trait PlatformOps {
    fn platform(&self) -> Platform;
    fn home_dir(&self) -> Result<PathBuf>;
    fn profile_path(&self) -> Result<PathBuf>;
    fn backup_root(&self) -> Result<PathBuf>;
    fn resolve_target(&self, target: Target) -> Result<PathBuf>;
    fn replace_file(&self, target: &Path, bytes: &[u8]) -> Result<BackupRecord>;
    fn set_private_file_permissions(&self, path: &Path) -> Result<()>;
}
```

Linux path resolution uses `DOTFILES_HOME` when set for tests, then `HOME`; `ProfileFile` is invalid on Linux. Linux backups use `XDG_STATE_HOME` or `$HOME/.local/state`, followed by `dotfiles/backups/<timestamp>/`. Windows path resolution uses `DOTFILES_HOME` for tests, then `USERPROFILE`; the bootstrap sets `DOTFILES_PROFILE` from PowerShell’s `$PROFILE`, and direct binary execution falls back to the PowerShell 7 profile path under `Documents/PowerShell/Microsoft.PowerShell_profile.ps1`. Windows backups use `LOCALAPPDATA` or the user-local `AppData/Local` fallback, followed by `dotfiles/backups/<timestamp>/`.

`ProfileSibling` joins the parent directory of the resolved profile path. Reject targets that escape their intended root through `..` after normalization.

- [ ] **Step 4: Implement staged replacement and platform permissions.**

Implement `apply_mapping` so it validates source bytes, creates the target parent, writes a temporary file in the target directory, flushes it, moves an existing target—including a symlink—to the timestamped backup, renames the staged file into place, and restores the backup if the final rename fails. Never create a symlink to the temporary binary or source checkout. On Linux, use mode `0600` only for secret credential files and preserve normal configuration permissions for mapped assets. On Windows, use `windows-sys` ACL calls to remove inherited access and grant the current user full control for secret files.

- [ ] **Step 5: Run mapping and native platform tests.**

Run on Linux:

```text
cargo fmt --check
cargo test --test mappings
```

Run on Windows CI:

```text
cargo test --test mappings
```

Expected: regular-file materialization, backup paths, profile expansion, failure restoration, Linux modes, and Windows current-user-only ACL checks pass.

- [ ] **Step 6: Commit the platform and mapping layer.**

```bash
git add src/platform src/mappings.rs src/model.rs src/lib.rs tests/mappings.rs
git commit -m "feat: add cross-platform safe asset mappings"
```

---

### Task 4: Add package-manager discovery, planning, and installation adapters

**Files:**
- Create: `src/packages.rs`
- Modify: `src/platform/mod.rs`
- Modify: `src/platform/unix.rs`
- Modify: `src/platform/windows.rs`
- Modify: `src/error.rs`
- Create: `tests/packages.rs`
- Create: `tests/support/runner.rs`

**Interfaces:**
- Consumes `PackageSpec`, `ModuleSpec`, `PackageManager`, and `Platform` from Tasks 1–2.
- Produces `CommandRunner`, `ManagerAdapter`, `CommandOutput`, `PackagePlan`, `PackageCandidate`, `PackageAction`, `build_package_plan`, and `install_package`.
- `PackagePlan` exposes logical name, selected manager, concrete package name, alternatives, and selected state to the CLI and engine.

- [ ] **Step 1: Write fake-command tests for discovery and command construction.**

Define `tests/support/runner.rs::FakeRunner` to record program, arguments, environment, and stdin, then use it to cover manager priority, installed package suppression, manager alternatives, `--yes` default selection, Linux `run0`/`sudo` selection, and every install command:

```rust
#[test]
fn linux_prefers_apt_and_run0_over_other_managers() {
    let runner = FakeRunner::available(["apt", "dnf", "brew", "run0"]);
    let plan = build_package_plan(&runner, Platform::Linux, &[git_module()], true).unwrap();
    assert_eq!(plan.actions()[0].manager, PackageManager::Apt);
    install_package(&runner, Platform::Linux, &plan.actions()[0]).unwrap();
    assert_eq!(runner.last_program(), "run0");
    assert_eq!(runner.last_args(), ["apt-get", "install", "-y", "git"]);
}

#[test]
fn installed_logical_packages_are_not_planned() {
    let runner = FakeRunner::available_and_installed(["winget"], [("winget", "Git.Git")]);
    let plan = build_package_plan(&runner, Platform::Windows, &[git_module()], true).unwrap();
    assert!(plan.actions().is_empty());
}
```

Also test exact manager commands: `dnf install -y`, `pacman -S --needed --noconfirm`, `brew install`, `mise use --global`, `winget install --id --exact --accept-source-agreements --accept-package-agreements`, and `scoop install`.

- [ ] **Step 2: Run package tests to verify they fail.**

Run: `cargo test --test packages`

Expected: FAIL because no command runner or package plan exists.

- [ ] **Step 3: Implement the command runner and manager adapters.**

Define:

```rust
pub struct CommandOutput {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub trait CommandRunner {
    fn available(&self, program: &str) -> bool;
    fn run(&self, program: &str, args: &[String], env: &[(String, String)], stdin: Option<&[u8]>) -> Result<CommandOutput>;
}

pub trait ManagerAdapter {
    fn manager(&self) -> PackageManager;
    fn available(&self, runner: &dyn CommandRunner) -> bool;
    fn installed(&self, runner: &dyn CommandRunner, package_name: &str) -> Result<bool>;
    fn install(&self, runner: &dyn CommandRunner, package_name: &str) -> Result<()>;
}

pub struct PackageAction {
    pub logical: String,
    pub module: String,
    pub manager: PackageManager,
    pub package_name: String,
    pub selected: bool,
}

pub struct PackageCandidate {
    pub manager: PackageManager,
    pub package_name: String,
}

pub struct PackagePlan {
    pub actions: Vec<PackageAction>,
    pub alternatives: std::collections::BTreeMap<String, Vec<PackageCandidate>>,
}
```

Implement availability and installed checks with the current manager-specific behavior, but keep all process execution behind `CommandRunner`. Build alternatives from only managers valid for the compiled platform. Select the first available manager in the platform priority list; when a logical package is already installed through any available alternative, omit it. With the noninteractive flag, mark all planned actions selected and do not enter the package selector.

Implement Linux privilege selection (`run0` first, otherwise `sudo`) and the Windows native manager commands. Return an error listing the logical package and supported managers when no available alternative exists.

- [ ] **Step 4: Run the package tests and inspect recorded commands.**

Run:

```text
cargo fmt --check
cargo test --test packages -- --nocapture
```

Expected: all fake-command assertions pass and no real package manager is invoked.

- [ ] **Step 5: Commit package adapters.**

```bash
git add src/packages.rs src/platform src/error.rs tests/packages.rs
git commit -m "feat: add typed package manager adapters"
```

---

### Task 5: Implement the unified CLI and terminal selectors

**Files:**
- Create: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/lib.rs`
- Create: `tests/cli.rs`

**Interfaces:**
- Consumes `registry::modules`, `PackagePlan`, and `Platform` from Tasks 2 and 4.
- Produces `Cli`, `InstallRequest`, `parse_modules`, `select_modules`, `select_packages`, and `TerminalIo`.
- `InstallRequest` contains selected module IDs, `yes`, `list`, and the compiled platform.

- [ ] **Step 1: Write parser and noninteractive behavior tests.**

```rust
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
}
```

Cover `--help`, `--list`, unknown options, missing option values, duplicate module IDs, empty explicit selections, unknown modules, and unsupported platform modules. Add a test that no explicit modules with redirected input returns the documented noninteractive terminal error instead of blocking.

- [ ] **Step 2: Run CLI tests to verify they fail.**

Run: `cargo test --test cli`

Expected: FAIL because the `clap` parser and selectors are not implemented.

- [ ] **Step 3: Implement `clap` parsing and module selection.**

Define:

```rust
#[derive(clap::Parser, Debug)]
pub struct Cli {
    #[arg(long = "modules", visible_alias = "apps")]
    pub modules: Option<String>,
    #[arg(long)]
    pub yes: bool,
    #[arg(long)]
    pub list: bool,
}

pub struct InstallRequest {
    pub modules: Vec<String>,
    pub yes: bool,
    pub platform: Platform,
}

impl InstallRequest {
    pub fn explicit<I, S>(modules: I, yes: bool, platform: Platform) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self { modules: modules.into_iter().map(Into::into).collect(), yes, platform }
    }
}
```

Use the compiled target to filter `registry::modules()`. `--list` prints `id - description` for supported modules and exits before package discovery. An explicit empty module list prints `No applications selected.` and exits successfully. Without `--modules`, use a `TerminalIo` abstraction and `crossterm` to implement Up/Down, Space, Enter, Esc, and `q`, preserving default selections. In redirected input/output, return a clear error requiring `--modules`.

Implement package-plan interaction with Up/Down, Left/Right, Space, Enter, `b`, Esc, and `q`. `b` returns to module selection only for interactive module selection; an explicit module list does not restart selection.

- [ ] **Step 4: Run the parser and selector tests.**

Run:

```text
cargo fmt --check
cargo test --test cli
```

Expected: parser aliases, exit categories, default selections, cancellation, redirected-terminal errors, and package selector controls pass.

- [ ] **Step 5: Commit the unified CLI.**

```bash
git add src/cli.rs src/main.rs src/lib.rs tests/cli.rs
git commit -m "feat: add unified dotfiles cli"
```

---

### Task 6: Implement native age decryption and scoped secret values

**Files:**
- Create: `src/secrets.rs`
- Modify: `src/error.rs`
- Modify: `src/platform/mod.rs`
- Modify: `src/platform/unix.rs`
- Modify: `src/platform/windows.rs`
- Modify: `src/lib.rs`
- Create: `tests/secrets.rs`
- Create: `tests/support/secrets.rs`

**Interfaces:**
- Consumes `SecretBundle` and embedded bytes from Task 1, environment/path access from Task 3, and `CommandRunner` only for later setup consumers.
- Produces `IdentitySource`, `SecretMode`, `SecretValues`, `decrypt_bundle`, `resolve_identity`, `required_values`, and `SecretScope`.
- `SecretValues::get(name)` returns a borrowed secret value for setup use without logging or serializing it.

- [ ] **Step 1: Write generated-identity tests before implementing decryption.**

Define `tests/support/secrets.rs::EncryptedFixture` to generate an age identity and encrypted in-memory payload during the test; also provide `synthetic_values()` for setup tests that need a non-decrypting secret scope. Use it to cover direct `AGE_IDENTITY`, path-based `AGE_IDENTITIES`, platform default identity path, optional missing identity, required missing identity, malformed assignment, invalid variable names, empty values, and cleanup/no plaintext temporary file:

```rust
#[test]
fn decrypts_embedded_style_age_payload_without_creating_plaintext_files() {
    let fixture = EncryptedFixture::new("ATUIN_USERNAME=test-user\nATUIN_PASSWORD=test-pass\n");
    let scope = decrypt_bundle(SecretBundle::Atuin, fixture.payload(), &fixture.identity(), SecretMode::Required).unwrap();
    assert_eq!(scope.values().get("ATUIN_USERNAME"), Some("test-user"));
    assert!(fixture.temp_plaintext_files().is_empty());
}
```

Assert that secret values never appear in `Debug` output or error strings.

- [ ] **Step 2: Run secret tests to verify they fail.**

Run: `cargo test --test secrets`

Expected: FAIL because identity resolution and age decryption are not implemented.

- [ ] **Step 3: Implement identity resolution and in-memory age decryption.**

Use the Rust `age` crate to parse direct identity contents or read the configured identity file, construct a decryptor over the embedded bytes, read into memory, parse only nonempty/noncomment `NAME=value` lines, reject malformed lines and invalid names, and wrap values in zeroizing secret storage. Use `SecretMode::Required` for Atuin and `SecretMode::Optional` for OpenCode. Interactive prompting must go through the terminal abstraction; noninteractive missing identities return a specific error or an optional skip result.

Never write the decrypted bundle or direct identity to disk. Do not add an `age` package to any module registry entry. Make `SecretScope` restore/drop all process-local values without modifying the parent process environment.

- [ ] **Step 4: Run the secret security tests.**

Run:

```text
cargo fmt --check
cargo test --test secrets
cargo test
```

Expected: generated age fixtures decrypt, malformed inputs fail, optional behavior is correct, and no plaintext temporary file is created.

- [ ] **Step 5: Commit native secret handling.**

```bash
git add src/secrets.rs src/error.rs src/platform src/lib.rs tests/secrets.rs
git commit -m "feat: decrypt embedded age secrets in rust"
```

---

### Task 7: Port provider bootstraps and Atuin setup to Rust

**Files:**
- Create: `src/setup/mod.rs`
- Create: `src/setup/http.rs`
- Create: `src/setup/providers.rs`
- Create: `src/setup/atuin.rs`
- Modify: `src/platform/mod.rs`
- Modify: `src/packages.rs`
- Modify: `src/lib.rs`
- Create: `tests/atuin.rs`
- Create: `tests/providers.rs`
- Modify: `tests/support/runner.rs`

**Interfaces:**
- Consumes `SetupKind`, `SecretValues`, `CommandRunner`, `PlatformOps`, and `PackagePlan` from Tasks 1, 3, 4, and 6.
- Produces `SetupContext`, `run_setup`, `install_homebrew`, `install_scoop`, and `login_atuin`.
- Provider setup returns with the manager discoverable by the package adapter; Atuin setup returns success only after status or login succeeds.

- [ ] **Step 1: Write fake-runner tests for provider and Atuin behavior.**

Cover:

- Homebrew already available: no network or install command.
- Homebrew absent: the Rust provider clones `https://github.com/Homebrew/brew` at shallow depth into `/home/linuxbrew/.linuxbrew`, creates the expected `bin`/`sbin` paths, updates the process-local PATH for the current phase, and verifies `bin/brew` is discoverable.
- Scoop already available: no bootstrap.
- Scoop absent: the Rust provider clones `https://github.com/ScoopInstaller/Scoop` at shallow depth into `<scoop-root>/apps/scoop/current`, creates `<scoop-root>/shims`, `<scoop-root>/apps`, `<scoop-root>/buckets`, `<scoop-root>/cache`, and `<scoop-root>/persist`, updates the process-local PATH, and verifies `apps/scoop/current/bin/scoop.ps1` is present.
- Atuin already logged in: no secret access or login command.
- Atuin login: exact username argument, password/key on stdin only, optional sync address, and no secret in recorded output.
- Missing required Atuin values and failed login.

```rust
#[test]
fn atuin_login_sends_only_password_and_key_to_stdin() {
    let runner = FakeRunner::atuin_not_logged_in();
    let secrets = SecretValues::from_pairs([
        ("ATUIN_USERNAME", "user"),
        ("ATUIN_PASSWORD", "password"),
        ("ATUIN_KEY", "encryption-key"),
    ]);
    login_atuin(&runner, &secrets, None).unwrap();
    assert_eq!(runner.last_args(), ["login", "-u", "user"]);
    assert_eq!(runner.last_stdin(), b"password\nencryption-key\n");
    assert!(!runner.log_contains("password"));
    assert!(!runner.log_contains("encryption-key"));
}
```

Provider tests must assert that no downloaded arbitrary shell or PowerShell setup hook is executed by the Rust installer. The only process invocations may be the explicit filesystem/git operations and the manager executable used by later package actions.

- [ ] **Step 2: Run provider and Atuin tests to verify they fail.**

Run: `cargo test --test providers --test atuin`

Expected: FAIL because setup dispatch and handlers are not implemented.

- [ ] **Step 3: Define setup context and dispatch.**

Define the `HttpClient` trait in the new `src/setup/http.rs` file so provider tests can inject a no-network transport, then define:

```rust
pub trait HttpClient {
    fn get_json(&self, url: &str) -> Result<serde_json::Value>;
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>>;
}

pub struct SetupContext<'a> {
    pub platform: &'a dyn PlatformOps,
    pub runner: &'a dyn CommandRunner,
    pub http: &'a dyn HttpClient,
    pub secrets: Option<&'a SecretValues>,
}

pub fn run_setup(kind: SetupKind, ctx: &SetupContext<'_>) -> Result<()>;
```

Implement `SetupKind::Homebrew` and `SetupKind::Scoop` as explicit provider adapters rather than script execution. Homebrew installation clones `https://github.com/Homebrew/brew` with `git clone --depth 1` into `/home/linuxbrew/.linuxbrew`, adds its `bin` and `sbin` directories to the current process PATH, and verifies the `brew` executable. Scoop installation clones `https://github.com/ScoopInstaller/Scoop` with `git clone --depth 1` into `<scoop-root>/apps/scoop/current`, creates its documented user directories, and verifies the `scoop.ps1` entry point. `PackageManager::Scoop` invokes that known entry point through `pwsh -NoProfile -File`; it never evaluates a downloaded installation script. Both handlers verify the manager after setup and return contextual errors.

Implement `SetupKind::Atuin` by running `atuin status`; if status succeeds, return without reading secrets. Otherwise require `ATUIN_USERNAME`, `ATUIN_PASSWORD`, and `ATUIN_KEY`, pass the password and key separated by newlines through stdin to `atuin login -u <username>`, set `ATUIN_SYNC_ADDRESS` only when nonempty, and discard the secret scope after the process exits.

- [ ] **Step 4: Run setup tests and manager-focused package tests.**

Run:

```text
cargo fmt --check
cargo test --test providers --test atuin
cargo test --test packages
```

Expected: provider discovery works after setup, Atuin status/login behavior is idempotent, and secret values never enter command logs.

- [ ] **Step 5: Commit provider and Atuin setup.**

```bash
git add src/setup src/platform src/packages.rs src/lib.rs tests/providers.rs tests/atuin.rs
git commit -m "feat: port provider and atuin setup to rust"
```

---

### Task 8: Port OpenCode configuration, credentials, and dynamic themes to Rust

**Files:**
- Create: `src/setup/jsonc.rs`
- Modify: `src/setup/http.rs`
- Create: `src/setup/opencode.rs`
- Modify: `src/setup/mod.rs`
- Modify: `src/platform/mod.rs`
- Modify: `src/platform/unix.rs`
- Modify: `src/platform/windows.rs`
- Create: `tests/opencode.rs`
- Create: `tests/support/opencode.rs`

**Interfaces:**
- Consumes `SetupContext`, `SecretValues`, `PlatformOps`, `Target`, and embedded module configuration from previous tasks.
- Produces `JsoncDocument`, the production implementation of the `HttpClient` trait, `configure_opencode`, and `ThemeTree`.
- `ThemeTree` contains validated theme filenames and bytes; `configure_opencode` stages all generated files, validates them, and performs per-file replacement only after all downloads and parsing succeed.

- [ ] **Step 1: Write JSONC, HTTP, and OpenCode behavior tests.**

Define `tests/support/opencode.rs::OpenCodeFixture` with an injected fake HTTP client and temporary configuration directories, then cover both Linux `opencode.json` and Windows `opencode.jsonc`, comments/trailing commas, preservation of unrelated fields, provider configuration with all four secrets, unchanged provider configuration with no secrets, failure on partial secrets, MCP/plugin idempotency, no default theme, theme flattening, preservation of unrelated existing themes, failed theme download with no managed-file changes, credential contents, Linux `0600`, and Windows current-user-only ACL. Export the production `SUPERPOWERS_PLUGIN` constant to keep assertions and setup behavior identical.

```rust
#[test]
fn opencode_is_idempotent_and_does_not_write_secret_values_to_config() {
    let fixture = OpenCodeFixture::with_jsonc_and_existing_plugin();
    configure_opencode(&fixture.context()).unwrap();
    let first = fixture.read_config();
    configure_opencode(&fixture.context()).unwrap();
    let second = fixture.read_config();

    assert_eq!(first, second);
    assert_eq!(fixture.plugin_count(SUPERPOWERS_PLUGIN), 1);
    assert_eq!(fixture.value_count_in_config("openrouter-test-secret"), 0);
}

#[test]
fn failed_theme_download_does_not_replace_config_credentials_or_themes() {
    let fixture = OpenCodeFixture::with_failing_theme_client();
    let before = fixture.snapshot_managed_outputs();
    assert!(configure_opencode(&fixture.context()).is_err());
    assert_eq!(fixture.snapshot_managed_outputs(), before);
}
```

- [ ] **Step 2: Run OpenCode tests to verify they fail.**

Run: `cargo test --test opencode`

Expected: FAIL because JSONC parsing, HTTP injection, and the native handler are not implemented.

- [ ] **Step 3: Implement JSONC normalization and typed configuration updates.**

Implement a state-machine JSONC normalizer that removes `//` and `/* */` comments only outside strings and removes trailing commas before `}` or `]`. Parse the normalized document with `serde_json`, preserve unrelated values, and serialize valid JSON. Add helpers that:

- set `provider.openrouter`, `provider.azure`, and `provider.digitalocean` to `{file:...}` references only when all required provider values are present;
- leave existing provider values unchanged when no provider values are present;
- reject partial provider values before staging any output;
- set the `mcp.ddg-search` local command to `[`"uvx"`, `"duckduckgo-mcp-server"`]` with `DDG_SAFE_SEARCH=OFF`;
- remove duplicate Superpowers plugin entries and append exactly one `superpowers@git+https://github.com/obra/superpowers.git`; and
- never write secret values into the JSON document.

- [ ] **Step 4: Implement bounded HTTP theme discovery and staged output.**

Implement the production `HttpClient` from Task 7 with `reqwest` Rustls, HTTPS support, connect/read timeouts of at most 60 seconds, and status/JSON validation. Discover only `themes/*.json` blobs from the GitHub tree response, download each file, validate it as JSON, and flatten the filename into the target themes directory while preserving unrelated files. Stage credentials, config, and themes under a private temporary directory, validate every staged file, then use `PlatformOps` replacement for each managed file. Clean all staging paths on both success and failure.

- [ ] **Step 5: Run native OpenCode tests on both platforms.**

Run:

```text
cargo fmt --check
cargo test --test opencode
```

Expected: JSONC compatibility, provider/secret behavior, idempotency, staging, theme failure safety, and platform credential protections pass without `jq`, `curl`, or external setup scripts.

- [ ] **Step 6: Commit OpenCode setup.**

```bash
git add src/setup src/platform tests/opencode.rs
git commit -m "feat: port opencode setup to rust"
```

---

### Task 9: Build the dependency resolver and end-to-end installation engine

**Files:**
- Create: `src/engine.rs`
- Modify: `src/cli.rs`
- Modify: `src/setup/mod.rs`
- Modify: `src/mappings.rs`
- Modify: `src/packages.rs`
- Modify: `src/main.rs`
- Modify: `src/lib.rs`
- Create: `tests/installer.rs`
- Create: `tests/support/installer.rs`

**Interfaces:**
- Consumes `InstallRequest`, registry modules, package planning, `SetupContext`, and mapping operations from Tasks 2–8.
- Produces `resolve_modules`, `ordered_phases`, `InstallContext`, and `install(request, context)`.
- The engine owns phase ordering but delegates package commands, setup behavior, secret decryption, and file replacement to their focused modules.

- [ ] **Step 1: Write end-to-end fake-runner tests before engine implementation.**

Define `tests/support/installer.rs::InstallFixture` and `test_context`, then cover:

- `git` selects the apt package and maps `.gitconfig` after successful package installation.
- `mise,git` installs the `mise` provider before `git`.
- `bash,oh-my-posh` maps only the selected shell assets and writes regular files.
- required shell modules are added for integrations.
- unavailable managers produce a contextual failure without writing mappings.
- package failure prevents setup and mappings for that phase.
- empty selection is a successful no-op.
- `--list` does not probe or install packages.
- optional OpenCode secrets skip only provider credentials.

```rust
#[test]
fn provider_phase_precedes_dependent_package_phase() {
    let runner = FakeRunner::apt_without_mise_then_mise_after_install();
    let request = InstallRequest::explicit(["mise", "git"], true, Platform::Linux);
    install(request, test_context(&runner)).unwrap();
    assert_eq!(runner.package_log(), ["mise", "git"]);
}

#[test]
fn package_failure_prevents_mapping() {
    let runner = FakeRunner::apt_install_failure();
    let fixture = InstallFixture::new(&runner);
    let request = InstallRequest::explicit(["git"], true, Platform::Linux);
    assert!(install(request, fixture.context()).is_err());
    assert!(!fixture.target(".gitconfig").exists());
}
```

- [ ] **Step 2: Run installer tests to verify they fail.**

Run: `cargo test --test installer`

Expected: FAIL because no resolver or engine implementation exists.

- [ ] **Step 3: Implement module resolution and ordered phases.**

Implement validation against `registry::find_module`, platform support checks, recursive `requires` resolution, provider-module resolution when no package alternative is currently available, duplicate elimination, and cycle detection. Preserve the current order rule: provider modules first, then required modules, then selected modules. Include an encrypted secret bundle’s dependency only in the secret-resolution path; do not add an `age` package action.

Define `InstallContext` with `PlatformOps`, `CommandRunner`, `HttpClient`, `TerminalIo`, and the current timestamp provider. Keep the engine independent from concrete terminal, HTTP, and process implementations.

- [ ] **Step 4: Implement phase execution.**

For each ordered phase:

1. Build the package plan from all resolved modules in the phase.
2. Invoke `select_packages` unless `request.yes` is true.
3. Install selected actions and refresh command lookup state.
4. Resolve/decrypt the selected module’s embedded secret bundle when its setup requires it.
5. Dispatch the native setup handler.
6. Apply only mappings for the phase’s active modules whose requirements are resolved.
7. Return contextual errors without continuing to later phases after a failure.

Ensure setup and mapping handlers receive embedded bytes, never source paths. Preserve messages for skipped package actions and successful backups, but redact all secret-bearing command output.

- [ ] **Step 5: Wire `main.rs` and run all Rust tests.**

`main.rs` parses `Cli`, obtains the compiled platform adapter, handles `--list`, converts the request to the engine, prints contextual errors without debug-dumping secret types, and exits using `ExitCategory`.

Run:

```text
cargo fmt --check
cargo test
cargo clippy -- -D warnings
```

Expected: all unit and integration tests pass on Linux; the Windows CI runner passes its native platform tests.

- [ ] **Step 6: Commit the installation engine.**

```bash
git add src tests/installer.rs
git commit -m "feat: orchestrate cross-platform rust installation"
```

---

### Task 10: Add ephemeral release bootstraps and GitHub Actions release builds

**Files:**
- Create: `bootstrap/install.sh`
- Create: `bootstrap/install.ps1`
- Create: `tests/bootstrap.sh`
- Create: `tests/bootstrap.ps1`
- Create: `.github/workflows/release.yml`
- Modify: `.github/workflows/tests.yml`
- Modify: `Cargo.toml`
- Modify: `README.md`

**Interfaces:**
- Consumes the binary name, target triples, release asset names, and CLI argument contract from Tasks 1, 5, and 9.
- Produces Linux and Windows one-liners, release artifacts named `dotfiles-linux-x86_64` and `dotfiles-windows-x86_64.exe`, matching `.sha256` files, and CI jobs that build/test/publish them.

- [ ] **Step 1: Write mocked bootstrap tests before writing the helpers.**

The Bash test must provide fake `curl`, `sha256sum`, and binary commands, set `DOTFILES_REPO_URL` and `DOTFILES_VERSION` to a local mock endpoint, assert the checksum is verified, assert all arguments are forwarded, and assert the temporary executable/directory is removed after execution. The PowerShell test must mock `Invoke-WebRequest`/`Get-FileHash` or use a local HTTP fixture, assert forwarding and checksum failure behavior, and assert cleanup in `finally`.

```bash
output="$(DOTFILES_REPO_URL="$fixture_url" DOTFILES_VERSION="v1" \
  sh bootstrap/install.sh --modules git --yes)"
assert_contains "$output" "--modules git --yes"
assert_file_not_exists "$recorded_temp_binary"
```

- [ ] **Step 2: Run bootstrap tests to verify they fail.**

Run on Linux: `bash tests/bootstrap.sh`

Run on Windows: `pwsh -NoProfile -File .\\tests\\bootstrap.ps1`

Expected: FAIL because `bootstrap/` and the mock-aware helpers do not exist.

- [ ] **Step 3: Implement the Linux bootstrap.**

`bootstrap/install.sh` must use `set -eu`, accept only Linux x86_64, default to `DOTFILES_REPO_URL=https://github.com/Lichthagel/dotfiles` and the latest release, permit `DOTFILES_VERSION` override, download the release executable and checksum into `mktemp -d`, verify with `sha256sum` or `shasum -a 256`, mark the executable runnable, execute it with `"$@"`, and remove the directory with a trap. Reject unsupported architecture before downloading.

- [ ] **Step 4: Implement the Windows bootstrap.**

`bootstrap/install.ps1` must accept Windows x86_64, default to the same repository and latest release, permit `DOTFILES_VERSION` override, set `DOTFILES_PROFILE=$PROFILE` for the child binary when available, download the `.exe` and checksum into a GUID-named temporary directory, verify with `Get-FileHash -Algorithm SHA256`, invoke the executable with all remaining arguments, and remove the directory in `finally`. It must preserve the child exit code and reject checksum mismatches before execution.

- [ ] **Step 5: Add release workflow and update test workflow.**

Create `.github/workflows/release.yml` triggered by version tags. Build locked release binaries for `x86_64-unknown-linux-gnu` on Ubuntu and `x86_64-pc-windows-msvc` on Windows, run `cargo build --release --locked`, rename the binaries to the documented asset names, generate SHA-256 files, and publish all four artifacts to the GitHub release. Update `.github/workflows/tests.yml` to install stable Rust, run `cargo fmt --check`, `cargo test`, and `cargo clippy -- -D warnings` on both native runners, then run the bootstrap tests. Remove ShellCheck/PSScriptAnalyzer jobs after the legacy scripts are deleted.

- [ ] **Step 6: Update README setup and release documentation.**

Replace checkout-based `install.sh`/`install.ps1` instructions with `dotfiles` CLI examples and the two `bootstrap/` one-liners. Document `--modules`, `--apps`, `--yes`, `--list`, `--help`, x86_64 support, ephemeral cleanup, checksum verification, `DOTFILES_REPO_URL`, `DOTFILES_VERSION`, `DOTFILES_PROFILE`, native age decryption, and the fact that installed mappings are regular files.

- [ ] **Step 7: Run bootstrap and release validation.**

Run:

```text
bash -n bootstrap/install.sh
bash tests/bootstrap.sh
cargo build --release --locked
```

On Windows run:

```text
pwsh -NoProfile -File .\\tests\\bootstrap.ps1
cargo build --release --locked
```

Expected: checksum failures prevent execution, successful fixtures clean up, arguments and exit codes are forwarded, and both release target builds are reproducible from a clean checkout.

- [ ] **Step 8: Commit release delivery.**

```bash
git add bootstrap tests/bootstrap.sh tests/bootstrap.ps1 .github Cargo.toml README.md
git commit -m "feat: add verified ephemeral release bootstraps"
```

---

### Task 11: Remove legacy installer engines and migrate repository documentation/tests

**Files:**
- Delete: `install.sh`
- Delete: `install.ps1`
- Delete: `lib/secrets.sh`
- Delete: `lib/secrets.ps1`
- Delete: `modules/manifest.conf`
- Delete: `modules/dependencies.conf`
- Delete: `modules/*/module.conf`
- Delete: `modules/atuin/setup.sh`
- Delete: `modules/atuin/setup.ps1`
- Delete: `modules/brew/setup.sh`
- Delete: `modules/opencode/setup.sh`
- Delete: `modules/opencode/setup.ps1`
- Delete: `modules/scoop/setup.ps1`
- Delete: `tests/test-helpers.sh`
- Delete: `tests/test-helpers.ps1`
- Delete: `tests/test-install.sh`
- Delete: `tests/test-install.ps1`
- Delete: `tests/test-packages.sh`
- Delete: `tests/test-packages.ps1`
- Delete: `tests/test-secrets.sh`
- Delete: `tests/test-opencode.sh`
- Delete: `tests/test-opencode.ps1`
- Modify: `README.md`
- Modify: `CONTRIBUTING.md`
- Modify: `secrets/README.md`
- Modify: `AGENTS.md`

Keep all static assets under `modules/`, including shell/profile snippets and `oh-my-posh.config.json`. Do not delete `secrets/atuin.env.age` or `secrets/opencode.env.age`.

**Interfaces:**
- Consumes the Rust binary, tests, bootstraps, and workflow from Tasks 1–10.
- Produces a repository with one installer implementation, no stale manifest/parser source, and documentation that describes the Rust binary rather than removed scripts.

- [ ] **Step 1: Add migration assertions before deleting legacy files.**

Add a final Rust test or CI shell check that fails if any of the following remain after cleanup: root `install.sh`/`install.ps1`, `lib/secrets.*`, `modules/manifest.conf`, `modules/dependencies.conf`, any `modules/*/module.conf`, or any module `setup.sh`/`setup.ps1`. The check must also assert that `secrets/atuin.env.age` and `secrets/opencode.env.age` remain and that `git ls-files 'secrets/*.env'` returns no paths.

- [ ] **Step 2: Update documentation before deleting referenced files.**

Rewrite `CONTRIBUTING.md` around `cargo fmt --check`, `cargo test`, `cargo clippy -- -D warnings`, native Linux/Windows CI, Rust module additions, asset embedding, native setup handlers, synthetic age fixtures, and bootstrap tests. Rewrite `secrets/README.md` to explain that encrypted bundles are embedded into releases, Rust decrypts them in memory, no `age` package is installed automatically, and plaintext files/private keys remain local and ignored. Update `AGENTS.md` repository shape, commands, module-extension rules, and operational constraints to refer to Cargo and the Rust registry instead of the two installers and manifest parser.

- [ ] **Step 3: Remove the old parser and setup implementations.**

Delete only the files listed above. Preserve all configuration/profile assets that are now embedded by `src/assets.rs`. Remove old test assertions that inspect deleted shell/PowerShell code and replace them with the Rust integration coverage from Tasks 1–10.

- [ ] **Step 4: Run stale-reference and full repository checks.**

Run:

```bash
for path in install.sh install.ps1 lib/secrets.sh lib/secrets.ps1 modules/manifest.conf modules/dependencies.conf; do
  test ! -e "$path" || { printf 'stale legacy path: %s\n' "$path" >&2; exit 1; }
done
find modules -type f \( -name module.conf -o -name 'setup.sh' -o -name 'setup.ps1' \) -print -quit | grep -q . && exit 1 || true
git ls-files 'secrets/*.env' | grep -q . && exit 1 || true
grep -RInE 'age --decrypt|jq ' src bootstrap tests .github README.md CONTRIBUTING.md secrets AGENTS.md && exit 1 || true
cargo fmt --check
cargo test
cargo clippy -- -D warnings
```

Expected: the explicit legacy-path checks pass, no plaintext secret is tracked, no current source invokes external age/jq setup logic, and all Rust verification passes.

- [ ] **Step 5: Commit the legacy cleanup.**

```bash
git add -A
git commit -m "refactor: remove legacy shell installers"
```

---

### Task 12: Perform the final native-platform acceptance pass

**Files:**
- Modify only files required by verified test failures from Tasks 1–11.
- No new behavior may be added without updating the spec and this plan.

**Interfaces:**
- Consumes the completed binary, bootstraps, workflows, assets, and tests.
- Produces release-ready Linux and Windows behavior with evidence from the complete verification matrix.

- [ ] **Step 1: Run the complete Linux verification matrix.**

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
bash -n bootstrap/install.sh
bash tests/bootstrap.sh
cargo build --release --locked --target x86_64-unknown-linux-gnu
```

Expected: all commands pass, and the release binary contains the embedded asset and encrypted bundle tests.

- [ ] **Step 2: Run the complete Windows verification matrix on the native CI runner.**

```powershell
cargo fmt --check
cargo test
cargo clippy -- -D warnings
pwsh -NoProfile -File .\tests\bootstrap.ps1
cargo build --release --locked --target x86_64-pc-windows-msvc
```

Expected: native ACL, profile path, package command, OpenCode, and bootstrap tests pass.

- [ ] **Step 3: Exercise the public CLI from each release artifact.**

Run the Linux artifact with `--help`, `--list`, `--modules git --yes`, and an unknown module. Run the Windows artifact with the same arguments. Verify help/list do not access package managers, explicit selection installs through fakes in integration tests, unknown modules exit `2`, and an empty explicit selection exits `0`.

- [ ] **Step 4: Verify release contents and secret boundaries.**

Inspect the release archive contents and Git index:

```bash
git ls-files 'secrets/*.env'
test -x target/x86_64-unknown-linux-gnu/release/dotfiles
file target/x86_64-unknown-linux-gnu/release/dotfiles
strings target/x86_64-unknown-linux-gnu/release/dotfiles | grep -F 'test-openrouter' && exit 1 || true
```

Expected: no plaintext test credential or private key is present, both encrypted bundles are embedded by the asset tests, and the Linux release artifact is an executable for the documented target.

- [ ] **Step 5: Commit only verified fixes and record the final status.**

For any test-driven fix, add the regression test first, run the focused test, implement the minimal fix, run the full matrix again, and commit with a message naming the behavior. Finish with:

```bash
git status --short
git log --oneline -8
```

Expected: the working tree is clean, the final commits include the design, each implementation task, and the verified cleanup, and no plaintext secret or generated installation state is staged.
