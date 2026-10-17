# Contributing

## Test Requirements

Run the complete local Rust matrix before opening a change:

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

The tests use isolated temporary homes, fake package-manager commands, fake
HTTP responses, generated age identities, and synthetic credentials. They do
not install packages or contact external services. Native Windows tests cover
profile paths and current-user-only credential ACLs; Linux CI covers Linux
permissions and release compilation.

GitHub Actions runs the Rust tests and Clippy on native Linux and Windows
runners, exercises both ephemeral bootstraps, and builds binaries for
`x86_64-unknown-linux-gnu` and `x86_64-pc-windows-msvc`. On each push to
`main`, the publish workflow overwrites the latest binaries and checksums under
the R2 `dotfiles/` prefix. Set the `CLOUDFLARE_ACCOUNT_ID`,
`CLOUDFLARE_R2_ACCESS_KEY_ID`, `CLOUDFLARE_R2_SECRET_ACCESS_KEY`, and
`CLOUDFLARE_R2_BUCKET` Actions secrets to enable uploads; no GitHub Release is
created.

## Adding a Module

Add a typed `ModuleSpec` entry and its package/mapping declarations to
`src/registry.rs`. Put human-editable static configuration under
`modules/<name>/` and add an explicit `AssetId` reference in `src/assets.rs`.
Mappings must use typed platform targets and list shell/profile requirements in
the registry.

If a module needs setup beyond package installation and asset mapping, add a
focused Rust handler under `src/setup/` and dispatch it through `SetupKind`.
Do not add arbitrary Bash or PowerShell setup hooks. Provider setup must use the
explicit typed adapter boundary and must not evaluate downloaded installer
scripts.

## Secrets and OpenCode

Encrypted bundles belong at `secrets/atuin.env.age` and
`secrets/opencode.env.age`; they are embedded explicitly into the release
binary. Rust decrypts them in memory using the age library. Never commit
plaintext environment files, age private keys, decrypted credential files, or
real credentials.

Secret tests must generate age identities and use synthetic values. OpenCode
coverage must preserve optional all-or-none provider behavior, protected
user-local credential files, JSON/JSONC idempotency, the
`uvx duckduckgo-mcp-server` MCP entry, the configured Superpowers Git plugin,
and latest Catppuccin theme fetching with staged failure-safe updates.

## Release and Bootstrap Changes

The Linux and Windows bootstrap helpers are delivery transport only. They must
download the current artifacts from `https://r2.licht.moe/dotfiles/`, select the
correct x86_64 binary, verify its SHA-256 checksum before running it, forward
arguments and exit status, and remove temporary files. They must not contain
module or setup logic.
