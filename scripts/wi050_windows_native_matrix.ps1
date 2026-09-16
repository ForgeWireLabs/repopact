[CmdletBinding()]
param(
    [string]$RepoRoot = (Split-Path -Parent $PSScriptRoot),
    [string]$Python = 'C:\Program Files\Python312\python.exe'
)

$ErrorActionPreference = 'Stop'

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator
)
if (-not $isAdmin) {
    throw 'Open PowerShell with Run as administrator.'
}

$resolvedRepo = (Resolve-Path -LiteralPath $RepoRoot).Path
if (-not (Test-Path -LiteralPath $Python -PathType Leaf)) {
    throw "Required Python interpreter is missing: $Python"
}

$key = Join-Path $env:TEMP "wi050-windows-test-operator-$PID.key"
$secretBytes = [byte[]]::new(32)
[Security.Cryptography.RandomNumberGenerator]::Fill($secretBytes)
$passphrase = [Convert]::ToBase64String($secretBytes)
$matrixExit = 1

$env:WI050_KEY_PATH = $key
$env:WI050_KEY_PASS = $passphrase
$env:WI050_KEY_PASSPHRASE = $passphrase

try {
    & $Python -c "import os; from pathlib import Path; from repopact.admission import Ed25519Signer; Ed25519Signer.generate(key_id='wi050-windows-test', operator_id='wi050-windows-test-operator').save(Path(os.environ['WI050_KEY_PATH']), os.environ['WI050_KEY_PASS'])"
    if ($LASTEXITCODE -ne 0) {
        throw "Temporary key generation failed with exit code $LASTEXITCODE"
    }

    $matrixArgs = @(
        (Join-Path $PSScriptRoot 'wi050_windows_native_matrix.py')
        '--native-destructive'
        '--source-root'
        $resolvedRepo
        '--key-file'
        $key
        '--delete-key'
    )
    & $Python @matrixArgs
    $matrixExit = $LASTEXITCODE
}
finally {
    Remove-Item Env:WI050_KEY_PATH -ErrorAction SilentlyContinue
    Remove-Item Env:WI050_KEY_PASS -ErrorAction SilentlyContinue
    Remove-Item Env:WI050_KEY_PASSPHRASE -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $key -Force -ErrorAction SilentlyContinue
    $passphrase = $null
    $secretBytes = $null
}

if ($matrixExit -ne 0) {
    throw "Windows native matrix failed with exit code $matrixExit"
}
