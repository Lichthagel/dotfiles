$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$fixture = Join-Path ([System.IO.Path]::GetTempPath()) ('bootstrap-fixture-' + [guid]::NewGuid())
$release = Join-Path $fixture 'repo\releases\download\v1'
$bin = Join-Path $release 'dotfiles-windows-x86_64.exe'
New-Item -ItemType Directory -Force -Path $release | Out-Null

try {
    $compiled = Join-Path $root 'target\debug\dotfiles.exe'
    if (-not (Test-Path -LiteralPath $compiled)) {
        & cargo build --locked
        if ($LASTEXITCODE -ne 0) { throw 'Unable to build the Windows test binary.' }
    }
    Copy-Item -LiteralPath $compiled -Destination $bin -Force
    $hash = (Get-FileHash -LiteralPath $bin -Algorithm SHA256).Hash.ToLowerInvariant()
    Set-Content -LiteralPath (Join-Path $release 'dotfiles-windows-x86_64.exe.sha256') -Value "$hash  dotfiles-windows-x86_64.exe" -NoNewline

    function Invoke-WebRequest {
        param([string]$Uri, [string]$OutFile)
        Copy-Item -LiteralPath ([Uri]$Uri).LocalPath -Destination $OutFile -Force
    }

    $oldRepo = $env:DOTFILES_REPO_URL
    $oldVersion = $env:DOTFILES_VERSION
    $oldProfile = $env:DOTFILES_PROFILE
    try {
        $env:DOTFILES_REPO_URL = ([Uri](Join-Path $fixture 'repo')).AbsoluteUri
        $env:DOTFILES_VERSION = 'v1'
        $PROFILE = Join-Path $fixture 'profile.ps1'
        $profileParent = Split-Path -Parent $PROFILE
        New-Item -ItemType Directory -Force -Path $profileParent | Out-Null
        $bootstrapText = Get-Content -LiteralPath (Join-Path $root 'bootstrap\install.ps1') -Raw
        if ($bootstrapText -notmatch 'IsWindows|OSVersion\.Platform') { throw 'Windows OS detection is missing' }
        $beforeTemp = @(Get-ChildItem -LiteralPath ([System.IO.Path]::GetTempPath()) -Directory -Filter 'dotfiles-bootstrap-*' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName)
        $output = (& (Join-Path $root 'bootstrap\install.ps1') --modules powershell --yes 2>&1 | Out-String)
        $status = $LASTEXITCODE
        if ($status -ne 0) { throw "bootstrap did not forward successful child status (got $status)" }
        if ($output -notmatch 'verified') { throw 'bootstrap did not report checksum verification' }
        if (-not (Test-Path -LiteralPath $PROFILE)) { throw 'DOTFILES_PROFILE was not forwarded to the child binary' }
        $afterTemp = @(Get-ChildItem -LiteralPath ([System.IO.Path]::GetTempPath()) -Directory -Filter 'dotfiles-bootstrap-*' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName)
        if (@($afterTemp | Where-Object { $_ -notin $beforeTemp }).Count -ne 0) { throw 'temporary bootstrap directory was not removed' }

        $argumentOutput = (& (Join-Path $root 'bootstrap\install.ps1') --unknown forwarded 2>&1 | Out-String)
        $argumentStatus = $LASTEXITCODE
        if ($argumentStatus -ne 2) { throw "bootstrap did not forward child status (got $argumentStatus)" }
        if ($argumentOutput -notmatch '--unknown') { throw 'bootstrap did not forward arguments' }
        Remove-Item -LiteralPath $PROFILE -Force
        Set-Content -LiteralPath (Join-Path $release 'dotfiles-windows-x86_64.exe.sha256') -Value ('0' * 64 + '  dotfiles-windows-x86_64.exe') -NoNewline
        $beforeBadTemp = @(Get-ChildItem -LiteralPath ([System.IO.Path]::GetTempPath()) -Directory -Filter 'dotfiles-bootstrap-*' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName)
        $badStatus = 0
        try { & (Join-Path $root 'bootstrap\install.ps1') --modules powershell --yes 2>$null } catch { $badStatus = 1 }
        if ($badStatus -eq 0) { throw 'checksum failure succeeded' }
        if (Test-Path -LiteralPath $PROFILE) { throw 'checksum failure executed the downloaded binary' }
        $afterBadTemp = @(Get-ChildItem -LiteralPath ([System.IO.Path]::GetTempPath()) -Directory -Filter 'dotfiles-bootstrap-*' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName)
        if (@($afterBadTemp | Where-Object { $_ -notin $beforeBadTemp }).Count -ne 0) { throw 'temporary bootstrap directory was not removed after checksum failure' }
    } finally {
        if ($null -eq $oldRepo) { Remove-Item Env:DOTFILES_REPO_URL -ErrorAction SilentlyContinue } else { $env:DOTFILES_REPO_URL = $oldRepo }
        if ($null -eq $oldVersion) { Remove-Item Env:DOTFILES_VERSION -ErrorAction SilentlyContinue } else { $env:DOTFILES_VERSION = $oldVersion }
        if ($null -eq $oldProfile) { Remove-Item Env:DOTFILES_PROFILE -ErrorAction SilentlyContinue } else { $env:DOTFILES_PROFILE = $oldProfile }
        Remove-Item Function:Invoke-WebRequest -ErrorAction SilentlyContinue
    }
} finally {
    Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Output 'bootstrap PowerShell tests passed'
