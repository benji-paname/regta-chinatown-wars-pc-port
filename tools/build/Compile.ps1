param([switch]$Test, [string]$ToolchainRoot = $env:CTW_RUST_TOOLCHAIN, [switch]$Offline)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$delivery = Join-Path $repoRoot 'outputs/CTW-Rust'
$reportDir = Join-Path $repoRoot 'target/validation'
$savedEnvironment = @{}
foreach ($name in @('PATH', 'CARGO_HOME', 'CARGO_TARGET_DIR', 'RUSTFLAGS')) {
    $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}
try {
    if (-not $ToolchainRoot -and -not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        # Compatibility with the existing local portable toolchain; not required by public builds.
        $localTools = Join-Path (Split-Path (Split-Path $repoRoot -Parent) -Parent) '2026-10-05/en/work'
        $candidate = Join-Path $localTools 'rust-portable'
        if (Test-Path -LiteralPath (Join-Path $candidate 'bin/cargo.exe')) { $ToolchainRoot = $candidate }
    }
    if ($ToolchainRoot) {
        $toolRoot = (Resolve-Path -LiteralPath $ToolchainRoot).Path
        $env:PATH = "$toolRoot\bin;$toolRoot\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained;" + $env:PATH
        $portableCargoHome = Join-Path (Split-Path $toolRoot -Parent) 'cargo-home'
        if (Test-Path -LiteralPath $portableCargoHome) {
            $env:CARGO_HOME = $portableCargoHome
            $Offline = $true
        }
    }
    $cargo = (Get-Command cargo -ErrorAction Stop).Source
    $env:CARGO_TARGET_DIR = Join-Path $repoRoot 'target'
    $env:RUSTFLAGS = ($env:RUSTFLAGS + ' -D warnings').Trim()
    New-Item -ItemType Directory -Path $reportDir -Force | Out-Null
    $common = @('--release', '--locked', '--workspace', '--manifest-path', (Join-Path $repoRoot 'Cargo.toml'))
    if ($Offline) { $common += '--offline' }
    $logs = @()
    $testResult = $null
    $steps = @('build')
    if ($Test) { $steps += 'test' }
    foreach ($step in $steps) {
        $lines = @(& $cargo $step @common 2>&1)
        $code = $LASTEXITCODE
        $lines | ForEach-Object { Write-Host $_ }
        $text = ($lines | ForEach-Object { "$_" }) -join "`n"
        [IO.File]::WriteAllText((Join-Path $reportDir "$step.log"), $text, [Text.UTF8Encoding]::new($false))
        $logs += $text
        if ($code -ne 0) { throw "Cargo $step failed ($code). Previous delivered executable retained." }
        if ($step -eq 'test') {
            $match = [regex]::Match($text, 'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;.*?(\d+) filtered out;')
            if (-not $match.Success) { throw 'Missing successful test summary.' }
            $testResult = @{ passed=[int]$match.Groups[1].Value; failed=[int]$match.Groups[2].Value; ignored=[int]$match.Groups[3].Value; filtered=[int]$match.Groups[4].Value }
            if ($testResult.failed -or $testResult.ignored -or $testResult.filtered) { throw 'Incomplete test run.' }
        }
    }
    if (($logs -join "`n") -match '(?m)^warning(?:\[|:)') { throw 'Compiler warning detected.' }
    Copy-Item -LiteralPath (Join-Path $env:CARGO_TARGET_DIR 'release/ctw-rust.exe') -Destination (Join-Path $delivery 'CTW-Rust.exe') -Force
    $report = [ordered]@{
        timestamp=(Get-Date -Format 'o'); version='0.1.2-alpha'; warnings=0; warnings_as_errors=$true
        tests=$testResult; executable_sha256=(Get-FileHash -LiteralPath (Join-Path $delivery 'CTW-Rust.exe') -Algorithm SHA256).Hash.ToLower()
        cargo=(& $cargo --version); rustc=(& rustc --version)
    }
    $json = $report | ConvertTo-Json -Depth 5
    [IO.File]::WriteAllText((Join-Path $reportDir 'build-report.json'), $json, [Text.UTF8Encoding]::new($false))
    Write-Host 'CTW-Recomp compiled and delivered to outputs/CTW-Rust/CTW-Rust.exe (0 warnings).'
} finally {
    foreach ($name in $savedEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name], 'Process')
    }
}
