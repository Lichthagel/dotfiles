$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$manifest = Get-Content (Join-Path $root 'modules\manifest.conf')
if ('module=atuin' -notin $manifest) { throw 'Atuin module registration missing' }
$atuinModule = Get-Content (Join-Path $root 'modules\atuin\module.conf')
if ('name=atuin' -notin $atuinModule) { throw 'Atuin module name missing' }
if ('default=false' -notin $atuinModule) { throw 'Atuin must be opt-in' }
$gitModule = Get-Content (Join-Path $root 'modules\git\module.conf')
if ('package=git|winget:Git.Git' -notin $gitModule) { throw 'winget package declaration missing' }
if ('package=atuin|winget:Atuinsh.Atuin' -notin $atuinModule) { throw 'Atuin WinGet package declaration missing' }
if ('package=atuin|scoop:atuin' -notin $atuinModule) { throw 'Atuin Scoop package declaration missing' }
if ('package=atuin|brew:atuin' -notin $atuinModule) { throw 'Atuin Homebrew package declaration missing' }
if ('package=atuin|mise:atuin' -notin $atuinModule) { throw 'Atuin mise package declaration missing' }
if ([string]::Join("`n", $atuinModule) -notmatch '(?m)^default=false$') { throw 'Atuin is not opt-in' }
if ($atuinModule -match '^secret=') { throw 'Atuin must not declare explicit secrets' }
if ('setup=windows:atuin/setup.ps1' -notin $atuinModule) { throw 'PowerShell Atuin setup declaration missing' }
if ([string]::Join("`n", (Get-Content (Join-Path $root 'install.ps1'))) -match 'Process-AtuinSecrets|atuin login|ATUIN_USERNAME') { throw 'PowerShell installer contains Atuin-specific code' }
$dependencies = Get-Content (Join-Path $root 'modules\dependencies.conf')
if ('dependency=age|winget:FiloSottile.age' -notin $dependencies) { throw 'age WinGet dependency missing' }
if ([string]::Join("`n", (Get-Content (Join-Path $root 'lib\secrets.ps1'))) -notmatch 'Read-Host.*Age identity') { throw 'PowerShell age identity prompt missing' }
if ([string]::Join("`n", (Get-Content (Join-Path $root 'install.ps1'))) -notmatch '-Yes') { throw 'Yes option missing' }
Write-Output 'PowerShell package tests passed'
