# Dotfiles

Minimal cross-platform dotfiles with one Rust installer binary. Linux and
Windows share the typed module registry, package planning, native setup
handlers, age secret handling, and embedded configuration assets.

## Setup

The published one-liners download the latest prebuilt x86_64 binary from
Cloudflare R2, verify its SHA-256 checksum, run it from a temporary directory,
and remove it when setup finishes. The binary is not installed persistently:

```sh
curl -fsSL https://r2.licht.moe/dotfiles/bootstrap/install.sh | sh
```

```powershell
irm https://r2.licht.moe/dotfiles/bootstrap/install.ps1 | iex
```

The bootstraps support Linux `x86_64` and Windows `x86_64`. Every push to
`main` validates and builds both binaries, then replaces the binaries,
checksums, and bootstrap scripts under the R2 `dotfiles/` prefix at
`https://r2.licht.moe/dotfiles/`. The bootstraps are published at
`/bootstrap/install.sh` and `/bootstrap/install.ps1`. There are no versioned
downloads; the bootstrap always installs the current build. Set
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
  the complete package plan and installation order without prompting.
- `--list` lists modules supported by the current binary.
- `--help` prints usage.

Without `--modules`/`--apps`, the binary opens the interactive module checklist
with the platform's defaults preselected. Review the checklist and press Enter
to approve that selection; toggle modules with Space. Provider modules such as
`brew` and `scoop` start unchecked and are never added implicitly. Already
available package managers can be used without selecting their provider module.
Installing or configuring a missing manager requires selecting its module.
`--yes` skips package and installation-order confirmation, but it does not
approve additional modules. The checklist marks previously applied modules
using a local state file, but that status does not change defaults or
prevent explicitly selected modules from running again. See [applied module state](docs/modules.md#applied-module-state) for paths
and behavior.

A package is skipped when its explicitly declared `command` is already in
`PATH`, so a tool installed outside the package manager is not reinstalled and
needs no available package manager. Each package has a stable `id` and nested
manager-specific `sources`; packages without a command use package-manager
records to detect existing installations.

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
`x86_64-pc-windows-msvc`, publishes matching `.sha256` files and the bootstrap
scripts to Cloudflare R2 and runs the native test suites. The publish job uses
the `production` Actions environment; configure `CLOUDFLARE_ACCOUNT_ID`,
`CLOUDFLARE_R2_ACCESS_KEY_ID`, `CLOUDFLARE_R2_SECRET_ACCESS_KEY`, and
`CLOUDFLARE_R2_BUCKET` as environment secrets for uploads. Environment
protection rules can gate deployments.

See the [module reference](docs/modules.md) for a table of supported modules,
platforms, and selection behavior.

The current modules are `git`, `bash`, `zsh`, `powershell`, `mise`, `brew`,
`scoop`, `atuin`, `oh-my-posh`, and `opencode`. `brew`, `scoop`, `atuin`,
`oh-my-posh`, and `opencode` start unchecked. On Linux, `git`, `bash`, `zsh`,
and `mise` start selected; on Windows, `git`, `powershell`, and `mise` start
selected. Existing targets are moved into
timestamped backup directories before updates. Bash, Zsh, and PowerShell
profiles update only their border-commented managed sections, preserving any
text outside those sections.
Other embedded mappings are materialized as regular files, so they do not depend
on the temporary binary or a source checkout.

Package-manager priority is `apt`, `dnf`, `pacman`, `brew`, `mise` on Linux and
`winget`, `scoop`, `brew`, `mise` on Windows. Linux system managers prefer
`run0`, then `sudo`.

### Oh My Posh

Oh My Posh is an opt-in cross-platform prompt module. Select it interactively
or use one of `--modules bash,oh-my-posh`, `--modules zsh,oh-my-posh`, or
`--modules powershell,oh-my-posh` together with the corresponding shell module,
unless that shell is already recorded as applied. The module installs the shared
prompt configuration and a shell-specific integration drop-in.

### Atuin

Atuin is an opt-in package module. Select it interactively or use
`--modules atuin`. The module installs the Atuin package and, when the embedded
encrypted bundle is available and an identity is configured, performs sync
login. Shell integration remains separate mapping behavior.

The binary embeds `secrets/atuin.env.age` and decrypts it in memory with the
Rust age library. It expects `ATUIN_USERNAME`, `ATUIN_PASSWORD`, and
`ATUIN_KEY`; `ATUIN_SYNC_ADDRESS` is optional. `AGE_IDENTITIES` supplies an
identity-file path, `AGE_IDENTITY` supplies direct key contents, and the
platform default identity path is used otherwise. Login passes the password via
stdin and the encryption key via `ATUIN_ENCRYPTION_KEY`, so neither secret is
added to command-line arguments. No external `age` command or age package is
required by the installer.

Shell modules load numbered integration drop-ins from their platform-specific
profile directories: Bash and Zsh use `~/.config/bashrc.d/` and
`~/.config/zshrc.d/`; PowerShell uses `Profile.d` beside its profile. Mapping
requirements are gates, not implicit module selections: a selected module's
drop-in is installed when its required shell module is selected or recorded as
previously applied. Use `--modules bash,atuin` or `--modules zsh,atuin` on Linux,
or `--modules powershell,atuin` on Windows, for a first installation with shell
integration. Once the shell is recorded, selecting only `atuin` or `mise` also
copies its shell integration without rerunning the shell module. Without a
selected or previously applied shell, these modules skip their shell integration
and do not create the shell profile.

### OpenCode

OpenCode is an opt-in cross-platform module. Select it interactively or use
`--modules opencode`. The module writes an OpenCode v2 config with the
[`https://opencode.ai/config.json`](https://opencode.ai/config.json) JSON Schema
reference. The binary embeds `secrets/opencode.env.age` and decrypts it in memory
when an age identity is available. Provider values are written to protected
user-local credential files and referenced from OpenCode configuration with
`{file:...}` values; secret values are never written to the JSON config.

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

Create `src/modules/<name>.rs` with a public typed `SPEC` and keep that
module's package and mapping declarations there. Put custom module behavior in
the same file; reuse shared operations from `src/setup/`, `src/mappings.rs`,
and `src/platform/`. Declare the module in `src/modules/mod.rs` and add its
`SPEC` explicitly to `src/registry.rs`, which remains the authoritative catalog.
Add static configuration under `modules/<name>/` and embed each file explicitly
beside its mapping in `src/modules/<name>.rs`. If the module uses an encrypted
secret bundle, embed its `secrets/*.env.age` file beside the module definition as
well. Update `docs/modules.md` and add focused tests. Do not add arbitrary shell
or PowerShell setup hooks.

Repeated application converges: identical regular files are not rewritten or
backed up, and private files have their permissions checked and repaired when
needed. Content-changing replacements retain the existing staged-write and
backup behavior. OpenCode still fetches and validates upstream themes on each
setup run.
