# Rust Installer Migration Design

**Date:** 2026-08-30
**Status:** Design approved in discussion; awaiting written-spec review

## Context

This repository currently provides parallel Bash and PowerShell installers for
Linux and Windows. Both installers parse the same style of module declarations,
select modules interactively or from command-line arguments, resolve package
manager providers and module requirements, install packages, run platform
specific setup scripts, decrypt optional age bundles, and map repository files
into user configuration locations.

The duplicated installers have accumulated equivalent but separate behavior.
The migration will make a single Rust executable the installation and setup
engine. Linux and Windows will share the registry, dependency resolution,
secret handling, setup logic, package planning, and mapping behavior. Only
operating-system operations and package-manager command details will be
platform-specific.

## Goals

- Replace the Bash and PowerShell installer engines with one Rust binary.
- Maximize shared behavior between Linux and Windows.
- Compile static configuration assets and encrypted secret bundles into the
  resulting executable.
- Decrypt embedded age bundles with a Rust library, without requiring an
  external `age` executable.
- Port module setup behavior, including Atuin and OpenCode, to Rust.
- Retain package-manager integration, module dependencies, backups, and
  interactive setup where they remain useful.
- Provide ephemeral Linux and Windows one-liners that download, verify, run, and
  remove a prebuilt x86_64 release binary.
- Preserve dynamic fetching of external OpenCode theme assets.
- Keep plaintext credentials and age private keys out of source and release
  artifacts.

## Non-goals

- Supporting ARM64 release artifacts in the initial migration.
- Persistently installing the `dotfiles` binary on the user’s machine.
- Removing external package-manager executables. The Rust program will still
  invoke an available package manager to install packages.
- Embedding assets that are intentionally fetched dynamically, such as the
  latest Catppuccin OpenCode themes.
- Preserving every spelling of the two existing platform-specific CLIs. The
  replacement will use one unified interface.

## Chosen architecture

The project will become a single Cargo binary with a shared orchestration
engine and narrow platform adapters:

```text
src/
  main.rs                 CLI and exit-code handling
  cli.rs                  argument parsing and interactive mode
  registry.rs             typed module/package/mapping registry
  assets.rs               embedded configuration and secret bytes
  engine.rs               dependency resolution and installation phases
  secrets.rs              native age decryption and temporary secret scope
  packages.rs             package-manager discovery and installation
  mappings.rs             target expansion, backup, and replacement
  setup/
    atuin.rs              Atuin login/setup
    opencode.rs           OpenCode configuration and themes
    package_managers.rs   manager-specific setup helpers where needed
  platform/
    mod.rs                shared platform interface
    unix.rs               Linux paths, permissions, and process behavior
    windows.rs            Windows paths, ACLs, and process behavior
```

The exact file split may be adjusted during implementation, but the boundaries
are intentional:

- The engine knows module semantics and installation order, not shell syntax or
  OS-specific path rules.
- The registry contains typed data and references to embedded assets.
- Setup handlers are Rust functions selected from typed setup variants; there is
  no arbitrary setup-script execution path.
- Platform adapters expose only operations that genuinely differ between Linux
  and Windows: path conventions, file permissions/ACLs, terminal behavior,
  process invocation details, and package-manager commands.

The release binary is the deployment unit. It does not require a repository
checkout to find module definitions, configuration files, setup scripts, or
secret bundles.

## Repository source layout and asset embedding

Static source assets remain human-editable and organized by module:

```text
modules/
  git/gitconfig
  bash/bashrc
  powershell/profile.ps1
  mise/bash/10-mise.bash
  mise/powershell/10-mise.ps1
  oh-my-posh/oh-my-posh.config.json
  ...
secrets/
  atuin.env.age
  opencode.env.age
bootstrap/
  install.sh
  install.ps1
```

Module definitions will be rewritten as typed Rust data. The old
`manifest.conf`, `dependencies.conf`, and per-module `module.conf` files will
be retired after parity is verified, so there is only one runtime source of
module behavior. The remaining source files stay in `modules/` because they are
useful configuration artifacts and are the natural inputs to compile-time
embedding. The registry will explicitly reference each asset with
`include_bytes!` or `include_str!` (or an equivalent build-time embedding
mechanism); it will not glob all files from `secrets/`.

Only the encrypted `*.env.age` bundles are eligible for secret embedding.
Plaintext local files such as `secrets/atuin.env` and
`secrets/opencode.env` remain development-only, must remain gitignored, and
must not be referenced by the build. Build or CI checks should make accidental
plaintext inclusion difficult to introduce.

At release time, the executable contains the static configuration bytes,
profile snippets, prompt configuration, and encrypted secret bytes. The user
runs the binary rather than accessing the repository source paths.

## Typed module registry

The registry will represent the current module capabilities with typed values.
Conceptually, a module contains:

- stable module ID and description;
- supported platforms and default-selection state;
- package alternatives, each with logical package name, manager enum, and
  manager-specific package name;
