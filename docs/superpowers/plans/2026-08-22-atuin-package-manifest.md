# Atuin Package Manifest Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Atuin as an opt-in dotfiles module with package-manager declarations for the supported Linux and Windows package managers.

**Architecture:** The existing installers read module metadata from `modules/<name>/module.conf`, build a platform-specific package plan, let the user choose a package manager and package, and install selected packages before applying module dotfiles. Atuin will follow that existing contract without adding shell integration, encrypted secrets, or sync-login behavior in this plan.

**Tech Stack:** Bash, PowerShell, plain-text module manifests, shell integration tests.

**Spec:** `docs/superpowers/specs/2026-08-22-dotfiles-design.md`

## Global Constraints

- The Atuin module is opt-in: `default=false`.
- Package declarations use `package=<logical>|<manager>:<name>`.
- Linux package managers are `apt`, `dnf`, `pacman`, `brew`, and `mise`.
- Windows package managers are `winget`, `scoop`, `brew`, and `mise`.
- The WinGet package identifier is `Atuinsh.Atuin`.
- The Scoop package identifier is `atuin`.
- The package logical name is `atuin` for every manager.
- This change must not add plaintext secrets, encrypted secret files, Atuin login commands, or shell profile mappings.
- Do not change package-manager priority or package installation behavior.
- Do not commit changes unless explicitly requested by the user.

---

## File Structure

- Create `modules/atuin/module.conf`: Atuin module metadata and package declarations.
- Modify `modules/manifest.conf`: Register the Atuin module.
- Modify `tests/test-packages.sh`: Verify Linux and shared package declarations.
- Modify `tests/test-packages.ps1`: Verify Windows package declarations.
- Modify `README.md`: Document Atuin as an opt-in package module.

### Task 1: Add Atuin Module Metadata

**Files:**
- Create: `modules/atuin/module.conf`
- Modify: `modules/manifest.conf`

**Interfaces:**
- Consumes: Existing installer module parsing for `name`, `description`, `platforms`, `default`, and `package` fields.
- Produces: A module named `atuin` available to both installers with no file mappings.

- [ ] **Step 1: Write the failing manifest assertions**

Add these assertions before the existing package assertions in `tests/test-packages.sh`:

```bash
grep -Fq 'module=atuin' "$ROOT/modules/manifest.conf" || failures=$((failures + 1))
grep -Fq 'name=atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'description=Atuin shell history' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'platforms=linux,windows' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'default=false' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
```

Add these assertions to `tests/test-packages.ps1`:

```powershell
$manifest = Get-Content (Join-Path $root 'modules\manifest.conf')
if ('module=atuin' -notin $manifest) { throw 'Atuin module registration missing' }
$atuinModule = Get-Content (Join-Path $root 'modules\atuin\module.conf')
if ('name=atuin' -notin $atuinModule) { throw 'Atuin module name missing' }
if ('default=false' -notin $atuinModule) { throw 'Atuin must be opt-in' }
```

- [ ] **Step 2: Run package tests and verify the new assertions fail**

Run:

```bash
bash tests/test-packages.sh
pwsh -NoProfile -File .\tests\test-packages.ps1
```

Expected: failures reporting the missing Atuin module and metadata.

- [ ] **Step 3: Create the module metadata**

Create `modules/atuin/module.conf` with exactly:

```text
name=atuin
description=Atuin shell history
platforms=linux,windows
default=false
```

Append this line to `modules/manifest.conf`:

```text
module=atuin
```

- [ ] **Step 4: Run the focused manifest assertions**

Run:

```bash
bash tests/test-packages.sh
pwsh -NoProfile -File .\tests\test-packages.ps1
```

Expected: the new metadata assertions pass; package identifier assertions may still fail until Task 2 is complete.

### Task 2: Add Package Manager Declarations

**Files:**
- Modify: `modules/atuin/module.conf`
- Modify: `tests/test-packages.sh`
- Modify: `tests/test-packages.ps1`

**Interfaces:**
- Consumes: The `atuin` module registered in Task 1.
- Produces: Package-plan candidates recognized by the existing Bash and PowerShell installers.

- [ ] **Step 1: Add failing package identifier assertions**

Add to `tests/test-packages.sh`:

```bash
grep -Fq 'package=atuin|apt:atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'package=atuin|dnf:atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'package=atuin|pacman:atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'package=atuin|brew:atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'package=atuin|mise:atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'package=atuin|winget:Atuinsh.Atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'package=atuin|scoop:atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
```

