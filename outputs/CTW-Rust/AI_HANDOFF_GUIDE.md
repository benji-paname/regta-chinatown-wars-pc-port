# ReGTA CTW — presentation checkpoint

Updated: 2026-10-24 22:36 Europe/Paris. User resumed work after the earlier pause.

## Completed this session

- English-first `docs/index.html` and README. Optional French website switch, saved locally; English fallback works without JavaScript/storage.
- Responsive CSS, accessible skip link/focus, visible by Benji Paname, prominent pre-alpha/no playable download notice, Tipeee and PayPal links. Honest distinction between prototype systems and verified PSP parity.
- Actual font-outline logos replace the hand-drawn drafts: user supplied Downloads/pricedown.zip and Downloads/bruce-mikita.zip. Pricedown Black 5.200 + Bruce Mikita Regular v1.0. Main 900x480; compact 900x260. Generator: tools/research/generate_brand_logos.py. Old generators disabled from overwriting public assets.
- Font files remain private in target/private-fonts; local fontTools 4.67.0 in target/fonttoolslib. No fonts committed. Pricedown fixed artwork terms checked; Bruce archive contains no explicit license. Public-use permission for that historical version is NOT established: see docs/BRANDING.md before publishing.
- docs/PUBLISHING.md gives two-line GitHub setup for empty public regta-ctw and Pages main /docs. No remote/account guessed, no remote configured, no push/deployment done.
- No Rust source or gameplay changed. Package/build metadata still says 0.1.2-alpha while release milestone is 0.1.4-alpha; discrepancy explicitly documented, not silently changed.

## Verification

- Mandatory `powershell -ExecutionPolicy Bypass -File .\Compiler.ps1 -Test`: 152 passed, 0 failed, 0 ignored, 0 compiler warnings. Latest log: target/presentation-tests.log.
- HTML static checks: English root, 69 bilingual elements, existing local references, unique anchors, valid SVG XML.
- JavaScript syntax and VM behaviour checks passed: English default, stored French, invalid/blocked storage, switching, exact GitHub Pages repo links.
- Both logos rasterized with Sharp and visually inspected: target/logo-preview.png and target/logo-ctw-preview.png. These are actual vector renders, not browser screenshots.
- Full website browser visual verification is still unavailable. Earlier file:// browser navigation was explicitly denied; do not circumvent through other browser surfaces, data URLs or headless browsers. Local preview network access was unavailable. No browser layout certification is claimed.

## Next steps

1. Show/obtain user review of presentation and logos. Resolve Bruce Mikita public-use terms before public distribution. The site is still local.
2. User creates GitHub repo; exact two-line instructions in docs/PUBLISHING.md. If resuming publication, inspect remote first and avoid copying private data/font files.
3. Once presentation stage is validated, investigate precision tattooing (overlay 142) and street races (overlay 140) from original evidence; no implementation started and no arbitrary gameplay values authorized.
4. Review package/release metadata separately when preparing the next actual release.

Resume from repository root:
```powershell
Get-Content .\outputs\CTW-Rust\AI_HANDOFF_GUIDE.md -TotalCount 45
git status --short
```

Historical project handoff follows; its old commit counts and fidelity claims may be stale.
---

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
