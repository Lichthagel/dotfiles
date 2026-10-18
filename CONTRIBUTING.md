# Contributing

## Test Requirements

Linux builds require `pkg-config`, OpenSSL development headers/libraries, and
system CA certificates. Install them before running Cargo:

```sh
# Debian/Ubuntu
sudo apt-get install pkg-config libssl-dev ca-certificates
# Fedora
sudo dnf install pkgconf-pkg-config openssl-devel ca-certificates
```

Windows uses Schannel and the system certificate store; no OpenSSL development
package is needed. Linux artifacts dynamically link to the build machine's
OpenSSL ABI, so verify runtime compatibility when distributing local builds.
Do not enable vendored OpenSSL just to avoid installing build dependencies;
that would put the TLS implementation back into the executable.

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
the R2 `dotfiles/` prefix. The publish job targets the GitHub Actions
`production` environment. Configure `CLOUDFLARE_ACCOUNT_ID`,
`CLOUDFLARE_R2_ACCESS_KEY_ID`, `CLOUDFLARE_R2_SECRET_ACCESS_KEY`, and
`CLOUDFLARE_R2_BUCKET` as environment secrets to enable uploads; environment
protection rules can gate deployments. No GitHub Release is created.

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

Release builds use size optimization (`opt-level = "z"`), full LTO, a single
codegen unit, and symbol stripping, as configured in `Cargo.toml`. Panic
unwinding remains enabled so terminal and secret guards can run their destructors.
Clap retains help, usage, and error context without optional color or typo
suggestions. HTTP downloads use synchronous `ureq` with native TLS and the
system certificate store (OpenSSL on Linux, Schannel on Windows), with no
optional compression or JSON features. Certificate and hostname verification
remain enabled; JSON validation stays in `serde_json`.

Build and compare native release artifacts when changing these settings or
network dependencies:

```sh
cargo build --release --locked --target x86_64-unknown-linux-gnu
```

```powershell
cargo build --release --locked --target x86_64-pc-windows-msvc
```

Compare `opt-level = "s"` against `"z"` rather than assuming `"z"` is smaller;
`"z"` was selected after measuring the smaller Linux artifact. Recheck both
platforms when changing toolchains or dependencies. Keep the embedded
configuration and age bundles intact, and do not use executable packers as a
substitute for compiler and dependency size reductions.

The Linux and Windows bootstrap helpers are delivery transport only. They must
download the current artifacts from `https://r2.licht.moe/dotfiles/`, select the
correct x86_64 binary, verify its SHA-256 checksum before running it, forward
arguments and exit status, and remove temporary files. They must not contain
module or setup logic.
