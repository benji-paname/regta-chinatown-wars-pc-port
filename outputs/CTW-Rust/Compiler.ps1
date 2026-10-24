param([switch]$Test)
$ErrorActionPreference = 'Stop'
$projectPath = $PSScriptRoot
$chatPath = Split-Path (Split-Path $projectPath -Parent) -Parent
$vcsTools = 'C:\Users\BENJAMIN\Documents\Codex\2026-10-05\en\work'
$portablePath = Join-Path $vcsTools 'rust-portable'
$env:CARGO_TARGET_DIR = Join-Path $chatPath 'work\ctw-build'
if (Test-Path -LiteralPath (Join-Path $portablePath 'bin\cargo.exe')) {
    $env:PATH = "$portablePath\bin;$portablePath\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained;" + $env:PATH
    $env:CARGO_HOME = Join-Path $vcsTools 'cargo-home'
    $cargoExecutable = Join-Path $portablePath 'bin\cargo.exe'
    $offline = @('--offline')
} else {
    $cargoExecutable = 'cargo'
    $offline = @()
}
& $cargoExecutable build --release --locked @offline --manifest-path (Join-Path $projectPath 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw 'La compilation a echoue.' }
if ($Test) {
    & $cargoExecutable test --release --locked @offline --manifest-path (Join-Path $projectPath 'Cargo.toml')
    if ($LASTEXITCODE -ne 0) { throw 'Un test a echoue.' }
}
Copy-Item -LiteralPath (Join-Path $env:CARGO_TARGET_DIR 'release\ctw-rust.exe') -Destination (Join-Path $projectPath 'CTW-Rust.exe') -Force
Write-Host 'CTW-Rust.exe compile. Exploration de la carte PSP, deplacement au sol et camera de suivi.'
