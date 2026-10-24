# Référence ctw-native — audit et parité complète PSP du 24 octobre 2026

Mise à jour après la validation complète : les 6 points de parité demandés par Benjamin d'après sa vidéo PSP (`PPSSPP 2026.10.24 - 06.00.37.01.mp4`) et les sources de `upstream-ctw` (`work/upstream-ctw/port/game/`) ont été entièrement implémentés et testés :
1. **Physique frein à main, dérapage contrôlé et traces de pneus (`traces de pneus`)** : `Espace` / touche `X` verrouille l'essieu arrière, réduit l'adhérence latérale arrière, permet un survirage puissant avec accélérateur maintenu, et dépose des rubans de traces de pneus OpenGL (`GL_TRIANGLE_STRIP`, `alpha = 12/31 ≈ 0.39`, maintien 15s puis fondu 4s).
2. **Circulation dans les voies de droite (`dans leurs voies`) & courbes de Bézier** : offset de voie `ai.bin` (+3,0m à droite du centre sur les voies bidirectionnelles, +4,5m sur autoroutes) et raccordement quadratique aux carrefours (`AISpline::fixUpTuplet`).
3. **Arrêt des véhicules IA devant les obstacles** : détection du joueur, des piétons vivants, des autres véhicules et des feux tricolores avec décélération jusqu'à l'arrêt complet (`speed = 0.0`) et délai de redémarrage `start_delay = 0.35s`.
4. **Budget et makeups de véhicules (`popinfo.bin`)** : 13 listes de véhicules terrestres et 8 makeups de zones (Midtown, Chinatown, Bohan, Docks, etc.) avec 5% de patrouilles de police. Les véhicules spéciaux (`Rhino`, `F1`, `Bulldozer`, `Gokart`, `Hearse`, `Bus`) n'apparaissent plus dans la circulation ambiante.
5. **PNJ sur les motos du trafic & animation assise** : toutes les motos non garées embarquent un pilote PNJ visible. Animations assises (`0x5C/0x5D`, `0xA6/0xA7`, `0xA4/0xA5`) à hauteurs de couches différenciées (`upper` à `1.0/0.75` et `legs` à `0.5/0.2998`) transformées en 3D selon l'angle et l'inclinaison de la moto.
6. **Esquive des piétons réaliste sans panique sur le trottoir** : suppression de la panique de proximité (les piétons sur le trottoir ne paniquent plus lorsque le joueur roule sur la route). Impulsion d'esquive compacte (`Roll::with_speed(heading, 2, 5)`) produisant un déplacement réaliste de ~2,4m qui dégage le gabarit du véhicule sans traverser toute l'avenue.

**100 tests unitaires passent avec 0 warning et 0 échec** (`Compiler.ps1 -Test`). Prochaine étape : objets dynamiques du monde (`world.bin` section 6 : 15 040 placements) et contacts/débris (`rigidbody.cpp` / `vehicledebris.cpp`). Les sections ci-dessous décrivent l’audit initial.

## Origine du port fourni

Le dossier `C:/Users/BENJAMIN/Desktop/GTA Chinatown Wars PC` utilise les données Android 4.4.243 : ses fichiers `data/game.pak`, `data/rom.wad` et `data/rom.toc` sont identiques octet pour octet aux trois fichiers de l’APK fourni par Benjamin. Les empreintes SHA-256 et les chemins sont conservés dans `data/ctw-native-reference/audit.json`.

`CTW-Native-Porter.exe` prépare l’installation depuis l’APK. Le programme de jeu est `GTACTW.exe`. Dans les sources de référence, l’installateur emploie Rust/Tauri, mais le moteur de jeu est en C++ avec SDL2/OpenGL. La révision exacte du binaire téléchargé n’a pas été identifiée ; les observations sur le code concernent la copie locale `work/upstream-ctw`, pas une correspondance de révision prouvée avec ce binaire.

Le mod Enhancements fourni est activé avec `psp_lighting=1` et `third_person.enabled=0`. L’apparence PSP peut donc venir de cet éclairage, sans changer l’origine Android des données.

## Travail utile à adapter

