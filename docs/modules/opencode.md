# OpenCode

- Platforms: Linux and Windows
- Selected by default: No
- Installs OpenCode, writes a v2 configuration referencing the
  [JSON Schema](https://opencode.ai/config.json), fetches themes, and optionally
  configures provider credentials.

## Configuration

The module writes v2 configuration keys: `providers`, `mcp.servers`, and
`plugins`. It adds `$schema: "https://opencode.ai/config.json"` for editor
validation and migrates legacy MCP-server and plugin entries. When provider
credentials are configured, legacy provider records are converted as well.
Provider credentials remain in protected files and are referenced with
`{file:...}`.

Provider setup is optional as a group, writes protected user-local credential
files, leaves existing provider configuration unchanged when absent, and keeps
secret values out of JSON configuration and diagnostics. Encrypted secret
bundles are embedded beside the module definition. Their sources remain under
`secrets/`; bundles are decrypted in memory and plaintext is never written.

## Theme Downloads

Theme downloads require the installed `curl` executable (`curl.exe` on Windows).
Curl runs directly, without a shell, using its TLS backend and default
certificate and hostname verification. User curlrc files and URL globbing are
disabled, and requests/redirects are restricted to HTTP/HTTPS. Requests have a
10-second connection timeout, a 60-second total request timeout, and a maximum
of 10 redirects. Theme-tree and individual theme responses are limited to 1 MiB
each, including responses without a declared content length. Downloaded JSON is
validated before managed outputs are considered for replacement. Every setup
rerun still fetches upstream themes, but converged config, credential, and theme
files are not rewritten or backed up.
