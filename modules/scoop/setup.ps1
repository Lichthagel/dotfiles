$ErrorActionPreference = 'Stop'

if (-not (Get-Command scoop -ErrorAction SilentlyContinue)) {
    Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
    Invoke-RestMethod -Uri https://get.scoop.sh | Invoke-Expression
}

$scoopShims = Join-Path $HOME 'scoop\shims'
$env:Path = "$scoopShims;$env:Path"
if (-not (Get-Command scoop -ErrorAction SilentlyContinue)) {
    throw 'Scoop was installed but is not available on PATH.'
}
