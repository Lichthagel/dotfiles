# Modules

The installer supports the modules below, as defined in
[`src/registry.rs`](../src/registry.rs). Default selections apply to the
interactive checklist on supported platforms; explicit `--modules` selection
installs only the requested modules.

| Module | Platforms | Selected by default | Purpose |
| --- | --- | --- | --- |
| `git` | Linux, Windows | Yes | Installs Git and writes `~/.gitconfig`. |
| `bash` | Linux | Yes | Writes `~/.bashrc`, which loads shell integration drop-ins. |
| `powershell` | Windows | Yes | Writes the PowerShell profile, which loads shell integration drop-ins. |
| `mise` | Linux, Windows | Yes | Installs the mise tool version manager and provides optional shell activation. |
| `brew` | Linux | No | Sets up Homebrew as a package manager and provides optional Bash integration. |
| `scoop` | Windows | No | Sets up Scoop as a package manager and provides optional PowerShell integration. |
| `atuin` | Linux, Windows | No | Installs Atuin shell history, provides optional shell integration, and configures sync login when credentials are available. |
| `oh-my-posh` | Linux, Windows | No | Installs Oh My Posh, writes the shared prompt configuration, and provides optional shell integration. |
| `opencode` | Linux, Windows | No | Installs and configures OpenCode, fetches themes, and optionally configures provider credentials. |

## Selecting Modules

Use comma-separated module IDs to skip the interactive checklist:

```sh
cargo run -- --modules git,bash,mise --yes
```

```powershell
cargo run -- --modules git,powershell,mise --yes
```

`--apps` is a compatibility alias for `--modules`. `--yes` accepts the package
plan without prompting; unattended setup still requires explicit module
selection. Use `--list` to list modules supported by the current binary.

Shell integration requires selecting the corresponding shell module as well:
`bash` on Linux or `powershell` on Windows. These requirements gate integration
files rather than implicitly selecting modules. For example, selecting only
`atuin` performs package/login setup without installing its shell integration.

Existing targets are backed up before replacement, and embedded configuration
assets are installed as regular files. See the [README](../README.md) for
package-manager priority and Atuin/OpenCode credential setup.

OpenCode theme downloads require the installed `curl` executable (`curl.exe`
on Windows). Curl runs directly, without a shell, using its TLS backend and
default certificate and hostname verification. User curlrc files and URL globbing
are disabled, and requests/redirects are restricted to HTTP/HTTPS. Requests have
a 10-second connection timeout, a 60-second total request timeout, and a maximum
of 10 redirects. Theme-tree and individual theme responses are limited
to 1 MiB each, including responses without a declared content length. Downloaded
JSON is validated before managed outputs are replaced.
