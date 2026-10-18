# Module Files and Idempotent Operations Implementation Plan

**Status:** Implementation complete. Baseline-before-edit results and native Windows runtime/ACL execution remain outstanding; Windows-target test code cross-check passes.

**Goal:** Give each supported dotfiles module one Rust file containing its typed definition and module-specific setup behavior, backed by shared, testable operations that avoid unnecessary changes on repeated runs.

**Approach:** First reorganize without changing behavior. Then introduce explicit convergence results and idempotent file operations. Finally remove module-specific secret decisions from the engine and exercise complete repeated installations. Keep each task independently reviewable and compiling.

## Scope and Constraints

- Preserve `src/registry.rs` as the single typed catalog. It assembles explicit definitions exported by module files; no filesystem discovery, manifests, runtime plugins, or registration macros.
- Keep `ModuleSpec`, `PackageSpec`, `MappingSpec`, and `MappingBehavior` in `src/registry.rs`. Preserve module IDs, order, descriptions, defaults, platforms, dependencies, package alternatives, mapping gates, setup kinds, and bundle associations during extraction.
- Keep the engine responsible for selection, dependency resolution, provider-first phases, package confirmation, scoped secret decryption, setup dispatch, mappings, and completion state.
- Keep `SetupKind` and typed dispatch. Do not introduce a module trait, dynamic hook collection, or generic action DSL for this refactor.
- Modules with only package and mapping declarations do not need an imperative setup function.
- Keep static assets in the existing root `modules/` directory and encrypted bundles under `secrets/`. Each module file explicitly embeds its own static assets and any encrypted bundle it uses.
- Preserve native Rust setup, command-runner injection, HTTP injection, platform permissions, staged replacement, backups before actual replacement, and rollback after a failed final rename.
- Preserve package-manager priority, `run0` before `sudo`, package selection controls, and command lookup refresh after every successful package installation.
- Preserve shell integration gates: a shell must be selected or previously recorded; mapping requirements do not implicitly install shell modules.
- Preserve Atuin's required credentials only when login is needed. OpenCode credentials remain optional as a group, private, and absent from configuration JSON and diagnostics.
- OpenCode continues downloading and validating themes on each setup. Idempotency does not mean skipping the network or silently accepting failed downloads.
- Applied-module state is historical, not proof that desired state still exists. Explicit reruns must repair missing or changed outputs.

## Target Layout

```text
src/
  registry.rs              # Shared spec types and explicit MODULES catalog
  modules/
    mod.rs                 # Module declarations and small declaration constants
    git.rs                 # SPEC, package alternatives, mappings
    bash.rs
    zsh.rs
    powershell.rs
    mise.rs
    brew.rs                # SPEC and Homebrew-specific setup
    scoop.rs               # SPEC and Scoop-specific setup
    atuin.rs                # SPEC, login/status, secret requirement decision
    oh_my_posh.rs
    opencode.rs             # SPEC, config/credential/theme orchestration
  setup/
    mod.rs                 # SetupContext, probe context, typed dispatch
    providers.rs           # Shared provider helpers and compatibility exports
    http.rs                # Existing injected HTTP transport
    http/tests.rs
    jsonc.rs               # Existing JSONC parsing/serialization
  engine.rs                # Generic installation orchestration
  packages.rs              # Package planning and manager adapters
  mappings.rs              # Mapping resolution and managed-section convergence
  platform/
    mod.rs                 # Safe filesystem operations and result types
    unix.rs                # Native permission inspection/application
    windows.rs             # Native ACL inspection/application
```

`src/registry.rs` imports the module definitions and keeps an explicit catalog:

```rust
pub static MODULES: &[ModuleSpec] = &[
    crate::modules::git::SPEC,
    crate::modules::bash::SPEC,
    crate::modules::zsh::SPEC,
    crate::modules::powershell::SPEC,
    crate::modules::mise::SPEC,
    crate::modules::brew::SPEC,
    crate::modules::scoop::SPEC,
    crate::modules::atuin::SPEC,
    crate::modules::oh_my_posh::SPEC,
    crate::modules::opencode::SPEC,
];
```

Each file exports `pub const SPEC: ModuleSpec`. Keep package arrays, mapping arrays, and module-specific markers private. Share only small repeated declaration constants, such as platform slices and shell requirement slices, through `src/modules/mod.rs` with restricted visibility. Do not turn declaration extraction into a macro/builder project.

