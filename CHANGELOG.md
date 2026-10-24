# Changelog

Changes are recorded by release. The project is in the alpha phase; a version number does not imply complete PSP behaviour.

## [Unreleased]

- Continue PSP provenance verification and replacement of experimental behaviour.
- Continue campaign, scripting, audio and save integration.

## [0.1.2-alpha] — 2026-10-24

### Changed

- Initialized a source-only local Git repository with explicit asset/build exclusions.
- Centralized Rust sources under `src/` and introduced a root Cargo workspace.
- Preserved the existing compiler, executable name and demo launcher paths.
- Consolidated compilation into `target/`; compiler warnings now fail the build.
- Made all 119 unit tests runnable without proprietary tables; retained optional private reference checks.
- Resolved optional local tables relative to the executable, independent of the working directory.
- Added `CTW_ISO` as an optional ISO-path environment variable; `--iso` takes precedence.
- Organized technical documentation, archived historical notes and indexed preserved research scripts.
- Added module status, provenance, release procedure and Tipeee/PayPal.Me support links.

### Maintenance

- Removed only the three named regenerable Cargo caches and BMP captures with verified pixel-identical PNG copies. Exact local results are recorded in the restructuring report.
- Preserved research scripts, C++ reference sources, original game files and nonidentical captures.

### Validation

- 119 unit tests passed; 0 failures, 0 ignored tests, 0 compiler warnings.
- This release reorganizes and validates the project; it does not certify full PSP parity.
- Local release preparation only; no remote repository or public release was published by this operation.

## Earlier development

Earlier work predates the local Git baseline. Historical notes are retained under `docs/archive/` and `docs/reference/`; no dates or release tags have been invented for that period.
