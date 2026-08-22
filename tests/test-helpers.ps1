Set-StrictMode -Version Latest

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw "Assertion failed: $Message" }
}

function Assert-Contains([string]$Value, [string]$Expected) {
    Assert-True ($Value.Contains($Expected)) "expected output to contain: $Expected"
}

function Assert-NotContains([string]$Value, [string]$Unexpected) {
    Assert-True (-not $Value.Contains($Unexpected)) "expected output not to contain: $Unexpected"
}

function Assert-FileExists([string]$Path) {
    Assert-True (Test-Path -LiteralPath $Path) "expected file to exist: $Path"
}

function Assert-FileNotExists([string]$Path) {
    Assert-True (-not (Test-Path -LiteralPath $Path)) "expected file not to exist: $Path"
}