### Deliberate Boundary: mise Repository Fallback

Mise currently has package-specific APT/DNF fallback methods in `src/packages.rs`, including RHEL and DNF-version branches. Keep those methods and command sequences unchanged in this plan. They run inside a confirmed package installation, whereas module setup runs afterward. Moving them into ordinary `mise::setup` would run them too late and could violate package selection.

The first `mise.rs` therefore owns the module declaration, packages, and mappings, but not every implementation detail of its package adapters. If package-specific strategies later multiply, design a separate typed package-install strategy carried through planning and execution. Do not add that extension merely to make this extraction appear completely self-contained.

## Idempotency Contract

A converged rerun must not replace unchanged destination files, create extra backups, repeat package installations already satisfied by manager queries, or perform another successful Atuin login. Inspection commands, theme downloads, state locking, and temporary download/validation staging remain permitted.

For file operations:

| Existing target | Desired result |
| --- | --- |
| Missing | Create a regular file; report content change; no backup. |
| Regular file with different bytes | Stage, back up, replace; report content change and backup. |
| Regular file with identical bytes | No destination write, replacement staging, or backup; report unchanged. |
| Private regular file with identical bytes and correct permissions | No content or permission mutation; report unchanged. |
| Private regular file with identical bytes and unsafe permissions | Repair permissions without rewriting content; report permission change. |
| Symlink, even to matching bytes | Never classify as unchanged; preserve replacement behavior for replace mappings and reject for managed-section mappings. |
| Directory or another non-regular target | Preserve existing mapping-specific replacement/rejection behavior; never classify as unchanged by reading through it. |
| Inspection/read/permission failure | Return a contextual error, not unchanged or absent. |

Use a shared result that distinguishes creation from no-op and permission repair:

```rust
pub enum FileChange {
    Unchanged,
    Content { backup: Option<PathBuf> },
    Permissions,
}

pub struct FileApplyResult {
    pub target: PathBuf,
    pub change: FileChange,
}
```

No result contains desired bytes or secret values. An `Option<PathBuf>` backup by itself cannot distinguish an unchanged file from a newly created file.

Keep existing unconditional replacement primitives as the lower-level mechanism; add `ensure_file`, timestamp-aware `ensure_file_with_timestamp`, and `ensure_private_file` as the production convergence boundary. Do not duplicate their staging, backup, or rollback implementation. Migrate production mapping and OpenCode callers to the ensure operations. Low-level replacement tests remain useful for failure injection.

Do not promise cross-file transactions, crash recovery, or protection against concurrent edits by unrelated programs. Preserve the installer's existing state lock and per-file failure guarantees; do not weaken existing link handling while adding equality checks.

## Task 1: Capture Behavior and Test Fixtures

**Files:** `tests/registry.rs`, `tests/installer.rs`, `tests/mappings.rs`, `tests/support/mappings.rs`, `tests/support/runner.rs`, `tests/support/opencode.rs`.

- [x] Review existing registry assertions and fill gaps so all ten definitions are covered, including registry order, package alternatives, exact mapping targets/markers, provider relationships, and encrypted bundle associations.
- [ ] Record baseline focused results before moving code: `cargo test --locked --test registry --test assets --test packages --test mappings --test atuin --test providers --test opencode --test installer --test cli`.
- [x] Add reusable fixture support for snapshotting destination bytes, modification times, regular-file/link type, backup paths/counts, and recorded process calls. Keep snapshots of synthetic credential files in memory only.
- [x] Make fake runners able to transition from absent to installed packages and from logged out to logged in, so repeat-run tests exercise real state changes rather than fixed canned success responses.
- [x] Use operation counters or deliberately recognizable timestamps to prove no-op behavior; do not rely on sleeps or filesystem timestamp granularity alone.

**Acceptance:** Existing behavior is characterized without asserting the new no-write behavior yet. No real manager, login service, or network is invoked by fixtures.

## Task 2: Extract Declarative Module Definitions

**Files:** create `src/modules/mod.rs` and all ten `src/modules/*.rs` files shown in the target layout; modify `src/registry.rs` and `src/lib.rs`.

