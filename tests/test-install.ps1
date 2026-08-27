$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
. (Join-Path $PSScriptRoot 'test-helpers.ps1')
$powershellProfile = Get-Content (Join-Path $root 'modules\powershell\profile.ps1') -Raw
Assert-True ($powershellProfile -match '\$PROFILE' -and $powershellProfile -match 'Profile\.d' -and $powershellProfile -match "-Filter '\*\.ps1'") 'PowerShell profile drop-in loader missing'
Assert-NotContains $powershellProfile 'atuin init'
foreach ($requiredProfileContent in @(
        'zoxide init powershell',
        'Import-Module posh-git',
        'function TabExpansion2',
        'CommandCompletion]::CompleteInput',
        'function l',
        'function la',
        'function ll',
        'function lla',
        'Set-Alias -Name ls',
        'function lt',
        'function zh',
        'function Update-Software',
        'uv generate-shell-completion powershell',
        'uvx --generate-shell-completion powershell',
        'function Edit-Path',
        'Microsoft.WinGet.CommandNotFound'
    )) {
    Assert-Contains $powershellProfile $requiredProfileContent
}
$temp = Join-Path ([System.IO.Path]::GetTempPath()) ('dotfiles-test-' + [guid]::NewGuid())
$testHome = Join-Path $temp 'home'
$local = Join-Path $temp 'local'
$profile = Join-Path $temp 'profile-drive\Documents\PowerShell\Microsoft.PowerShell_profile.ps1'
New-Item -ItemType Directory -Force -Path $testHome | Out-Null
$env:LOCALAPPDATA = $local
$env:DOTFILES_HOME = $testHome
Assert-True ('default=true' -in (Get-Content (Join-Path $root 'modules\git\module.conf'))) 'git default missing'
Assert-True ('default=true' -in (Get-Content (Join-Path $root 'modules\powershell\module.conf'))) 'powershell default missing'
Assert-True ([string]::Join("`n", (Get-Content (Join-Path $root 'install.ps1'))) -match 'Escape|Q') 'PowerShell cancellation handling missing'
$output = & (Join-Path $root 'install.ps1') -List | Out-String
Assert-Contains $output 'git - Git configuration'
Assert-Contains $output 'powershell - PowerShell profile'
Assert-Contains $output 'oh-my-posh - Oh My Posh prompt'
Assert-True ($output -notmatch '(?m)^shell -') 'legacy shell module is still listed'
$helpOutput = & (Join-Path $root 'install.ps1') -Help | Out-String
Assert-Contains $helpOutput 'Usage: install.ps1'
$opencodeModule = Get-Content (Join-Path $root 'modules\opencode\module.conf')
Assert-True ('setup=windows:opencode/setup.ps1' -in $opencodeModule) 'OpenCode setup declaration was not discovered'
$installerText = [string]::Join("`n", (Get-Content (Join-Path $root 'install.ps1')))
Assert-True ($installerText -notmatch '(?i)opencode|openrouter|azure_api_key|digitalocean|theme|mcp|plugin') 'PowerShell installer contains OpenCode-specific setup logic'
& pwsh -NoProfile -Command "`$PROFILE = '$profile'; & '$root\install.ps1' -Apps powershell" | Out-Null
$target = $profile
Assert-FileExists $target
$unknownSucceeded = $true
try { & (Join-Path $root 'install.ps1') -Apps unknown 2>$null } catch { $unknownSucceeded = $false }
Assert-True (-not $unknownSucceeded) 'unknown module succeeded'
Assert-Contains ((& (Join-Path $root 'install.ps1') -Apps '') -join "`n") 'No applications selected'
Remove-Item -LiteralPath $temp -Recurse -Force
$env:DOTFILES_HOME = $null
Write-Output 'PowerShell installer tests passed'
