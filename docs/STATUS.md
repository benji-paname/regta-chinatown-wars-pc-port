# Implementation status and evidence limits

Date: 2026-10-24. Release milestone: 0.1.4-alpha. Package/build metadata still reports 0.1.2-alpha; this documentation does not silently change the build version.

The goal is native PSP behaviour in Rust, with no emulated CPU in the runtime. The current implementation contains both PSP-verified resource interpretation and experimental gameplay. It is not a complete automated static translation of the MIPS program, and no full-game equivalence proof exists.

## Resource and gameplay status

| Modules | Evidence and current limits |
| --- | --- |
| `iso`, `psp`, `wad`, `pak`, `texture`, `model`, `world` | Read the user's PSP disc, resources and world geometry; local format validation exists. |
| `collision`, `ped_paths`, `pavement`, `props` | PSP world sections decoded, including 13,530 pavement nodes and 9,654 reciprocal sector bridges. Behaviour still needs scene-by-scene validation. |
| `ped`, `roll`, `vault`, `camera` | PSP animations and binary/video investigations inform the implementation. Camera profiles and locomotion exist; obstacle handling and all camera modes are not proven fully equivalent. |
| `radar`, `gps`, `hud`, `locale`, `gamepad` | Original radar resources/measurements, route guidance, FR/EN and XInput support. Large-map interface and route algorithm are not a complete PSP UI/algorithm port. |
| `vehicle`, `actors`, `police`, `weapons`, `explosions` | Playable prototype systems and reference-derived algorithms. Several behaviours remain reconstructed; tests alone do not establish original PSP timing/AI. |
| `audio`, `radio`, `weather` | Native infrastructure and managers exist. Locally loaded lookup tables have Android provenance; complete PSP sound/weather parity is outstanding. |
| `stats` | Statistics and serialization-related structures with tests. A full compatible game-save and campaign system is not delivered. |
| `gpu`, `shaders`, `ui`, `play` | Native Windows rendering/application loop. Stylized effects and prototype flows remain. |

## Local tables: provenance correction

All ten files previously in `outputs/CTW-Rust/data/tables/` were SHA-256 compared with the supplied Android PC installation: gameplay, population, radio, render, restart, shard, sound, wanted, weapon and weather tables were identical. They are now organized under root `data/tables/`, remain private and are excluded from Git. Their original bytes are preserved.

The runtime currently optionally loads four of these reference tables: weapons, radio, weather and sound. Their presence supports experimental systems; it does **not** establish PSP provenance. A public checkout supplies none of them. Replacing these dependencies with verified PSP reads is separate gameplay/reverse-engineering work, not something a directory reorganization can certify.

This corrects older statements that called those files “authentic PSP” or said no Android table was used. Historical reports are retained for traceability, not as current evidence.

## What this release validates

- A compilable monorepo with zero emitted compiler warnings.
- 160 passing unit tests independent of proprietary files.
- Optional private table integration checks, original-ISO graph checks and a short native rendering smoke test, recorded separately.
- Source-only Git tracking and preservation of private research.

No full playthrough, physical-controller regression, complete sound audit or 1:1 PSP gameplay certification is implied.

## Presentation update — 24 October 2026

English-first website and README, optional French website text, donation links and GitHub Pages publishing instructions prepared locally. Logos now use actual Pricedown Black and Bruce Mikita outlines; Bruce Mikita public-use terms still need documentation. See [branding provenance](BRANDING.md). Logo raster previews were inspected. Browser visual verification has not been completed because local preview access was blocked.
