# PAUSED — resume checkpoint (2026-10-24 19:55 Europe/Paris)

User explicitly requested a pause. Do not resume development until asked.

## Latest user decisions (override older presentation plans)
- Website default and primary language: ENGLISH. French may remain optional.
- GitHub README / public repository presentation: ENGLISH.
- Logo: GTA must use the official cover typography; Chinatown Wars and CTW must use the corresponding cover lettering. Existing hand-drawn SVG paths are NOT verified authentic fonts despite their comments. Verify font sources and licensing before claiming an exact match. Do not use an image generator to approximate the lettering.

## Exact stopping point
- HEAD before this checkpoint: a25ce02 on main; no Git remote configured.
- Only new website scaffolding exists: docs/assets/site.css, site.js, favicon.svg, and docs/.nojekyll. These are not yet connected to docs/index.html.
- Existing docs/index.html, README.md, logo.svg and logo-ctw.svg have NOT been revised in this session.
- site.js currently defaults to French: change this to English when resuming, alongside English HTML fallback content. No code change made after the pause request.
- Last activity was research into authentic logo fonts (Pricedown and possible Chinese Rocks). Search output was truncated; no font identification, license check or download has been confirmed. Check Windows font folders and Python fontTools availability again as needed.
- No Rust/gameplay source modified. Mandatory Compiler.ps1 -Test completed successfully during this session: 152 passed, 0 failed, 0 ignored, 0 compiler warnings. Evidence: target/site-baseline-tests.log and target/validation/build-report.json. Tests were not rerun for this documentation-only checkpoint.
- README and version metadata need consistency review: Cargo/build report still say 0.1.2-alpha while release documentation says 0.1.4-alpha. Root handoff previously contained stale 131-test information; use this checkpoint first.

## Resume in ten seconds
From repository root:
```powershell
Get-Content .\outputs\CTW-Rust\AI_HANDOFF_GUIDE.md -TotalCount 65
```
Then inspect git status and the four scaffolding files. Finish typography research, then English-first website and README. Preserve prominent pre-alpha / no playable download notice, visible "by Benji Paname", Tipeee https://fr.tipeee.com/benji-paname/ and PayPal https://paypal.me/BenjiLsg. Avoid unsupported claims of guaranteed 60 FPS, full fidelity or complete missions. 166 indexed overlays do not mean 166 implemented missions.

## Pending deliverables
1. Authentic-font-based main and compact SVG logos, with provenance/licensing verified; update or retire old generators tools/research/generate_authentic_logo.py and generate_compact_logo.py so they do not overwrite the result.
2. Complete docs/index.html using responsive scaffold, English first with optional French, accessible navigation and honest progress/donation copy.
3. English README and publishing instructions for public empty repo regta-ctw. User requested exactly two concise instruction lines for github.com/new and first git push; username is not yet known. GitHub Pages should use main /docs. Do not invent an account URL or publish without authorization.
4. Static checks and visual review where permitted; commit coherent changes and update handoff. Gameplay (tattoo Overlay 142 and streetrace Overlay 140) only after presentation stage validation; no new gameplay work started.

## Preview limitation / tooling state
- Local browser preview failed. Explicit browser policy denied file:// navigation; do NOT bypass via alternate browser, data URLs, CDP or headless browser. Local network permission request was not granted. User was told visual browser verification is blocked.
- A Python docs preview server was started on 127.0.0.1:8765 (exec session 22174); it may still exist. Do not terminate unrelated Python processes. It is not needed to resume file editing.
- Browser CUA bindings may be stale; read/rewrite its documentation before any further permitted browser work.
- Python paths: C:/Python314/python.exe and C:/Users/BENJAMIN/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe. PIL and reportlab available; fontTools not confirmed. No successful visual logo render this session.

---
Previous project handoff below is historical and may contain unverified fidelity claims or stale commit counts.
# ReGTA CTW — guide de reprise

Nom complet : **ReGTA CTW PSP Recomp**. Version **0.1.4-alpha**.
Mise à jour : **2026-10-24T19:20+02:00**, Europe/Paris. Restructuration terminée, validée et enrichie des références officielles PSP.

## Reprendre en dix secondes

Depuis la racine du dépôt :

