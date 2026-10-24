# Changelog

Changes are recorded by release. The project is in the alpha phase; a version number does not imply complete PSP behaviour.

## [Unreleased]

- Interactive PDA Email market tip push alerts and Ammu-Nation web store ordering.
- Minigames execution via loaded PSP ELF overlays (sinkingcarapp, hotwiringapp, safecrackingapp).

## [0.1.3-alpha] — 2026-10-24

### Added

- **PSP Code Overlays Engine (`src/overlays.rs`)**: Automatic detection, indexing, and decompression of all 166 code overlays stored inside `OVERLAYS.PAK` (20KP / 10SA container format) into PSP memory Slot 0 (0x08b87460, 114 mission scripts) and Slot 1 (0x08bb2480, 52 minigames & PDA apps).
- **Story Missions Engine (`src/missions.rs`)**: State machine orchestrating story missions across the 8 boss contacts (Uncle Kenny, Chan Jaoming, Zhou Ming, Hsin Jaoming, Wade Heston, Lester Leroc, Rudy D'Avanzo, Xin Shan). Waypoint guidance, mission briefing banners, objectives tracking, and cash rewards.
- **Drug Dealing Economy (`src/drugs.rs`)**: Authentic 6 commodities (Acid, Coke, Downers, Ecstasy, Heroin, Weed), gang supply/demand wholesale dynamics (7 gangs), 81 street dealers tracked in `save_progress.dealer`, player 100-capacity bag, and police confiscation on arrest.
- **Safehouses & Real Estate (`src/safehouse.rs`)**: 21 purchasable properties across Liberty City, Save Bed (advances clock 6h, restores 127 health, commits savegame count), Stash Box (unlimited cash & drug storage immune to police), and Wardrobe.
- **Gameplay HUD & Radar Integration (`src/gpu.rs`, `src/actors.rs`, `src/play.rs`)**:
  - Live HUD cash counter in authentic bright green (`$00000500`).
  - Contextual interaction prompt bar (`[ E / Y ] MISSION : Wu 'Kenny' Lee`, `[ E / Y ] DEALER...`, `[ E / Y ] PLANQUE...`).
  - Announcement banners for mission pass/fail, safehouse saved, and dealer discovery.
  - Minimap radar and full city map markers: golden contact diamonds ('K', 'C', 'Z', etc.), green safehouses, cyan drug dealers, and flashing mission target waypoints.

### Validation

- 132/132 unit tests passing, 0 warnings with `-D warnings`.
- Native runtime loads all 166 code overlays from UMD ISO in 1.18 seconds.

## [0.1.2-alpha] — 2026-10-24

### Changed

- Renamed the project **ReGTA CTW**, full name **ReGTA CTW PSP Recomp**; retained executable/launcher paths for compatibility.
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
