# CTW-Rust — Chinatown Wars PSP vers PC

Objectif : un port PC en Rust de **GTA Chinatown Wars PSP**, à partir de l'ISO de Benjamin. Le projet VCS-Rust sert de précédent local ; [ctw-native](https://github.com/official-kryo-to/ctw-native) sert de référence technique.

État au **24 octobre 2026** : **vertical slice jouable de présentation**. Ville PSP, Huang, passants, voitures pilotables, conduite, combat simple, GPS et shaders. Le radar, les jauges de vie et d'armure utilisent les mesures retrouvées dans le programme PSP. **Le pack HD fourni par Benjamin est prioritaire**, avec les polices améliorées sélectionnées sans doublons. La signature **by Benji Paname** est bien visible. Interface **français / anglais**, choix mémorisé, commandes PC et Xbox affichées ensemble dans les bandes facultatives. Voir [RADAR-PSP.md](RADAR-PSP.md) et [TEXTURES-HD.md](TEXTURES-HD.md).

## Lancer et présenter

**Presenter-la-demo.cmd** ouvre la scène depuis le trottoir. **Present-demo-English.cmd** la lance en anglais. **CTW-Rust.exe** démarre aussi directement la scène complète. Lire **VERTICAL-SLICE.md** pour une présentation en deux minutes, les commandes et les limites.

| Action | PC | Xbox |
|---|---|---|
| Marcher / courir | ZQSD / WASD / flèches | Stick gauche |
| Sprinter | Maj | A |
| Roulade / franchissement | Espace | X |
| Frapper | J | B |
| Monter / descendre à l'arrêt | E | Y |
| Accélérer / freiner-reculer | Z/W / S | RT / LT |
| Braquer / frein à main | Q/A, D / Espace | Stick / X |
| GPS | M | Back |
| Bandes d’informations | F5 | RB + Back |
| Français / anglais | F4 | LB + clic RS |
| Rotation radar | G | Croix droite |
| Caméra PSP / Mobile / DS | F3 | Clic RS |
| Caméra libre | F2 | Croix haut |
| Recentrer | C maintenu | LB maintenu |
| Pause | P | Start |
| Retour au départ | R | LB + Y |
| Collisions | F1 | Croix bas |
| Capture | F12 | Croix gauche |
| Quitter hors carte | Échap | Start + B |

La copie locale de l'ISO se trouve dans `../../work/Chinatown-Wars-PSP.iso`, relativement à l'exécutable. Seule l'édition **ULES01347 1.01** est acceptée. Aucun émulateur ni APK n'est lancé. Les sources DS et mobile ont servi à étudier les caméras.

## ATH, ville et limites

La minicarte emploie les **vraies tuiles PSP**, leur échelle et leur transparence. Vie à gauche, armure à droite, plage 0–127 ; aucune armure au départ. **Essayer-armure.cmd** lance une démonstration équipée. Les symboles du radar, le poing et la police bénéficient du pack HD, aux dimensions logiques PSP. Une comparaison avec la vidéo originale est dans `data/radar-comparison.png`. **G / croix droite** bascule radar tournant/nord fixe. Grande carte avec destination, zoom, guidage et recalcul ; voir **GPS.md**.

Huit voitures et jusqu'à vingt passants animent la zone. Les civils apparaissent, déambulent et fuient sur le **réseau piéton PSP** (`world.bin` section 3 : 8 703 nœuds globaux après fusion des 4 827 paires de raccords inter-secteurs), avec repli sur le masque radar hors réseau. Les voitures se pilotent et bloquent le passage à pied ; le franchissement peut monter sur leur toit. Combat au pied avec réactions, fuite/riposte, santé et réinitialisation. Le rendu possède une passe GLSL de couleur, contraste et vignette avec repli si indisponible.

Cette démo n'est **pas encore le jeu complet ni une recompilation MIPS**. Conduite, trafic, contacts et combat sont des reconstructions arcade provisoires. Un modèle de voiture et quatre tenues civiles ; pas encore de missions, police, armes à feu, son ou sauvegarde de partie. Des défauts de toits, raccords et matériaux restent à traiter. Les trois profils caméra conservent leur travail issu des programmes originaux, sans prétendre à une parité intégrale ; voir **CAMERAS-ORIGINALES.md**.

## Roulade, franchissement et fidélité PSP