```powershell
powershell -ExecutionPolicy Bypass -File .\Compiler.ps1 -Test
```

L'entrée `outputs/CTW-Rust/Compiler.ps1 -Test` reste compatible. Les sources actives sont uniquement dans **src/**. Le workspace et Cargo.lock sont à la racine ; le package **regta-ctw** conserve son manifeste dans `outputs/CTW-Rust/Cargo.toml`.

## Règle Obligatoire Permanente (Directive de Benjamin)

> Avant CHAQUE tâche de développement ou d'ingénierie inverse, exécuter un contrôle de synchronisation avec le dépôt voisin `references/chinatown-wars-psp-recomp` pour vérifier et intégrer ses avancées récentes :
> ```powershell
> git -C references/chinatown-wars-psp-recomp fetch --all
> git -C references/chinatown-wars-psp-recomp status -uno
> ```

## État exact

- Dernière compilation : **152 tests réussis, 0 échec, 0 ignoré, 0 avertissement**, avec `-D warnings`.
- Git local initialisé : 15 commits sur `main`. Dernier commit `5ead17b` : implémentation de `contactsapp` (Overlay 014) et `carbombfittingapp` (Overlay 010).
- Exécutable : `outputs/CTW-Rust/CTW-Rust.exe`.
- Indexation automatique au lancement : les 166 code overlays PSP sont détectés et chargés directement depuis `OVERLAYS.PAK` dans l'ISO UMD originale en 1.18 s.
- Nouveaux modules intégrés :
  - `src/pda.rs` : Carnet d'adresses `contactsapp` avec routage GPS, boutique Ammu-Nation, cours du marché, messagerie email.
  - `src/minigames.rs` : Piégeage de véhicule `carbombfittingapp`, pare-brise sous-marin `sinkingcarapp`, démarrage aux fils `hotwiringapp`, loterie `scratchcardapp`, coffre-fort `safecrackingapp`.
  - `src/jobs.rs` : Missions secondaires complètes (Taxi avec passagers/destinations/streaks/médaillon nitro, Autodéfense/Vigilante en 5 vagues avec gilet pare-balles doublé, Pompiers avec lance à eau et immunité au feu, Ambulance avec transport de blessés et sprint infini, Livraison de nouilles à scooter).
  - `src/collectibles.rs` : 100 Caméras de sécurité LCPD destructibles (sauvegarde `package[100]` + remises dealers), 30 Sauts uniques USJ (`unique_jump[30]`), 35 Carnages/Rampages avec médailles, 2 Lions de Fo, bennes à ordures interactives.
  - `src/audio.rs` : Synthèse sonore 8-bit PCM pour caisse enregistreuse, notification PDA, bris de vitre, dispatch police, défibrillateur et fanfare de victoire.
- Cache actif unique : `target/`. Rapports courants : `target/validation/build-report.json`.
- Nouvelles références intégrées dans `references/` :
  - `references/chinatown-wars-psp-recomp` (Static recompilation officielle PSP ULUS-10490 v1.01 par highcmd avec extraction des 167 code overlays de `OVERLAYS.PAK`).
  - `references/psp-re` (PSP Reverse Engineering HQ, NIDs, scripts Ghidra Allegrex).

## Ressources externes vérifiées

- **ISO PSP Europe originale** : `F:\JEUX EMULATION\PSP GAME\Grand Theft Auto - Chinatown Wars (Europe) (En,Fr,De,Es,It).iso` (accessible et vérifiée).
- **ISO locale de travail** : `work/Chinatown-Wars-PSP.iso` (883 Mo, SHA-256 `8c62ef8d...`).
- **Émulateur PPSSPP officiel** : `C:\Users\BENJAMIN\Desktop\Grand Theft Auto_ Chinatown Wars PSP OG.lnk` -> `C:\Users\BENJAMIN\Documents\EMULATION\EMULATEUR\PSP\PPSSPP\PPSSPPWindows64.exe`.
- **Port PC Android local** : `C:\Users\BENJAMIN\Desktop\GTA Chinatown Wars PC\GTACTW.exe`.

## Liens de soutien & financement

- Tipeee : https://fr.tipeee.com/benji-paname/
- PayPal : https://paypal.me/BenjiLsg
