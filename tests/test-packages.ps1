$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$manifest = Get-Content (Join-Path $root 'modules\manifest.conf')
if ('module=atuin' -notin $manifest) { throw 'Atuin module registration missing' }
if ('module=oh-my-posh' -notin $manifest) { throw 'oh-my-posh module registration missing' }
if ('module=mise' -notin $manifest) { throw 'mise module registration missing' }
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
Write-Output 'PowerShell package tests passed'
