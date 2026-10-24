# Propulsion de la roulade — correction du 21 octobre 2026

L'ancienne vitesse constante de 8 unités/s était une approximation inadaptée. Elle est supprimée. La roulade prend désormais la main sur la vitesse : impulsion vers l'avant, freinage progressif, puis reprise de la locomotion. Aucun multiplicateur supplémentaire de marche ou de sprint n'est ajouté.

## Valeurs relevées dans le programme PSP

Source locale : ELF déchiffré ULES01347, SHA256 `9eaf850f09d743e5ecfc8d1c6a0a8c290399142319b1ef3a732a03d7278bffe0`. Les adresses suivantes sont des adresses virtuelles PSP. Désassemblage conservé dans `../../work/roll-reference/psp-roll-propulsion-trace.txt`.

- `0x089cfc00..14` : l'action crée l'état 11 avec une force de `0x1e`, soit 30.
- `0x0883d4bc` : l'état 11 appelle l'effet physique 0.
- `0x0883ba38..bad4` : cet effet construit une vitesse dans la direction de l'action, multiplie par la force puis par le pourcentage de réaction physique, et remplace la vitesse du personnage. Il n'additionne pas la vitesse de marche/course.
- `0x089a71a8..b4` : le pourcentage de réaction est initialisé à 100. Le stockage et la lecture sont à `0x0899b670..b6b0`. Le prototype utilise ce réglage normal ; les modifications de ce paramètre par les scripts du jeu ne sont pas implémentées.
- `0x0883b2c8` : le traitement de l'animation appelle le calcul physique après la sélection de l'animation et de l'impulsion.
- `0x0883c2b0..c36c` : calcul de la vitesse horizontale au carré ; arrêt sous 0,5 unité/s.
- `0x0883c510..c538` : coefficient 40 au sol, 4 hors sol. Le prototype autorise la roulade seulement au sol et utilise donc 40. La correspondance de la fonction PSP `0x0899a928` avec `cPed::IsOnGround` a été corroborée via le symbole Android et son appel dans `ApplyPhysics`.
- `0x0883c68c..c7d8` : freinage entier décrit ci-dessous, borne basse `0x28`, borne haute `0xfae`.

Les composantes `vx`, `vy` sont en unités 1/4096 :

```text
speed2 = vx*vx + vy*vy
si speed2 < 0x400000 : vx = vy = 0
sinon :
  k = clamp(((((speed2 >> 12) * 136) >> 12) * 40) >> 12, 40, 4014)
  vx = (vx * (4096-k)) >> 12
  vy = (vy * (4096-k)) >> 12
```

Le freinage est appliqué au pas de jeu à 30 Hz avant l'intégration. Le facteur d'intégration 17/512 est celui déjà utilisé par le déplacement du prototype. Les sous-pas de collision répartissent ce déplacement ; ils ne réappliquent pas le freinage. Un découpage du pas en deux donne le même résultat.

Pour une direction axiale libre, la vitesse initiale avant freinage est 122880 en unités 1/4096. Les 16 vitesses calculées sont :

```text
87030,74302,66374,60734,56419,52961,50090,47668,
45584,43758,42144,40703,39401,38227,37153,36164
```

Cela donne 6,636669 unités sur les 16 pas de la roulade actuelle, avant arrondi des coordonnées et collisions. Ce résultat est calculé, pas choisi comme distance cible ni mesuré en unités du monde dans la vidéo. La première vitesse effective de déplacement est environ 21,16 unités/s, contre 8 dans l'ancienne version. Le mouvement ralentit ensuite.

## Comparaison vidéo et essais

Nouvelle vidéo fournie : `PPSSPP 2026.10.21 - 18.45.18.01.mp4`, durée 13,218 s. Benjamin précise les trois conditions : arrêt, marche, course. Les captures examinées couvrent notamment la roulade à l'arrêt vers 0,9–1,4 s, le passage en marche vers 5,3–5,8 s et le passage en course vers 9,3–9,8 s. Les changements de position sont examinés avec les repères du décor, et pas seulement la position du personnage à l'écran puisque la caméra suit. Planches locales dans `../../work/roll-reference/psp-new-*.jpg`. Les anciennes captures restent disponibles ; l'ancien fichier vidéo dédié n'était plus présent à son chemin initial lors de cette correction.

58 tests passent, dont trois nouveaux : impulsion depuis les quatre niveaux de déplacement même sans entrée maintenue, suite entière de freinage et seuil d'arrêt, indépendance vis-à-vis du découpage du pas. Les contrôles existants des huit directions, murs, eau/rebords, reprise du sprint et priorité au franchissement passent également.

Essais sur la carte : `data/roll-impulse-idle/`, `data/roll-impulse-walk/`, `data/roll-impulse-sprint/`. Chaque essai termine une roulade avec 16 échantillons libres ; première vitesse effective environ 21,16, puis décroissance. L'essai `data/roll-impulse-car/` conserve la montée sur l'Admiral en caméra DS. Les rapports incluent la vitesse réelle par pas.

## Limites précises

Les constantes d'impulsion et la formule de freinage proviennent du PSP. Cela ne valide pas le port complet de sa physique : les contacts et refus aux rebords/eau restent conservateurs, les scripts qui changent le pourcentage de réaction ne sont pas présents, et l'ordonnancement complet du moteur original n'est pas reproduit. La durée de 16/30 s et le partage des phases d'animation restent ceux de la correction précédente, calés visuellement ; cette étape ne les modifie pas pour forcer une distance. Aucune parité complète du déplacement n'est revendiquée sur la seule comparaison vidéo.
