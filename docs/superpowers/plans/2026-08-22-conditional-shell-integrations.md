# Conditional Shell Integrations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add composable Bash and PowerShell drop-in directories so an integration such as Atuin is installed only when both the integration module and its shell module are selected.

**Architecture:** Shell modules own profile loaders, while integration modules own shell-specific drop-in files. Extend the restricted `map=` metadata with an optional `requires=module1,module2` suffix; installers install a mapping only when its owning module and all requirements are selected. Atuin authentication remains in its existing setup scripts, and this change does not remove previously installed files when a module is later deselected.

**Tech Stack:** POSIX Bash, PowerShell, restricted `key=value` module metadata, shell integration fixtures, temporary home directories.

**Spec:** `docs/superpowers/specs/2026-08-22-dotfiles-design.md`

## Global Constraints

- `module.conf` files are parsed as restricted `key=value` data, not sourced or executed.
- Preserve platform-qualified `map=`, `package=`, `secret=`, and `setup=` entries.
- `default=true|false` controls the interactive checklist only; `--apps`/`-Apps` explicitly overrides defaults.
- `atuin` is opt-in (`default=false`) and can run package installation plus platform-specific setup; do not add Atuin-specific logic to the installers.
- Existing targets are backed up before replacement; symlinks are preferred and Windows may fall back to copying.
- Do not add uninstall behavior or remove stale managed drop-ins.
- Never commit plaintext Atuin credentials, age private keys, or decrypted secret files.
- Run `bash -n install.sh && bash tests/test-install.sh`, `bash tests/test-packages.sh`, `bash tests/test-secrets.sh`, `pwsh -NoProfile -File .\tests\test-install.ps1`, and `pwsh -NoProfile -File .\tests\test-packages.ps1` before completion when the tools are available.

---

### Task 1: Add failing conditional-map parser tests

**Files:**
- Modify: `tests/test-install.sh`
- Modify: `tests/test-install.ps1`
- Modify: `tests/test-packages.sh`
- Modify: `tests/test-packages.ps1`

**Interfaces:**
- Consumes: Current `map=platform:source|target` declarations and installer selection behavior.
- Produces: Failing tests that define the supported `|requires=module1,module2` metadata contract.

- [ ] **Step 1: Add Bash metadata and behavior assertions**

Add checks that the shell module metadata contains these exact declarations:

```bash
grep -Fq 'map=linux:bash/bashrc|.bashrc' "$ROOT/modules/bash/module.conf" || failures=$((failures + 1))
grep -Fq 'map=windows:powershell/profile.ps1|Documents/PowerShell/Microsoft.PowerShell_profile.ps1' "$ROOT/modules/powershell/module.conf" || failures=$((failures + 1))
grep -Fq 'requires=bash' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
```

Add temporary-home cases that install `bash` alone and `bash,atuin` with fake `apt`, `atuin`, `age`, and `apt-get` commands. Assert that `bash` alone creates no `50-atuin.bash` file and `bash,atuin` creates the Atuin drop-in. The `bash,atuin` fixture must provide `ATUIN_TEST_LOG`, `DOTFILES_SECRETS_FILE`, and all three required Atuin variables through the existing encrypted-secret test fixture pattern.

- [ ] **Step 2: Add PowerShell metadata and behavior assertions**

Extend the PowerShell test to assert the `requires=powershell` declaration. Add a test fixture that selects `powershell` without Atuin and confirms no `Profile.d\50-atuin.ps1` file exists. Add a second fixture selecting `powershell,atuin` with fake package and Atuin commands and confirm that the drop-in exists and contains the PowerShell initialization command.

- [ ] **Step 3: Run the focused tests to confirm failure**

Run:

```bash
bash tests/test-install.sh
bash tests/test-packages.sh
```

Run on Windows:

```powershell
pwsh -NoProfile -File .\tests\test-install.ps1
pwsh -NoProfile -File .\tests\test-packages.ps1
```

