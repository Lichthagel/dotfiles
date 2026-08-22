$scoopShims = Join-Path $HOME 'scoop\shims'
if (Test-Path -LiteralPath $scoopShims) {
    $env:Path = "$scoopShims;$env:Path"
}
