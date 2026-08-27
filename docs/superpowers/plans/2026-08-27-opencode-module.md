# OpenCode Module Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an opt-in cross-platform OpenCode module that installs OpenCode, configures file-backed provider credentials, fetches the latest Catppuccin themes, and enables the requested MCP server and plugin without overwriting unrelated OpenCode settings.

**Architecture:** Keep module discovery, package selection, secret decryption, and setup ordering in the existing generic installers. Put OpenCode-specific behavior in `modules/opencode/setup.sh` and `modules/opencode/setup.ps1`; those hooks will write protected credential files, merge managed JSON keys, and fetch upstream themes. Use `jq` on Linux and native PowerShell JSON handling on Windows so no Node or Python dependency is introduced.

**Tech Stack:** Bash, PowerShell 7+, restricted `module.conf` metadata, `age`, `jq` on Linux, PowerShell `ConvertFrom-Json`/`ConvertTo-Json` on Windows, GitHub Contents/tree and raw HTTP endpoints.

**Spec:** `docs/superpowers/specs/2026-08-27-opencode-module-design.md`

## Global Constraints

- The module is opt-in and supports Linux and Windows.
- OpenCode is installed only when unavailable through the selected package manager.
- Provider secrets come from `secrets/opencode.env.age` and are never written into tracked files or `opencode.json`.
- The required secret names are `OPENROUTER_API_KEY`, `AZURE_API_KEY`, `AZURE_RESOURCE_NAME`, and `DIGITALOCEAN_ACCESS_TOKEN`.
- Credential files use mode `600` on Linux and user-only ACLs on Windows.
- Existing unrelated OpenCode settings must survive configuration updates.
- Managed keys are `provider.openrouter`, `provider.azure`, `provider.digitalocean`, `mcp.ddg-search`, and `plugin`.
- No model aliases, model maps, default model, or default theme may be written.
- Theme files are fetched from the latest upstream `catppuccin/opencode` `main` branch at setup time and are not stored in-repo.
- Theme downloads must be complete before replacing managed theme files; temporary downloads must be cleaned up.
- The MCP command is `uvx duckduckgo-mcp-server` with `DDG_SAFE_SEARCH=OFF`.
- The plugin value is `superpowers@git+https://github.com/obra/superpowers.git`.
- Installer code must not contain OpenCode-, Catppuccin-, DuckDuckGo-, or Superpowers-specific branches.
- Do not commit plaintext credentials, age private keys, decrypted secret files, or installation state.

---

### Task 1: Add Failing Module and Package Assertions

**Files:**
- Create: `tests/test-opencode.sh`
- Create: `tests/test-opencode.ps1`
- Modify: `modules/manifest.conf`
- Create: `modules/opencode/module.conf`

**Interfaces:**
- Consumes: Existing manifest parser, package planner, and setup declaration formats.
- Produces: A registered `opencode` module with Linux/Windows package declarations and explicit `jq` package declarations for the setup hooks.

- [ ] **Step 1: Write Bash manifest and package assertions.**

  Assert that `modules/manifest.conf` contains `module=opencode`, the module metadata has `name=opencode`, `platforms=linux,windows`, and `default=false`, and the module contains package entries for:

  ```text
  package=opencode|brew:anomalyco/tap/opencode
  package=opencode|mise:github:anomalyco/opencode
  package=opencode|scoop:opencode
  ```

  Also assert the module contains these explicit `jq` package declarations:

  ```text
  package=jq|apt:jq
  package=jq|dnf:jq
  package=jq|pacman:jq
  package=jq|brew:jq
  package=jq|winget:jqlang.jq
  package=jq|scoop:jq
  ```

- [ ] **Step 2: Write equivalent PowerShell assertions.**

  Read the manifest and module as text and fail if registration, metadata, Windows package identifiers, or setup declarations are missing. Keep the test independent of installed package managers.

- [ ] **Step 3: Run the focused assertions and verify they fail.**

  Run:

  ```sh
  bash tests/test-opencode.sh
  ```

  ```powershell
  pwsh -NoProfile -File .\tests\test-opencode.ps1
  ```

  Expected: both tests fail because the module and declarations do not yet exist.

