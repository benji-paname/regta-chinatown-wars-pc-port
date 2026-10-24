# ReGTA CTW — guide de reprise

Nom complet : **ReGTA CTW PSP Recomp**. Version **0.1.2-alpha**.
Mise à jour : **2026-10-24T11:01+02:00**, Europe/Paris. Restructuration terminée et vérifiée.

## Reprendre en dix secondes

Depuis la racine du dépôt :

```powershell
.\Compiler.ps1 -Test
```

L'entrée `outputs/CTW-Rust/Compiler.ps1 -Test` reste compatible. Les sources actives sont uniquement dans **src/**. Le workspace et Cargo.lock sont à la racine ; le package **regta-ctw** conserve son manifeste dans `outputs/CTW-Rust/Cargo.toml`. Ne pas recréer une seconde copie des sources à l'ancien emplacement.

## État exact

- Dernière compilation : **119 tests réussis, 0 échec, 0 ignoré, 0 avertissement**, après renommage, avec `-D warnings`.
- Export Git sans assets : **119/119** également. Tests synthétiques générés en mémoire, jamais utilisés comme données de gameplay.
- Contrôles privés des tables : **119/119** avant le changement de nom ; provenance Android explicite.
- Exécutable : `outputs/CTW-Rust/CTW-Rust.exe`, SHA-256 `40709cbefedbafc9efd6d007a264fba5167cd0f99a774560fa84aff40c8a648f`.
- Cache actif unique : `target/`. Rapports courants : `target/validation/build-report.json` et `work/restructure/final-report.json`.
- Smoke natif final : 60 images, pack HD actif, 20 passants, 12 voitures, lancement depuis un autre dossier. Réseau PSP vérifié : 13 530 nœuds, 9 654 raccords.

## Travail terminé

- Git local initialisé, baseline `7d3d173`, migration `31ae193`, tests/documentation `0a567f3` ; consulter `git log -4 --oneline` pour le commit final.
- Arborescence `src/`, `tools/`, `docs/`, `data/tables/`, `references/`, cache unique et lanceurs compatibles.
- README public, CHANGELOG, statut honnête des modules, provenance/licence et procédure de release.
- Soutien : https://fr.tipeee.com/benji-paname/ (principal), https://paypal.me/BenjiLsg. Aucun envoi/publication distante.
- Suppression vérifiée des trois anciens caches et de 37 BMP redondants : 424 859 280 octets au total. Les BMP non identiques ou sans PNG sont conservés.
- 1198 fichiers de recherche conservés et vérifiés par empreinte ; les dix tables déplacées sont identiques. Python/C++ et recherches intacts, index dans `tools/research/README.md`.

## Contraintes et prochaine étape utile

L'objectif reste un portage natif PSP en Rust sans émulation. **La fidélité intégrale n'est pas démontrée.** Lire `docs/STATUS.md` : les dix tables locales sont Android, et quatre restent optionnellement utilisées par les systèmes expérimentaux. Ne pas les présenter comme PSP. La prochaine étape de rétro-ingénierie utile est d'identifier leurs équivalents PSP et de remplacer ces dépendances avec des preuves, sans inventer de comportements. Ne pas lancer ce travail au titre de la restructuration déjà terminée.

Les fichiers propriétaires et les recherches restent hors de Git. Ne pas déplacer/modifier les sources C++ ni les scripts historiques pour suivre les nouveaux chemins sans une tâche dédiée. Conserver les trois caméras, le HUD, les commandes et le pack HD local.

ISO utilisée pour les vérifications : `work/Chinatown-Wars-PSP.iso`, SHA-256 `8c62ef8d7263ff0d42db40901927de4e86afd02f3d3450c3a96d0e2bc8d5168e`. Elle correspond à la source PSP précédemment inspectée. Le lecteur accepte `--iso` ou `CTW_ISO`. L'ISO indiquée par l'utilisateur sur F: n'était pas accessible dans cette session.

Avant toute interruption : terminer l'écriture des fichiers, mettre à jour ce guide avec heure et résultats réels, puis committer un état compilable. Ne pas déclarer un test exécuté s'il ne l'a pas été. Les notes dans `docs/archive/` sont historiques et peuvent contenir des affirmations corrigées depuis.

Voir [rapport de restructuration](../../docs/RESTRUCTURING.md) et [compilation](../../docs/BUILDING.md).
