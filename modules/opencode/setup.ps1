$ErrorActionPreference = 'Stop'

$requiredSecrets = @(
    'OPENROUTER_API_KEY'
    'AZURE_API_KEY'
    'AZURE_RESOURCE_NAME'
    'DIGITALOCEAN_ACCESS_TOKEN'
)
foreach ($name in $requiredSecrets) {
    if ([string]::IsNullOrEmpty([Environment]::GetEnvironmentVariable($name))) {
        throw "Missing required OpenCode secret: $name"
    }
}

$configDir = Join-Path $env:APPDATA 'opencode'
$configFile = Join-Path $configDir 'opencode.json'
New-Item -ItemType Directory -Path $configDir -Force | Out-Null

function Protect-SecretFile {
    param([string]$Path)

    $acl = Get-Acl -LiteralPath $Path
    $acl.SetAccessRuleProtection($true, $false)
    foreach ($rule in @($acl.Access)) {
        $acl.RemoveAccessRule($rule) | Out-Null
    }
    $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
    $rule = [System.Security.AccessControl.FileSystemAccessRule]::new(
        $identity,
        [System.Security.AccessControl.FileSystemRights]::FullControl,
        [System.Security.AccessControl.AccessControlType]::Allow
    )
    $acl.AddAccessRule($rule)
    Set-Acl -LiteralPath $Path -AclObject $acl
}

function Write-ProtectedSecret {
    param(
        [string]$Name,
        [string]$Value
    )

    $temp = Join-Path $configDir ('.' + $Name + '.' + [guid]::NewGuid().ToString('N') + '.tmp')
    try {
        [System.IO.File]::WriteAllText($temp, $Value, [System.Text.UTF8Encoding]::new($false))
        Protect-SecretFile -Path $temp
        Move-Item -LiteralPath $temp -Destination (Join-Path $configDir $Name) -Force
    } finally {
        if (Test-Path -LiteralPath $temp) { Remove-Item -LiteralPath $temp -Force -ErrorAction SilentlyContinue }
    }
}

Write-ProtectedSecret 'openrouter-api-key' $env:OPENROUTER_API_KEY
Write-ProtectedSecret 'azure-api-key' $env:AZURE_API_KEY
Write-ProtectedSecret 'azure-resource-name' $env:AZURE_RESOURCE_NAME
Write-ProtectedSecret 'digitalocean-access-token' $env:DIGITALOCEAN_ACCESS_TOKEN

if (Test-Path -LiteralPath $configFile) {
    $config = Get-Content -LiteralPath $configFile -Raw | ConvertFrom-Json
} else {
    $config = [pscustomobject]@{}
}

if (-not $config.PSObject.Properties['provider']) {
    $config | Add-Member -MemberType NoteProperty -Name provider -Value ([pscustomobject]@{})
}
$config.provider | Add-Member -MemberType NoteProperty -Name openrouter -Value ([pscustomobject]@{
    options = [pscustomobject]@{ apiKey = ('{file:' + (Join-Path $configDir 'openrouter-api-key') + '}') }
}) -Force
$config.provider | Add-Member -MemberType NoteProperty -Name azure -Value ([pscustomobject]@{
    options = [pscustomobject]@{
        apiKey = ('{file:' + (Join-Path $configDir 'azure-api-key') + '}')
        resourceName = ('{file:' + (Join-Path $configDir 'azure-resource-name') + '}')
    }
}) -Force
$config.provider | Add-Member -MemberType NoteProperty -Name digitalocean -Value ([pscustomobject]@{
    options = [pscustomobject]@{ apiKey = ('{file:' + (Join-Path $configDir 'digitalocean-access-token') + '}') }
}) -Force

if (-not $config.PSObject.Properties['mcp']) {
    $config | Add-Member -MemberType NoteProperty -Name mcp -Value ([pscustomobject]@{})
}
$config.mcp | Add-Member -MemberType NoteProperty -Name 'ddg-search' -Value ([pscustomobject]@{
    type = 'local'
    command = @('uvx', 'duckduckgo-mcp-server')
    environment = [pscustomobject]@{ DDG_SAFE_SEARCH = 'OFF' }
}) -Force

$pluginName = 'superpowers@git+https://github.com/obra/superpowers.git'
$plugins = if ($config.PSObject.Properties['plugin']) { @($config.plugin) } else { @() }
$config.plugin = @($plugins | Where-Object { $_ -ne $pluginName }) + $pluginName

$configTemp = Join-Path $configDir ('.opencode.' + [guid]::NewGuid().ToString('N') + '.tmp')
try {
    $json = $config | ConvertTo-Json -Depth 20
    [System.IO.File]::WriteAllText($configTemp, $json, [System.Text.UTF8Encoding]::new($false))
    Get-Content -LiteralPath $configTemp -Raw | ConvertFrom-Json | Out-Null
    Move-Item -LiteralPath $configTemp -Destination $configFile -Force
} finally {
    if (Test-Path -LiteralPath $configTemp) { Remove-Item -LiteralPath $configTemp -Force -ErrorAction SilentlyContinue }
}

$themesDir = Join-Path $configDir 'themes'
$themeApiUrl = if ($env:OPENCODE_THEME_API_URL) { $env:OPENCODE_THEME_API_URL } else { 'https://api.github.com/repos/catppuccin/opencode/git/trees/main?recursive=1' }
$themeRawUrl = if ($env:OPENCODE_THEME_RAW_URL) { $env:OPENCODE_THEME_RAW_URL } else { 'https://raw.githubusercontent.com/catppuccin/opencode/main' }
$themeTemp = Join-Path $configDir ('.themes.' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $themeTemp -Force | Out-Null
try {
    $themeApi = [Uri]$themeApiUrl
    $tree = if ($themeApi.IsFile) {
        Get-Content -LiteralPath $themeApi.LocalPath -Raw | ConvertFrom-Json
    } else {
        Invoke-RestMethod -Uri $themeApiUrl -TimeoutSec 30
    }
    $themePaths = @($tree.tree | Where-Object {
        $_.type -eq 'blob' -and $_.path -match '^themes/[^/]+\.json$'
    } | ForEach-Object { [string]$_.path })
    if ($themePaths.Count -eq 0) { throw 'No Catppuccin OpenCode themes discovered' }

    foreach ($themePath in $themePaths) {
        $themeName = Split-Path $themePath -Leaf
        $themeFile = Join-Path $themeTemp $themeName
        $themeUri = [Uri]($themeRawUrl.TrimEnd('/') + '/' + $themePath)
        if ($themeUri.IsFile) {
            Copy-Item -LiteralPath $themeUri.LocalPath -Destination $themeFile
        } else {
            Invoke-WebRequest -Uri $themeUri.AbsoluteUri -OutFile $themeFile -TimeoutSec 30
        }
        Get-Content -LiteralPath $themeFile -Raw | ConvertFrom-Json | Out-Null
    }

    New-Item -ItemType Directory -Path $themesDir -Force | Out-Null
    Get-ChildItem -LiteralPath $themesDir -Filter 'catppuccin-*.json' -File | Remove-Item -Force
    Get-ChildItem -LiteralPath $themeTemp -Filter '*.json' -File | Move-Item -Destination $themesDir -Force
} finally {
    if (Test-Path -LiteralPath $themeTemp) { Remove-Item -LiteralPath $themeTemp -Recurse -Force -ErrorAction SilentlyContinue }
}
