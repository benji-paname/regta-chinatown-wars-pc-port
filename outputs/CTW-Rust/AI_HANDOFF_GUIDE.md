# CTW-Recomp — guide de reprise

Mise a jour : 2026-10-24 10:53 +02:00. Version : 0.1.2-alpha.

## Reprise en dix secondes

Depuis la racine du depot :

```powershell
.\Compiler.ps1 -Test
```

L'entree outputs/CTW-Rust/Compiler.ps1 -Test reste compatible. Les sources actives sont uniquement dans src/. Le manifeste de package conserve son ancien chemin dans le workspace Cargo racine. Ne pas recreer outputs/CTW-Rust/src/.

## Etat verifie

- Baseline : 119 tests passes, 0 echec, aucun avertissement affiche.
- Apres deplacement et adaptation : 119 tests passes sans tables privees ; 119 tests passes avec controles prives explicites.
- Compilation avec -D warnings, aucun nouveau masquage de lint. Les allow(dead_code) historiques sont conserves et documentes.
- Executable livre : outputs/CTW-Rust/CTW-Rust.exe ; empreinte et horodatage dans target/validation/build-report.json.
- Commits deja crees : 7d3d173 (baseline), 31ae193 (sources/workspace/compiler).

## Fait et reste a faire a ce jalon

Sources regroupees, cache unique target/, tests synthetiques independants des assets, chemins des tables independants du dossier courant, documentation publique/financement rediges. Tables privees deplacees dans data/tables/ ; les dix fichiers sont identiques a la reference Android, pas une preuve PSP. Les originaux Python/C++ et recherches restent dans work/.

Nettoyage seulement prepare : 3 caches nommes + 37 BMP verifies pixel par pixel, 424859280 octets au total. Deux BMP roof-before/roof-after different de leur PNG et doivent rester. Prochaines actions : appliquer le nettoyage verifie, controler la preservation, tester un checkout source seul et lancer un smoke test ISO. Ne pas annoncer ces controles comme deja passes.

## Regles de reprise

- Objectif : portage natif PSP en Rust, sans emulation. La parite complete n'est PAS encore etablie. Lire docs/STATUS.md avant toute affirmation.
- Aucun asset proprietaire dans Git. work/, target/, les tables et outputs/CTW-Rust/data/ sont ignores.
- ISO locale de secours : work/Chinatown-Wars-PSP.iso. ISO utilisateur indiquee : F:\JEUX EMULATION\PSP GAME\Grand Theft Auto - Chinatown Wars (Europe) (En,Fr,De,Es,It).iso ; acces F: indisponible dans cette session. Ne pas toucher aux originaux.
- Preserver les cameras PSP/mobile/DS, le HUD, les controles et le pack HD local.
- Support indique par Benjamin : https://fr.tipeee.com/benji-paname/ et https://paypal.me/BenjiLsg. Aucun site/release distant publie.
- Avant interruption : ecrire un etat exact et horodate ici, puis committer un etat compilable. Ne pas gonfler les resultats de tests ou la fidelite PSP.

Les anciennes notes sont conservees dans docs/archive/. Elles ne remplacent pas ce guide ni les preuves du code actuel.