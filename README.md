# Dotfiles

Minimal cross-platform dotfiles with a single Rust installer binary. Linux and
Windows use the same module registry, package planning, native setup handlers,
secret handling, and embedded configuration assets.

## Setup

The published one-liners download the matching prebuilt x86_64 binary, verify
its SHA-256 checksum, run it from a temporary directory, and remove it when
setup finishes. The binary is not installed persistently:

```sh
curl -fsSL https://raw.githubusercontent.com/Lichthagel/dotfiles/main/bootstrap/install.sh | sh
```

```powershell
irm https://raw.githubusercontent.com/Lichthagel/dotfiles/main/bootstrap/install.ps1 | iex
```

The bootstraps support Linux `x86_64` and Windows `x86_64`. Set
`DOTFILES_REPO_URL` to test a fork or another release repository and
`DOTFILES_VERSION` to pin a release tag. The Windows bootstrap passes the
current PowerShell `$PROFILE` through `DOTFILES_PROFILE` so profile mappings
work when the binary is run outside PowerShell.

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
Explicit module selection is required for unattended setup.

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

GitHub Actions builds release binaries for `x86_64-unknown-linux-gnu` and
`x86_64-pc-windows-msvc`, publishes matching `.sha256` files, and runs the
native test suites.

The current modules are `git`, `bash`, `powershell`, `mise`, `brew`, `scoop`,
`atuin`, `oh-my-posh`, and `opencode`. `atuin`, `oh-my-posh`, and `opencode`
are opt-in. Existing targets are moved into timestamped backup directories
before replacement. Embedded mappings are installed as regular files, so they
do not depend on the temporary binary or a source checkout.

### Oh My Posh

Oh My Posh is an opt-in cross-platform prompt module. Select it interactively
or use `--modules bash,oh-my-posh` / `--modules powershell,oh-my-posh` alongside
the corresponding shell module. The module installs the shared prompt
configuration and a shell-specific integration drop-in.

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
profile directories. Select both modules to enable an integration:
`--modules bash,atuin` installs Atuin's Bash drop-in, and
`--modules powershell,atuin` installs its PowerShell drop-in. Selecting only
`atuin` performs package and login setup without changing a shell profile.

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
from upstream. The module does not select or add a default theme. OpenCode also configures the
local MCP command `uvx duckduckgo-mcp-server`. The Superpowers plugin is
managed from `superpowers@git+https://github.com/obra/superpowers.git`.

## Legacy checkout compatibility

Checkouts from before the Rust binary migration retain the old entry points for
that checkout only:

```sh
./install.sh --apps opencode
curl -fsSL https://raw.githubusercontent.com/Lichthagel/dotfiles/main/install.sh | sh
```

```powershell
.\install.ps1 -Apps opencode
irm https://raw.githubusercontent.com/Lichthagel/dotfiles/main/install.ps1 | iex
```

These legacy commands are not used by the current release bootstraps and will
be removed after migration parity is complete. Never commit the plaintext
secret environment file.

## Adding Modules

Add a typed module entry to `src/registry.rs`, add its static configuration
files under `modules/<name>/`, and reference those files explicitly from
`src/assets.rs`. Setup behavior belongs in a Rust handler under `src/setup/`;
do not add arbitrary shell or PowerShell setup hooks. Keep platform-specific
mappings typed and declare shell/profile requirements in the registry.
