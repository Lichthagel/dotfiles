$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$temp = Join-Path ([System.IO.Path]::GetTempPath()) ('dotfiles-test-' + [guid]::NewGuid())
$testHome = Join-Path $temp 'home'
$local = Join-Path $temp 'local'
New-Item -ItemType Directory -Force -Path $testHome | Out-Null
$env:LOCALAPPDATA = $local
$env:DOTFILES_HOME = $testHome
if ('default=true' -notin (Get-Content (Join-Path $root 'modules\git\module.conf'))) { throw 'git default missing' }
if ('default=true' -notin (Get-Content (Join-Path $root 'modules\powershell\module.conf'))) { throw 'powershell default missing' }
if ([string]::Join("`n", (Get-Content (Join-Path $root 'install.ps1'))) -notmatch "Escape|Q") { throw 'PowerShell cancellation handling missing' }
$output = & (Join-Path $root 'install.ps1') -List | Out-String
if ($output -notmatch 'git - Git configuration' -or $output -notmatch 'powershell - PowerShell profile') { throw 'list output missing module' }
if ($output -match '(?m)^shell -') { throw 'legacy shell module is still listed' }
& (Join-Path $root 'install.ps1') -Apps git | Out-Null
$target = Join-Path $testHome '.gitconfig'
if (-not (Test-Path $target)) { throw 'gitconfig was not installed' }
$unknownSucceeded = $true
try { & (Join-Path $root 'install.ps1') -Apps unknown 2>$null } catch { $unknownSucceeded = $false }
if ($unknownSucceeded) { throw 'unknown module succeeded' }
if ((& (Join-Path $root 'install.ps1') -Apps '') -notmatch 'No applications selected') { throw 'empty app selection was not a no-op' }
Remove-Item -LiteralPath $temp -Recurse -Force
$env:DOTFILES_HOME = $null
Write-Output 'PowerShell installer tests passed'