- module requirements;
- package-manager provider capability, when applicable;
- optional encrypted secret bundle reference;
- typed setup variant, if any;
- typed mappings from platform to embedded asset and target.

Examples of typed concepts include `Platform::{Linux, Windows}`,
`PackageManager::{Apt, Dnf, Pacman, Brew, Mise, Winget, Scoop}`, a `Setup`
enum for `Atuin` and `OpenCode`, and an `Asset` enum for embedded files.
Mappings use target types such as a home-relative path, an absolute path, or a
PowerShell-profile-relative path rather than interpreting Bash or PowerShell
expressions at runtime.

The registry will retain the current modules and their meaningful behavior:
`git`, `bash`, `powershell`, `mise`, `brew`, `scoop`, `atuin`, `oh-my-posh`, and
`opencode`. Provider modules and shell/profile requirements remain represented
as data, not duplicated in the engine.

Adding a module will require a Rust registry change, its static assets, and a
Rust setup handler when setup behavior cannot be expressed by package or
mapping data. This is an intentional tradeoff: arbitrary shell hooks are not
part of the new extension model.

## Installation data flow

The common engine will execute the following high-level flow:

1. Parse the unified CLI and identify the compiled target platform.
2. List modules or select them interactively when no explicit module list was
   supplied.
3. Validate module IDs and platform support.
4. Resolve required modules, package-manager provider modules, and installation
   order.
5. Discover available managers and create a package plan using platform
   priority.
6. Allow manager/package selection interactively, or use the highest-priority
   available manager and include all planned packages when `--yes` is supplied.
7. Install selected packages through the platform package adapter.
8. Run native Rust setup handlers for the selected modules.
9. Materialize embedded assets into their mapped targets, backing up existing
   targets first.
10. Report completed, skipped, and failed work without exposing secret values.

The operation fails fast on a package or required setup failure and identifies
the affected module. Package installation is inherently not transactional and
is not rolled back. Setup handlers stage their own generated files before
committing them.

## Secrets and age handling

The binary embeds encrypted `atuin.env.age` and `opencode.env.age` bytes only.
At runtime, identity resolution follows the existing contract:

1. `AGE_IDENTITY` supplies direct identity contents.
2. `AGE_IDENTITIES` supplies an identity-file path.
3. Otherwise the platform default age identity path is checked.
4. Interactive mode may prompt for an identity path or direct key when the
   default is absent.
5. Noninteractive mode reports a missing identity; optional modules may skip
   optional secret-dependent behavior.

The Rust age library decrypts the embedded bytes in memory. The decrypted
bundle is parsed strictly as environment assignments with valid environment
variable names. Secret values are scoped to the setup operation and dropped as
soon as possible. No temporary plaintext `.env` file is created, and no
external `age` process or automatic `age` package dependency is needed.

Atuin has required credentials when login is necessary. Its native handler
checks login status, validates `ATUIN_USERNAME`, `ATUIN_PASSWORD`, and
`ATUIN_KEY`, optionally supplies `ATUIN_SYNC_ADDRESS`, and passes sensitive
login values through the child process input rather than printing them.

OpenCode treats provider credentials as optional as a group. If none are
available, existing provider configuration is left unchanged. If only some are
available, the setup fails with the missing variable names rather than writing
a partial provider configuration. Secret values are written only to the intended
user-local credential files, not to the tracked or generated OpenCode config.

## Native OpenCode setup

The Rust OpenCode handler will replace both current shell implementations. It
will:

- locate the platform-appropriate user configuration directory;
- read existing JSON or JSONC configuration, including comments and trailing
  commas where supported by the existing file format;
- preserve unrelated settings while updating managed provider, MCP, and plugin
  entries;
- stage provider credential files and apply `0600` permissions on Linux or a
  current-user-only ACL on Windows;
- add the configured `uvx duckduckgo-mcp-server` MCP entry;
- ensure the configured Superpowers Git plugin appears once;
- fetch the latest Catppuccin theme tree and theme JSON files over HTTPS;
- validate fetched JSON and all generated configuration before replacement;
- atomically commit staged configuration, credentials, and themes; and
- remove staging directories on success and failure.

The handler will use Rust HTTP and JSON facilities rather than requiring
`curl` or `jq`. Theme fetching remains dynamic and therefore requires network
access when the OpenCode module is selected. Because the generated files are
separate filesystem entries, staging provides failure-safe per-file replacement,
not a cross-file transaction.

## Platform operations and package managers

Package managers remain external tools, but discovery, installed checks, and
command construction are implemented in Rust behind a shared interface.

Linux manager priority is `apt`, `dnf`, `pacman`, then `brew` and `mise`.
Windows manager priority is `winget`, `scoop`, `brew`, then `mise`. The provider
module mechanism remains: when a package alternative needs a manager that is
not available, its provider module can be resolved and installed first.

Linux system package commands prefer `run0` and fall back to `sudo`. Windows
commands use the corresponding native manager invocation. Package operations
will be tested through injectable/fake command runners so tests never install
software on the host.

## Materialized mappings and backups