- [ ] **Step 4: Add the minimal module metadata and package declarations.**

  Create `modules/opencode/module.conf` with `name`, description, Linux/Windows platforms, `default=false`, one setup entry for each platform, the OpenCode package entries, and the six exact `jq` package entries from Step 1. The module will therefore select the same available manager for OpenCode and `jq` when possible; if only mise is available, the package plan must fail clearly because the Bash merge hook cannot run without `jq`.

- [ ] **Step 5: Run the focused assertions and verify they pass.**

  Run both focused commands from Step 3. Expected: PASS.

- [ ] **Step 6: Commit the module contract.**

  ```sh
  git add modules/manifest.conf modules/opencode/module.conf tests/test-opencode.sh tests/test-opencode.ps1
  git commit -m "feat: add OpenCode module contract"
  ```

### Task 2: Implement Linux Secret Files and JSON Merge

**Files:**
- Create: `modules/opencode/setup.sh`
- Modify: `tests/test-opencode.sh`
- Modify: `secrets/README.md`

**Interfaces:**
- Consumes: Installer-exported `DOTFILES_ROOT`, temporary decrypted environment variables, and `jq`.
- Produces: A Bash setup hook that validates four secrets, writes protected credential files, atomically merges managed OpenCode JSON keys, and preserves unrelated keys.

- [ ] **Step 1: Add failing Linux setup fixtures.**

  Extend `tests/test-opencode.sh` with fake `age`/secret setup behavior or invoke the setup hook directly with temporary environment variables. Cover an existing JSON file containing an unrelated key, then assert after setup that:

  - The unrelated key remains.
  - Provider entries contain `{file:...}` references and no test secret values.
  - `mcp.ddg-search` has the exact command and `DDG_SAFE_SEARCH` value.
  - The plugin occurs exactly once.
  - Credential files contain the expected values and have mode `600`.

  Add an idempotency case that runs setup twice and an invalid-JSON case that asserts the original file is unchanged and setup fails.

- [ ] **Step 2: Run the new Linux setup tests to verify failure.**

  ```sh
  bash tests/test-opencode.sh
  ```

  Expected: setup assertions fail because `modules/opencode/setup.sh` is absent.

- [ ] **Step 3: Implement secret validation and protected file writes.**

  In `setup.sh`, require non-empty `OPENROUTER_API_KEY`, `AZURE_API_KEY`, `AZURE_RESOURCE_NAME`, and `DIGITALOCEAN_ACCESS_TOKEN`. Resolve the config directory from `${XDG_CONFIG_HOME:-$HOME/.config}/opencode`, create a private temporary directory, write one value per credential file, `chmod 600` each file, and use `mv` only after each file is complete.

- [ ] **Step 4: Implement atomic `jq` configuration merging.**

  Read `${config_dir}/opencode.json` if present. First validate it with `jq empty`; on failure return without changing it. Use `jq --arg` to construct file references with correctly escaped paths, set the three provider objects, set the exact MCP object, and add the Superpowers plugin only if absent. Write to a temporary file in the same directory and atomically replace the config after successful validation.

- [ ] **Step 5: Update secret documentation.**

  Add the `opencode.env.age` filename and required variable names to `secrets/README.md`, explicitly state that values are materialized into protected user-local files, and do not add example secret values.

- [ ] **Step 6: Run the Linux setup tests to verify they pass.**

  ```sh
  bash tests/test-opencode.sh
  ```

  Expected: PASS for secret validation, protected files, merge preservation, idempotency, and invalid JSON handling.

- [ ] **Step 7: Commit the Linux setup.**

  ```sh
  git add modules/opencode/setup.sh tests/test-opencode.sh secrets/README.md
  git commit -m "feat: configure OpenCode on Linux"
  ```

### Task 3: Implement Windows Secret Files and JSON Merge

