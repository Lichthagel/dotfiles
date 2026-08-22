$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$gitModule = Get-Content (Join-Path $root 'modules\git\module.conf')
if ('package=git|winget:Git.Git' -notin $gitModule) { throw 'winget package declaration missing' }
if ([string]::Join("`n", (Get-Content (Join-Path $root 'install.ps1'))) -notmatch '-Yes') { throw 'Yes option missing' }
Write-Output 'PowerShell package tests passed'
