# Restructuring validation — ReGTA CTW

Completed: **2026-10-24T11:01+02:00**, Europe/Paris. Version: **0.1.2-alpha**.

## Build and tests

- `outputs/CTW-Rust/Compiler.ps1 -Test`: **119 passed, 0 failed, 0 ignored, 0 compiler warnings** after the final rename.
- A fresh export of the staged Git sources, without ISO or private tables, also passed **119 tests** with `-D warnings`.
- Optional integration checks against the preserved private tables passed **119 tests** before the branding-only change.
- Toolchain: rustc 1.90.0 (1159e78c4 2025-09-14); cargo 1.90.0 (840b83a10 2025-07-30).
- Delivered executable SHA-256: `40709cbefedbafc9efd6d007a264fba5167cd0f99a774560fa84aff40c8a648f`.
- Local native smoke: **60 frames**, 20 pedestrians, 12 cars, optional HD pack active, launched outside the game directory using `CTW_ISO`.
- Original PSP graph: **13,530 nodes**, **9,654 reciprocal bridges** verified.

The source-only check used a Git archive. A local `git clone` attempt encountered a Windows/MSYS namespace permission error; no network fetch or bypass was needed. No remote repository/release was published.

## Storage and preservation

- Three explicitly authorized old Cargo caches removed: **305,049,282 bytes**.
- **37 BMPs** removed only after exact decoded RGBA pixel equality and SHA-256 rechecks against their surviving PNGs: **119,809,998 bytes**.
- Total removed: **424,859,280 bytes** (about 425 MB / 405 MiB), before accounting for the newly built single `target/` cache.
- `work/roof-before.bmp` and `work/roof-after.bmp` differ from their PNGs and were retained. Unpaired captures were retained.
- **1198 retained work files** matched their pre-restructuring hashes, including all inventoried Python/C++ research. Only authorized redundant BMP removals were excluded from that preservation count.
- **10 local tables** moved to `data/tables/` with identical bytes. They match the supplied Android reference installation and remain ignored.

All research originals remain in `work/`. The research index covers **161 Python files** including upstream/analysis subdirectories; no research script was rewritten or deleted. Historical scripts that patch old source paths are explicitly documented as archival tools.

## Source layout and Git

Active implementation: `src/`. Root Cargo workspace and lockfile; package `regta-ctw` retains the legacy manifest and executable name for launcher compatibility. Tools, docs, private-data placeholders and references have dedicated directories.

The tracked-file policy check rejects game assets, private research, generated binaries/caches, unexpected extensions and binary payloads. The baseline and subsequent changes are saved as local commits. Existing dead-code lint exceptions are documented; no new blanket warning suppression was introduced.

## Limits

This validates the restructuring, build, fixtures and short startup checks. It does not certify full PSP gameplay equivalence, physical-controller behaviour or a complete campaign. Optional Android-derived table dependencies and reconstructed behaviours remain identified in [STATUS.md](STATUS.md).

Public project name: **ReGTA CTW**, full name **ReGTA CTW PSP Recomp**. Tipeee and PayPal.Me support links were supplied by the owner and added to the README and GitHub funding configuration; their pages were not accessible to the browsing tool, so destination/account ownership was not independently verified.

Detailed local logs and manifests: `work/restructure/`; current build diagnostics: `target/validation/`. These private/generated reports are not committed.
