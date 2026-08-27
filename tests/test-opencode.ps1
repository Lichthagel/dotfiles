$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$manifest = Get-Content (Join-Path $root 'modules\manifest.conf')
$module = Get-Content (Join-Path $root 'modules\opencode\module.conf')

if ('module=opencode' -notin $manifest) { throw 'OpenCode module registration missing' }
if ('name=opencode' -notin $module) { throw 'OpenCode module name missing' }
if ('platforms=linux,windows' -notin $module) { throw 'OpenCode platforms missing' }
if ('default=false' -notin $module) { throw 'OpenCode must be opt-in' }
if ('setup=linux:opencode/setup.sh' -notin $module) { throw 'OpenCode Bash setup declaration missing' }
if ('setup=windows:opencode/setup.ps1' -notin $module) { throw 'OpenCode PowerShell setup declaration missing' }

if ('package=opencode|brew:anomalyco/tap/opencode' -notin $module) { throw 'OpenCode Homebrew package declaration missing' }
if ('package=opencode|mise:github:anomalyco/opencode' -notin $module) { throw 'OpenCode mise package declaration missing' }
if ('package=opencode|scoop:opencode' -notin $module) { throw 'OpenCode Scoop package declaration missing' }

if ('package=jq|apt:jq' -notin $module) { throw 'jq apt package declaration missing' }
if ('package=jq|dnf:jq' -notin $module) { throw 'jq dnf package declaration missing' }
if ('package=jq|pacman:jq' -notin $module) { throw 'jq pacman package declaration missing' }
if ('package=jq|brew:jq' -notin $module) { throw 'jq Homebrew package declaration missing' }
if ('package=jq|winget:jqlang.jq' -notin $module) { throw 'jq WinGet package declaration missing' }
if ('package=jq|scoop:jq' -notin $module) { throw 'jq Scoop package declaration missing' }

Write-Output 'OpenCode PowerShell assertions passed'
