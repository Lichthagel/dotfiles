$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$manifest = Get-Content (Join-Path $root 'modules\manifest.conf')
if ('module=atuin' -notin $manifest) { throw 'Atuin module registration missing' }
if ('module=oh-my-posh' -notin $manifest) { throw 'oh-my-posh module registration missing' }
if ('module=mise' -notin $manifest) { throw 'mise module registration missing' }
if ('module=brew' -notin $manifest) { throw 'brew module registration missing' }
if ('module=scoop' -notin $manifest) { throw 'scoop module registration missing' }
$scoopModule = Get-Content (Join-Path $root 'modules\scoop\module.conf')
if ('name=scoop' -notin $scoopModule) { throw 'scoop module name missing' }
if ('platforms=windows' -notin $scoopModule) { throw 'scoop module must be Windows-only' }
if ('default=false' -notin $scoopModule) { throw 'scoop module must be opt-in' }
if ('provides=scoop' -notin $scoopModule) { throw 'scoop provider declaration missing' }
if ('setup=windows:scoop/setup.ps1' -notin $scoopModule) { throw 'scoop setup declaration missing' }
if ((Get-Content (Join-Path $root 'modules\scoop\setup.ps1') -Raw) -notmatch 'get\.scoop\.sh') { throw 'scoop setup script missing official installer' }
if ((Get-Content (Join-Path $root 'modules\scoop\powershell\05-scoop.ps1') -Raw) -notmatch 'scoop\\shims') { throw 'scoop PowerShell integration missing' }
$brewModule = Get-Content (Join-Path $root 'modules\brew\module.conf')
if ('name=brew' -notin $brewModule) { throw 'brew module name missing' }
if ('platforms=linux' -notin $brewModule) { throw 'brew module must be Linux-only' }
if ('default=false' -notin $brewModule) { throw 'brew module must be opt-in' }
if ('provides=brew' -notin $brewModule) { throw 'brew provider declaration missing' }
if ('setup=linux:brew/setup.sh' -notin $brewModule) { throw 'brew setup declaration missing' }
if ((Get-Content (Join-Path $root 'modules\brew\setup.sh') -Raw) -notmatch 'brew shellenv') { throw 'brew setup script missing shellenv configuration' }
if ((Get-Content (Join-Path $root 'modules\brew\bash\05-brew.bash') -Raw) -notmatch 'brew shellenv') { throw 'brew Bash integration missing' }
$miseModule = Get-Content (Join-Path $root 'modules\mise\module.conf')
if ('name=mise' -notin $miseModule) { throw 'mise module name missing' }
if ('default=true' -notin $miseModule) { throw 'mise module must be enabled by default' }
if ('provides=mise' -notin $miseModule) { throw 'mise provider declaration missing' }
if ('package=mise|winget:jdx.mise' -notin $miseModule) { throw 'mise WinGet package declaration missing' }
if ('map=windows:mise/powershell/10-mise.ps1|$PROFILE\..\Profile.d\10-mise.ps1|requires=powershell' -notin $miseModule) { throw 'mise PowerShell integration declaration missing' }
if ((Get-Content (Join-Path $root 'modules\mise\powershell\10-mise.ps1') -Raw) -notmatch 'mise activate pwsh') { throw 'mise PowerShell activation missing' }
if ((Get-Content (Join-Path $root 'modules\mise\bash\10-mise.bash') -Raw) -notmatch 'mise activate bash') { throw 'mise Bash activation missing' }
if ((Get-Content (Join-Path $root 'modules\powershell\profile.ps1') -Raw) -match 'mise activate') { throw 'mise activation remains in the general PowerShell profile' }
$atuinModule = Get-Content (Join-Path $root 'modules\atuin\module.conf')
if ('name=atuin' -notin $atuinModule) { throw 'Atuin module name missing' }
if ('default=false' -notin $atuinModule) { throw 'Atuin must be opt-in' }
$gitModule = Get-Content (Join-Path $root 'modules\git\module.conf')
$ohMyPoshModule = Get-Content (Join-Path $root 'modules\oh-my-posh\module.conf')
if ('platforms=linux,windows' -notin $ohMyPoshModule) { throw 'oh-my-posh platforms missing' }
if ('default=false' -notin $ohMyPoshModule) { throw 'oh-my-posh must be opt-in' }
if ('map=linux:oh-my-posh/oh-my-posh.config.json|${HOME}/.config/oh-my-posh/config.json' -notin $ohMyPoshModule) { throw 'oh-my-posh Linux config mapping missing' }
if ('map=windows:oh-my-posh/oh-my-posh.config.json|$PROFILE\..\oh-my-posh.config.json' -notin $ohMyPoshModule) { throw 'oh-my-posh Windows config mapping missing' }
if ('map=linux:oh-my-posh/bash/10-oh-my-posh.bash|${HOME}/.config/bashrc.d/10-oh-my-posh.bash|requires=bash' -notin $ohMyPoshModule) { throw 'oh-my-posh Bash drop-in declaration missing' }
if ('map=windows:oh-my-posh/powershell/10-oh-my-posh.ps1|$PROFILE\..\Profile.d\10-oh-my-posh.ps1|requires=powershell' -notin $ohMyPoshModule) { throw 'oh-my-posh PowerShell drop-in declaration missing' }
if ('package=oh-my-posh|winget:JanDeDobbeleer.OhMyPosh' -notin $ohMyPoshModule) { throw 'oh-my-posh WinGet package declaration missing' }
if ('package=oh-my-posh|scoop:oh-my-posh' -notin $ohMyPoshModule) { throw 'oh-my-posh Scoop package declaration missing' }
if ('package=git|winget:Git.Git' -notin $gitModule) { throw 'winget package declaration missing' }
if ('package=atuin|winget:Atuinsh.Atuin' -notin $atuinModule) { throw 'Atuin WinGet package declaration missing' }
if ('package=atuin|scoop:atuin' -notin $atuinModule) { throw 'Atuin Scoop package declaration missing' }
if ('package=atuin|brew:atuin' -notin $atuinModule) { throw 'Atuin Homebrew package declaration missing' }
if ('package=atuin|mise:atuin' -notin $atuinModule) { throw 'Atuin mise package declaration missing' }
if ([string]::Join("`n", $atuinModule) -notmatch '(?m)^default=false$') { throw 'Atuin is not opt-in' }
if ($atuinModule -match '^secret=') { throw 'Atuin must not declare explicit secrets' }
if ('setup=windows:atuin/setup.ps1' -notin $atuinModule) { throw 'PowerShell Atuin setup declaration missing' }
if ('map=windows:atuin/powershell/50-atuin.ps1|$PROFILE\..\Profile.d\50-atuin.ps1|requires=powershell' -notin $atuinModule) { throw 'PowerShell Atuin drop-in declaration missing' }
if (-not (Test-Path (Join-Path $root 'modules\atuin\powershell\50-atuin.ps1'))) { throw 'PowerShell Atuin drop-in missing' }
if ((Get-Content (Join-Path $root 'modules\atuin\powershell\50-atuin.ps1') -Raw) -notmatch 'atuin init powershell') { throw 'PowerShell Atuin initialization missing' }
if ([string]::Join("`n", (Get-Content (Join-Path $root 'install.ps1'))) -match 'Process-AtuinSecrets|atuin login|ATUIN_USERNAME') { throw 'PowerShell installer contains Atuin-specific code' }
$dependencies = Get-Content (Join-Path $root 'modules\dependencies.conf')
if ('dependency=age|winget:FiloSottile.age' -notin $dependencies) { throw 'age WinGet dependency missing' }
if ([string]::Join("`n", (Get-Content (Join-Path $root 'lib\secrets.ps1'))) -notmatch 'Read-Host.*Age identity') { throw 'PowerShell age identity prompt missing' }
if ([string]::Join("`n", (Get-Content (Join-Path $root 'install.ps1'))) -notmatch '-Yes') { throw 'Yes option missing' }
$installer = [string]::Join("`n", (Get-Content (Join-Path $root 'install.ps1')))
if ($installer -match 'Install-Mise|https://mise\.run') { throw 'mise-specific bootstrap logic remains in the installer' }
if ($installer -notmatch 'Resolve-ModulePhases') { throw 'generic module phase resolver missing' }
if ($installer -match 'return if') { throw 'PowerShell provider resolver uses invalid return-if syntax' }
$opencodeModule = Get-Content (Join-Path $root 'modules\opencode\module.conf')
if ('setup=windows:opencode/setup.ps1' -notin $opencodeModule) { throw 'OpenCode setup declaration was not discovered' }
if ($installer -match '(?i)opencode|openrouter|azure_api_key|digitalocean|theme|mcp|plugin') { throw 'PowerShell installer contains OpenCode-specific setup logic' }

