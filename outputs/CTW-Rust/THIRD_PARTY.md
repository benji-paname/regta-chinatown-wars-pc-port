# Provenance et licences

`src/ped_paths.rs` et `src/pavement.rs` : description du réseau piéton et logique de parcours (`cWanderPath`, `GetRandomNode`, `ResolveBridgeNode`, `cFleeOnFoot`, `AddPathingNodes`) adaptées de `port/src/world/collision.cpp/.h`, `port/game/peds.cpp` et `port/game/localarea.cpp` de ctw-native, Copyright (C) 2026 Kryo.to, GPL-3.0-only. Enveloppe PSP, coordonnées et raccords vérifiés directement sur l’ISO de l’utilisateur ; aucune donnée Android ajoutée au runtime.

## Pack HD fourni localement le 24 octobre 2026

`CW-Texture-Pack-master`, fourni par Benjamin. Son README est conservé dans `data/texture-pack/PACK-README.md` ; les images préparées restent hors des sources versionnées. Les crédits et droits des images restent ceux du pack et de leurs ayants droit. Voir `TEXTURES-HD.md` pour les correspondances et exclusions. Le runtime Rust utilise uniquement les fichiers préparés, sans intégrer PPSSPP.

Les outils locaux d'import reproduisent les empreintes QuickTexHash de PPSSPP (Copyright PPSSPP Project, GPL-2.0-or-later), d'après `GPU/Common/TextureDecoder.cpp`, et vérifient les clés de palette décrites dans `TextureCacheCommon.cpp/.h`. Cette recherche et les scripts d'audit restent dans `work/`. Le rendu du radar utilise les constantes mesurées dans le binaire PSP de l'utilisateur et `SS_Radar.bin` ; voir `RADAR-PSP.md`.

## ctw-native â€” GPL-3.0-only

Copyright (C) 2026 Kryo.to.

Projet : https://github.com/official-kryo-to/ctw-native

RÃ©vision consultÃ©e : `5fa1e36672ee88b0b232c3dc23652be0a33c7f17`.

Adaptations Rust :

- `src/wad.rs` : algorithme de hash de `port/tools/wad.py` et lecture des chaÃ®nes dÃ©crite/implÃ©mentÃ©e dans `port/src/text/gxt.cpp`.
- `src/model.rs` : organisation des hiÃ©rarchies, composition des transformations et principe de triangulation des strips, d'aprÃ¨s `port/src/gfx/model.cpp` et `port/src/world/worldmap.cpp`. Le format binaire PSP est spÃ©cifique et a Ã©tÃ© Ã©tudiÃ© localement.

- `src/world.rs` : repÃ¨re et dÃ©coupage du monde, dâ€™aprÃ¨s `port/src/world/worldmap.cpp/.h` ; structure PSP Ã©tablie indÃ©pendamment.
- `src/collision.rs` : structure et principes de requÃªtes dâ€™aprÃ¨s `port/src/world/collision.cpp/.h`, adaptÃ©s au rÃ©pertoire PSP de 8 octets.
- `src/camera.rs` : adaptation de `port/game/followcam.cpp/.h` et principes de `cameraobstruction.cpp/.h` ; ressorts et angles de la rÃ©fÃ©rence, contacts simplifiÃ©s. La camÃ©ra libre est un outil additionnel.

- `src/ped.rs` : progression des animations et construction des couches dâ€™aprÃ¨s `port/src/gfx/pedsprites.cpp/.h` et `port/game/player.cpp`. Le format PSP Ã  552 animations / couches 8 octets / UV 9 bits a Ã©tÃ© mesurÃ© indÃ©pendamment.
- `src/collision.rs`, contrÃ´leur Walker : paliers, compteurs, virages, intÃ©gration et gravitÃ© adaptÃ©s de `port/game/player.cpp/.h`. Les contacts ne sont pas une traduction complÃ¨te de ConstrainByCollision.

Les lecteurs Android de PAK, PVRTC/DXT et modÃ¨les MG ne sont pas utilisÃ©s pour dÃ©coder les donnÃ©es PSP. Aucune table ou ressource Android n'est embarquÃ©e.

Le texte intÃ©gral de la licence GPL-3.0 est dans `LICENSE`.

## Projet VCS-Rust local

`src/iso.rs` dÃ©rive du lecteur ISO9660 Ã©crit pour le projet VCS-Rust de Benjamin. Des contrÃ´les supplÃ©mentaires ont Ã©tÃ© ajoutÃ©s Ã  cette copie. Le dÃ©marrage du contexte OpenGL/Windows dans `src/gpu.rs` est Ã©galement adaptÃ© de ce projet. Le projet VCS dâ€™origine nâ€™a pas Ã©tÃ© modifiÃ©.

## DÃ©pendances Cargo

Les licences des dÃ©pendances restent applicables : notamment minifb (MIT/Apache-2.0), flate2 (MIT/Apache-2.0), serde et serde_json (MIT/Apache-2.0), sha2 (MIT/Apache-2.0), ainsi que leurs dÃ©pendances transitives. Cargo.lock fixe les versions ; leurs notices sont distribuÃ©es avec leurs sources Cargo.

## Ressources GTA

Les droits sur le jeu, ses donnÃ©es et ses marques restent Ã  leurs ayants droit. L'ISO et les ressources extraites restent hors du code source du projet. Les captures locales sont uniquement des contrÃ´les de lecture/rendu.

## Franchissement, voiture et recherche PSP

`src/vault.rs` : reconstruction expérimentale propre au port, pas issue d’une fonction PSP identifiée. `src/vehicle.rs` : formules de dimensions adaptées de ctw-native `port/game/vehicle.cpp`, Copyright (C)2026 Kryo.to, GPL-3.0-only. Modèle et textures chargés depuis l’ISO PSP à l’exécution.

Recherche locale : [John-K/pspdecrypt1.0](https://github.com/John-K/pspdecrypt/releases/tag/1.0), outil GPL-3.0 utilisé séparément pour déchiffrer EBOOT.BIN. Il reste dans `work/psp-analysis/tools`, n’est ni lié au moteur ni inclus dans CTW-Rust.exe. Capstone5.0.9 sert au désassemblage local ; pas de dépendance du jeu. Voir PSP-FIDELITE.md pour les empreintes et les limites des conclusions.
