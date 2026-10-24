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
