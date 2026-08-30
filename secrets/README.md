# Encrypted Secrets

The Rust release binary embeds `secrets/atuin.env.age` and
`secrets/opencode.env.age` as encrypted payloads. It decrypts a selected bundle
in memory with the Rust age library during setup. The installer does not
install or invoke an external `age` command, and it never writes a temporary
plaintext `.env` bundle.

Do not commit a plaintext `.env` file, an age private key, or a decrypted
credential file. Plaintext source files are local development inputs and are
ignored by Git.

## Atuin

Create a local plaintext file with these values:

```text
ATUIN_USERNAME=your-username
ATUIN_PASSWORD=your-password
ATUIN_KEY=your-atuin-encryption-key
ATUIN_SYNC_ADDRESS=
```

Encrypt it to the repository path using a recipient from your age identity:

```sh
age -r AGE_RECIPIENT -o secrets/atuin.env.age atuin.env
rm atuin.env
```

When `atuin` is selected, the binary reads the encrypted bundle and expects
`ATUIN_USERNAME`, `ATUIN_PASSWORD`, and `ATUIN_KEY`; `ATUIN_SYNC_ADDRESS` is
optional. Configure the decryption identity with `AGE_IDENTITIES` as an
identity-file path, `AGE_IDENTITY` as direct key contents, or the platform
default age identity path. Interactive setup can prompt for a path or direct
key; unattended setup must provide an identity through the environment or
platform default.

## OpenCode

The OpenCode bundle requires these variable names:

```text
OPENROUTER_API_KEY
AZURE_API_KEY
AZURE_RESOURCE_NAME
DIGITALOCEAN_ACCESS_TOKEN
```

Provider setup is optional as a group. Without a bundle or identity, existing
provider configuration remains unchanged. A partially populated bundle fails
before managed output is replaced. When all values are available, Rust writes
them to protected user-local credential files and stores `{file:...}`
references in the OpenCode configuration; secret values are not written to
tracked files or JSON configuration.
