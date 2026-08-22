$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$manifest = Get-Content (Join-Path $root 'modules\manifest.conf')
if ('module=atuin' -notin $manifest) { throw 'Atuin module registration missing' }
if ('module=oh-my-posh' -notin $manifest) { throw 'oh-my-posh module registration missing' }
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
Write-Output 'PowerShell package tests passed'
