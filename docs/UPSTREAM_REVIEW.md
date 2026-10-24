# Upstream Repositories Review — 25 October 2026

## 1. Upstream Mirrors & Roles

| Upstream Mirror | Origin / Author | Platform / Target | Current Revision | Role & Provenance Policy |
|---|---|---|---|---|
| `references/ctw-native` | [`official-kryo-to/ctw-native`](https://github.com/official-kryo-to/ctw-native.git) | Android 4.4.243 / C++ | `b0506abc` (2026-10-22) | **Game systems reference**: Class names, state machines, math formulas, dealer/commodity tables. Useful for understanding decompiled game logic, but Android-specific adaptations must be checked against PSP Europe (`ULES-01347`). |
| `references/chinatown-wars-psp-recomp` | [`highcmd/chinatown-wars-psp-recomp`](https://github.com/highcmd/chinatown-wars-psp-recomp.git) | PSP USA `ULUS-10490` v1.01 / C++ + PSP HLE | `6562163b` (2026-10-24) | **PSP executable & MIPS recompilation reference**: Overlay structure (`OVERLAYS.PAK`), MIPS register conventions, PSP memory layout, and HLE wrapper patterns. |
| **ReGTA CTW** *(Our Target)* | [`benji-paname/regta-chinatown-wars-pc-port`](https://github.com/benji-paname/regta-chinatown-wars-pc-port) | PSP Europe `ULES-01347` v1.01 / **Native Rust (Windows x64)** | Local workspace | **Clean-room Native Rust PC Port**: Independent implementation with zero emulated CPU at runtime, reading original game assets from legal PSP discs. |

---

## 2. Recent Upstream Advances

### A. `ctw-native` (Android / C++)
- `b0506ab` wip
- `a31e94b` Release Porter 0.3.0
- `48f20b5` Port the death app: the shattering screen, the Wasted jingle and WASTED! text
- `9642011` Add sirens: the toggle in siren vehicles and their looping sound
- `17e51d9` Hold thrown peds' sprites until they land; add the landing thud and pain comments
- `ed73363` Get in by the nearer door: port the player's seat plans

**Analysis & Applicability to ReGTA CTW**:
- The pedestrian throwing physics and landing sound, vehicle siren sound loop, and nearest-door seat determination in `ctw-native` provide valuable structural logic for our pedestrian AI (`src/actors.rs`) and vehicle interaction (`src/vehicle.rs`).
- However, `ctw-native`'s percentage metrics reflect decompiled symbols from Android binaries. They do not translate directly to PSP Europe MIPS function offsets.

### B. `chinatown-wars-psp-recomp` (highcmd PSP USA)
- `6562163` Launcher 0.1.5 (rebuilds the game: HUD size on F3)

**Analysis & Applicability to ReGTA CTW**:
- highcmd continues work on launcher and HUD scaling for the USA release (`ULUS-10490`).
- ReGTA CTW already supports dynamic multi-resolution HUD rendering, camera profiles (PSP / Mobile / DS), and responsive scaling directly in native Rust OpenGL (`src/gpu.rs`).

---

## 3. Provenance & Coverage Policy

- **No borrowed coverage percentages**: The progress percentage seen in Android decompilation charts measures Android ELF symbol coverage, not PSP MIPS completion.
- ReGTA CTW does not display an unverified percentage until an exact build-specific inventory of European PSP routines (`docs/progress/psp-routines.json`) is audited.
- All progress is tracked through **verifiable, testable counts** in our codebase (unit tests, overlays, subsystems, and game features).
