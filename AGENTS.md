# Agent Instructions

## Repository Shape

- `install.sh` is the Linux entry point; `install.ps1` is the Windows entry point.
- `modules/manifest.conf` controls module discovery; each listed module must have `modules/<name>/module.conf`.
- Module `module.conf` files are parsed as restricted `key=value` data, not sourced or executed; preserve platform-qualified `map=`, `package=`, `secret=`, and `setup=` entries.
- `default=true|false` controls the interactive checklist only; `--apps`/`-Apps` explicitly overrides defaults.
- `atuin` is opt-in (`default=false`) and can run package installation plus platform-specific setup; do not add Atuin-specific logic to the installers.

## Commands

- Linux syntax and installer tests: `bash -n install.sh && bash tests/test-install.sh`
- Linux package tests: `bash tests/test-packages.sh`
- Linux secret/setup tests: `bash tests/test-secrets.sh`
- Windows installer, package, and parser tests: `pwsh -NoProfile -File .\tests\test-install.ps1; pwsh -NoProfile -File .\tests\test-packages.ps1`
- Windows PowerShell setup requires `pwsh`; use `-Apps` and `-Yes` in noninteractive tests or automation.
- Run the focused platform test after changing its installer or module declarations, then run all available Bash and PowerShell tests before completion.

## Operational Constraints

- Interactive module selection uses Up/Down, Space, Enter, and Esc/q; package selection additionally uses Left/Right and `b` to return to module selection.
- Package modules require an available platform manager; Linux preference is apt, dnf, pacman, brew, mise, while Windows preference is winget, scoop, brew, mise.
- Use `--yes`/`-Yes` to bypass package confirmation; this does not bypass module selection unless `--apps`/`-Apps` is also supplied.
- Existing targets are backed up before replacement; symlinks are preferred and Windows may fall back to copying.
- Never commit plaintext Atuin credentials, age private keys, or decrypted secret files; encrypted secrets belong at `secrets/atuin.env.age`.
- Secret values are expected as `ATUIN_USERNAME`, `ATUIN_PASSWORD`, and `ATUIN_KEY`; `ATUIN_SYNC_ADDRESS` is optional.
- Do not commit installation state, plaintext secret files, or OS/editor noise covered by `.gitignore`.
