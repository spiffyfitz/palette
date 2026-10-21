param(
    [string]$Action = 'run',
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$CargoArguments
)
$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $taskRoot
try {
    & cargo $Action @CargoArguments
    $taskExitCode = $LASTEXITCODE
} finally {
    Pop-Location
}
exit $taskExitCode
