# Changelog

Changes are recorded by release. The project is in the alpha phase; a version number does not imply complete PSP behaviour.

## [Unreleased]

- Tattoo parlor precision needle minigame (`tattoominigame`, Overlay 142).
- Street races circuit checkpoint racing (`streetrace`, Overlay 140).

## [0.1.4-alpha] — 2026-10-24

### Added

- **Refonte officielle de l'identité visuelle et des logos ReGTA CTW (`docs/assets/logo.svg`, `docs/assets/logo-ctw.svg`, `docs/BRANDING.md`)**:
  - Typographie Pricedown Black authentique pour `ReGTA` : le "Re" en grand intégré avec "GTA", avec le 'e' en minuscule. Coloris deux tons : "Re" en dégradé rouge Recomp (`#FF3B30` → `#E52521` → `#7F1D1D`) et "GTA" en blanc/platine pur GTA (`#FFFFFF` → `#CBD5E1`) sous contour noir massif et ombre portée.
  - Typographie Bruce Mikita pour `CTW` avec les coloris exacts du jeu original Chinatown Wars : corps rouge laqué impérial (`#F82B2B` → `#DC1414` → `#7A0808`), liseré doré jaune vif (`#FFF875` → `#FFD700` → `#D97706`), et rehauts calligraphiques intérieurs.
  - Deux versions officielles vectorielles : Version 1 horizontale (`logo-ctw.svg`, 960x260) pour les bannières et barres de navigation, et Version 2 verticale empilée (`logo.svg`, 900x480) avec le CTW positionné proprement sous le ReGTA pour la vitrine principale.
  - Validation visuelle directe par rendu headless Microsoft Edge.
- **Interactive PDA Contacts and Address Book (`src/pda.rs`, `contactsapp` Overlay 014)**:
  - Intégration du carnet de contacts des 8 boss d'histoire (Kenny, Chan, Zhou, Hsin, Heston, Lester, Rudy, Xin Shan) avec leurs districts, affiliations de gang et numéros de téléphone.
  - Sélection interactive et routage GPS automatique vers le point de rencontre du contact.
- **Minijeu de Piégeage de Véhicule (`src/minigames.rs`, `carbombfittingapp` Overlay 010)**:
  - Minijeu d'installation et de câblage d'un engin explosif sous le capot d'une voiture (câble d'alimentation, masse et détonateur) avec amorçage pour explosion au démarrage.
- **Odd Jobs and Emergency Services Engine (`src/jobs.rs`)**:
  - `taxi` (Overlay 143): Chauffeur de taxi avec détection du véhicule Cabbie, passagers sur trottoir, destinations dynamiques, tarifs calculés selon distance et pourboire dégressif, streaks et déblocage du médaillon de taxi (boost nitro).
  - `vigilante` (Overlay 152): Autodéfense via l'ordinateur de police en patrouille (5 vagues, élimination de criminels en fuite, récompense gilet pare-balles doublé 200 AP).
  - `firefighter` (Overlay 028): Interventions d'urgence en camion de pompiers, extinction des brasiers à la lance à eau, déblocage de l'immunité totale au feu (Huang ignifugé).
  - `paramedicapp` (Overlay 094): Secours en ambulance vers les urgences de Schottler, transport de blessés et déblocage du sprint infini.
  - `chinesedelivery` (Overlay 013): Tournées de livraisons express de nouilles à scooter avec minuterie et pourboires.
- **Collectibles and World Challenges Engine (`src/collectibles.rs`)**:
  - 100 Caméras de sécurité LCPD réparties sur Liberty City, destructibles par balles et explosifs, synchronisées avec `save_progress.package[100]`, accordant jusqu'à 20% de remise chez les dealers.
  - 30 Sauts uniques USJ (`save_progress.unique_jump[30]`) avec détection de rampe, phase aérienne et validation d'atterrissage sur roues.
  - 35 Carnages / Rampages (`rampage`, Overlay 116) avec munitions infinies et médailles de bronze/argent/or dans `save_progress.ambient_medal`.
  - 2 Statues sacrées des Lions de Fo (`monumentapp`, Overlay 091).
  - Bennes à ordures rouges (armes cachées) et vertes (nourriture, cash et bric-à-brac) fouillables.
- **Synthétiseur et Déclencheurs Audio Dédiés (`src/audio.rs`)**:
  - Bruitage de caisse enregistreuse et transaction (`play_cash_register`).
  - Carillon de notification email et alerte cours PDA (`play_pda_notification`).
  - Craquement et bris de pare-brise sous-marin (`play_glass_break`).
  - Gazouillis radio de dispatch de police (`play_police_dispatch`).
  - Choc électrique de défibrillateur (`play_defibrillator`).
  - Fanfare triomphale de mission et job réussi (`play_mission_fanfare`).
- **HUD & Radar Live Tracking (`src/gpu.rs`)**:
  - Balises radar contextuelles pour les jobs actifs (passagers 'P', suspects 'V', incendies 'F', ambulances '+', livraisons 'D').

### Validation

- **152/152 unit tests passing**, 0 failed, 0 warnings under `-D warnings`.
- Binaire compilé et actualisé sous `outputs/CTW-Rust/CTW-Rust.exe`.

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
