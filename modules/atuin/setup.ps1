$statusOutput = & atuin status 2>$null
if ($LASTEXITCODE -eq 0) {
    Write-Output 'Atuin is already logged in; skipping login.'
    exit 0
}
if (-not $env:ATUIN_USERNAME -or -not $env:ATUIN_PASSWORD -or -not $env:ATUIN_KEY) { throw 'Atuin login secrets are missing.' }
$previousSync = $env:ATUIN_SYNC_ADDRESS
try {
    @($ATUIN_PASSWORD, $ATUIN_KEY) | & atuin login -u $ATUIN_USERNAME
    if ($LASTEXITCODE -ne 0) { throw 'Atuin login failed.' }
} finally {
    $env:ATUIN_SYNC_ADDRESS = $previousSync
}
