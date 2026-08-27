$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$manifest = Get-Content (Join-Path $root 'modules\manifest.conf')
$module = Get-Content (Join-Path $root 'modules\opencode\module.conf')
$readme = Get-Content (Join-Path $root 'README.md') -Raw
$contributing = Get-Content (Join-Path $root 'CONTRIBUTING.md') -Raw

if ('module=opencode' -notin $manifest) { throw 'OpenCode module registration missing' }
if ('name=opencode' -notin $module) { throw 'OpenCode module name missing' }
if ('platforms=linux,windows' -notin $module) { throw 'OpenCode platforms missing' }
if ('default=false' -notin $module) { throw 'OpenCode must be opt-in' }
if ('secrets=optional' -notin $module) { throw 'OpenCode secret setup must be optional' }
if ('setup=linux:opencode/setup.sh' -notin $module) { throw 'OpenCode Bash setup declaration missing' }
if ('setup=windows:opencode/setup.ps1' -notin $module) { throw 'OpenCode PowerShell setup declaration missing' }

foreach ($text in @('./install.sh --apps opencode', '.\install.ps1 -Apps opencode', 'secrets/opencode.env.age', 'uvx duckduckgo-mcp-server', 'superpowers@git+https://github.com/obra/superpowers.git', 'latest Catppuccin OpenCode themes', 'does not select or add a default theme', 'Never commit the plaintext')) {
    if ($readme -notlike "*$text*") { throw "README OpenCode documentation missing: $text" }
}
foreach ($secretName in @('OPENROUTER_API_KEY', 'AZURE_API_KEY', 'AZURE_RESOURCE_NAME', 'DIGITALOCEAN_ACCESS_TOKEN')) {
    if ($readme -notlike "*$secretName*") { throw "README OpenCode secret name missing: $secretName" }
}
foreach ($text in @('protected user-local files', 'do not commit that bundle')) {
    if ($contributing -notlike "*$text*") { throw "CONTRIBUTING OpenCode documentation missing: $text" }
}

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
$configRoot = Join-Path $testRoot 'home'
$configDir = Join-Path $configRoot '.config\opencode'
$configFile = Join-Path $configDir 'opencode.jsonc'
$secretValues = @{
    OPENROUTER_API_KEY = 'openrouter-test-secret'
    AZURE_API_KEY = 'azure-test-secret'
    AZURE_RESOURCE_NAME = 'azure-resource-test'
    DIGITALOCEAN_ACCESS_TOKEN = 'digitalocean-test-secret'
}
$oldHome = $env:HOME
$oldSecrets = @{}
foreach ($name in $secretValues.Keys) {
    $oldSecrets[$name] = [Environment]::GetEnvironmentVariable($name)
    Set-Item "Env:$name" $secretValues[$name]
}

