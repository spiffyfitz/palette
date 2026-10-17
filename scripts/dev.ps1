param(
    [string]$Action = 'run',
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$CargoArguments
)
$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
$localAssembler = Join-Path $taskRoot 'target/build-tools/nasm-3.01'
if (-not (Get-Command nasm -ErrorAction SilentlyContinue)) {
    if (Test-Path -LiteralPath (Join-Path $localAssembler 'nasm.exe')) {
        $env:PATH = $localAssembler + [IO.Path]::PathSeparator + $env:PATH
    } else {
        throw 'AVIF builds on x86/x64 require NASM on PATH. See docs/BUILD.md.'
    }
}
Push-Location -LiteralPath $taskRoot
try {
    & cargo $Action @CargoArguments
    $taskExitCode = $LASTEXITCODE
} finally {
    Pop-Location
}
exit $taskExitCode
