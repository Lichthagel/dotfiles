# Phased Module Installation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the single package/setup pass with dependency-ordered module phases so provider modules such as `mise` install and configure before packages that need them.

**Architecture:** Add generic `provides=<manager>` and module-scope `requires=<module[,module...]>` metadata to both restricted parsers. Resolve provider modules from package declarations before package planning, topologically order provider and explicit module dependencies, and execute each phase as package plan, package install, setup, and mapping. Keep provider additions internal to the selected module set and remove the installer-specific `mise_required`/`install_mise` bootstrap path.

**Tech Stack:** Bash, PowerShell 7+, restricted `key=value` module manifests, existing shell installer test scripts.

**Spec:** `docs/superpowers/specs/2026-08-22-phased-module-installation-design.md`

## Global Constraints

- Preserve platform-qualified `map=`, `package=`, `secret=`, and `setup=` entries.
- Module configuration is parsed as restricted data and must not be sourced or executed.
- Provider dependencies are soft additions inferred from package-manager declarations; consumers must not explicitly list `requires=mise`.
- A provider module must not install itself through the manager it provides.
- Existing `--yes`/`-Yes`, interactive package selection, backups, and noninteractive error behavior remain intact.
- Do not add Atuin-specific logic to either installer.
- Run focused platform tests after each installer change, then all available Bash and PowerShell tests before completion.

---

### Task 1: Extend Module Metadata Parsing

**Files:**
- Modify: `install.sh:51-87` module arrays and `load_module`
- Modify: `install.ps1:42-70` module data parser
- Test: `tests/test-packages.sh`
- Test: `tests/test-packages.ps1`

**Interfaces:**
- Produces Bash `PROVIDES` and `REQUIRES` module indexes.
- Produces PowerShell module properties `.provides` and `.requires`.
- Accepts `provides=<manager>` and `requires=<module[,module...]>`; rejects malformed values with the existing manifest-line error style.

- [ ] **Step 1: Write failing parser assertions**

Add a temporary `mise` manifest assertion requiring `provides=mise`, and add a parser fixture/assertion for a comma-separated module requirement. Assert that a malformed provider or requirement is rejected instead of being silently ignored.

- [ ] **Step 2: Run focused tests to verify failure**

Run `bash tests/test-packages.sh` and `pwsh -NoProfile -File .\tests\test-packages.ps1`.

Expected: FAIL because the parser does not recognize the new metadata and the current `mise` module lacks `provides=mise`.

- [ ] **Step 3: Implement restricted metadata parsing**

In Bash, add a module-keyed provider map and requirement map. Parse `provides=` as one manager token matching the existing manager names and parse `requires=` as one or more module names separated by commas. In PowerShell, add equivalent properties to each module object and validate each name with the existing module-name grammar. Do not source or execute values.

- [ ] **Step 4: Run focused tests to verify the parser**

Run the same Bash and PowerShell package tests. Expected: PASS for valid metadata and malformed-line rejection.

- [ ] **Step 5: Review the parser diff**

Run `git diff --check` and verify that existing map/package/setup parsing is unchanged.

### Task 2: Add Generic Dependency And Provider Resolution

**Files:**
- Modify: `install.sh` near `manager_valid`, package planning, and module selection helpers
- Modify: `install.ps1` near manager helpers and `Get-PackagePlan`
- Test: `tests/test-packages.sh`
- Test: `tests/test-packages.ps1`

**Interfaces:**
- Bash resolver returns an ordered internal module list and phase boundaries without changing the user’s explicit `requested_apps`.
- PowerShell resolver returns ordered phase objects containing module names and whether each module was explicitly selected or implicitly added.
- Provider resolution maps a package manager token to the unique module declaring `provides=<manager>`.

- [ ] **Step 1: Write failing provider-order tests**

Create a test scenario where `git` is selected, `mise` is unavailable, and the only usable package path for a dependent package is `mise`. Assert that the internal `mise` provider is added, that it precedes the dependent module, and that a provider package is not planned through `mise` itself. Add a cycle/duplicate-provider manifest assertion and a missing-provider error assertion.

- [ ] **Step 2: Run tests to verify failure**

Run `bash tests/test-packages.sh` and `pwsh -NoProfile -File .\tests\test-packages.ps1`.

Expected: FAIL because no provider graph or ordered phase resolver exists.

- [ ] **Step 3: Implement provider discovery**

Before package-manager availability errors, scan package declarations for the current module set. If a declaration names an unavailable manager and a module provides that manager, add the provider module internally. If multiple modules provide the same manager, fail with both module names. If no provider exists, retain the existing no-manager error.

- [ ] **Step 4: Implement explicit dependency expansion and topological ordering**

Expand module-scope `requires` entries, validate referenced modules, detect cycles with a clear path, and topologically order provider modules and explicit dependencies before their consumers. Preserve manifest order as the deterministic tie-breaker for unrelated modules.

- [ ] **Step 5: Prevent provider self-install**

When constructing a provider’s package plan, exclude package declarations whose manager equals the provider’s `provides` value. Fail clearly if the provider has no non-self bootstrap package declaration for the current platform.

- [ ] **Step 6: Run focused tests to verify resolution**

Run both package test scripts and confirm provider order, duplicate-provider, missing-provider, cycle, and self-install cases pass.

