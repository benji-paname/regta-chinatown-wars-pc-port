# Roulade et manette — 21 octobre 2026

**Espace**, le bouton à l'écran ou **X sur une manette Xbox/XInput** déclenchent la même action. Devant un obstacle accessible, Huang franchit ou monte dessus. Sinon, il effectue une roulade dans la direction où il regarde. Un appui déclenche une action ; relâcher avant de recommencer. Les appuis pendant une action ne mettent pas une seconde roulade en attente.

## Commandes de la manette

| Xbox / XInput | Correspondance PSP | Action disponible |
|---|---|---|
| Stick gauche | Stick analogique | Déplacement ; faible inclinaison pour marcher |
| A maintenu | Croix | Sprint |
| X | Carré | Roulade / franchissement contextuel |
| LB maintenu | L | Recentrage de la caméra |
| Start / Menu | Start | Pause et reprise du prototype |
| Clic stick droit | Ajout PC | Choix PSP → Mobile → DS |

B/Cercle (combat), Y/Triangle (entrer dans un véhicule), RB/R (ciblage) et Back/Select (interactions/inventaire) attendent l'implémentation de ces fonctions du jeu. Ils ne sont pas réaffectés à des actions différentes. La pause ne contient pas encore le PDA PSP. F2 conserve la caméra libre au clavier. Au clavier : P pour pause/reprise ; autres commandes dans README.md.

La disposition suit les positions physiques des boutons PSP, documentées dans le [guide PSP publié par BradyGames/Pearson, page 2](https://ptgmedia.pearsoncmg.com/images/9780744011753/samplepages/1175-3_gtactwpsp.pdf). Le stick à faible inclinaison et le clic droit pour choisir la caméra sont des adaptations PC.

XInput est lu directement, sans émulation du clavier : recherche parmi les quatre emplacements, utilisation d'une seule manette à la fois, reconnexion automatique, zone neutre radiale de 7849 et amplitude limitée à 1. Les boutons maintenus au branchement ou au retour dans la fenêtre doivent être relâchés avant d'agir ; les entrées sont neutralisées hors focus ou à la déconnexion. Les essais déterministes ignorent les périphériques physiques. Références Microsoft : [XINPUT_GAMEPAD](https://learn.microsoft.com/en-us/windows/win32/api/xinput/ns-xinput-xinput_gamepad), [versions XInput](https://learn.microsoft.com/en-us/windows/win32/xinput/xinput-versions). Seules les DLL du dossier système sont chargées : 1.4, puis 9.1.0 en secours. Aucun pilote ni logiciel supplémentaire installé.

## Animation PSP et vidéos

**Correction après le signalement de Benjamin : 408/409 était un coup de pied.** L'ancienne attribution à la roulade et sa prétendue validation visuelle étaient erronées.

La roulade enchaîne maintenant **336/337** (plongeon, 3 images), puis **334/335** (rotation et redressement, 9 images), ressources originales PSP PAK202. Comparaison visuelle : les poses de334 correspondent à la vidéo dédiée, notamment les jambes au-dessus de la tête et le redressement final.

Trace PSP : `HandlePlayerDive` à0x089cfc00 crée l'état11 ; `SetupForState` à0x0883d4e0 choisit0x3c/0x3d, soit336/337 avec la base276. La transition joueur ordinaire à0x0883ca54 passe à l'état10 ; à0x0883d400, celui-ci choisit0x3a/0x3b, soit334/335. Les autres branches de chute/réaction ne sont pas portées ici. Preuve conservée dans `../../work/roll-reference/psp-roll-corrected-trace.txt`.

La hauteur est également corrigée : l'état10 indique une interpolation entre les poses1 et0, et non des hauteurs littérales1 et0. Le calcul PSP à0x089b1558 et l'initialisation de la table à0x089b285c donnent, pour les quatre couches, [4096,3072,2048,1228] vers [8192,6144,4096,2048], en unités1/4096. Le rendu suit la progression de l'animation avec quantification. Preuve : `psp-roll-height-trace.txt`.

Le cycle du prototype reste **16/30 s** (environ0,533s), avec un quart pour le plongeon et trois quarts pour la rotation. Cette répartition est calée visuellement sur la vidéo ; elle ne constitue pas une reproduction démontrée de l'ordonnancement PSP. Les deux ressources ont une cadence7680 ; ce nombre seul ne prouve pas30images/s.

Vidéos fournies et inspectées :

- `PPSSPP 2026.10.21 - 00.02.10.01.mp4` : roulades répétées vers40–43s.
- `Desktop 2026.10.21 - 00.36.02.02.mp4` : version DS, roulades vers35–37s.
- `PPSSPP 2026.10.21 - 00.48.21.03.mp4` : séquence dédiée,21,232s ; détail du cycle vers12,97–13,50s. Les planches de recherche sont dans `../../work/roll-reference/`.

**Propulsion corrigée depuis le programme PSP.** La vitesse constante provisoire de8unités/s est supprimée. L'effet physique impose une force30 avec le coefficient normal100%, puis applique le freinage entier original au sol. La poussée agit à l'arrêt, en marchant ou en courant, même sans direction maintenue. Sur16pas libres, le calcul donne environ6,64unités ; cette distance n'est pas un réglage arbitraire. Détails, adresses originales, comparaison avec la nouvelle vidéo et limites dans **ROULADE-PROPULSION-PSP.md**. Les collisions aux murs, à l'eau et aux rebords restent conservatrices ; la parité complète de la physique n'est pas encore validée.

## Contrôles effectués

**58 tests automatisés passent**, dont le nouveau contrôle du redressement progressif des couches et les contrôles existants des collisions, transitions et entrées XInput. Compilation release hors ligne réussie.

Sur la carte, le rapport `data/roll-fixed-90/play-validation.json` montre le passage336 →334 →276, les douze images sources visitées et une roulade terminée, sans coup de pied408. Les16échantillons de collision sont libres. Captures des phases dans `data/roll-fixed-{6,12,22,30}.bmp` ; planche `data/roll-corrected-poses.png`. La montée sur l'Admiral en caméra DS reste fonctionnelle (`data/roll-fixed-car/`). Le test de muret avec reprise du sprint a également réussi pendant la correction (`data/roll-corrected-vault/`, avant le seul ajustement des hauteurs).

Aucune manette physique détectée pendant ces essais ; son maniement reste à essayer. Les anciens rapports `roll-open`, `roll-mid` et `ui-roll-xinput` concernent la version au coup de pied et ne valident pas l'animation corrigée.
Diagnostics : `--action-at T` injecte l'action contextuelle au tick T ; `--vault-at T` conserve l'ancien essai de franchissement avec mémoire d'appui. Rapports corrigés `data/roll-fixed-90/`, `data/roll-fixed-car/` ; synthèse et empreinte du binaire dans `data/build-validation.json`.


Dernière validation du déplacement : `data/roll-impulse-idle/`, `data/roll-impulse-walk/`, `data/roll-impulse-sprint/`, `data/roll-impulse-car/`. Les rapports `roll-fixed-*` cités plus haut concernent la correction antérieure des sprites.