Expected: the new assertions fail because conditional mappings and shell drop-in files do not exist yet. Existing unrelated assertions should continue to pass.

- [ ] **Step 4: Commit the failing tests**

```bash
git add tests/test-install.sh tests/test-install.ps1 tests/test-packages.sh tests/test-packages.ps1
git commit -m "test: define conditional shell integration behavior"
```

### Task 2: Implement conditional map requirements in both installers

**Files:**
- Modify: `install.sh:51-75,323-348`
- Modify: `install.ps1:55-64,237-260`

**Interfaces:**
- Consumes: Map entries in the form `module|platform:source|target|requires=module1,module2` after module parsing.
- Produces: An internal mapping representation with source, target, and zero or more required module names; only eligible mappings reach the existing backup/link/copy code.

- [ ] **Step 1: Add a Bash failing parser/selection test**

Add a temporary test-only module declaration or direct fixture assertion proving that a mapping with `requires=bash,atuin` is skipped when either module is absent and installed when both are selected. Keep the test within `tests/test-install.sh`; do not add executable logic to module configuration files.

- [ ] **Step 2: Run the Bash test and verify it fails**

Run:

```bash
bash tests/test-install.sh
```

Expected: FAIL because the Bash installer currently treats everything after the second pipe as part of the target mapping and has no requirement check.

- [ ] **Step 3: Implement Bash map parsing and requirement matching**

When reading `map=` lines, preserve the complete declaration. In the mapping loop, split the declaration into `platform_source`, `target_relative`, and an optional `requirements` field. Add a small helper with this contract:

```bash
requirements_met() {
    local requirements="$1" requirement
    [ -z "$requirements" ] && return 0
    IFS=',' read -ra required_modules <<< "${requirements#requires=}"
    for requirement in "${required_modules[@]}"; do
        is_selected "$requirement" || return 1
    done
    return 0
}
```

Skip a mapping when the optional field is present and `requirements_met` returns false. Reject malformed optional fields with the same `Invalid manifest line` style already used for module parsing rather than silently treating them as target paths. Leave ordinary two-part mappings unchanged.

- [ ] **Step 4: Implement PowerShell map parsing and requirement matching**

Store each map as an object with `Source`, `Target`, and `Requires` properties. Parse the optional suffix using a restricted expression equivalent to:

```powershell
if ($line -match '^map=([^|]+)\|([^|]+)(?:\|requires=([A-Za-z0-9_-]+(?:,[A-Za-z0-9_-]+)*))?$') {
    $requirements = if ($Matches[3]) { @($Matches[3].Split(',')) } else { @() }
    $data.maps += [pscustomobject]@{
        Source = $Matches[1]
        Target = $Matches[2]
        Requires = $requirements
    }
}
```

Before processing a map, require every name in `Requires` to be present in `$selectedNames`. Keep the existing platform filtering, backup, symlink, and copy fallback behavior. Do not make the installer mention `atuin`; the generic `requires` mechanism must handle it.

- [ ] **Step 5: Run focused tests and syntax checks**

Run:

```bash
bash -n install.sh
bash tests/test-install.sh
```

Run on Windows:

```powershell
pwsh -NoProfile -Command '$errors = $null; [System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path .\install.ps1), [ref]$null, [ref]$errors) | Out-Null; if ($errors.Count) { $errors | ForEach-Object { $_.Message }; exit 1 }'
```

Expected: the conditional-map tests pass, with existing installer tests still passing.

- [ ] **Step 6: Commit the generic installer support**

```bash
git add install.sh install.ps1 tests/test-install.sh tests/test-install.ps1
git commit -m "feat: support required modules on mappings"
```

### Task 3: Add shell profile drop-in loaders

**Files:**
- Modify: `modules/bash/bashrc`
- Modify: `modules/powershell/profile.ps1`

**Interfaces:**
- Consumes: Drop-ins installed at `~/.config/bashrc.d/*.bash` on Linux and beside the standard PowerShell profile in `Profile.d\*.ps1` on Windows.
- Produces: Shell startup loaders that execute sorted, readable integration files without changing Atuin authentication behavior.

