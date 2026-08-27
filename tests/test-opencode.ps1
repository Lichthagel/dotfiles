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

$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('dotfiles-opencode-' + [guid]::NewGuid())
$configRoot = Join-Path $testRoot 'appdata'
$configDir = Join-Path $configRoot 'opencode'
$configFile = Join-Path $configDir 'opencode.json'
$secretValues = @{
    OPENROUTER_API_KEY = 'openrouter-test-secret'
    AZURE_API_KEY = 'azure-test-secret'
    AZURE_RESOURCE_NAME = 'azure-resource-test'
    DIGITALOCEAN_ACCESS_TOKEN = 'digitalocean-test-secret'
}
$oldAppData = $env:APPDATA
$oldSecrets = @{}
foreach ($name in $secretValues.Keys) {
    $oldSecrets[$name] = [Environment]::GetEnvironmentVariable($name)
    Set-Item "Env:$name" $secretValues[$name]
}

try {
    New-Item -ItemType Directory -Path $configDir -Force | Out-Null
    @{
        '$schema' = 'https://opencode.ai/config.json'
        custom = @{ preserved = $true }
        provider = @{ legacy = @{ options = @{ keep = 'no' } } }
        plugin = @(
            'local-plugin'
            'superpowers@git+https://github.com/obra/superpowers.git'
            'superpowers@git+https://github.com/obra/superpowers.git'
        )
    } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $configFile -Encoding utf8
    $env:APPDATA = $configRoot

    & (Join-Path $root 'modules\opencode\setup.ps1')

    $result = Get-Content -LiteralPath $configFile -Raw | ConvertFrom-Json
    if (-not $result.custom.preserved) { throw 'Unrelated JSON property was not preserved' }
    if ($result.provider.openrouter.options.apiKey -ne ('{file:' + (Join-Path $configDir 'openrouter-api-key') + '}')) { throw 'OpenRouter provider value is incorrect' }
    if ($result.provider.azure.options.apiKey -ne ('{file:' + (Join-Path $configDir 'azure-api-key') + '}')) { throw 'Azure API key value is incorrect' }
    if ($result.provider.azure.options.resourceName -ne ('{file:' + (Join-Path $configDir 'azure-resource-name') + '}')) { throw 'Azure resource value is incorrect' }
    if ($result.provider.digitalocean.options.apiKey -ne ('{file:' + (Join-Path $configDir 'digitalocean-access-token') + '}')) { throw 'DigitalOcean provider value is incorrect' }
    if ($result.mcp.'ddg-search'.type -ne 'local') { throw 'DuckDuckGo MCP type is incorrect' }
    if ([string]::Join('|', $result.mcp.'ddg-search'.command) -ne 'uvx|duckduckgo-mcp-server') { throw 'DuckDuckGo MCP command is incorrect' }
    if ($result.mcp.'ddg-search'.environment.DDG_SAFE_SEARCH -ne 'OFF') { throw 'DuckDuckGo MCP environment is incorrect' }
    $pluginName = 'superpowers@git+https://github.com/obra/superpowers.git'
    if (@($result.plugin | Where-Object { $_ -eq $pluginName }).Count -ne 1) { throw 'Superpowers plugin is not exactly once' }
    if (($result | ConvertTo-Json -Depth 10) | Select-String -Pattern ($secretValues.Values -join '|')) { throw 'Secret value was written to JSON' }
    foreach ($fileName in @('openrouter-api-key', 'azure-api-key', 'azure-resource-name', 'digitalocean-access-token')) {
        $credentialFile = Join-Path $configDir $fileName
        if (-not (Test-Path -LiteralPath $credentialFile)) { throw "Credential file missing: $fileName" }
    }
    if ((Get-Content (Join-Path $configDir 'openrouter-api-key') -Raw) -ne $secretValues.OPENROUTER_API_KEY) { throw 'OpenRouter credential content is incorrect' }
    if ((Get-Content (Join-Path $configDir 'azure-api-key') -Raw) -ne $secretValues.AZURE_API_KEY) { throw 'Azure credential content is incorrect' }
    if ((Get-Content (Join-Path $configDir 'azure-resource-name') -Raw) -ne $secretValues.AZURE_RESOURCE_NAME) { throw 'Azure resource credential content is incorrect' }
    if ((Get-Content (Join-Path $configDir 'digitalocean-access-token') -Raw) -ne $secretValues.DIGITALOCEAN_ACCESS_TOKEN) { throw 'DigitalOcean credential content is incorrect' }

    & (Join-Path $root 'modules\opencode\setup.ps1')
    $result = Get-Content -LiteralPath $configFile -Raw | ConvertFrom-Json
    if (@($result.plugin | Where-Object { $_ -eq $pluginName }).Count -ne 1) { throw 'Superpowers plugin is not idempotent' }

    $invalid = '{ "unrelated": "must remain"'
    Set-Content -LiteralPath $configFile -Value $invalid -Encoding utf8
    try {
        & (Join-Path $root 'modules\opencode\setup.ps1')
        throw 'Invalid JSON unexpectedly succeeded'
    } catch {
        if ($_.Exception.Message -eq 'Invalid JSON unexpectedly succeeded') { throw }
    }
    if ((Get-Content -LiteralPath $configFile -Raw) -ne ($invalid + [Environment]::NewLine)) { throw 'Invalid JSON was replaced' }
} finally {
    if ($null -eq $oldAppData) { Remove-Item Env:APPDATA -ErrorAction SilentlyContinue } else { $env:APPDATA = $oldAppData }
    foreach ($name in $secretValues.Keys) {
        if ($null -eq $oldSecrets[$name]) { Remove-Item "Env:$name" -ErrorAction SilentlyContinue } else { Set-Item "Env:$name" $oldSecrets[$name] }
    }
    Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Output 'OpenCode PowerShell assertions passed'