- [x] Export the Rust `modules` namespace from `src/lib.rs`.
- [x] Move each `ModuleSpec` and its private package/mapping constants to its corresponding file. Preserve the existing declarations exactly.
- [x] Replace the registry's large inline definitions with the explicit `SPEC` catalog. Keep `modules()`, `find_module()`, spec types, and existing method behavior available at their current paths.
- [x] Leave setup implementations in their current files for this task. Definitions still reference the same `SetupKind` values.
- [x] Keep manager priority behavior unchanged; moving unrelated model/adapter methods is out of scope.
- [x] Run `cargo fmt --check` and `cargo test --locked --test registry --test assets --test cli --test packages --test installer`.

**Acceptance:** Registry parity tests pass unchanged except for newly added coverage. CLI listing order, defaults, provider resolution, and package planning are identical.

## Task 3: Colocate Custom Setup With Its Module

**Files:** `src/modules/{atuin,brew,scoop,opencode}.rs`, `src/setup/{mod,providers,atuin,opencode}.rs`, existing setup integration tests and fixtures.

- [x] Move Atuin status/login/setup implementation into `src/modules/atuin.rs`, alongside its `SPEC`.
- [x] Move OpenCode configuration, credential, and theme orchestration into `src/modules/opencode.rs`. Keep HTTP and JSONC utilities in `src/setup/`; update imports without changing behavior.
- [x] Split Homebrew and Scoop orchestration into `src/modules/brew.rs` and `src/modules/scoop.rs`.
- [x] Keep genuinely shared provider operations in `src/setup/providers.rs`: ensure Git, clone-if-absent mechanics, directory creation, process PATH handling, verification, and contextual errors. Give helpers only the visibility the module handlers need.
- [x] Keep old public setup paths as thin re-exports, not duplicate implementations: `setup::atuin`, `setup::opencode`, and the existing public provider functions. This avoids an unrelated library API break and keeps integration-test imports stable.
- [x] Keep `setup::run_setup` as exhaustive `SetupKind` dispatch to the canonical implementations. Do not introduce per-module install loops.
- [x] Run `cargo fmt --check` and `cargo test --locked --test atuin --test providers --test opencode --test packages --test installer`.

**Acceptance:** Custom behavior has one implementation in its owning module file. Existing setup APIs, command sequences, error categories, and security tests retain parity.

## Task 4: Implement Idempotent Filesystem Primitives

**Files:** `src/platform/{mod,unix,windows}.rs`, `tests/mappings.rs`, `tests/support/mappings.rs`, other `PlatformOps` fixtures affected by new default methods.

- [x] Add failing tests for identical regular-file no-op, first creation, changed-content backup, same-content symlink replacement, private permission repair, private already-correct no-op, and inspection failures.
- [x] Introduce `FileChange` and `FileApplyResult`, then add ensure operations to `PlatformOps` backed by the existing replacement machinery.
- [x] Inspect the target with `symlink_metadata` before reading bytes. Only compare regular files. Treat only `NotFound` as absence; propagate other errors.
- [x] For equal regular-file bytes, return before replacement staging or backup creation. Preserve existing staged-write/rollback logic for actual content changes.
- [x] Add native permission inspection alongside permission application. On Unix, check the effective private-file mode; on Windows, inspect the protected current-user ACL policy rather than checking a read-only flag or assuming protection from equal bytes.
- [x] Give test doubles a clear permission-inspection implementation; native security tests must use actual filesystem modes/ACLs, not no-op stubs.
- [x] For unchanged private content with unsafe permissions, repair and verify permissions in place, then report `Permissions`. Never return unchanged merely because secret bytes match.
- [x] Preserve private staging protection before installation and protection of private backups. Continue zeroizing buffers used for secret bytes; do not add debug output of file contents or plaintext temporary bundle files.
- [x] Retain regression tests for failed staging, backup failures, final rename failure/restoration, dangling links, and directory targets.
- [ ] Run `cargo fmt --check` and `cargo test --locked --test mappings --test opencode`. Run native Windows permission tests as well as Unix tests before merging.

**Acceptance:** The idempotency matrix passes on both native platforms without weakening existing replacement or credential protection guarantees.

## Task 5: Adopt Ensure Operations in Mappings and OpenCode

