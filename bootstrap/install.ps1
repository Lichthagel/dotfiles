$ErrorActionPreference = 'Stop'

if ([Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT -or
    -not [Environment]::Is64BitOperatingSystem -or
    [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne [System.Runtime.InteropServices.Architecture]::X64) {
    throw 'This bootstrap supports Windows x86_64 only.'
}

$baseUrl = if ($env:DOTFILES_BASE_URL) { $env:DOTFILES_BASE_URL.TrimEnd('/') } else { 'https://r2.licht.moe' }

$temporaryDir = Join-Path ([System.IO.Path]::GetTempPath()) ('dotfiles-bootstrap-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $temporaryDir | Out-Null
try {
    $binary = Join-Path $temporaryDir 'dotfiles-windows-x86_64.exe'
    $checksum = Join-Path $temporaryDir 'dotfiles-windows-x86_64.exe.sha256'
    Invoke-WebRequest -Uri "$baseUrl/dotfiles/dotfiles-windows-x86_64.exe" -OutFile $binary
    Invoke-WebRequest -Uri "$baseUrl/dotfiles/dotfiles-windows-x86_64.exe.sha256" -OutFile $checksum

    $line = Get-Content -LiteralPath $checksum | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -First 1
    $expected = if ($line) { ($line.Trim() -split '\s+')[0].ToLowerInvariant() } else { '' }
    if (-not $expected) { throw 'Release checksum is empty.' }
    $actual = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) { throw 'Release checksum verification failed.' }
    Write-Output 'verified dotfiles-windows-x86_64.exe'

    if ($PROFILE) { $env:DOTFILES_PROFILE = [string]$PROFILE }
    # Do not exit here: this bootstrap is commonly evaluated by Invoke-Expression.
    & $binary @args
} finally {
    Remove-Item -LiteralPath $temporaryDir -Recurse -Force -ErrorAction SilentlyContinue
}