$themeServer = Join-Path $testRoot 'themeserver'

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
    (Get-Content -LiteralPath $configFile -Raw).Replace('"preserved": true', '"preserved": true // JSONC comment') | Set-Content -LiteralPath $configFile -Encoding utf8
    $env:HOME = $configRoot

    New-Item -ItemType Directory -Path (Join-Path $themeServer 'themes\frappe'), (Join-Path $themeServer 'themes\mocha') -Force | Out-Null
    @{ tree = @(
        @{ path = 'themes/frappe/catppuccin-frappe-shared.json'; type = 'blob' }
        @{ path = 'themes/mocha/catppuccin-mocha-shared.json'; type = 'blob' }
        @{ path = 'README.md'; type = 'blob' }
    ) } | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $themeServer 'tree.json')
    '{"name":"frappe"}' | Set-Content (Join-Path $themeServer 'themes\frappe\catppuccin-frappe-shared.json')
    '{"name":"mocha"}' | Set-Content (Join-Path $themeServer 'themes\mocha\catppuccin-mocha-shared.json')
    $env:OPENCODE_THEME_API_URL = ([Uri]::new((Join-Path $themeServer 'tree.json'))).AbsoluteUri
    $env:OPENCODE_THEME_RAW_URL = ([Uri]::new($themeServer)).AbsoluteUri.TrimEnd('/')
    $themesDir = Join-Path $configDir 'themes'
    New-Item -ItemType Directory -Path $themesDir -Force | Out-Null
    '{"name":"old"}' | Set-Content (Join-Path $themesDir 'catppuccin-old.json')
    '{"name":"keep"}' | Set-Content (Join-Path $themesDir 'unrelated.json')
    New-Item -ItemType Directory -Path (Join-Path $themesDir 'local') -Force | Out-Null
    '{"name":"local"}' | Set-Content (Join-Path $themesDir 'local\catppuccin-local.json')

    & (Join-Path $root 'modules\opencode\setup.ps1')

    $result = Get-Content -LiteralPath $configFile -Raw | ConvertFrom-Json
    foreach ($theme in @('catppuccin-frappe-shared.json', 'catppuccin-mocha-shared.json')) {
        if (-not (Test-Path (Join-Path $themesDir $theme))) { throw "Theme missing: $theme" }
    }
    if ((Get-Content (Join-Path $themesDir 'catppuccin-frappe-shared.json') -Raw | ConvertFrom-Json).name -ne 'frappe') { throw 'Frappé theme was not flattened correctly' }
    if ((Get-Content (Join-Path $themesDir 'catppuccin-mocha-shared.json') -Raw | ConvertFrom-Json).name -ne 'mocha') { throw 'Mocha theme was not flattened correctly' }
    if (-not (Test-Path (Join-Path $themesDir 'catppuccin-old.json'))) { throw 'Existing theme was removed' }
    if (-not (Test-Path (Join-Path $themesDir 'unrelated.json'))) { throw 'Unrelated theme was removed' }
    if (-not (Test-Path (Join-Path $themesDir 'local\catppuccin-local.json'))) { throw 'Nested unrelated theme was removed' }
    if ($null -ne $result.theme) { throw 'Default theme was added' }
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
    if ([System.Environment]::OSVersion.Platform -eq [System.PlatformID]::Win32NT) {
        $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
        $acl = Get-Acl -LiteralPath (Join-Path $configDir 'openrouter-api-key')
        $identityRule = @($acl.Access | Where-Object { $_.IdentityReference.Value -eq $identity -and $_.FileSystemRights -band [System.Security.AccessControl.FileSystemRights]::FullControl })
        if ($identityRule.Count -ne 1 -or $identityRule[0].IsInherited) { throw 'Credential ACL is not user-only' }
    }
    if ((Get-Content (Join-Path $configDir 'openrouter-api-key') -Raw) -ne $secretValues.OPENROUTER_API_KEY) { throw 'OpenRouter credential content is incorrect' }
    if ((Get-Content (Join-Path $configDir 'azure-api-key') -Raw) -ne $secretValues.AZURE_API_KEY) { throw 'Azure credential content is incorrect' }
    if ((Get-Content (Join-Path $configDir 'azure-resource-name') -Raw) -ne $secretValues.AZURE_RESOURCE_NAME) { throw 'Azure resource credential content is incorrect' }
    if ((Get-Content (Join-Path $configDir 'digitalocean-access-token') -Raw) -ne $secretValues.DIGITALOCEAN_ACCESS_TOKEN) { throw 'DigitalOcean credential content is incorrect' }

    & (Join-Path $root 'modules\opencode\setup.ps1')
    $result = Get-Content -LiteralPath $configFile -Raw | ConvertFrom-Json
    if (@($result.plugin | Where-Object { $_ -eq $pluginName }).Count -ne 1) { throw 'Superpowers plugin is not idempotent' }

    $configBeforeNoSecrets = Get-Content -LiteralPath $configFile -Raw
    $credentialsBeforeNoSecrets = @('openrouter-api-key', 'azure-api-key', 'azure-resource-name', 'digitalocean-access-token') | ForEach-Object { "${_}:$((Get-Content (Join-Path $configDir $_) -Raw))" }
    $savedSecrets = @{}
    foreach ($name in $secretValues.Keys) { $savedSecrets[$name] = [Environment]::GetEnvironmentVariable($name); Remove-Item "Env:$name" }
    try { & (Join-Path $root 'modules\opencode\setup.ps1') } finally { foreach ($name in $secretValues.Keys) { [Environment]::SetEnvironmentVariable($name, $savedSecrets[$name], 'Process') } }
    if ((Get-Content -LiteralPath $configFile -Raw) -ne $configBeforeNoSecrets) { throw 'Provider config changed without secrets' }
    $credentialsAfterNoSecrets = @('openrouter-api-key', 'azure-api-key', 'azure-resource-name', 'digitalocean-access-token') | ForEach-Object { "${_}:$((Get-Content (Join-Path $configDir $_) -Raw))" }
    if ([string]::Join('|', $credentialsAfterNoSecrets) -ne [string]::Join('|', $credentialsBeforeNoSecrets)) { throw 'Provider credentials changed without secrets' }

    $withoutPlugin = Get-Content -LiteralPath $configFile -Raw | ConvertFrom-Json
    $withoutPlugin.PSObject.Properties.Remove('plugin')
    $withoutPlugin | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $configFile -Encoding utf8
    & (Join-Path $root 'modules\opencode\setup.ps1')
    $result = Get-Content -LiteralPath $configFile -Raw | ConvertFrom-Json
    if (@($result.plugin | Where-Object { $_ -eq $pluginName }).Count -ne 1) { throw 'Superpowers plugin was not created' }

    $configBeforeMissing = Get-Content -LiteralPath $configFile -Raw
    $credentialsBeforeMissing = @('openrouter-api-key', 'azure-api-key', 'azure-resource-name', 'digitalocean-access-token') | ForEach-Object { "${_}:$((Get-Content (Join-Path $configDir $_) -Raw))" }
    $savedAzureKey = $env:AZURE_API_KEY
    Remove-Item Env:AZURE_API_KEY
    try {
        try { & (Join-Path $root 'modules\opencode\setup.ps1'); throw 'Missing secret unexpectedly succeeded' } catch {
            if ($_.Exception.Message -eq 'Missing secret unexpectedly succeeded') { throw }
        }
    } finally { $env:AZURE_API_KEY = $savedAzureKey }
    if ((Get-Content -LiteralPath $configFile -Raw) -ne $configBeforeMissing) { throw 'Config changed after missing secret' }
    $credentialsAfterMissing = @('openrouter-api-key', 'azure-api-key', 'azure-resource-name', 'digitalocean-access-token') | ForEach-Object { "${_}:$((Get-Content (Join-Path $configDir $_) -Raw))" }
    if ([string]::Join('|', $credentialsAfterMissing) -ne [string]::Join('|', $credentialsBeforeMissing)) { throw 'Credentials changed after missing secret' }

    $configBeforeThemeFailure = Get-Content -LiteralPath $configFile -Raw
    $credentialsBeforeThemeFailure = @('openrouter-api-key', 'azure-api-key', 'azure-resource-name', 'digitalocean-access-token') | ForEach-Object { "${_}:$((Get-Content (Join-Path $configDir $_) -Raw))" }
    @{ tree = @(@{ path = 'themes/macchiato/catppuccin-macchiato-missing.json'; type = 'blob' }) } | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $themeServer 'tree.json')
    $themesBefore = @(Get-ChildItem -LiteralPath $themesDir -File -Recurse | ForEach-Object { $_.FullName.Substring($themesDir.Length + 1) } | Sort-Object)
    try { & (Join-Path $root 'modules\opencode\setup.ps1'); throw 'Missing theme unexpectedly succeeded' } catch {
        if ($_.Exception.Message -eq 'Missing theme unexpectedly succeeded') { throw }
    }
    $themesAfter = @(Get-ChildItem -LiteralPath $themesDir -File -Recurse | ForEach-Object { $_.FullName.Substring($themesDir.Length + 1) } | Sort-Object)
    if ([string]::Join('|', $themesAfter) -ne [string]::Join('|', $themesBefore)) { throw 'Theme set changed after failed download' }
    if ((Get-Content -LiteralPath $configFile -Raw) -ne $configBeforeThemeFailure) { throw 'Config changed after failed theme setup' }
    $credentialsAfterThemeFailure = @('openrouter-api-key', 'azure-api-key', 'azure-resource-name', 'digitalocean-access-token') | ForEach-Object { "${_}:$((Get-Content (Join-Path $configDir $_) -Raw))" }
    if ([string]::Join('|', $credentialsAfterThemeFailure) -ne [string]::Join('|', $credentialsBeforeThemeFailure)) { throw 'Credentials changed after failed theme setup' }
    if (@(Get-ChildItem -LiteralPath $configDir -Directory -Filter '.themes.*').Count -ne 0) { throw 'Theme staging directory was not cleaned up' }

    $invalid = '{ "unrelated": "must remain"'
    Set-Content -LiteralPath $configFile -Value $invalid -Encoding utf8
    $credentialsBeforeInvalid = @('openrouter-api-key', 'azure-api-key', 'azure-resource-name', 'digitalocean-access-token') | ForEach-Object { "${_}:$((Get-Content (Join-Path $configDir $_) -Raw))" }
    try {
        & (Join-Path $root 'modules\opencode\setup.ps1')
        throw 'Invalid JSON unexpectedly succeeded'
    } catch {
        if ($_.Exception.Message -eq 'Invalid JSON unexpectedly succeeded') { throw }
    }
    if ((Get-Content -LiteralPath $configFile -Raw) -ne ($invalid + [Environment]::NewLine)) { throw 'Invalid JSON was replaced' }
    $credentialsAfterInvalid = @('openrouter-api-key', 'azure-api-key', 'azure-resource-name', 'digitalocean-access-token') | ForEach-Object { "${_}:$((Get-Content (Join-Path $configDir $_) -Raw))" }
    if ([string]::Join('|', $credentialsAfterInvalid) -ne [string]::Join('|', $credentialsBeforeInvalid)) { throw 'Credentials changed after invalid JSON' }
} finally {
    if ($null -eq $oldHome) { Remove-Item Env:HOME -ErrorAction SilentlyContinue } else { $env:HOME = $oldHome }
    foreach ($name in $secretValues.Keys) {
        if ($null -eq $oldSecrets[$name]) { Remove-Item "Env:$name" -ErrorAction SilentlyContinue } else { Set-Item "Env:$name" $oldSecrets[$name] }
    }
    Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Output 'OpenCode PowerShell assertions passed'
