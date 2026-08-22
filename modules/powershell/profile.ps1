# User PowerShell additions belong here.
if (-not $env:EDITOR) { $env:EDITOR = "notepad.exe" }

$profileDirectory = Join-Path (Split-Path -Parent $PROFILE) 'Profile.d'
if (Test-Path -LiteralPath $profileDirectory) {
    Get-ChildItem -LiteralPath $profileDirectory -Filter '*.ps1' |
        Sort-Object Name |
        ForEach-Object { . $_.FullName }
}
