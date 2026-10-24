# ReGTA CTW PC Recomp — media and naming checkpoint

Updated: 2026-10-25, Europe/Paris. Current media work was published to GitHub and GitHub Pages in commit 4dbbc6c. No YouTube upload was performed.


## Obsolete-media cleanup — 25 October 2026

Owner explicitly requested removal of old logo/video versions and their superseded production work. Deleted 71 files (263,822,158 bytes; exact inventory in ignored target/obsolete-media-cleanup.json): old devlog MP4, old thumbnail, video/temp, old review images/previews and obsolete logo/video/README generators. Current four logo variants, aliases, raw custom_gameplay.mp4, launch-film outputs/work files, fonts and current generators were kept. SHA-256 comparison confirmed 109 protected files unchanged. No gameplay or research source removed. Git history was not rewritten.

## Latest user decisions
- Start the promotional film from scratch; new film uses Benjamin's original native capture, not the old finished montage.
- Public name is **ReGTA CTW PC Recomp**, not PSP Recomp. Technical PSP original-data references remain PSP.
- Two logo layouts (horizontal + vertical), each on black and transparent backgrounds.

## Completed
- Flat cover-inspired red/yellow CTW, white integrated ReGTA with lowercase e at 78% scale. Pricedown Black + Bruce Mikita Two. Four named variants in docs/assets and outputs/CTW-Rust/branding, SVG + 2x PNG; default aliases remain transparent. Board: outputs/CTW-Rust/branding/logo-variants-preview.png.
- New English film: outputs/CTW-Rust/video/launch-film/ReGTA_CTW_First_Look_EN.mp4. 1920x1080 / 60 fps, approximately 78.55 seconds container duration. 13 native footage selects, new synthetic narration and original score, concise headings, nine-second donation end card. All footage is from custom_gameplay.mp4, supplied by Benjamin. No new gameplay capture or invented footage was needed.
- New thumbnail, English SRT, narration, YouTube description and exact timeline in the same launch-film folder. Source: tools/video/build_launch_film.py + launch_film.json. Deleted the obsolete generators and old movie at the owner's request. The current generator has no emulator-footage fallback.
- Updated website, README, active docs, build output, package description, runtime window/PDA/help branding to PC Recomp. Archived historical documents and upstream repository names remain unchanged. No gameplay mechanics modified.
- docs/PROMO_REVIEW.md records the review; docs/YOUTUBE_CREATOR_KIT.md is the current upload kit. Bruce Mikita publication terms remain undocumented (pre-existing issue in BRANDING.md); no font files redistributed.

## Verification
- Compiler.ps1 -Test after runtime string changes: 160 passed, 0 failed, 0 warnings. Log target/promo-final-validation.log.
- Complete final MP4 decode successful; final video contact sheet visually checked. Audio peak -3.6 dBFS, no sample clipping observed; automated audio checks are not a full listening review.
- All four logo variants visually inspected, PNG alpha/background checked and SVG XML parsed. New video/thumbnail/end-card graphics inspected.
- Website browser preview was not performed. Existing browser restrictions remain; no headless browser workaround used. Logos are rasterized with Sharp, video with FFmpeg/Pillow.

## Resume
```powershell
Get-Content .\outputs\CTW-Rust\AI_HANDOFF_GUIDE.md -TotalCount 45
git status --short
```
Review the new film and logos with Benjamin. Do not publish automatically. Subsequent gameplay priorities remain in the historical handoff below, but its fidelity claims are not certified by this media work.

---
## Previous gameplay handoff (historical)
# ReGTA CTW — presentation, street races, and autonomous progress

Updated: 2026-10-24 23:50 Europe/Paris. Multi-AI autonomous handoff guide.

## Completed this session