- [ ] **Step 1: Add loader assertions to the shell tests**

Assert that the Bash profile contains a glob limited to `*.bash` and the PowerShell profile references `$PROFILE`, `Split-Path`, `Profile.d`, `*.ps1`, and `Sort-Object`. Also assert that the loader does not contain an Atuin-specific command; the shell profile must remain integration-neutral.

- [ ] **Step 2: Run the assertions and verify they fail**

Run:

```bash
bash tests/test-install.sh
```

Run on Windows:

```powershell
pwsh -NoProfile -File .\tests\test-install.ps1
```

Expected: FAIL because the current profiles contain only editor settings.

- [ ] **Step 3: Implement the Bash loader**

Append this loader to `modules/bash/bashrc` after the existing editor export:

```bash
for file in "$HOME/.config/bashrc.d/"*.bash; do
    [ -r "$file" ] && . "$file"
done
```

Use the quoted directory and glob exactly so an absent directory produces no startup error and only Bash drop-ins are sourced.

- [ ] **Step 4: Implement the PowerShell loader using `$PROFILE`**

Append this loader to `modules/powershell/profile.ps1`:

```powershell
$profileDirectory = Join-Path (Split-Path -Parent $PROFILE) 'Profile.d'
if (Test-Path -LiteralPath $profileDirectory) {
    Get-ChildItem -LiteralPath $profileDirectory -Filter '*.ps1' |
        Sort-Object Name |
        ForEach-Object { . $_.FullName }
}
```

Do not concatenate `$HOME` with a hard-coded remaining profile path. `$PROFILE` is the source of truth for the active PowerShell profile location.

- [ ] **Step 5: Run loader and shell-only tests**

Run:

```bash
bash tests/test-install.sh
```

Run on Windows:

```powershell
pwsh -NoProfile -File .\tests\test-install.ps1
```

Expected: shell-only installation tests pass and no Atuin command appears in either base profile.

- [ ] **Step 6: Commit the shell loaders**

```bash
git add modules/bash/bashrc modules/powershell/profile.ps1 tests/test-install.sh tests/test-install.ps1
git commit -m "feat: load shell integration drop-ins"
```

### Task 4: Add Atuin shell drop-ins and conditional declarations

**Files:**
- Create: `modules/atuin/bash/50-atuin.bash`
- Create: `modules/atuin/powershell/50-atuin.ps1`
- Modify: `modules/atuin/module.conf`
- Modify: `tests/test-secrets.sh`
- Modify: `tests/test-packages.ps1`

**Interfaces:**
- Consumes: Generic `requires=` map handling and shell loader paths from Tasks 2 and 3.
- Produces: Atuin initialization files installed only for `bash,atuin` or `powershell,atuin`; existing login setup remains unchanged.

- [ ] **Step 1: Add exact Atuin declaration and content assertions**

Add assertions for these exact metadata lines:

```ini
map=linux:atuin/bash/50-atuin.bash|.config/bashrc.d/50-atuin.bash|requires=bash
map=windows:atuin/powershell/50-atuin.ps1|Documents/PowerShell/Profile.d/50-atuin.ps1|requires=powershell
```

Add assertions that the Bash drop-in contains:

```bash
eval "$(atuin init bash)"
```

and the PowerShell drop-in contains:

```powershell
atuin init powershell | Out-String | Invoke-Expression
```

- [ ] **Step 2: Run the assertions and verify they fail**

Run:

```bash
bash tests/test-secrets.sh
bash tests/test-packages.sh
```

Run on Windows:

```powershell
pwsh -NoProfile -File .\tests\test-packages.ps1
```

Expected: FAIL because the Atuin drop-in files and conditional declarations do not exist.

- [ ] **Step 3: Create the Bash Atuin drop-in**

Create `modules/atuin/bash/50-atuin.bash` with only:

```bash
eval "$(atuin init bash)"
```

Do not put login, secret, package-installation, or profile-loader logic in this file.

