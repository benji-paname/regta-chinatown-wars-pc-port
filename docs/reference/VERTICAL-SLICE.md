# Démo de présentation — 24 octobre 2026

Mise à jour : pack HD prioritaire (48 ressources, 47 images), polices 4096 × 4096 sélectionnées, radar positionné et dimensionné selon le programme PSP, vie et armure fonctionnelles sur 0–127. Voir TEXTURES-HD.md et RADAR-PSP.md. Les textures améliorées remplacent désormais leurs équivalents disponibles de l'ISO ; les mesures originales restent conservées. `Essayer-armure.cmd` permet de voir la jauge bleue et l'absorption des dégâts.

Lancer **Presenter-la-demo.cmd**, ou **Present-demo-English.cmd** pour une présentation en anglais. L'exécutable utilise l'ISO PSP locale déjà configurée. Le départ sur le trottoir montre immédiatement les voitures et les passants. La signature **by Benji Paname** reste visible en haut à gauche.

Cette version montre la vision du port : ville PSP chargée par quartiers, Huang, passants animés, voitures pilotables, combat simple, GPS, trois caméras et rendu colorimétrique par shader. C'est une démonstration native Rust, pas encore le jeu complet.

## Présenter en deux minutes

1. Marcher sur le trottoir, sprinter et faire une roulade. Les passants circulent dans les zones piétonnes.
2. S'approcher de l'Admiral bleu clair et utiliser **E / Y** pour monter.
3. Accélérer, tourner, freiner ; s'arrêter puis **E / Y** pour descendre. **R / LB+Y** remet la scène au départ.
4. À pied, approcher un passant et utiliser **J / B**. Le coup de pied provient des animations PSP ; certains passants ripostent, les autres fuient. La vie apparaît sur le bord du radar.
5. Ouvrir le GPS avec **M / Back**, choisir une route avec clic/Entrée/A, revenir avec **Échap / B** et suivre le tracé.
6. **F3 / clic RS** change PSP/Mobile/DS ; **F4 / LB+clic RS** change français/anglais. **F12 / croix gauche** enregistre une capture.

## Commandes principales

| Action | PC | Xbox/XInput |
|---|---|---|
| Déplacement | ZQSD / WASD / flèches | Stick gauche |
| Sprint | Maj | A |
| Roulade / franchissement | Espace | X |
| Combat à pied | J | B |
| Monter / descendre à faible vitesse | E | Y |
| Accélérer | Z / W / flèche haut | RT ou stick haut |
| Freiner puis reculer | S / flèche bas | LT ou stick bas |
| Braquer | Q/A et D | Stick gauche/droite |
| Frein à main | Espace | X |
| Pause | P | Start |
| Carte GPS | M | Back |
| Bandes d’informations | F5 | RB + Back |
| Langue français/anglais | F4 | LB + clic RS |
| Rotation radar | G | Croix droite |
| Caméra PSP/Mobile/DS | F3 | Clic RS |
| Caméra libre | F2 | Croix haut |
| Recentrer la caméra | C maintenu | LB maintenu |
| Réinitialiser la scène | R | LB + Y |
| Quitter hors carte | Échap | Start + B |

Les commandes PC et Xbox sont affichées ensemble dans les bandes noires **F5 / RB+Back**. Elles sont masquées par défaut pour la présentation. La langue, la rotation et les bandes sont mémorisées dans `data/hud-settings.json`. La carte ouverte suspend le jeu et utilise ses propres commandes affichées. Les diagnostics `--language fr|en` et `--minimap-rotation on|off` permettent de forcer les choix au lancement.

## Ce qui vient de la PSP

Carte, textures, Admiral, sprites et animations viennent de l'ISO. Le radar utilise maintenant les **40 tuiles PSP 332–371**, l'annuaire **331** (en-tête de 8 octets, grille 8×7, origine 461/160), et la transformation cartographique Q12 1638/4096. Le poing vient de l'atlas original 374. L'ATH reprend la disposition montrée dans la vidéo PSP : radar rond en bas à gauche, vie verte sur son pourtour, flèche blanche, horloge et poing en haut à droite. Les commandes et la signature sont les ajouts du port. La police **PSP_Hel12x12.bin / PAK261**, la flèche et le contour du radar **PAK378** sont également lus dans l’ISO. La vue est désormais en **1280×720, 16:9**. Les boutons de la grande carte restent une adaptation PC ; ne pas présenter cet écran comme une réplique exacte du menu PSP.

Les passants utilisent désormais le **réseau piéton PSP** (`world.bin` section 3 : 13 530 nœuds sectoriels fusionnés en 8 703 nœuds globaux via 4 827 paires de raccords réciproques) pour apparaître, déambuler de nœud en nœud (`cWanderPath`) et fuir le long des liens (`cFleeOnFoot`), tout en vérifiant les collisions réelles. Le masque couleur beige/vert des tuiles radar reste utilisé en repli lorsqu'un civil quitte le réseau.

## Limites de cette présentation

Conduite, trafic et réactions des passants sont des systèmes arcade provisoires. Ils ne reproduisent pas encore la physique et l'IA PSP. Un seul modèle de voiture avec plusieurs teintes et quatre variantes de tenues civiles sont utilisés. Pas encore de missions, police, armes à feu, son ou sauvegarde de partie. La petite horloge est celle de la démo. Le shader applique couleur, contraste et vignette ; il ne reproduit pas l'éclairage original complet. Les défauts de certains toits et raccords de carte restent à traiter.

## Vérification

81 tests automatisés et 40 scénarios sur la vraie carte : conduite, virages, freinage/sortie, réactions de combat, population, GPS, trois caméras, français/anglais, rotation, jauges, armure, priorité HD et repli sans shader. Voir `data/build-validation.json`, `slice-validation.json`, `hud-validation.json`, `gps-validation.json` et `radar-validation.json`. Les 48 remplacements ont aussi été contrôlés sans perte et les 7587 textures PSP décodées. Captures inspectées ; manipulation avec une manette physique encore à confirmer.