**Files:**
- Create: `modules/opencode/setup.ps1`
- Modify: `tests/test-opencode.ps1`

**Interfaces:**
- Consumes: Installer-exported secret environment variables and the Windows OpenCode config directory.
- Produces: A PowerShell setup hook with the same managed-key and credential-file behavior as the Bash hook.

- [ ] **Step 1: Add failing Windows setup fixtures.**

  Extend `tests/test-opencode.ps1` with an isolated temporary `APPDATA`/config root and temporary secret values. Assert preservation of unrelated JSON properties, exact managed values, absence of secret values in JSON, plugin idempotency, and failure without replacing invalid JSON.

- [ ] **Step 2: Run the focused PowerShell test to verify failure.**

  ```powershell
  pwsh -NoProfile -File .\tests\test-opencode.ps1
  ```

  Expected: setup assertions fail because the Windows hook is absent.

- [ ] **Step 3: Implement Windows credential files.**

  Validate the four required environment variables, create the OpenCode config and credential directories, write UTF-8 files without a trailing secret-bearing log message, and apply an ACL that grants the current Windows identity full control while removing inherited broad access.

- [ ] **Step 4: Implement native JSON merge.**

  Parse an existing `opencode.json` with `ConvertFrom-Json`; preserve unrelated properties; replace the managed provider, MCP, and plugin properties; remove duplicate Superpowers entries before adding the managed value; serialize to a temporary file; parse the temporary output again; and atomically move it over the original. On parse or write failure, leave the original unchanged.

- [ ] **Step 5: Run the focused PowerShell tests to verify they pass.**

  ```powershell
  pwsh -NoProfile -File .\tests\test-opencode.ps1
  ```

  Expected: PASS.

- [ ] **Step 6: Commit the Windows setup.**

  ```sh
  git add modules/opencode/setup.ps1 tests/test-opencode.ps1
  git commit -m "feat: configure OpenCode on Windows"
  ```

### Task 4: Add Latest Catppuccin Theme Fetching

**Files:**
- Modify: `modules/opencode/setup.sh`
- Modify: `modules/opencode/setup.ps1`
- Modify: `tests/test-opencode.sh`
- Modify: `tests/test-opencode.ps1`

**Interfaces:**
- Consumes: The upstream `catppuccin/opencode` Git tree and the user OpenCode themes directory.
- Produces: A complete temporary theme set installed without changing the selected default theme.

- [ ] **Step 1: Add fixture-backed download tests.**

  Make the setup hooks accept a test-only API/raw base URL environment variable. Test fixtures must expose multiple flavor/accent JSON files through the same discovery shape as the GitHub tree response. Assert all discovered files arrive in the themes directory, unrelated files remain, no `theme` key is added, and a failed download leaves the prior managed set intact.

- [ ] **Step 2: Run both theme tests to verify failure.**

  Run the focused Bash and PowerShell tests. Expected: theme assertions fail because the fetch path is not implemented.

- [ ] **Step 3: Implement Bash discovery and staged download.**

  Use the GitHub recursive tree endpoint for `catppuccin/opencode` `main`, filter `themes/*.json`, download each matching raw file into a temporary directory, validate each JSON file with `jq empty`, and require at least one discovered theme. Only after every file succeeds, remove the prior managed `catppuccin-*.json` files and move the staged files into the user themes directory. Clean the temporary directory with a trap.

- [ ] **Step 4: Implement PowerShell discovery and staged download.**

  Use `Invoke-RestMethod` against the same recursive tree endpoint, filter JSON theme paths, download each file into a temporary directory, validate with `ConvertFrom-Json`, and replace managed files only after the complete set is staged. Use a `try/finally` cleanup block.

- [ ] **Step 5: Run both theme tests to verify they pass.**

  ```sh
  bash tests/test-opencode.sh
  ```

  ```powershell
  pwsh -NoProfile -File .\tests\test-opencode.ps1
  ```

  Expected: PASS for discovery, complete staging, preservation of unrelated themes, no default theme, and cleanup on failure.