$fixture = Join-Path ([System.IO.Path]::GetTempPath()) ('dotfiles-opencode-package-' + [guid]::NewGuid())
$fixtureBin = Join-Path $fixture 'bin'
$fixtureAppData = Join-Path $fixture 'appdata'
$fixtureLocalAppData = Join-Path $fixture 'localappdata'
$fixtureLog = Join-Path $fixture 'events.log'
$fixtureSecrets = Join-Path $fixture 'opencode.env.age'
$fixtureThemes = Join-Path $fixture 'themes'
$oldPath = $env:Path
$oldHome = $env:HOME
$oldLocalAppData = $env:LOCALAPPDATA
$oldDotfilesHome = $env:DOTFILES_HOME
$oldSecretFile = $env:DOTFILES_SECRETS_FILE
$oldAgeIdentity = $env:AGE_IDENTITY
$oldThemeApi = $env:OPENCODE_THEME_API_URL
$oldThemeRaw = $env:OPENCODE_THEME_RAW_URL
try {
    New-Item -ItemType Directory -Force -Path $fixtureBin, $fixtureAppData, $fixtureLocalAppData, (Join-Path $fixtureThemes 'themes') | Out-Null
    @'
@echo off
if "%1"=="list" (
    if exist "%SCOOP_FIXTURE_BIN%\opencode.cmd" echo     opencode 1.0
    exit /b 0
)
if "%1"=="install" (
    echo package-start:%2>>"%SCOOP_FIXTURE_LOG%"
    if "%2"=="opencode" copy /y "%SCOOP_FIXTURE_BIN%\opencode-template.cmd" "%SCOOP_FIXTURE_BIN%\opencode.cmd" >nul
    echo package-complete:%2>>"%SCOOP_FIXTURE_LOG%"
    exit /b 0
)
exit /b 1
'@ | Set-Content -LiteralPath (Join-Path $fixtureBin 'scoop.cmd') -Encoding ascii
    '@echo off`r`nexit /b 0' | Set-Content -LiteralPath (Join-Path $fixtureBin 'opencode-template.cmd') -Encoding ascii
    @'
@echo off
echo setup-start>>"%SCOOP_FIXTURE_LOG%"
echo OPENROUTER_API_KEY=test-openrouter
echo AZURE_API_KEY=test-azure
echo AZURE_RESOURCE_NAME=test-resource
echo DIGITALOCEAN_ACCESS_TOKEN=test-digitalocean
'@ | Set-Content -LiteralPath (Join-Path $fixtureBin 'age.cmd') -Encoding ascii
    '{"tree":[{"path":"themes/test.json","type":"blob"}]}' | Set-Content -LiteralPath (Join-Path $fixtureThemes 'tree.json') -Encoding utf8
    '{"name":"test theme"}' | Set-Content -LiteralPath (Join-Path $fixtureThemes 'themes\test.json') -Encoding utf8
    'encrypted fixture' | Set-Content -LiteralPath $fixtureSecrets -Encoding ascii

    $env:Path = "$fixtureBin;$oldPath"
    $env:SCOOP_FIXTURE_BIN = $fixtureBin
    $env:SCOOP_FIXTURE_LOG = $fixtureLog
    $env:HOME = Join-Path $fixture 'home'
    $env:LOCALAPPDATA = $fixtureLocalAppData
    $env:DOTFILES_HOME = Join-Path $fixture 'home'
    $env:DOTFILES_SECRETS_FILE = $fixtureSecrets
    $env:AGE_IDENTITY = 'fixture-age-key'
    $env:OPENCODE_THEME_API_URL = ([Uri]::new((Join-Path $fixtureThemes 'tree.json'))).AbsoluteUri
    $env:OPENCODE_THEME_RAW_URL = ([Uri]::new($fixtureThemes)).AbsoluteUri.TrimEnd('/')
    function scoop { & (Join-Path $env:SCOOP_FIXTURE_BIN 'scoop.cmd') @args }
    function age { & (Join-Path $env:SCOOP_FIXTURE_BIN 'age.cmd') @args }

    # An existing OpenCode command must suppress its package install.
    Copy-Item (Join-Path $fixtureBin 'opencode-template.cmd') (Join-Path $fixtureBin 'opencode.cmd')
    New-Item -ItemType Directory -Force -Path (Join-Path $env:HOME '.config\opencode') | Out-Null
    '{"plugin":[]}' | Set-Content -LiteralPath (Join-Path $env:HOME '.config\opencode\opencode.jsonc') -Encoding utf8
    Set-Content -LiteralPath $fixtureLog -Value '' -Encoding ascii
    & (Join-Path $root 'install.ps1') -Apps opencode -Yes
    if (Select-String -LiteralPath $fixtureLog -Pattern '^package-(start|complete):' -Quiet) { throw 'Existing OpenCode command triggered package installation' }
    if (-not (Test-Path (Join-Path $env:HOME '.config\opencode\opencode.jsonc'))) { throw 'Windows OpenCode setup declaration was not executed' }

    # With both commands missing, scoop is the preferred available manager.
    Remove-Item -LiteralPath (Join-Path $fixtureBin 'opencode.cmd')
    Set-Content -LiteralPath $fixtureLog -Value '' -Encoding ascii
    Remove-Item -LiteralPath $fixtureAppData -Recurse -Force
    New-Item -ItemType Directory -Force -Path $fixtureAppData | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $env:HOME '.config\opencode') | Out-Null
    '{"plugin":[]}' | Set-Content -LiteralPath (Join-Path $env:HOME '.config\opencode\opencode.jsonc') -Encoding utf8
    & (Join-Path $root 'install.ps1') -Apps opencode -Yes
    $events = @(Get-Content -LiteralPath $fixtureLog)
    if ('package-complete:opencode' -notin $events) { throw 'Preferred OpenCode package was not installed' }
    $setupIndex = [array]::IndexOf($events, 'setup-start')
    $packageIndexes = @($events | ForEach-Object { [array]::IndexOf($events, $_) } | Where-Object { $events[$_] -like 'package-complete:*' })
    if ($setupIndex -lt 0 -or @($packageIndexes | Where-Object { $_ -ge $setupIndex }).Count) { throw 'Setup started before package completion' }
} finally {
    $env:Path = $oldPath
    foreach ($name in @('SCOOP_FIXTURE_BIN', 'SCOOP_FIXTURE_LOG')) { Remove-Item "Env:$name" -ErrorAction SilentlyContinue }
    if ($null -eq $oldHome) { Remove-Item Env:HOME -ErrorAction SilentlyContinue } else { $env:HOME = $oldHome }
    if ($null -eq $oldLocalAppData) { Remove-Item Env:LOCALAPPDATA -ErrorAction SilentlyContinue } else { $env:LOCALAPPDATA = $oldLocalAppData }
    if ($null -eq $oldDotfilesHome) { Remove-Item Env:DOTFILES_HOME -ErrorAction SilentlyContinue } else { $env:DOTFILES_HOME = $oldDotfilesHome }
    if ($null -eq $oldSecretFile) { Remove-Item Env:DOTFILES_SECRETS_FILE -ErrorAction SilentlyContinue } else { $env:DOTFILES_SECRETS_FILE = $oldSecretFile }
    if ($null -eq $oldAgeIdentity) { Remove-Item Env:AGE_IDENTITY -ErrorAction SilentlyContinue } else { $env:AGE_IDENTITY = $oldAgeIdentity }
    if ($null -eq $oldThemeApi) { Remove-Item Env:OPENCODE_THEME_API_URL -ErrorAction SilentlyContinue } else { $env:OPENCODE_THEME_API_URL = $oldThemeApi }
    if ($null -eq $oldThemeRaw) { Remove-Item Env:OPENCODE_THEME_RAW_URL -ErrorAction SilentlyContinue } else { $env:OPENCODE_THEME_RAW_URL = $oldThemeRaw }
    Remove-Item Function:scoop, Function:age -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Output 'PowerShell package tests passed'
