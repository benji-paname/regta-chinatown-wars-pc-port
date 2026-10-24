# Radar PSP et pack de textures — 24 octobre 2026

Le radar utilise maintenant les coordonnées du programme PSP ULES01347 1.01, les définitions de `SS_Radar.bin` et les tuiles de l'ISO. Le pack fourni par Benjamin remplace les images correspondantes en priorité ; les coordonnées restent celles de la PSP.

## Valeurs retrouvées dans les fichiers PSP

| Élément | Donnée originale |
|---|---|
| Écran de référence | 480 × 272 |
| Centre du radar | (48, 224) |
| Bordure | sprite 43, rectangle 96 × 96, ancre (48,48) |
| Flèche | sprite 38, rectangle 19 × 18, ancre (9,9) |
| Nord | sprite 37, rectangle 13 × 13, ancre (7,7), rayon de placement 38 |
| Disque de carte | rayon 38 ; contour entier PSP à 256 segments |
| Pourtour des jauges | rayon extérieur 41,83734 après conversion GE float24 |
| Carte / fond des jauges | alpha 191/255 |
| Échelle monde → carte | 1638/4096, matrice de dessin 32767/32768 |
| Zone visible | diamètre d'environ 190,0522 unités du monde |
| Vie / armure | 0 à 127 ; 128 segments par demi-cercle |
| Vie | gauche, RGB (82,123,82) |
| Armure | droite, RGB (53,113,156) |
| Vie faible | moins de 25 segments : RGB (123,54,54), masquée lorsque compteur & 0x1c == 0x1c |

Repères dans le binaire PSP déchiffré : constructeur `0x089e3ba4`, disposition `0x089e55b0` (placement de la bordure à `0x089e5a70`), projection/limite nord `0x089e67f0` (rayon à `0x089e6c38`), dessin `0x089e7324`, couleurs et remplissage `0x089e772c–0x089e789c`, tuiles `0x089e7d30`. Les ressources de la carte sont l'annuaire 331, les tuiles 332–371 et la mer 372 (32 × 32). L'atlas assemblé conserve chaque texel et ajoute seulement une bordure de mer pour le filtrage.

Le passage à 1280 × 720 applique séparément les rapports 1280/480 et 720/272. Les bandes d'informations PC sont une exception volontaire : elles remontent le radar de 130 pixels pour qu'il reste visible. Elles sont désactivées au départ. Les trois profils caméra et l'option de rotation restent disponibles.

## Vie et armure fonctionnelles

Le joueur commence avec 127 de vie et zéro armure, comme les valeurs d'initialisation PSP (`0x089a7010`). L'armure absorbe les dégâts ; seul l'excédent atteint la vie (`ReduceArmour`, `0x089c967c`). Les dégâts des passants restent ceux du combat simplifié du prototype. Le bonus de double armure et les ramassages ne sont pas encore implémentés.

`Essayer-armure.cmd` démarre une démonstration équipée. Pour les contrôles déterministes : `--armour 0..127` et `--health 0..127`. Réinitialiser restaure les valeurs choisies au lancement.

## Comparaison vidéo

Trois images ont été relues à 0, 6 et 12 secondes dans `PPSSPP 2026.10.21 - 18.45.18.01.mp4`. Les anciens extraits conservés de l'enregistrement `00.02.10.01` ont également servi de référence. Comparaison en coordonnées PSP, sans recadrer séparément la bordure pour la faire coïncider : `data/radar-comparison.png`.

La comparaison vérifie l'emplacement, les proportions, les symboles et la transparence. La position exacte du joueur, l'orientation et le décor sous la carte ne sont pas synchronisés image par image. Les marqueurs de missions du jeu original ne sont pas inventés dans le prototype. Les symboles du pack HD ont leur propre dessin : `--original-textures` permet de comparer avec les images de l'ISO.

L'ensemble de l'ATH n'est pas déclaré identique pixel par pixel : horloge de démonstration, grande carte et guidage GPS adaptés au PC, signature et bandes supplémentaires, missions non implémentées.
