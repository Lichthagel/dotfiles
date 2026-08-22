function Initialize-DotfilesSecrets {
    $identity = if ($env:AGE_IDENTITIES) { $env:AGE_IDENTITIES } else { Join-Path ($env:APPDATA ?? (Join-Path $env:USERPROFILE 'AppData\Roaming')) 'age\keys.txt' }
    if (-not $env:DOTFILES_SECRET_FILE -or -not (Test-Path -LiteralPath $env:DOTFILES_SECRET_FILE)) { throw 'Encrypted secrets file not found.' }
    if (-not (Get-Command age -ErrorAction SilentlyContinue)) { throw 'age is required for secrets setup.' }
    $temp = $null
    if ($env:AGE_IDENTITY) {
        $temp = Join-Path ([System.IO.Path]::GetTempPath()) ('dotfiles-secrets-' + [guid]::NewGuid())
        New-Item -ItemType Directory -Force -Path $temp | Out-Null
        $identity = Join-Path $temp 'identity.txt'
        Set-Content -LiteralPath $identity -Value $env:AGE_IDENTITY -NoNewline
    } elseif (-not (Test-Path -LiteralPath $identity)) {
        if ([Console]::IsInputRedirected -or [Console]::IsOutputRedirected) { throw 'Age identity file not found. Set AGE_IDENTITIES for noninteractive setup.' }
        $identityInput = Read-Host 'Age identity file path or key (default not found)'
        if ($identityInput.StartsWith('AGE-SECRET-KEY-')) {
            $temp = Join-Path ([System.IO.Path]::GetTempPath()) ('dotfiles-secrets-' + [guid]::NewGuid())
            New-Item -ItemType Directory -Force -Path $temp | Out-Null
            $identity = Join-Path $temp 'identity.txt'
            Set-Content -LiteralPath $identity -Value $identityInput -NoNewline
        } else { $identity = $identityInput }
    }
    if (-not (Test-Path -LiteralPath $identity)) { throw 'Age identity file not found.' }
    if (-not $temp) { $temp = Join-Path ([System.IO.Path]::GetTempPath()) ('dotfiles-secrets-' + [guid]::NewGuid()); New-Item -ItemType Directory -Force -Path $temp | Out-Null }
    $acl = Get-Acl -LiteralPath $temp
    $acl.SetAccessRuleProtection($true, $false)
    $rule = New-Object System.Security.AccessControl.FileSystemAccessRule([System.Security.Principal.WindowsIdentity]::GetCurrent().Name, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
    $acl.AddAccessRule($rule)
    Set-Acl -LiteralPath $temp -AclObject $acl
    $plain = Join-Path $temp 'secrets.env'
    & age --decrypt -i $identity $env:DOTFILES_SECRET_FILE > $plain
    if ($LASTEXITCODE -ne 0) { Remove-Item -LiteralPath $temp -Recurse -Force; throw 'Unable to decrypt secrets.' }
    $values = @{}
    foreach ($line in Get-Content -LiteralPath $plain) {
        if ([string]::IsNullOrWhiteSpace($line) -or $line.StartsWith('#')) { continue }
        if ($line -notmatch '^([^=]+)=(.*)$') { Remove-Item -LiteralPath $temp -Recurse -Force; throw 'Invalid secrets format.' }
        $values[$Matches[1]] = $Matches[2]
    }
    foreach ($key in $env:DOTFILES_SECRET_KEYS.Split(',')) {
        if ($key -and (-not $values.ContainsKey($key) -or [string]::IsNullOrEmpty($values[$key]))) { Remove-Item -LiteralPath $temp -Recurse -Force; throw 'Required secret is missing.' }
    }
    $previous = @{}
    foreach ($key in $values.Keys) {
        $previous[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
        [Environment]::SetEnvironmentVariable($key, $values[$key], 'Process')
    }
    [pscustomobject]@{ Temp = $temp; Keys = @($values.Keys); Previous = $previous }
}

function Remove-DotfilesSecrets($state) {
    if (-not $state) { return }
    foreach ($key in $state.Keys) { [Environment]::SetEnvironmentVariable($key, $state.Previous[$key], 'Process') }
    if ($state.Temp) { Remove-Item -LiteralPath $state.Temp -Recurse -Force -ErrorAction SilentlyContinue }
}