- [ ] **Step 4: Create the PowerShell Atuin drop-in**

Create `modules/atuin/powershell/50-atuin.ps1` with only:

```powershell
atuin init powershell | Out-String | Invoke-Expression
```

Do not use `$HOME` to locate the profile and do not duplicate the loader.

- [ ] **Step 5: Add the conditional Atuin mappings**

Append the two `map=` lines shown in Step 1 to `modules/atuin/module.conf`. Keep `default=false`, both existing `setup=` entries, all package declarations, and the restricted metadata format unchanged.

- [ ] **Step 6: Run the Atuin-focused tests**

Run:

```bash
bash tests/test-secrets.sh
bash tests/test-packages.sh
```

Run on Windows:

```powershell
pwsh -NoProfile -File .\tests\test-packages.ps1
```

Expected: Atuin authentication tests still pass, Atuin package declarations remain valid, and the conditional drop-in content is recognized.

- [ ] **Step 7: Commit the Atuin integration**

```bash
git add modules/atuin/bash/50-atuin.bash modules/atuin/powershell/50-atuin.ps1 modules/atuin/module.conf tests/test-secrets.sh tests/test-packages.ps1
git commit -m "feat: add conditional Atuin shell integration"
```

### Task 5: Document the drop-in convention and run the complete suite

**Files:**
- Modify: `README.md`
- Modify: `AGENTS.md` only if the test instructions need to mention the new focused test cases

**Interfaces:**
- Consumes: Final shell loader and `requires=` mapping behavior from Tasks 2 through 4.
- Produces: User-facing documentation and verified cross-platform behavior.

- [ ] **Step 1: Add documentation assertions or a documentation checklist**

Update the README module section to state that:

```text
Shell modules load integration drop-ins from their platform-specific profile directories. An integration drop-in is installed only when both the integration module and its required shell module are selected. Module selection does not uninstall an existing drop-in.
```

Document the concrete examples `bash,atuin` and `powershell,atuin`, and explain that future integrations should add a numbered drop-in plus a `requires=` mapping rather than duplicate the full profile.

- [ ] **Step 2: Run all Bash verification commands**

Run:

```bash
bash -n install.sh
bash tests/test-install.sh
bash tests/test-packages.sh
bash tests/test-secrets.sh
```

Expected: every command exits with status 0 and prints its existing success message.

- [ ] **Step 3: Run all PowerShell verification commands**

Run:

```powershell
pwsh -NoProfile -File .\tests\test-install.ps1
pwsh -NoProfile -File .\tests\test-packages.ps1
pwsh -NoProfile -Command '$errors = $null; [System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path .\install.ps1), [ref]$null, [ref]$errors) | Out-Null; if ($errors.Count) { $errors | ForEach-Object { $_.Message }; exit 1 }'
```

Expected: every command exits with status 0.

- [ ] **Step 4: Inspect the final diff for scope and secret safety**

Run:

```bash
git diff --check
git status --short
git diff -- README.md AGENTS.md install.sh install.ps1 modules tests
```

Confirm that the diff contains no plaintext credentials, private keys, decrypted secret files, generated state, or uninstall logic.

- [ ] **Step 5: Commit the documentation and verification-ready changes**

```bash
git add README.md AGENTS.md
git commit -m "docs: describe shell integration drop-ins"
```

## Self-Review Checklist

- [x] The plan covers the generic conditional-map parser in both installers.
- [x] The plan covers Bash and PowerShell shell loaders.
- [x] The plan covers Atuin drop-ins and the requirement that both modules be selected.
- [x] The PowerShell loader uses `$PROFILE` rather than reconstructing the profile path from `$HOME`.
- [x] The plan explicitly excludes uninstallation and stale-drop-in removal.
- [x] Existing Atuin package, authentication, and secret cleanup behavior is covered by focused tests.
- [x] Every task identifies exact files, interfaces, test commands, and commit boundaries.
- [x] No implementation step requires installer-specific Atuin logic.
