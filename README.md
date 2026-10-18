# Dotfiles

Minimal cross-platform dotfiles with one Rust installer binary. Linux and
Windows share the typed module registry, package planning, native setup
handlers, age secret handling, and embedded configuration assets.

## Setup

The published one-liners download the latest prebuilt x86_64 binary from
Cloudflare R2, verify its SHA-256 checksum, run it from a temporary directory,
and remove it when setup finishes. The binary is not installed persistently:

```sh
curl -fsSL https://raw.githubusercontent.com/Lichthagel/dotfiles/main/bootstrap/install.sh | sh
```

```powershell
irm https://raw.githubusercontent.com/Lichthagel/dotfiles/main/bootstrap/install.ps1 | iex
```

The bootstraps support Linux `x86_64` and Windows `x86_64`. Every push to
`main` validates and builds both binaries, then replaces the files under the
R2 `dotfiles/` prefix at `https://r2.licht.moe/dotfiles/`. There are no
versioned downloads; the bootstrap always installs the current build. Set
`DOTFILES_BASE_URL` to test another download endpoint. The Windows bootstrap
passes the current PowerShell `$PROFILE` through `DOTFILES_PROFILE` so profile
mappings work when the binary is run outside PowerShell. Direct binary runs
resolve the Windows Documents known folder, so redirected locations such as
`D:\Documents\PowerShell` are respected too.

OpenCode theme downloads reuse the installed `curl` executable (`curl.exe` on
Windows), including its TLS backend and default certificate verification. Curl
and a working certificate trust store are required when selecting OpenCode;
other modules do not need an HTTP downloader. The Linux bootstrap already uses
curl. The installer binary does not link OpenSSL or embed an HTTP/TLS stack.
User curl configuration files are ignored to preserve download safeguards.
See [Contributing](CONTRIBUTING.md) for test dependencies.

For a local development build:

```sh
cargo run -- --modules git,bash --yes
```

```powershell
cargo run -- --modules git,powershell --yes
```

## Options

- `--modules git,bash` selects modules without prompting.
- `--apps git,bash` is a compatibility alias for `--modules`.
- `--yes` selects the highest-priority available package manager and accepts
  the complete package plan without prompting.
- `--list` lists modules supported by the current binary.
- `--help` prints usage.

Without `--modules`/`--apps`, the binary opens the interactive module checklist.
Explicit module selection is required for unattended setup. The checklist marks
previously applied modules using a local state file; selected modules still run
again. See [applied module state](docs/modules.md#applied-module-state) for paths
and behavior.

A package is skipped when its command is already in `PATH`, using the logical name from the module declaration, so a tool installed outside the package manager is not reinstalled and needs no available package manager.

## Testing

Run the Rust verification matrix from a checkout:

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked -- -D warnings
bash -n bootstrap/install.sh
bash tests/bootstrap.sh
```

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --locked -- -D warnings
pwsh -NoProfile -File .\tests\bootstrap.ps1
```

GitHub Actions builds binaries for `x86_64-unknown-linux-gnu` and
`x86_64-pc-windows-msvc`, publishes matching `.sha256` files to Cloudflare R2,
and runs the native test suites. The publish job uses the `production`
Actions environment; configure `CLOUDFLARE_ACCOUNT_ID`,
`CLOUDFLARE_R2_ACCESS_KEY_ID`, `CLOUDFLARE_R2_SECRET_ACCESS_KEY`, and
`CLOUDFLARE_R2_BUCKET` as environment secrets for uploads. Environment
protection rules can gate deployments.

See the [module reference](docs/modules.md) for a table of supported modules,
platforms, and default selections.

The current modules are `git`, `bash`, `powershell`, `mise`, `brew`, `scoop`,
`atuin`, `oh-my-posh`, and `opencode`. `atuin`, `oh-my-posh`, and `opencode`
are opt-in. Existing targets are moved into timestamped backup directories
before updates. Bash and PowerShell profiles update only their border-commented
managed sections, preserving any text outside those sections. Other embedded
mappings are materialized as regular files, so they do not depend on the
temporary binary or a source checkout.

Package-manager priority is `apt`, `dnf`, `pacman`, `brew`, `mise` on Linux and
`winget`, `scoop`, `brew`, `mise` on Windows. Linux system managers prefer
`run0`, then `sudo`.

### Oh My Posh

Oh My Posh is an opt-in cross-platform prompt module. Select it interactively
or use `--modules bash,oh-my-posh` / `--modules powershell,oh-my-posh` alongside
the corresponding shell module, unless that shell is already recorded as
applied. The module installs the shared prompt configuration and a shell-specific
integration drop-in.

### Atuin

Atuin is an opt-in package module. Select it interactively or use
`--modules atuin`. The module installs the Atuin package and, when the embedded
encrypted bundle is available and an identity is configured, performs sync
login. Shell integration remains separate mapping behavior.

The binary embeds `secrets/atuin.env.age` and decrypts it in memory with the
Rust age library. It expects `ATUIN_USERNAME`, `ATUIN_PASSWORD`, and
`ATUIN_KEY`; `ATUIN_SYNC_ADDRESS` is optional. `AGE_IDENTITIES` supplies an
identity-file path, `AGE_IDENTITY` supplies direct key contents, and the
platform default identity path is used otherwise. No external `age` command or
age package is required by the installer.

Shell modules load numbered integration drop-ins from their platform-specific
profile directories. Mapping requirements are gates, not implicit module
selections: a selected module's drop-in is installed when its required shell
module is selected or recorded as previously applied. Use `--modules bash,atuin`
on Linux or `--modules powershell,atuin` on Windows for a first installation
with shell integration. Once the shell is recorded, selecting only `atuin` or
`mise` also copies its shell integration without rerunning the shell module.
Without a selected or previously applied shell, these modules skip their shell
integration and do not create the shell profile.

### OpenCode

OpenCode is an opt-in cross-platform module. Select it interactively or use
`--modules opencode`. The binary embeds `secrets/opencode.env.age` and decrypts
it in memory when an age identity is available. Provider values are written to
protected user-local credential files and referenced from OpenCode configuration
with `{file:...}` values; secret values are never written to the JSON config.

The expected provider variables are `OPENROUTER_API_KEY`, `AZURE_API_KEY`,
`AZURE_RESOURCE_NAME`, and `DIGITALOCEAN_ACCESS_TOKEN`. Provider setup is
optional as a group: absent values leave existing provider configuration
unchanged, while a partial bundle fails before managed files are replaced.
Never commit a plaintext environment file, age private key, or decrypted
credential file.

Each setup rerun fetches and validates the latest Catppuccin OpenCode themes
from upstream. The module does not select or add a default theme. OpenCode also
configures the local MCP command `uvx duckduckgo-mcp-server`. The Superpowers
plugin is managed from `superpowers@git+https://github.com/obra/superpowers.git`.

## Adding Modules

Add a typed module entry to `src/registry.rs`, add its static configuration
files under `modules/<name>/`, and reference those files explicitly from
`src/assets.rs`. Setup behavior belongs in a Rust handler under `src/setup/`;
do not add arbitrary shell or PowerShell setup hooks. Keep platform-specific
mappings typed and declare shell/profile requirements in the registry.
