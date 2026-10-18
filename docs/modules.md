# Modules

The installer supports the modules below, as defined in
[`src/registry.rs`](../src/registry.rs). Default selections apply to the
interactive checklist on supported platforms; explicit `--modules` selection
installs only the requested modules.

| Module | Platforms | Selected by default | Purpose |
| --- | --- | --- | --- |
| `git` | Linux, Windows | Yes | Installs Git and writes `~/.gitconfig`. |
| `bash` | Linux | Yes | Updates a managed section in `~/.bashrc`, which loads Bash integration drop-ins. |
| `zsh` | Linux | Yes | Updates a managed section in `~/.zshrc`, which loads Zsh integration drop-ins. |
| `powershell` | Windows | Yes | Updates a managed section in the active PowerShell profile, using `$PROFILE` or the Windows Documents known folder, and loads PowerShell integration drop-ins. |
| `mise` | Linux, Windows | Yes | Installs the mise tool version manager and provides optional shell activation. |
| `brew` | Linux | No | Sets up Homebrew as a package manager and provides optional Bash and Zsh integration. |
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

Shell integration requires the corresponding shell module to be selected in
this run or recorded as previously applied: `bash` or `zsh` on Linux, and
`powershell` on Windows. These requirements gate integration files rather than
implicitly selecting or rerunning shell modules. For example, `--modules mise`
copies the Bash activation script when `bash` is recorded, or the Zsh activation
script when `zsh` is recorded, without replacing either profile. If the shell
module is neither selected nor recorded, its integration is skipped. Existing
shell files alone do not satisfy this gate.

Bash, Zsh, and PowerShell profile mappings replace only the section between
their border comments. Bash uses `# >>> dotfiles managed bash section >>>` and
`# <<< dotfiles managed bash section <<<`; Zsh and PowerShell use the same
markers with `zsh` and `powershell` in place of `bash`. If a profile has no
managed section, the installer appends one while preserving the existing
contents; malformed or unmatched borders stop the update without changing the
file. Existing targets are backed up before updates, and embedded configuration
assets are installed as regular files. See the [README](../README.md) for
package-manager priority and Atuin/OpenCode credential setup.

## Applied Module State

After each module's setup and file mappings succeed, the installer records its
ID in a local JSON file. Completed modules remain recorded if a later module
fails. The interactive checklist labels these modules as `previously applied`;
this does not change default selections or prevent explicitly selected modules
from running again. Previously applied modules also satisfy mapping requirements
for selected modules, such as mise's shell activation, without being rerun.

The state file lives beside the backup directory:

- Linux: `$XDG_STATE_HOME/dotfiles/state.json`, falling back to
  `~/.local/state/dotfiles/state.json`.
- Windows: `%LOCALAPPDATA%\dotfiles\state.json`, falling back to
  `~\AppData\Local\dotfiles\state.json`.

```json
{
  "schema_version": 1,
  "modules": ["bash", "git", "mise"]
}
```

`schema_version` describes the file format only. There are no timestamps,
installer versions, or outdated checks. Entries mean the module was previously
applied, not that its packages or files are still present. Modules are recorded
even when package installation is intentionally skipped, as long as setup and
mappings succeed. Existing entries are retained on failed reruns.

State updates use atomic replacement. An OS-managed lock on the adjacent
`state.lock` file prevents concurrent installations from overwriting state and
is released when the installer exits. Invalid or unsupported state causes an
error before installation changes rather than silently discarding records.
State files are local to the user and should not be committed.

## OpenCode Theme Downloads

OpenCode theme downloads require the installed `curl` executable (`curl.exe`
on Windows). Curl runs directly, without a shell, using its TLS backend and
default certificate and hostname verification. User curlrc files and URL globbing
are disabled, and requests/redirects are restricted to HTTP/HTTPS. Requests have
a 10-second connection timeout, a 60-second total request timeout, and a maximum
of 10 redirects. Theme-tree and individual theme responses are limited
to 1 MiB each, including responses without a declared content length. Downloaded
JSON is validated before managed outputs are replaced.