- [ ] **Step 6: Commit theme fetching.**

  ```sh
  git add modules/opencode/setup.sh modules/opencode/setup.ps1 tests/test-opencode.sh tests/test-opencode.ps1
  git commit -m "feat: fetch latest Catppuccin OpenCode themes"
  ```

### Task 5: Verify Package Planning and Setup Ordering

**Files:**
- Modify: `tests/test-packages.sh`
- Modify: `tests/test-packages.ps1`
- Modify: `tests/test-secrets.sh`
- Modify: `tests/test-install.ps1`

**Interfaces:**
- Consumes: The completed module declarations and setup hooks.
- Produces: Regression coverage proving generic installers install packages before setup and do not contain provider-specific branches.

- [ ] **Step 1: Add isolated package-plan fixtures.**

  Fake OpenCode, package managers, `jq`, `age`, and network commands. Assert that an existing `opencode` command causes no OpenCode package install, while a missing command selects the preferred available package. Assert that setup runs after the package command and that the Linux jq requirement is available before the setup hook.

- [ ] **Step 2: Add installer-agnostic secret/setup assertions.**

  Assert both installers discover `setup=linux:opencode/setup.sh` or `setup=windows:opencode/setup.ps1`, and assert the installer files do not contain OpenCode/provider/theme/MCP/plugin-specific identifiers.

- [ ] **Step 3: Run focused package and secret tests.**

  ```sh
  bash tests/test-packages.sh
  bash tests/test-secrets.sh
  ```

  ```powershell
  pwsh -NoProfile -File .\tests\test-packages.ps1
  pwsh -NoProfile -File .\tests\test-install.ps1
  ```

  Expected: existing tests pass and new assertions pass.

- [ ] **Step 4: Commit installer-contract coverage.**

  ```sh
  git add tests/test-packages.sh tests/test-packages.ps1 tests/test-secrets.sh tests/test-install.ps1
  git commit -m "test: cover OpenCode package and setup ordering"
  ```

### Task 6: Document Usage and Run the Full Matrix

**Files:**
- Modify: `README.md`
- Modify: `CONTRIBUTING.md`
- Modify: `tests/test-opencode.sh`
- Modify: `tests/test-opencode.ps1`

**Interfaces:**
- Consumes: The completed module behavior and secret file contract.
- Produces: User-facing setup instructions and final regression evidence.

- [ ] **Step 1: Document module selection and secret preparation.**

  Add Linux and Windows examples using `--apps opencode`/`-Apps opencode`, explain the encrypted `secrets/opencode.env.age` bundle and required names, describe file-backed credentials, and state that reruns fetch the latest upstream Catppuccin themes without selecting a default.

- [ ] **Step 2: Document MCP/plugin prerequisites.**

  State that `uvx duckduckgo-mcp-server` must be available at runtime and that OpenCode loads the Superpowers plugin from its configured Git URL. Do not document or expose secret values.

- [ ] **Step 3: Run syntax and focused tests.**

  ```sh
  bash -n install.sh
  bash tests/test-install.sh
  bash tests/test-packages.sh
  bash tests/test-secrets.sh
  bash tests/test-opencode.sh
  ```

  ```powershell
  pwsh -NoProfile -File .\tests\test-install.ps1
  pwsh -NoProfile -File .\tests\test-packages.ps1
  pwsh -NoProfile -File .\tests\test-opencode.ps1
  ```

- [ ] **Step 4: Run available lint and full test checks.**

  ```sh
  shellcheck --severity=error install.sh lib/*.sh modules/**/*.sh tests/*.sh
  ```

  ```powershell
  Invoke-ScriptAnalyzer -Path . -Recurse -Severity Error -EnableExit
  ```

  Record unavailable tools explicitly rather than weakening tests.

- [ ] **Step 5: Inspect the final worktree and commit documentation.**

  ```sh
  git status --short
  git diff --check
  git diff --cached --stat
  git add README.md CONTRIBUTING.md tests/test-opencode.sh tests/test-opencode.ps1
  git commit -m "docs: document OpenCode module"
  ```
