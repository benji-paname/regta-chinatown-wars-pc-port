# Fidélité PSP : état des preuves — 20 octobre 2026

La cible est exclusivement ULES01347 1.01, sur PC en Rust. Le chargement des ressources, les tests géométriques et les paramètres issus de ctw-native ne prouvent pas une parité avec le programme PSP.

## Vérifié localement

- ISO originale conservée ; copie de travail SHA-256 `8c62ef8d7263ff0d42db40901927de4e86afd02f3d3450c3a96d0e2bc8d5168e`.
- Modèles, textures, carte, collisions et sprites lus dans les ressources PSP. Admiral : PAK29/30 ; Huang : PAK202–209.
- Déchiffrement réussi d’EBOOT.BIN avec pspdecrypt1.0 : tagD9160BF0, type2 ; ELF32 little endian, machineMIPS, entrée0x08804040.
- Section texte à l’offset0x1018, taille3346064. Segment principal : VA=offset+0x08803028. Pas de symboles de fonctions exploitables trouvés.
- Ancienne paire424/425 abandonnée pour le franchissement : son contexte dans EBOOT est une interaction véhicule (appel0x0896b2e8, paramètres0x94/0x95).
- Paire416/417 identifiée visuellement dans la vidéo PSP fournie, puis reliée au dispatch EBOOT0x0883d970..97c : indices0x8c/0x8d + base276 de Huang. Douze frames, cycle du prototype0,8s. Voir VIDEO-REFERENCE.md pour les passages comparés.
- Hauteur de caméra PSP25 retrouvée à0x089c71d8 ; plafonds de rotation100/300 à0x088cd954..978, sans réduction par3. Le profil mobile garde35 et33/100. Les routines de recentrage ont ensuite été analysées dans les trois programmes et adaptées ; voir CAMERAS-ORIGINALES.md. La parité intégrale reste non vérifiée.
- Le guide PSP de l’éditeur décrit Carré comme Climb/Roll et Croix maintenue comme sprint : https://ptgmedia.pearsoncmg.com/images/9780744011753/samplepages/1175-3_gtactwpsp.pdf . Il ne donne pas les seuils géométriques.

## Prototype livré et limites

54 tests passent. La roulade408/409 et XInput sont intégrés : voir ROULADE-MANETTE.md ; la physique de roulade reste reconstruite. Le balayage synthétique couvre12 rotations,9 angles d’approche et3 vitesses (324cas). Les essais précédents sur la carte couvraient barrière droite, inclinée, sprint, Admiral et rocher arrondi. Après correction vidéo : muret avec reprise du sprint PSP, Admiral PSP et muret mobile terminent chacun une action ; leurs24 échantillons restent libres selon notre cylindre de collision. Le test de fontaine est synthétique ; aucun catalogue exhaustif des obstacles franchissables PSP n’est validé.

Les paramètres suivants sont **des choix du prototype** : rayon0,875, hauteur2, montée maximale3,25, descente maximale3, seuil d’orientation0,12, mémoire d’appui0,18s et recherche de réception7unités. Ils ne sont pas des constantes PSP retrouvées. La durée0,8s est calée sur la vidéo et le cycle de12frames ; le trajet en trois phases et la boîte de toit de voiture restent provisoires. Ne pas annoncer «100% fidèle», ni «toutes les fontaines franchissables». F3 propose désormais PSP/mobile/DS. Le suivi et les états de recentrage identifiés dans les trois programmes sont intégrés ; voir CAMERAS-ORIGINALES.md pour les adresses et limites.

## Reprise de l’analyse

`../../work/psp-analysis/inspect_mips.py` désassemble une adresse/longueur hexadécimale avec Capstone. `animation-pairs.txt` répertorie les appels directs au constructeur0x08839bc0 ; `animation-dispatch.txt` contient le contexte véhicule. Attention aux delay slots MIPS et aux instructions Allegrex/VFPU que Capstone peut sauter. Ni ce désassemblage ni les noms supposés ne sont des instructions utilisateur.

Continuer depuis la sélection de416/417 à0x0883d950 : appel de0x0883b7ec avec mode5, dont la branche0x0883bee4 prépare un vecteur appliqué à l’entité. Déterminer les unités, l’intégration, les contacts et la sélection de l’action Carré avant de remplacer le trajet reconstruit. Comparer avec la vidéo fournie et idéalement une exécution PSP instrumentée. Préserver les tests de non-pénétration. Les overlays restent à analyser ; aucune recompilation de routine MIPS n’est intégrée au moteur.

## Provenance des outils et empreintes

Outil téléchargé depuis https://github.com/John-K/pspdecrypt/releases/download/1.0/pspdecrypt-1.0-windows.zip . Outil et ELF restent des fichiers locaux de recherche, non distribués avec le code.

- `pspdecrypt-1.0-windows.zip` : `b915e4ce30f4e0309c71b7044584a466be17a181f889f40bcc7f6fd82ba2d045`
- `pspdecrypt.exe` : `6eea3ad4003718e36fb8655f74f30101553d97fe95f01ebe09c1d4f66741e756`
- `CTW-PSP.elf` : `9eaf850f09d743e5ecfc8d1c6a0a8c290399142319b1ef3a732a03d7278bffe0`