**Files:** `src/mappings.rs`, `src/engine.rs`, `src/modules/opencode.rs`, `tests/{mappings,opencode,installer}.rs`, `tests/support/{mappings,opencode,installer}.rs`.

- [x] Change `apply_mapping`, `apply_embedded_mapping`, and `apply_module_mappings` to return the new file-application result(s); update callers and fixture assertions together.
- [x] Route replace mappings through timestamp-aware ensure operations. For managed sections, keep the existing merge and validation rules, then ensure the merged bytes instead of unconditionally replacing the destination.
- [x] Test repeated Bash, Zsh, and PowerShell profile applications: preserve surrounding content, keep exactly one managed section, retain malformed-marker rejection, and create no backup/write on the second converged run.
- [x] Make engine output distinguish `installed:` for content changes, `unchanged:` for converged files, and `secured:` for permission-only repairs. Print `backed up:` only when an actual backup exists. Retain module completion and intentionally skipped-package messages.
- [x] Route OpenCode config and theme outputs through `ensure_file`, and credential outputs through `ensure_private_file`.
- [x] Preserve OpenCode's validate-all-downloads-before-destination-mutation order. Do not start applying config or credential changes while theme fetching/validation can still fail.
- [x] Preserve unrelated config fields/themes, grouped optional provider behavior, plugin deduplication, existing JSONC normalization behavior, and curl transport limits.
- [x] Add repeat-run assertions for OpenCode config, theme, and credential destination bytes/mtimes and backup counts, plus permission-only credential repair.
- [x] Confirm that a changed downloaded theme updates only that destination; identical outputs remain unchanged. Failed downloads or invalid JSON must still leave managed destination files untouched.
- [x] Run `cargo fmt --check` and `cargo test --locked --test mappings --test opencode --test installer`.

**Acceptance:** Normal mappings and OpenCode use the same convergence primitives. Repeated setup still fetches themes but does not rewrite converged destination files or create redundant backups.

## Task 6: Move Secret Requirement Decisions Out of the Engine

**Files:** `src/setup/mod.rs`, `src/modules/{atuin,opencode}.rs`, `src/engine.rs`, `tests/{atuin,installer,secrets}.rs`.

- [x] Add a small `SetupProbeContext` containing platform and runner access, without secrets. Add typed `secret_request(kind, probe) -> Result<Option<SecretMode>>` dispatch.
- [x] Make Atuin's module-owned requirement function inspect login status: return no secret request when logged in, otherwise `Required`. OpenCode returns `Optional`; modules without secret setup return none.
- [x] Replace the engine's direct `SetupKind::Atuin` comparison and `atuin_is_logged_in` call with the generic secret-request result plus `module.secret_bundle`.
- [x] Keep identity prompting, in-memory decryption, and secret-scope lifetime owned by the engine. Preserve existing OpenCode and interactive prompting rules rather than changing the identity UX in this refactor.
- [x] Retain the defensive login-status check in Atuin's public setup function. Two read-only status checks during engine-driven setup are acceptable; avoid a new stateful prepared-setup abstraction just to remove one query.
- [x] Test logged-in Atuin with no identity available, logged-out Atuin with valid and missing credentials, status-command execution failure, optional OpenCode identity absence, and secret-request failure before mapping/state completion.
- [x] Keep `SetupKind` exhaustive. Do not add engine branches naming individual module IDs or reaching into module implementations.
- [x] Run `cargo fmt --check` and `cargo test --locked --test atuin --test secrets --test installer --test opencode`.

**Acceptance:** The engine does not know how Atuin determines its credential requirement. No secret is requested or decrypted for an already authenticated Atuin instance.

## Task 7: Verify Provider and End-to-End Convergence

**Files:** `src/setup/providers.rs`, `src/modules/{brew,scoop}.rs`, `tests/{providers,packages,installer}.rs`, `tests/support/{runner,installer}.rs`, state unit tests in `src/state.rs`.

