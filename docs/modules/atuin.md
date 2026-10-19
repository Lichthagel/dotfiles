# Atuin

- Platforms: Linux and Windows
- Selected by default: No
- Installs Atuin shell history, provides optional shell integration, and
  configures sync login when credentials are available.

Atuin values are `ATUIN_USERNAME`, `ATUIN_PASSWORD`, and `ATUIN_KEY`, with
optional `ATUIN_SYNC_ADDRESS`. Its encrypted bundle is embedded beside the
module definition; the source remains at `secrets/atuin.env.age`. It is decrypted
in memory and plaintext is never written.
