param([switch]$Test, [string]$ToolchainRoot = $env:CTW_RUST_TOOLCHAIN, [switch]$Offline)
$ErrorActionPreference = 'Stop'
& (Join-Path $PSScriptRoot 'tools/build/Compile.ps1') @PSBoundParameters