The current checkout-based installer can symlink a target to a source file.
That behavior cannot survive an ephemeral binary: after the bootstrap removes
the temporary executable, a symlink to its extracted source would be broken.

The new mapping layer will therefore materialize embedded assets as regular
managed files. It will:

- expand typed home/profile/environment targets using platform rules;
- create parent directories;
- detect files, directories, and symlinks at the destination;
- move an existing destination to a timestamped backup before replacement;
- write and validate through a temporary file, then rename into place; and
- report replacement failures without leaving a partially written managed file.

Linux backups use the existing XDG state convention, and Windows backups use
the existing local application data convention. The backup behavior remains
user-visible and timestamped, but installed mappings are intentionally
independent of the bootstrap binary and source checkout.

## CLI and bootstrap distribution

The unified binary will use the following canonical interface:

```text
dotfiles [OPTIONS]

  --modules <a,b,c>   Select modules without the module checklist
  --yes               Skip package-plan confirmation
  --list              List modules supported by this binary
  --help              Show help
```

`--apps` will be accepted as a compatibility alias for `--modules`; the old
PowerShell-only `-Apps` spelling is not required. With no explicit module list,
the binary opens the interactive selector. Explicit modules are required for
unattended setup, preventing a piped bootstrap from waiting for module input.

The repository will retain only minimal delivery helpers in `bootstrap/`:

```sh
curl -fsSL https://raw.githubusercontent.com/Lichthagel/dotfiles/main/bootstrap/install.sh | sh
```

```powershell
irm https://raw.githubusercontent.com/Lichthagel/dotfiles/main/bootstrap/install.ps1 | iex
```

Each helper detects the operating system and x86_64 architecture, downloads the
matching GitHub Release executable and SHA-256 checksum, verifies the checksum,
forwards arguments, runs from a temporary location, and removes the temporary
files in normal cleanup paths. A version or repository environment override
will support pinned releases and forks. The helper never installs a persistent
`dotfiles` command.

GitHub Actions will build and publish at least:

- `x86_64-unknown-linux-gnu` for Linux;
- `x86_64-pc-windows-msvc` for Windows.

Release builds will be locked, formatted, linted, and accompanied by checksums.

## CLI behavior and error policy

The command will use stable high-level outcomes:

- invalid options or unknown/unsupported modules exit with status `2`;
- interactive cancellation exits successfully with a cancellation message;
- failed package installation, unavailable package manager, decryption failure,
  invalid embedded asset, or required setup failure exits nonzero;
- optional OpenCode provider secrets leave provider configuration unchanged when
  absent, but incomplete provider bundles fail clearly;
- no progress, error, or diagnostic output may contain secret values.

The implementation should use contextual errors that identify the module,
package, target, or operation without including sensitive input. It should not
claim full rollback after a package-manager operation.

## Testing and acceptance criteria

The migration is complete when the Rust binary and release process provide:

- one shared Rust implementation for Linux and Windows orchestration;
- no runtime dependency on Bash, PowerShell, `jq`, `curl`, or external `age`
  for installer/setup logic (package executables and the bootstrap transport
  are exceptions);
- static module assets and encrypted secret bundles present in the release
  binary;
- native age decryption with no plaintext secret or private key in source,
  tests, or release artifacts;
- native Atuin and OpenCode setup parity for the currently supported behavior;
- regular-file mapping with backups and safe replacement;
- ephemeral, checksum-verified x86_64 Linux and Windows bootstraps;
- dynamic OpenCode theme fetching retained; and
- passing automated verification.

Automated verification will include:

- `cargo fmt --check`, `cargo test`, and `cargo clippy -- -D warnings`;
- registry/platform filtering and dependency/provider resolution tests;
- fake package-manager tests for availability, installed checks, priority, and
  command construction;
- temporary-home tests for target expansion, backups, regular-file mapping,
  and atomic replacement;
- generated-identity age decryption and strict secret parsing tests;
- Atuin process interaction tests that assert sensitive values are not logged;
- OpenCode JSON/JSONC merge, idempotency, credential permission, and staged
  theme tests;
- CLI and exit-status integration tests;
- Linux and Windows integration tests on their native CI runners;
- bootstrap syntax and mocked download/checksum/cleanup tests; and
- release builds for both target triples.

All tests use temporary homes and synthetic credentials. No repository secret,
private key, plaintext environment bundle, or real external credential is used
by CI.

## Deliberate behavior changes and risks

1. **Mappings are regular files, not source symlinks.** This is required by
   ephemeral execution and makes installed configuration portable after setup.
2. **Module extensions require Rust.** Removing arbitrary shell setup improves
   portability but raises the cost of adding a behavior-rich module.
3. **Package managers remain prerequisites for package installation.** The
   binary is self-contained with respect to its installer logic, not a package
   manager replacement.
4. **OpenCode themes remain network-dependent.** This preserves the current
   latest-theme behavior but means OpenCode setup can fail when the upstream
   service is unavailable.
5. **Release artifacts must be rebuilt when static assets change.** This is
   intentional: the binary, not the checkout, is the source of runtime assets.
