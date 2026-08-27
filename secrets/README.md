# Encrypted Secrets

The Atuin module expects `secrets/atuin.env.age` when `atuin` is selected. The installer automatically adds the `age` package whenever a selected module has a matching encrypted secrets bundle.
Do not commit a plaintext `.env` file or an age private key.

Create a local plaintext file with these values:

```text
ATUIN_USERNAME=your-username
ATUIN_PASSWORD=your-password
ATUIN_KEY=your-atuin-encryption-key
ATUIN_SYNC_ADDRESS=
```

Encrypt it to the repository path using the recipient from your age identity setup:

```sh
age -r AGE_RECIPIENT -o secrets/atuin.env.age atuin.env
rm atuin.env
```

The installers decrypt this file only into a temporary directory, use the values to run `atuin login`, and remove the temporary plaintext file afterward. The age identity is read from `AGE_IDENTITIES`, `AGE_IDENTITY` (direct key contents), or the platform default path. If no identity is found in an interactive terminal, the installer prompts for a path or direct key.

The OpenCode module expects the encrypted bundle `secrets/opencode.env.age` with these required variable names:

```text
OPENROUTER_API_KEY
AZURE_API_KEY
AZURE_RESOURCE_NAME
DIGITALOCEAN_ACCESS_TOKEN
```

When OpenCode is selected, the installer decrypts the bundle only for setup. The setup hook materializes the values into protected, user-local credential files under the OpenCode configuration directory; the values are not written to tracked files or `opencode.json`.