La roulade utilise désormais l'enchaînement PSP **336/337 →334/335** : plongeon, rotation et redressement. L'ancienne paire408/409 était un coup de pied ; cette erreur est corrigée. La hauteur des couches suit le redressement original. La durée de0,533s reste calée visuellement sur la vidéo. Espace ou X déclenche une action ; devant un obstacle accessible, le franchissement reste prioritaire. L'impulsion et le freinage utilisent désormais les valeurs du programme PSP ; les contacts restent reconstruits. Voir **ROULADE-PROPULSION-PSP.md** et **ROULADE-MANETTE.md**.

La détection considère les boîtes, cylindres et faces verticales basses des collisions PSP. Elle accepte les approches obliques, mémorise brièvement un appui avant le contact et reprend l’allure après la réception. Le trajet entier doit rester libre ; l’eau, les plafonds bas et les réceptions obstruées sont refusés. Les petits ressauts jusqu’à 0,5 unité gardent le passage automatique ; au-delà, Espace déclenche le franchissement si possible.

**Ce comportement reste une reconstruction expérimentale, pas encore celui de la PSP à l’identique.** L’ancienne animation 424/425 est remplacée par la paire PSP 416/417 : les jambes passent sur le côté comme dans la vidéo fournie. Cette paire est aussi sélectionnée dans le programme PSP à 0x0883d970. Le cycle dure 0,8 seconde, d’après la séquence et la vidéo ; la trajectoire et les critères de franchissement restent reconstruits. Les fontaines sont couvertes par un test géométrique synthétique ; toutes les fontaines et barrières de la carte ne sont pas validées. L’Admiral utilise le modèle et les textures PSP, avec une boîte de collision simplifiée ; la scène de présentation ajoute désormais une conduite arcade. Voir **PSP-FIDELITE.md** pour les preuves et points à résoudre.

## Validation

- 90 tests automatisés : lecteurs, décompression, géométrie, collisions, réseau piéton PSP (fusion des raccords, sélection de nœuds et fuite), caméra, sprites PSP, vitesse, freinage, virages, gravité, raccourcis, jauges et chargement des textures améliorées.
- 324 combinaisons rotation/angle/vitesse dans un test de franchissement ; obstacles arrondis, triangles, plafonds, eau, appui anticipé, reprise de course et absence de passage à travers les collisions.
- Avant la correction issue de la vidéo, cinq essais du 20 octobre : barrière droite, inclinée, sprint, Admiral et rocher arrondi. Une action achevée par essai ; aucune interruption ni pénétration détectée sur les échantillons enregistrés.
- 552 animations PSP / 3 612 frames structurellement validées ; six atlas, deux palettes appliquées ; arrêt, marche, course et sprint intégrés. Le contour rose de couleur-clé est corrigé (pixels transparents neutralisés, filtrage ponctuel des atlas).
- Audit intégral de la carte : 1 886 blocs, 67 059 placements uniques hors proxies d'eau, 9 286 modèles, 812 565 triangles ; aucune erreur de lecture ou de bornes dans les variantes reconnues.
- 5 758 textures uniques référencées décodées, dont les images de 30 séquences animées.
- 14 000 cellules de collision : 50 894 boîtes, 12 527 cylindres, 19 018 triangles (compteurs de cellules, pouvant inclure des répétitions).
- Origine absolue des secteurs corrigée de 30 unités sur X et Y ; régression fondée sur un pilier identifiable dans la carte et les collisions PSP.
- Trois parcours de 4 secondes avec les nouvelles animations : 23,51 unités en marche, 45,22 en course, 64,14 en sprint avec 41 contacts de collision. Contrôles séparés du freinage, du demi-tour, de la limitation à la marche et d’une chute de 5 unités.
- Inspection précédente de 300 unités en caméra libre ; les anciens rapports de marche correspondent au contrôleur provisoire remplacé.
- Rendu contrôlé localement sur NVIDIA GeForce RTX 5070 Ti. Les tests ne prouvent pas que chaque rue est parcourable, ni que le port est fidèle à 100 %.

Les nouveaux essais roulade/manette sont décrits dans `ROULADE-MANETTE.md` et `data/build-validation.json` : cycle, huit directions, collisions, transitions et entrées manette. Animation corrigée contrôlée par captures de ses phases et rapports sur la carte ; montée sur voiture également vérifiée. Le maniement avec une manette physique reste à essayer.