1. **All 16 Street Races & Time Trials Engine (`src/races.rs`, Overlay 140 `streetrace`)**:
   - 16 authentic circuits across Broker, Dukes, Bohan, and Algonquin:
     - 0: Transporterror (Broker - BOABO)
     - 1: Midtown Mayhem (Dukes - Meadow Hills)
     - 2: South Boulevard Boogie (Broker - South Slopes)
     - 3: Beach Blitz (Broker - Firefly Island)
     - 4: Boulevard Boogie (Bohan - Boulevard)
     - 5: Industrial Drift (Bohan - Industrial)
     - 6: Bridge Burner (Dukes - East Island City)
     - 7: Alleyway Dash (Bohan - Chase Point)
     - 8: Star Junction GP (Algonquin - Star Junction)
     - 9: Algonquin Expressway (Algonquin - Middle Park East)
     - 10: Castle Gardens Sprint (Algonquin - Castle Gardens)
     - 11: Meat Quarter Dash (Algonquin - Meat Quarter)
     - 12: Suffolk Circuit (Algonquin - Suffolk)
     - 13: Heliport Rush (Algonquin - Lower Easton)
     - 14: Times Square Shakedown (Algonquin - Triangle)
     - 15: Liberty City Grand Prix (Endurance cross-borough championship)
   - Dynamic checkpoint detection (14m radius), 3-second starting grid countdown, real-time lap times, and personal records.
   - 3 dynamic AI rival racers (`STREETRACER1` to `STREETRACER5`, `STREETRACER1 MOD`) with authentic rubber-banding pace adjustment.
   - Live position tracking (1st through 4th), medal tiers (Gold, Silver, Bronze), cash prizes, and persistent record logging in `SaveProgress::race_best_time[16]` (IDs `0x4C` to `0x5B`).
   - Anti-cheat/forfeit timer (10s if vehicle is abandoned mid-race).
   - HUD banner and radar golden diamond waypoint indicators.

2. **Tattoo Parlor Needle Tracing Minigame (`src/minigames.rs`, Overlay 141 `tattooingapp`)**:
   - 4 Triad & gang motifs: Dragon of Broker ($1200), Koi of Dukes ($800), Tiger of Bohan ($950), Viper of Algonquin ($1500).
   - Stencil needle tracing, ink reservoir replenishment, client pain meter, and flinch/failure handling.

3. **YouTube Creator Production Kit (`docs/YOUTUBE_CREATOR_KIT.md`)**:
   - Comprehensive video kit for Benjamin's first devlog/promotion video:
     - 5 high-CTR titles, thumbnail design with PNG logos.
     - Minute-by-minute script (Hook, Rust recomp vs emulator, 60fps/4k demo, live gameplay, drug wars / tattooing / street racing, roadmap, call to action with Tipeee and PayPal.Me).
     - Copy-paste description with timestamps, links, legal disclaimer, tags.
     - OBS Studio 60 FPS recording guide with fullscreen launcher commands.

4. **Official Project Branding, PNG Assets & Trilingual Showcase**:
   - 4 vector SVGs and 4 high-resolution PNGs delivered:
     - `docs/assets/logo.png` & `logo-transparent.png` (Vertical Stacked, 1800 × 1000)
     - `docs/assets/logo-ctw.png` & `logo-ctw-transparent.png` (Horizontal Inline, 1920 × 520)
   - GitHub Pages showcase deployed with trilingual EN / FR / ES switcher and persistence.
   - Repository renamed to `regta-chinatown-wars-pc-port`.

## Verification

- **Mandatory compiler check**: `powershell -ExecutionPolicy Bypass -File .\Compiler.ps1 -Test`
  - **160 passed, 0 failed, 0 ignored, 0 compiler warnings (`-D warnings`)**.
  - Binary delivered to `outputs/CTW-Rust/CTW-Rust.exe`.
- **Upstream check**: `references/chinatown-wars-psp-recomp` branch `master` clean and up to date.

## Next Roadmap Priorities (Autonomous Continuation)

1. **YouTube Promotion Video Recording**:
   - Benjamin records and publishes the first video using `docs/YOUTUBE_CREATOR_KIT.md`.
2. **Story Missions Execution & Cutscenes Engine (`src/missions.rs`)**:
   - Implement cutscene camera staging, scripted boss dialogue, and mission chains across the 8 boss contacts.
3. **Vehicle Radio Audio Stations Synthesis**:
   - Integrate radio audio playback loops for all 11 Chinatown Wars in-car radio stations.

Resume command:
```powershell
powershell -ExecutionPolicy Bypass -File .\Compiler.ps1 -Test
```