Add to `tests/test-packages.ps1`:

```powershell
if ('package=atuin|winget:Atuinsh.Atuin' -notin $atuinModule) { throw 'Atuin WinGet package declaration missing' }
if ('package=atuin|scoop:atuin' -notin $atuinModule) { throw 'Atuin Scoop package declaration missing' }
if ('package=atuin|brew:atuin' -notin $atuinModule) { throw 'Atuin Homebrew package declaration missing' }
if ('package=atuin|mise:atuin' -notin $atuinModule) { throw 'Atuin mise package declaration missing' }
```

- [ ] **Step 2: Run the focused tests and verify the package assertions fail**

Run:

```bash
bash tests/test-packages.sh
pwsh -NoProfile -File .\tests\test-packages.ps1
```

Expected: failures for the missing Atuin package declarations.

- [ ] **Step 3: Add all supported package declarations**

Append these lines to `modules/atuin/module.conf`:

```text
package=atuin|apt:atuin
package=atuin|dnf:atuin
package=atuin|pacman:atuin
package=atuin|brew:atuin
package=atuin|mise:atuin
package=atuin|winget:Atuinsh.Atuin
package=atuin|scoop:atuin
```

Do not add a `map=` entry in this task. Atuin shell integration and sync login require a separate design and should not run merely because the package module is selected.

- [ ] **Step 4: Run the focused package tests**

Run:

```bash
bash tests/test-packages.sh
pwsh -NoProfile -File .\tests\test-packages.ps1
```

Expected: both package test scripts pass.

### Task 3: Verify Package-Plan Integration

**Files:**
- Modify: `tests/test-packages.sh`
- Modify: `tests/test-packages.ps1`
- Modify: `README.md`

**Interfaces:**
- Consumes: `modules/atuin/module.conf` package declarations.
- Produces: Documentation and regression coverage proving Atuin is available to the existing package planner but remains opt-in.

- [ ] **Step 1: Add an opt-in regression assertion**

Add to `tests/test-packages.sh`:

```bash
if PATH="$tmp/bin" HOME="$tmp/home" XDG_STATE_HOME="$tmp/state-atuin" bash "$ROOT/install.sh" --apps atuin --yes >/dev/null 2>&1; then
    printf 'FAIL: Atuin package setup unexpectedly succeeded without a package manager\n' >&2
    failures=$((failures + 1))
fi
```

The assertion must be placed in a fixture environment where no Atuin package manager candidate is available, so it proves the module is recognized and package resolution occurs rather than silently skipping the module.

Add to `tests/test-packages.ps1`:

```powershell
if ([string]::Join("`n", $atuinModule) -notmatch '(?m)^default=false$') { throw 'Atuin is not opt-in' }
```

- [ ] **Step 2: Run the regression tests before documentation changes**

Run:

```bash
bash tests/test-packages.sh
pwsh -NoProfile -File .\tests\test-packages.ps1
```

Expected: both scripts pass.

- [ ] **Step 3: Document Atuin package setup**

Add to `README.md`:

```markdown
### Atuin

Atuin is available as an opt-in package module. Select `atuin` interactively or use `--apps atuin` / `-Apps atuin`. The module currently installs the Atuin package only; encrypted credentials, shell integration, and sync login are separate setup behavior.
```

- [ ] **Step 4: Run the complete verification commands**

Run:

```bash
bash -n install.sh
bash tests/test-install.sh
bash tests/test-packages.sh
pwsh -NoProfile -File .\tests\test-install.ps1
pwsh -NoProfile -File .\tests\test-packages.ps1
pwsh -NoProfile -Command '$errors = $null; [System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path .\install.ps1), [ref]$null, [ref]$errors) | Out-Null; if ($errors.Count) { $errors | ForEach-Object { $_.Message }; exit 1 }'
git diff --check
```

Expected: every command exits successfully and no whitespace errors are reported.

## Self-Review

- Spec coverage: module registration, opt-in default, all seven package-manager declarations, package tests, and documentation are covered by Tasks 1 through 3.
- Scope: encrypted age secrets, Atuin login, shell integration, and Atuin runtime configuration are intentionally excluded and require a separate security-sensitive plan.
- Marker scan: no implementation step contains an incomplete marker or unspecified behavior.
- Interface consistency: both installers already consume the exact `package=<logical>|<manager>:<name>` format used by this plan.
- Package identifiers: Linux package names are `atuin`; Windows identifiers are `Atuinsh.Atuin` for WinGet and `atuin` for Scoop.