### Task 3: Replace Bash Single-Pass Execution With Phases

**Files:**
- Modify: `install.sh:118-240` package manager and planning helpers
- Modify: `install.sh:218-240` setup execution
- Modify: `install.sh:323-397` main package/setup/mapping flow
- Modify: `modules/mise/module.conf`
- Test: `tests/test-packages.sh`
- Test: `tests/test-install.sh`

**Interfaces:**
- `run_phase` accepts a phase module list and performs package plan, confirmation, installation, setup, and mappings as one unit.
- Package-manager discovery is refreshed between phases with `hash -r` and command lookup.

- [ ] **Step 1: Write failing Bash phase-order integration test**

Extend the fake-manager test harness so installing the `mise` provider creates a fake `mise` executable. Select a dependent module whose package can then be installed through `mise`, record command order, and assert the provider package/setup/mapping occurs before the dependent package/setup/mapping.

- [ ] **Step 2: Run the Bash integration test to verify failure**

Run `bash tests/test-packages.sh`.

Expected: FAIL because the current installer uses one package plan and has special-case `mise` bootstrap functions.

- [ ] **Step 3: Implement Bash phase execution**

Replace `mise_required` and `install_mise` with the generic phase resolver from Task 2. For each ordered phase, build `PLAN_*` only for that phase, preserve the existing interactive package UI and `--yes` behavior, install selected packages, run only that phase’s setup entries, and apply only that phase’s mappings. Refresh manager discovery before resolving the next phase.

- [ ] **Step 4: Preserve cancellation and failure semantics**

Ensure package-plan `b` returns to module selection only for the current phase, package decline stops before later phases, and package/setup failure prevents dependent phases and mappings. Keep explicit `--apps` behavior unchanged.

- [ ] **Step 5: Update the mise module metadata**

Add `provides=mise` to `modules/mise/module.conf`. Keep only system-manager package declarations for the provider and retain shell maps as module-owned integrations.

- [ ] **Step 6: Run Bash verification**

Run `bash -n install.sh`, `bash tests/test-install.sh`, `bash tests/test-packages.sh`, and `bash tests/test-secrets.sh`. Expected: all pass.

### Task 4: Replace PowerShell Single-Pass Execution With Phases

**Files:**
- Modify: `install.ps1:72-167` manager, resolver, and package helpers
- Modify: `install.ps1:133-157` setup execution
- Modify: `install.ps1:227-275` main package/setup/mapping flow
- Test: `tests/test-packages.ps1`
- Test: `tests/test-install.ps1`

**Interfaces:**
- `Resolve-ModulePhases($selectedNames)` returns ordered phase objects matching the Bash resolver semantics.
- `Invoke-ModulePhase($phase)` performs package planning, confirmation, installation, setup, and mappings for one phase.

- [ ] **Step 1: Write failing PowerShell phase-order assertions**

Add parser/integration assertions for implicit provider addition, provider-before-consumer ordering, provider self-install avoidance, and dependency cycle errors. Use the existing static parser tests for manifest errors and a temporary fake-command PATH to record executable order for the provider-before-consumer scenario.

- [ ] **Step 2: Run PowerShell tests to verify failure**

Run `pwsh -NoProfile -File .\tests\test-packages.ps1` and `pwsh -NoProfile -File .\tests\test-install.ps1`.

Expected: FAIL because `Install-Mise` is still a special pre-plan path and there is no generic phase executor.

- [ ] **Step 3: Implement PowerShell phase resolution and execution**

Port the generic provider/dependency resolver from Bash using PowerShell collections. Preserve `Select-PackagePlan`, `-Yes`, `back`, `Escape`, and `Q` behavior. Run setup and mappings only after the phase package operations succeed, then refresh command lookup before resolving later phases.

- [ ] **Step 4: Remove PowerShell-specific mise bootstrap**

Delete `Install-Mise` and its direct call. The provider phase must install `mise` through its normal package declarations and be the only bootstrap path.

- [ ] **Step 5: Run PowerShell verification**

Run `pwsh -NoProfile -File .\tests\test-install.ps1` and `pwsh -NoProfile -File .\tests\test-packages.ps1`. Expected: PASS.

### Task 5: Final Cross-Platform Regression And Spec Consistency

**Files:**
- Modify: `tests/test-packages.sh`
- Modify: `tests/test-packages.ps1`
- Modify: `tests/test-install.sh` only if phase behavior changes existing assertions
- Modify: `tests/test-install.ps1` only if phase behavior changes existing assertions

- [ ] **Step 1: Add final behavior assertions**

Verify that default selection does not add a provider unless a selected package needs it, implicit providers are absent from explicit validation output, provider mappings are guarded by shell-module requirements, and existing backup behavior remains intact.

- [ ] **Step 2: Run the complete test matrix**

Run:

```bash
bash -n install.sh
bash tests/test-install.sh
bash tests/test-packages.sh
bash tests/test-secrets.sh
pwsh -NoProfile -File .\tests\test-install.ps1
pwsh -NoProfile -File .\tests\test-packages.ps1
```

Expected: all commands exit successfully with their existing pass messages.

- [ ] **Step 3: Perform final static review**

Run `git diff --check`, inspect `git diff --stat`, verify no plaintext secrets or installation state were added, and confirm no `mise_required`, `install_mise`, or Atuin-specific installer logic remains.