Les essais des trois caméras sont dans `data/recenter-psp/`, `data/recenter-mobile/` et `data/recenter-ds/`. Les contrôles de franchissement de cette version sont dans `data/vault-recenter-psp/` et `data/car-recenter-ds/` : une action achevée par essai, 24 échantillons libres chacun.

Les essais après comparaison vidéo sont dans `data/vault-psp-after/` (muret et reprise du sprint), `data/car-psp-after/` (toit de l’Admiral), `data/vault-mobile-after/` et `data/camera-psp/`, `data/camera-mobile/`. Les trois franchissements se terminent sans interruption et sans pénétration détectée sur leurs 24 échantillons. Les rapports précédents sont dans `data/vault-straight/`, `data/vault-rotated/`, `data/vault-sprint/`, `data/car-climb/` et `data/round-obstacle/`. Les rapports des étapes précédentes sont dans `data/map-audit.json`, `data/huang-walk/`, `data/huang-run/`, `data/huang-sprint/`, `data/test-freecam/` et `data/build-validation.json`. Les compteurs ne représentent pas un pourcentage de portage.

## Compilation et diagnostics

Fermer la démo avant de remplacer son exécutable. `Compiler.ps1 -Test` utilise les outils Rust portables du projet VCS sur cette machine, dans un dossier de compilation propre à CTW ; sinon Cargo installé sur le système. Cargo.lock fixe les dépendances.

```powershell
.\Compiler.ps1 -Test
.\CTW-Rust.exe --map
.\CTW-Rust.exe --vault-demo
.\CTW-Rust.exe --car-demo
.\CTW-Rust.exe --headless --frames 90 --no-test-car --action-at 0
.\CTW-Rust.exe --camera psp
.\CTW-Rust.exe --camera mobile
.\CTW-Rust.exe --camera ds
.\CTW-Rust.exe --freecam
.\CTW-Rust.exe --map-audit
.\CTW-Rust.exe --headless --frames 240 --walk 0 1 --pace walk --capture marche.bmp
.\CTW-Rust.exe --pos -1500 -1000 --headless --frames 120 --capture ville.bmp
.\CTW-Rust.exe --verify --hash-iso
.\CTW-Rust.exe --model 30
.\CTW-Rust.exe --texture 1000
.\CTW-Rust.exe --iso 'chemin\vers\jeu.iso' --map
```

`--camera psp|mobile|ds` impose un profil au lancement ; les essais `--headless` ou `--frames` ignorent la préférence enregistrée pour rester reproductibles. `--action-at T` déclenche la roulade ou le franchissement au tick T. `--output dossier` choisit le dossier des rapports. En mode carte, `--frames N` effectue N images avec une simulation déterministe à 60 images / 30 étapes de jeu par seconde ; `--walk X Y` maintient une direction relative à la caméra ; `--pace walk|run|sprint` sélectionne l’allure des contrôles scriptés. `--headless` cache la fenêtre mais nécessite un contexte graphique Windows/OpenGL. Sans nombre d'images, il rend une image. `--capture` écrit un BMP à la fin de la séquence, ou à la fermeture en mode interactif. `--debug-collisions` affiche les boîtes dès le départ.

Les visionneuses `--model` et `--texture` restent disponibles : les modèles isolés utilisent un rendu de diagnostic gris. `--verify` audite les ressources ; `--map-audit` audite la carte sans créer de fenêtre.

## Prochaines étapes

1. Vérifier davantage de rues, rampes, ponts et intersections ; corriger les raccords et collisions manquantes.
2. Compléter le personnage : interactions, collisions plus fidèles, animations contextuelles, puis objets dynamiques.
3. Adapter la conduite, les véhicules et la population en Rust.
4. Analyser EBOOT.BIN et les overlays PSP pour identifier les comportements et routines à traduire ; intégrer scripts, missions, son et sauvegardes.

EBOOT.BIN est un PRX chiffré ; sa copie a été déchiffrée localement pour l’analyse. BOOT.BIN contient des zéros. Des comportements identifiés dans le programme sont adaptés manuellement en Rust ; aucun traducteur automatique Allegrex/MIPS n’est intégré. Le chargement des ressources originales ne doit pas être présenté comme une recompilation achevée.

## Sources

Code GPL-3.0-only : voir LICENSE et THIRD_PARTY.md. Les données du jeu restent dans l'ISO locale ; ne pas inclure l'ISO ni les ressources extraites lors d'une publication du code. Les captures locales contiennent des images du jeu.

Pour poursuivre : REPRISE-IA.md et FORMATS-PSP.md.
