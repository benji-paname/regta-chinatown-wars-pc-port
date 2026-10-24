# Build and local validation

## Standard Windows build

Install a Rust Windows x64 toolchain and its linker. Run from the repository root:

```powershell
.\Compiler.ps1 -Test
```

The script builds release mode with `--locked --workspace`, applies `-D warnings`, runs every unit test and copies the successful binary to `outputs/CTW-Rust/CTW-Rust.exe`. It restores its environment changes before returning. Build/test logs and the executable SHA-256 are saved under `target/validation/`. A failure preserves the previously delivered executable.

The equivalent compatibility command remains:

```powershell
.\outputs\CTW-Rust\Compiler.ps1 -Test
```

The source tree is `src/`. Do not recreate a second source tree under `outputs/CTW-Rust/src/`. The root lockfile is authoritative. `cargo test --locked --workspace` also works with a configured toolchain; Cargo-only builds do not perform the delivery step.

## Portable or offline builds

```powershell
.\Compiler.ps1 -Test -ToolchainRoot "D:\Tools\rust-portable" -Offline
```

`CTW_RUST_TOOLCHAIN` can supply the same toolchain directory. When a sibling `cargo-home` exists beside that portable installation, the script uses it offline. The existing local VCS portable installation is detected only as a compatibility fallback when Cargo is absent from PATH. Public checkouts do not depend on it. `-Offline` requires previously downloaded dependencies.

One active build cache is used: `<repository>/target/`. Historical scripts under `work/` may still name retired caches; they are research records, not current build entry points.

## ISO and demo

Supported input: **PSP ULES01347 1.01**. No game files are downloaded by the build.

```powershell
.\outputs\CTW-Rust\CTW-Rust.exe --iso "D:\Games\Chinatown-Wars-PSP.iso" --slice
.\outputs\CTW-Rust\CTW-Rust.exe --iso "D:\Games\Chinatown-Wars-PSP.iso" --pavement-audit --output "target\validation\pavement"
```

Alternatively set `$env:CTW_ISO` to the ISO path. Explicit `--iso` overrides it. The existing private `work/Chinatown-Wars-PSP.iso` fallback remains for the original installation. Save settings and optional HD textures remain beside the delivered executable in `outputs/CTW-Rust/data/`.

## Tests without game assets

The 119 unit tests generate their own synthetic fixtures. They exercise parsers and implemented behaviour; those fixtures are never used as runtime game data. Tests no longer silently pass solely because a private file is missing.

Additional checks of the owner's existing tables can be enabled explicitly:

```powershell
$env:CTW_TEST_REFERENCE_TABLES = (Resolve-Path .\data\tables).Path
try { .\Compiler.ps1 -Test } finally { Remove-Item Env:CTW_TEST_REFERENCE_TABLES }
```

This directory must contain the sound, weapon, radio and weather tables being checked. Missing requested files fail the checks. These local tables currently match the Android reference installation; this integration check is **not** a PSP fidelity certificate.

`-D warnings` enforces zero emitted compiler warnings. Existing targeted/module-level `allow(dead_code)` attributes remain from the baseline; this restructuring does not claim that all currently unused prototype code has been eliminated.