| Système | Référence dans `work/upstream-ctw` | Application à notre port PSP/Rust |
| --- | --- | --- |
| Passants | `port/src/world/collision.cpp`, `port/game/peds.cpp` | Lire le réseau piéton PSP, puis remplacer progressivement les déplacements guidés par les couleurs de la minimap. |
| Voitures et circulation | `port/game/vehicle.cpp`, `traffic.cpp`, `aispline.cpp`, `roads.cpp`, `cargens.cpp` | Étendre le modèle Admiral et la conduite simplifiée ; lire les placements et caractéristiques PSP. |
| Objets et contacts | `port/game/rigidbody.cpp`, `vehicledebris.cpp` | Étudier les contacts, dégâts et débris ; vérifier les tables PSP avant adaptation. |
| Combat | `port/game/weapons.cpp`, ciblage et projectiles | Réutiliser l’organisation des armes, munitions et projectiles ; éviter d’importer les constantes Android comme preuves PSP. |
| Son et ambiance | `port/game/sound.cpp`, radio, météo | Préparer les lecteurs et les événements audio, puis vérifier les ressources PSP correspondantes. |
| Fluidité | `port/game/interpolation.cpp` | Étudier l’interpolation d’affichage autour de la simulation à fréquence fixe. |

Ces systèmes sont plus développés dans la référence que dans notre prototype. Cela ne prouve ni leur parité intégrale avec l’original, ni leur présence exacte dans l’exécutable téléchargé. Les missions, sauvegardes, poursuites et plusieurs interfaces/comportements restent incomplets dans la référence inspectée.

Nos trois profils caméra, le radar PSP, le GPS et le chargement prioritaire du pack HD restent des acquis à préserver. Le HUD mobile ne fournit pas les positions et dimensions PSP recherchées.

## Réutilisation concrète : réseau piéton retrouvé dans l’ISO PSP

Le lecteur d’audit `work/audit-downloaded-port.py` adapte la description du réseau de ctw-native à l’enveloppe PSP et lit directement `world.bin` depuis l’ISO originale. Il contrôle les tailles décompressées, les bornes des nœuds/liens et les correspondances réciproques entre secteurs.

Résultat vérifié :

- 2 504 cellules contenant un réseau ;
- 13 530 nœuds et 19 784 entrées de liaison internes ;
- 9 654 raccords intersectoriels, tous retrouvés réciproquement, aucun non résolu ;
- aucun indice ou raccord hors limites ;
- 50 nœuds à moins de 100 unités du point de départ de la démo ;
- 505 enregistrements de placement de voitures et 15 040 enregistrements d’objets, dont la taille a été vérifiée. Leurs champs et leur comportement restent à interpréter complètement.

### Format PSP constaté

`world.bin` contient un répertoire de 14 000 cellules de 8 octets chacune, indexées `x*100+y`, puis 12 sections par cellule. Le bit supérieur de l’offset indique la compression zlib. Dans le second mot, les 16 bits bas donnent la taille stockée et les 16 bits hauts l’excédent décompressé. Le lecteur Android utilise une autre enveloppe : ne pas le copier tel quel.

La section 3 contient le réseau. Son en-tête fait 44 octets ; l’offset de la table des nœuds est à +32, les nombres de liens/nœuds à +36/+38, le nombre de raccords à +42. Les liens sont des indices u16 à +44. Un nœud de 10 octets contient `u16 premier_lien, u16 flags, i16 x, i16 y, i8 z, u8 cover`. Les positions sont divisées par 8 pour X/Y et par 2 pour Z ; le nombre de liens vaut `(flags >> 2) & 7`.

Les huit emplacements de raccords de l’en-tête contiennent chacun un nœud u16 et un secteur u16. Les 14 bits bas du secteur emploient `y*140+x`, différent de l’index du répertoire. La validation recherche le raccord inverse et la même position X/Y dans le secteur voisin. Les sections 6 et 7 contiennent respectivement les enregistrements d’objets et de voitures, avec un compteur u32 et des entrées de 20 octets.

Ce résultat valide la lecture et la cohérence du graphe. Il ne valide pas encore le comportement des passants en jeu, les traversées de rue, les vitesses, les animations ou le choix des itinéraires.

## État livré et ordre de reprise

Cette étape produit l’audit et une lecture exploitable des données PSP. **Aucun changement du runtime Rust ou de `CTW-Rust.exe` n’a été fait pendant cet audit.** Les exécutables téléchargés n’ont pas été lancés et le dossier fourni n’a pas été modifié.

Priorité de reprise : intégrer le réseau PSP aux passants, puis les placements de voitures et la conduite/circulation ; poursuivre ensuite combat, sons et ambiance. Chaque adaptation devra être testée dans le runtime et vérifiée contre les données PSP avant d’annoncer une fidélité à l’original.

Les sources de ctw-native sont sous GPL-3.0-only, Copyright (C) 2026 Kryo.to. Conserver cette provenance et les crédits lors des adaptations Rust, en complément de `THIRD_PARTY.md`. Les tables extraites de l’APK ne doivent pas devenir silencieusement des paramètres PSP.