- [x] Verify already available providers remain no-op bootstraps. Test a valid provider installation present on disk but absent from process PATH: restore discoverability and verify it without recloning.
- [x] Make shared PATH prepending avoid adding duplicate entries on repeated calls, using platform-appropriate path comparisons. Preserve existing order and unrelated entries.
- [x] Keep clone helpers honest: an existing path is not proof of successful setup. Validate the expected manager entry point and reject invalid nonempty destinations with actionable errors. Do not delete or overwrite a user directory to repair it.
- [x] Test partial provider failures followed by a retry. Reuse a valid clone and finish missing managed directories; fail safely for invalid existing destinations. Full automatic recovery of arbitrary broken clones is out of scope.
- [x] Preserve command lookup refresh after successful package installations. Refresh after provider PATH changes where needed so downstream phases observe the provider.
- [x] Add a two-run installer test with shell, mise, and Git selections. The first run installs missing packages and applies files; the second queries installed status but performs no package installs, destination replacements, or new backups.
- [x] Add a two-run Atuin installer test: login once, then rerun without a usable identity and prove there is no second login or decryption attempt.
- [x] Verify explicit reruns repair deleted/changed managed outputs even when the module is already in state. Verify an already recorded shell still gates integrations without rewriting its profile.
- [x] Verify a failed module is not newly recorded and does not erase existing state. A retry must converge already completed files rather than backing them up again unnecessarily.
- [x] Verify repeated `StateStore::record` calls preserve state bytes and mtime, keeping the existing schema and lock semantics. State locking itself is permitted on every run.
- [x] Run `cargo fmt --check` and `cargo test --locked --test providers --test packages --test installer`, plus the state unit tests with `cargo test --locked state::tests`.

**Acceptance:** Full reruns converge rather than reinstalling, relogging, or rewriting. Partial failures remain safely retryable within the documented per-file and provider limits.

## Task 8: Document and Validate the Finished Refactor

**Files:** `docs/modules.md`, `README.md`, `AGENTS.md`; `.github/workflows/tests.yml` only if needed to execute missing native coverage.

- [x] Update `docs/modules.md` to explain module definition ownership, explicit registry assembly, unchanged-file behavior, permission-only repair, and network-dependent OpenCode reruns. Preserve the supported-platform/default-selection table unless a deliberate behavior change requires otherwise.
- [x] Update the README's module-authoring instructions: create `src/modules/<name>.rs`, export `SPEC`, declare it in `src/modules/mod.rs`, add it explicitly to `src/registry.rs`, embed its static assets and any encrypted bundle locally, add focused tests, and update module documentation.
- [x] Clarify `AGENTS.md` that `src/registry.rs` remains the authoritative catalog while declarations and custom setup live in `src/modules/`. Preserve all operational/security constraints.
- [x] Audit production callers for unconditional file replacement, direct module-specific engine calls, duplicate setup implementations, and accidental plaintext diagnostics. Document any intentional low-level replacement callers.
- [x] Run all available checks listed below. Report unavailable native checks explicitly; cross-compilation does not replace native filesystem/ACL tests.

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked -- -D warnings
bash -n bootstrap/install.sh && bash tests/bootstrap.sh
```

On native Windows:

```powershell
pwsh -NoProfile -File .\tests\bootstrap.ps1
cargo build --release --locked --target x86_64-pc-windows-msvc
```

On native Linux:

```sh
cargo build --release --locked --target x86_64-unknown-linux-gnu
```

Run Rust tests and lint on both native platforms through CI. Confirm existing release jobs still build the embedded binary with no source-checkout dependency.

**Acceptance:** All available checks pass, native Linux/Windows CI covers permissions and filesystem behavior, and module-authoring documentation reflects the new layout.

## Suggested Review Boundaries

1. Characterization tests and declarative extraction (Tasks 1-2).
2. Custom setup colocation, preserving public re-exports (Task 3).
3. File convergence primitives and caller migration (Tasks 4-5).
4. Secret-requirement encapsulation (Task 6).
5. Provider/retry convergence and end-to-end regression tests (Task 7).
6. Documentation and full validation (Task 8).

Do not bundle the structural extraction and idempotency changes into one unreviewable diff. Do not commit implementation or documentation changes automatically; follow the operator's requested commit workflow.

## Deferred Work

- Moving mise-specific repository fallback behind a new typed package-install strategy.
- General module traits, registration macros, runtime plugins, or an operation DSL.
- Offline theme caching, conditional HTTP requests, and pinning upstream theme versions.
- Cross-file rollback, package rollback, generalized transaction journals, or changed state schemas.
- Removing compatibility re-exports or unconditional low-level replacement APIs.
- Splitting large module implementations into subdirectories unless file size later justifies it.
