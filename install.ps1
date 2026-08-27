[CmdletBinding()]
param(
    [string]$Apps,
    [switch]$List,
    [switch]$Help,
    [switch]$Yes
)

$ErrorActionPreference = 'Stop'
$Root = $PSScriptRoot
$DotfilesHome = if ($env:DOTFILES_HOME) { $env:DOTFILES_HOME } else { $HOME }
$BackupRoot = Join-Path ($env:LOCALAPPDATA ?? (Join-Path $DotfilesHome 'AppData\Local')) 'dotfiles\backups'

if (-not (Test-Path (Join-Path $Root 'modules\manifest.conf'))) {
    if (-not $env:DOTFILES_REPO_URL) { throw 'Set DOTFILES_REPO_URL when running install.ps1 from a pipe.' }
    $bootstrapDir = Join-Path ([System.IO.Path]::GetTempPath()) ('dotfiles-' + [guid]::NewGuid())
    New-Item -ItemType Directory -Force -Path $bootstrapDir | Out-Null
    try {
        $archive = Join-Path $bootstrapDir 'repo.zip'
        Invoke-WebRequest -Uri "$($env:DOTFILES_REPO_URL)/archive/refs/heads/main.zip" -OutFile $archive
        Expand-Archive -LiteralPath $archive -DestinationPath $bootstrapDir
        $extracted = Get-ChildItem -LiteralPath $bootstrapDir -Directory | Where-Object { $_.Name -ne 'repo.zip' } | Select-Object -First 1
        if (-not $extracted) { throw 'Repository archive did not contain a root directory.' }
        $forwarded = @{}
        if ($PSBoundParameters.ContainsKey('Apps')) { $forwarded.Apps = $Apps }
        if ($List) { $forwarded.List = $true }
        if ($Help) { $forwarded.Help = $true }
        if ($Yes) { $forwarded.Yes = $true }
        & (Join-Path $extracted.FullName 'install.ps1') @forwarded
        exit $LASTEXITCODE
    } finally {
        Remove-Item -LiteralPath $bootstrapDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($Help) {
    Write-Output 'Usage: install.ps1 [-Apps name1,name2] [-Yes] [-List] [-Help]'
    Write-Output 'Install selected dotfiles modules. Without -Apps, selection is interactive.'
    exit 0
}

$modules = @{}
$dependencies = @()
$dependencyManifest = Join-Path $Root 'modules\dependencies.conf'
foreach ($line in Get-Content $dependencyManifest) {
    if ($line -match '^dependency=([^|]+)\|([^:]+):(.+)$') { $dependencies += [pscustomobject]@{ Logical = $Matches[1]; Manager = $Matches[2]; Name = $Matches[3] } }
    elseif (-not [string]::IsNullOrWhiteSpace($line) -and -not $line.StartsWith('#')) { throw "Invalid dependency line: $line" }
}
$manifest = Get-Content (Join-Path $Root 'modules\manifest.conf')
foreach ($entry in $manifest) {
    if ([string]::IsNullOrWhiteSpace($entry) -or $entry.StartsWith('#')) { continue }
    if ($entry -notmatch '^module=([A-Za-z0-9_-]+)$') { throw "Invalid manifest line: $entry" }
    $name = $Matches[1]
    $config = Join-Path $Root "modules\$name\module.conf"
    $data = @{ maps = @(); packages = @(); setups = @(); requires = @(); provides = $null; secretsOptional = $false }
    foreach ($line in Get-Content $config) {
        if ($line -match '^name=(.+)$') { $data.name = $Matches[1] }
        elseif ($line -match '^description=(.+)$') { $data.description = $Matches[1] }
        elseif ($line -match '^platforms=(.+)$') { $data.platforms = $Matches[1].Split(',') }
        elseif ($line -match '^default=(true|false)$') { $data.default = [bool]::Parse($Matches[1]) }
        elseif ($line -match '^secrets=(optional)$') { $data.secretsOptional = $true }
        elseif ($line -match '^secrets=') { throw "Invalid module line: $line" }
        elseif ($line -match '^provides=(apt|dnf|pacman|brew|mise|scoop|winget)$') { $data.provides = $Matches[1] }
        elseif ($line -match '^provides=') { throw "Invalid module line: $line" }
        elseif ($line -match '^requires=([A-Za-z0-9_-]+(?:,[A-Za-z0-9_-]+)*)$') { $data.requires = @($Matches[1].Split(',')) }
        elseif ($line -match '^package=([^|]+)\|([^:]+):(.+)$') { $data.packages += [pscustomobject]@{ Logical = $Matches[1]; Manager = $Matches[2]; Name = $Matches[3] } }
        elseif ($line -match '^setup=(linux|windows):(.+)$') { $data.setups += [pscustomobject]@{ Platform = $Matches[1]; Path = $Matches[2] } }
        elseif ($line -match '^map=([^|]+)\|([^|]+)(?:\|requires=([A-Za-z0-9_-]+(?:,[A-Za-z0-9_-]+)*))?$') {
            $requirements = if ($Matches[3]) { @($Matches[3].Split(',')) } else { @() }
            $data.maps += [pscustomobject]@{ Source = $Matches[1]; Target = $Matches[2]; Requires = $requirements }
        }
        elseif (-not [string]::IsNullOrWhiteSpace($line) -and -not $line.StartsWith('#')) { throw "Invalid module line: $line" }
    }
    $modules[$name] = $data
}

$managerPriority = @('winget', 'scoop', 'brew', 'mise')
$managerValid = @{ windows = @('winget', 'scoop', 'brew', 'mise') }
function Test-ManagerAvailable($manager) { return [bool](Get-Command $manager -ErrorAction SilentlyContinue) }
function Get-ProviderModule($manager) {
    $matches = @($modules.Keys | Where-Object { $modules[$_].provides -eq $manager -and $modules[$_].platforms -contains 'windows' })
    if ($matches.Count -gt 1) { throw "Multiple modules provide $manager`: $($matches -join ', ')" }
    if ($matches.Count) { return $matches[0] }
    return $null
}
function Resolve-ModulePhases($selectedNames) {
    $resolved = [System.Collections.Generic.List[string]]::new()
    $selectedNames | ForEach-Object { if (-not $resolved.Contains($_)) { $resolved.Add($_) } }
    $inputs = @($selectedNames | ForEach-Object { $modules[$_].packages })
    if ($selectedNames | Where-Object { $env:DOTFILES_SECRETS_FILE -or (Test-Path -LiteralPath (Join-Path $Root "secrets\$_.env.age")) }) { $inputs += $dependencies }
    do {
        $changed = $false
        foreach ($moduleName in @($resolved)) {
            foreach ($required in @($modules[$moduleName].requires)) {
                if (-not $modules.ContainsKey($required)) { throw "Unknown module dependency: $required" }
                if ($modules[$required].platforms -notcontains 'windows') { throw "Module is not supported on Windows: $required" }
                if (-not $resolved.Contains($required)) { $resolved.Add($required); $inputs += $modules[$required].packages; $changed = $true }
            }
            foreach ($logical in @($inputs.Logical | Sort-Object -Unique)) {
                $options = @($inputs | Where-Object Logical -eq $logical)
                if (@($options | Where-Object { Test-ManagerAvailable $_.Manager }).Count) { continue }
                foreach ($option in $options) {
                    if (Test-ManagerAvailable $option.Manager) { continue }
                    $provider = Get-ProviderModule $option.Manager
                    if ($provider -and -not $resolved.Contains($provider)) { $resolved.Add($provider); $inputs += $modules[$provider].packages; $changed = $true }
                }
            }
        }
    } while ($changed)
    $ordered = [System.Collections.Generic.List[string]]::new()
    while ($ordered.Count -lt $resolved.Count) {
        $progress = $false
        foreach ($moduleName in $resolved) {
            if ($ordered.Contains($moduleName)) { continue }
            $ready = $true
            if (-not $modules[$moduleName].provides) {
                foreach ($providerName in $resolved) {
                    if ($modules[$providerName].provides -and -not $ordered.Contains($providerName)) { $ready = $false }
                }
            }
            foreach ($required in @($modules[$moduleName].requires)) { if (-not $ordered.Contains($required)) { $ready = $false } }
            foreach ($package in @($modules[$moduleName].packages)) {
                $provider = Get-ProviderModule $package.Manager
                if ($provider -and $provider -ne $moduleName -and $resolved.Contains($provider) -and -not $ordered.Contains($provider)) { $ready = $false }
            }
            if ($ready) { $ordered.Add($moduleName); $progress = $true }
        }
        if (-not $progress) { throw 'Module dependency cycle detected.' }
    }
    return @($ordered | ForEach-Object { [pscustomobject]@{ Modules = @($_) } })
}
function Test-PackageInstalled($manager, $package) {
    switch ($manager) {
        'winget' { return [bool]((& winget list --id $package --exact --accept-source-agreements 2>$null) -match [regex]::Escape($package)) }
        'scoop' { $output = & scoop list $package 2>$null | Out-String; return [bool]($output -match "(?mi)^\s*$([regex]::Escape($package))\s") }
        'brew' { & brew list --versions $package *> $null; return $LASTEXITCODE -eq 0 }
        'mise' { if ($package -eq 'mise') { return (Test-ManagerAvailable 'mise') }; return [bool]((& mise list 2>$null) -match "(?m)^$([regex]::Escape($package))\s") }
    }
    return $false
}
function Get-PackagePlan($selectedNames) {
    $plan = @()
    $inputs = @()
    foreach ($moduleName in $selectedNames) { $inputs += [pscustomobject]@{ Module = $moduleName; Packages = $modules[$moduleName].packages } }
    if ($selectedNames | Where-Object { $env:DOTFILES_SECRETS_FILE -or (Test-Path -LiteralPath (Join-Path $Root "secrets\$_.env.age")) }) {
        $inputs += [pscustomobject]@{ Module = '__dependency'; Packages = $dependencies }
    }
    foreach ($input in $inputs) {
        $moduleName = $input.Module
        $logicalNames = @($input.Packages.Logical | Sort-Object -Unique)
        foreach ($logical in $logicalNames) {
            $provider = if ($moduleName -ne '__dependency') { $modules[$moduleName].provides } else { $null }
            $options = @($input.Packages | Where-Object { $_.Logical -eq $logical -and $_.Manager -in $managerValid.windows -and $_.Manager -ne $provider -and (Test-ManagerAvailable $_.Manager) })
            if (-not $options.Count) { throw "No supported package manager is available for $logical." }
            $chosen = $null
            foreach ($manager in $managerPriority) {
                $candidate = $options | Where-Object Manager -eq $manager | Select-Object -First 1
                if ($candidate) { if (Test-PackageInstalled $candidate.Manager $candidate.Name) { $chosen = $null; break }; if (-not $chosen) { $chosen = $candidate } }
            }
            if ($chosen) { $plan += [pscustomobject]@{ Module = $moduleName; Logical = $logical; Options = $options; Manager = $chosen.Manager; Name = $chosen.Name; Selected = $true } }
        }
    }
    return @($plan)
}
function Select-PackagePlan($plan) {
    if (-not $plan.Count) { return 'ok' }
    if ([Console]::IsInputRedirected -or [Console]::IsOutputRedirected) { throw 'Package confirmation requires a terminal. Use -Yes for noninteractive setup.' }
    $cursor = 0
    while ($true) {
        Clear-Host
        Write-Host 'Package plan (Up/Down move, Left/Right manager, Space toggle, Enter install, b back):'
        for ($i = 0; $i -lt $plan.Count; $i++) {
            $marker = if ($plan[$i].Selected) { 'x' } else { ' ' }
            $pointer = if ($i -eq $cursor) { '>' } else { ' ' }
            Write-Host "$pointer [$marker] $($plan[$i].Logical) -> $($plan[$i].Manager)"
        }
        $key = [Console]::ReadKey($true)
        switch ($key.Key) {
            'UpArrow' { if ($cursor -gt 0) { $cursor-- } }
            'DownArrow' { if ($cursor -lt ($plan.Count - 1)) { $cursor++ } }
            'LeftArrow' { $options = @($plan[$cursor].Options); $position = [array]::IndexOf($options.Manager, $plan[$cursor].Manager); $position = ($position - 1 + $options.Count) % $options.Count; $plan[$cursor].Manager = $options[$position].Manager; $plan[$cursor].Name = $options[$position].Name }
            'RightArrow' { $options = @($plan[$cursor].Options); $position = [array]::IndexOf($options.Manager, $plan[$cursor].Manager); $position = ($position + 1) % $options.Count; $plan[$cursor].Manager = $options[$position].Manager; $plan[$cursor].Name = $options[$position].Name }
            'Spacebar' { $plan[$cursor].Selected = -not $plan[$cursor].Selected }
            'B' { Clear-Host; return 'back' }
            'Escape' { Clear-Host; return 'cancel' }
            'Q' { Clear-Host; return 'cancel' }
            'Enter' { Clear-Host; return 'ok' }
        }
    }
}
function Invoke-ModuleSetup($selectedNames) {
    . (Join-Path $Root 'lib\secrets.ps1')
    foreach ($moduleName in $selectedNames) {
        $setup = @($modules[$moduleName].setups | Where-Object Platform -eq 'windows' | Select-Object -First 1)
        if (-not $setup.Count) { continue }
        $previousRoot = $env:DOTFILES_ROOT
        $previousFile = $env:DOTFILES_SECRET_FILE
        $previousKeys = $env:DOTFILES_SECRET_KEYS
        $state = $null
        try {
            $env:DOTFILES_ROOT = $Root
            $defaultSecret = Join-Path $Root "secrets\$moduleName.env.age"
            $env:DOTFILES_SECRET_FILE = if ($env:DOTFILES_SECRETS_FILE) { $env:DOTFILES_SECRETS_FILE } elseif (Test-Path -LiteralPath $defaultSecret) { $defaultSecret } else { '' }
            $env:DOTFILES_SECRET_KEYS = ''
            $identity = if ($env:AGE_IDENTITIES) { $env:AGE_IDENTITIES } else { Join-Path ($env:APPDATA ?? (Join-Path $env:USERPROFILE 'AppData\Roaming')) 'age\keys.txt' }
            $hasIdentity = $env:AGE_IDENTITY -or $env:AGE_IDENTITIES -or (Test-Path -LiteralPath $identity)
            if ($env:DOTFILES_SECRET_FILE -and (-not $modules[$moduleName].secretsOptional -or $hasIdentity)) { $state = Initialize-DotfilesSecrets }
            elseif ($env:DOTFILES_SECRET_FILE) { $env:DOTFILES_SECRET_FILE = '' }
            & (Join-Path $Root "modules\$($setup[0].Path)")
            if ($LASTEXITCODE -ne 0) { throw "Module setup failed: $moduleName" }
        } finally {
            Remove-DotfilesSecrets $state
            $env:DOTFILES_ROOT = $previousRoot
            $env:DOTFILES_SECRET_FILE = $previousFile
            $env:DOTFILES_SECRET_KEYS = $previousKeys
        }
    }
}
function Install-ModuleMappings($selectedNames) {
    $timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $failures = 0
    foreach ($name in $selectedNames) {
        foreach ($mapping in $modules[$name].maps) {
            if ($mapping.Source -notmatch '^windows:(.+)$') { continue }
            if (@($mapping.Requires | Where-Object { $_ -notin $resolvedNames }).Count) { continue }
            $source = Join-Path $Root "modules\$($Matches[1])"
            $target = Resolve-MapTarget $mapping.Target
            if (-not (Test-Path -LiteralPath $source)) { Write-Error "Missing source: $source"; $failures++; continue }
            $parent = Split-Path $target -Parent
            New-Item -ItemType Directory -Force -Path $parent | Out-Null
            if (Test-Path -LiteralPath $target) {
                $backupRelative = $target.TrimStart('\', '/')
                $backup = Join-Path $BackupRoot "$timestamp\$backupRelative"
                New-Item -ItemType Directory -Force -Path (Split-Path $backup -Parent) | Out-Null
                Move-Item -LiteralPath $target -Destination $backup
                Write-Output "backed up: $target -> $backup"
            }
            try {
                New-Item -ItemType SymbolicLink -Path $target -Target $source -ErrorAction Stop | Out-Null
                Write-Output "linked: $target"
            } catch {
                Copy-Item -LiteralPath $source -Destination $target -Recurse -Force
                Write-Output "copied: $target"
            }
        }
    }
    if ($failures -gt 0) { throw 'One or more mappings failed.' }
}
function Install-Package($manager, $package) {
    switch ($manager) {
        'winget' { & winget install --id $package --exact --accept-source-agreements --accept-package-agreements }
        'scoop' { & scoop install $package }
        'brew' { & brew install $package }
        'mise' { & mise use --global $package }
        default { throw "Unsupported package manager: $manager" }
    }
    if ($LASTEXITCODE -ne 0) { throw "Package installation failed: $package" }
}
function Resolve-MapTarget($target) {
    $target = $target.Replace('$PROFILE', [string]$PROFILE)
    $target = [regex]::Replace($target, '\$env:([A-Za-z_][A-Za-z0-9_]*)', {
        param($match) [Environment]::GetEnvironmentVariable($match.Groups[1].Value)
    })
    if ([System.IO.Path]::IsPathRooted($target)) { return $target }
    return (Join-Path $DotfilesHome $target)
}

if ($List) {
    foreach ($name in $modules.Keys | Sort-Object) {
        if ($modules[$name].platforms -contains 'windows') { Write-Output "$name - $($modules[$name].description)" }
    }
    exit 0
}

if (-not $PSBoundParameters.ContainsKey('Apps')) {
    $available = @($modules.Keys | Where-Object { $modules[$_].platforms -contains 'windows' } | Sort-Object)
    if ([Console]::IsInputRedirected -or [Console]::IsOutputRedirected) { throw 'Interactive selection requires a terminal. Use -Apps for noninteractive setup.' }
    $selected = @($available | Where-Object { $modules[$_].default })
    $cursor = 0
    $cancelled = $false
    try {
        while ($true) {
            Clear-Host
            Write-Output 'Select applications (Up/Down to move, Space to toggle, Enter to confirm):'
            for ($i = 0; $i -lt $available.Count; $i++) {
                $marker = if ($selected -contains $available[$i]) { 'x' } else { ' ' }
                $pointer = if ($i -eq $cursor) { '>' } else { ' ' }
                Write-Output "$pointer [$marker] $($available[$i]) - $($modules[$available[$i]].description)"
            }
            $key = [Console]::ReadKey($true)
            switch ($key.Key) {
                'UpArrow' { if ($cursor -gt 0) { $cursor-- } }
                'DownArrow' { if ($cursor -lt ($available.Count - 1)) { $cursor++ } }
                'Spacebar' {
                    if ($selected -contains $available[$cursor]) { $selected = @($selected | Where-Object { $_ -ne $available[$cursor] }) }
                    else { $selected += $available[$cursor] }
                }
                'Enter' { $Apps = $selected -join ','; break }
                'Escape' { $cancelled = $true; break }
                'Q' { $cancelled = $true; break }
            }
            if ($key.Key -eq 'Enter' -or $cancelled) { break }
        }
    } finally {
        Clear-Host
    }
    if ($cancelled) { Write-Output 'Selection cancelled.'; exit 0 }
}

if ([string]::IsNullOrWhiteSpace($Apps)) { Write-Output 'No applications selected.'; exit 0 }

$selectedNames = @($Apps.Split(',') | ForEach-Object { $_.Trim() } | Where-Object { $_ })
foreach ($name in $selectedNames) {
    if (-not $modules.ContainsKey($name)) { throw "Unknown application: $name" }
    if ($modules[$name].platforms -notcontains 'windows') { throw "Application not supported on Windows: $name" }
}

$phases = Resolve-ModulePhases $selectedNames
$resolvedNames = @($phases.Modules | Sort-Object -Unique)
foreach ($phase in $phases) {
    $packagePlan = Get-PackagePlan $phase.Modules
    if ($packagePlan.Count) {
        if (-not $Yes) {
            $packageResult = Select-PackagePlan $packagePlan
            if ($packageResult -eq 'cancel') { Write-Output 'Selection cancelled.'; exit 0 }
            if ($packageResult -eq 'back') {
                if ($PSBoundParameters.ContainsKey('Apps')) { Write-Output 'Module selection is fixed by -Apps.'; exit 0 }
                & $PSCommandPath
                exit $LASTEXITCODE
            }
            $confirmation = Read-Host 'Install these packages? [y/N]'
            if ($confirmation -notmatch '^(y|yes)$') { Write-Output 'Package installation declined.'; exit 0 }
        }
        foreach ($package in $packagePlan) {
            if ($package.Selected) { Install-Package $package.Manager $package.Name }
            else { Write-Output "Skipped package $($package.Logical) for module $($package.Module); dotfiles were still installed." }
        }
    }
    $env:Path = "$HOME\.local\bin;$HOME\scoop\shims;$env:Path"
    if ($env:LOCALAPPDATA) { $env:Path = "$(Join-Path $env:LOCALAPPDATA 'mise\bin');$(Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Links');$env:Path" }
    Invoke-ModuleSetup $phase.Modules
    Install-ModuleMappings $phase.Modules
}
