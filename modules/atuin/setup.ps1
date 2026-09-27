$statusOutput = & atuin status 2>$null
if ($LASTEXITCODE -eq 0) {
    Write-Output 'Atuin is already logged in; skipping login.'
    exit 0
}
if (-not $env:ATUIN_USERNAME -or -not $env:ATUIN_PASSWORD -or -not $env:ATUIN_KEY) { throw 'Atuin login secrets are missing.' }
# ATUIN_SYNC_ADDRESS is optional and, when set, is already inherited from the
# environment that exported the rest of the secrets.
# Pass the secrets as flags instead of piping them on stdin: atuin asks for the
# encryption key before the password on the default (Hub) sync path and in the
# opposite order on the legacy path, and the password prompt reads the terminal
# rather than stdin, so no fixed stdin order satisfies both.
& atuin login -u $ATUIN_USERNAME --password $ATUIN_PASSWORD --key $ATUIN_KEY
if ($LASTEXITCODE -ne 0) { throw 'Atuin login failed.' }
