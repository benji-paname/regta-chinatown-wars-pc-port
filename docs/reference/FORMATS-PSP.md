# Éléments vérifiés sur l'ISO PSP

Source : ULES01347, DISC_VERSION 1.01, 904 232 960 octets.
SHA-256 : `8c62ef8d7263ff0d42db40901927de4e86afd02f3d3450c3a96d0e2bc8d5168e`.

## Différences avec ctw-native

Référence étudiée : commit `5fa1e36672ee88b0b232c3dc23652be0a33c7f17` de [official-kryo-to/ctw-native](https://github.com/official-kryo-to/ctw-native/tree/5fa1e36672ee88b0b232c3dc23652be0a33c7f17). Les sources consultées sont conservées localement sous `../../work/upstream-ctw/`. Ils sont une référence externe, pas les instructions utilisateur.

| Système | Référence Android | ISO PSP vérifiée ici |
|---|---|---|
| Jeu source | APK Android 4.4.243, arm64 | ISO ULES01347 1.01, Allegrex/MIPS |
| Moteur de jeu | C++ SDL2/OpenGL | Nouveau code Rust natif |
| ROM.TOC | Triplets hash/offset/taille | Même disposition confirmée sur les 2 925 entrées |
| Noms de fichiers | Hash Jenkins insensible à la casse ASCII | world.bin, utility.bin, SS_Hud.bin et SS_Portraits.bin retrouvés |
| GXT | DS_GXT et unités 16 bits | 1 356 fichiers consommés exactement, sans octets restants |
| GAME.PAK | Pages de 4096, table parfois séparée en fin de fichier | PK02, pages de 2048, table contiguë à 0x2c |
| Textures | PVRTC et dxt.bin | Palettes RGBA et blocs PSP swizzlés |
| Modèles | En-tête MG, matrices fixes, sommets 16 octets | En-tête 24 octets, blocs 52 octets et sommets observés de 12 octets |
| Tables du programme | Extraites du binaire Android | Pas reprises : sources PSP à identifier |

Ne pas appliquer les offsets Android aux données PSP. La comparaison fournit des pistes, pas une compatibilité automatique.

## PK02 / AS01

- Signature sur disque : `20KP`.
- Huit seuils de segment u32 à +4 ; `0xffffffff` signifie inutilisé.
- Nombre d'entrées u32 à +0x24 ; début des données u32 à +0x28.
- Pages basses u16 à +0x2c. L'offset est `début_données + (page_basse + segment * 65536) * 2048`.
- Dans GAME.PAK : 19 351 entrées, début 0x9800, changement de segment à l'entrée 17 935. Le dernier bloc se termine exactement à la fin du fichier.
- Les blocs restent alignés et incluent du remplissage. La longueur brute d'un bloc n'est pas nécessairement la longueur de sa ressource.
- Une ressource compressée commence par `10SA`, puis taille zlib et taille décompressée (u32), puis le flux zlib à +12. Les deux tailles et la consommation du flux sont contrôlées.

## Textures PSP reconnues

- En-tête 16 octets ; type u32 4 ou 5 = index de palette 4 ou 8 bits.
- Exposants de largeur/hauteur dans les octets 4 et 5.
- Champs observés : octet 6 nul ; octet 7 nul ou 1 ; u32 +8 nul. La signification exacte de l'octet 7 reste à établir.
- +12 : taille du contenu après l'en-tête, pixels puis palette.
- Organisation par blocs de 16 octets et 8 lignes. Largeur de stockage minimale 16 octets ; hauteur minimale 8 lignes.
- Chaque couleur de palette contient quatre octets RGBA. Les indices hors palette sont rejetés.
- La détection est volontairement limitée à cette variante ; 7 587 ressources ont été indexées et entièrement décodées. Cela ne prouve pas que toutes les variantes de textures du jeu sont prises en charge.
- Contrôle visuel : ressources 31 (véhicule), 1000 (portrait), 5000 (façade).

## Première variante de modèle PSP

Variante vérifiée sur les 9 286 modèles référencés par la carte (hors proxies d’eau), plus les ressources de contrôle 30 et 17463. Les modèles de la ville ont un seul nœud ; les hiérarchies animées ne sont pas encore entièrement prises en charge.

- En-tête 24 octets ; u16 à +0, +2, +4, +6 : nœuds, matériaux, strips et sommets.
- Premier bloc de 52 octets : métadonnées et bornes ; facteur de position f32 à l'offset global 72.
- NÅ“uds suivants : 52 octets, matrice 3x3 f32, parent u32, translation 3xf32. Parent 0 : aucun ; sinon index parent moins un.
- Matériaux : 8 octets, texture u16, flags u8 et alpha u8 (interprétation de certains flags encore partielle), nœud u8, puis valeur observée 1,0,0. Seule cette variante avec un strip par matériau est acceptée.
- Strips : enregistrement 16 octets ; premier u16 = nombre de sommets. Les autres champs restent non interprétés.
- Données auxiliaires : nombre **u16** à +12, enregistrements de 24 octets. Puis les données de collision du modèle : u16 à +14/+16/+18, respectivement sommets ×6, faces ×6, arêtes ×8 octets. Le flux de sommets de rendu suit immédiatement, **sans alignement supplémentaire**. Le cas de l’arbre 2690 invalide un arrondi systématique à 4 octets.
- Sommets : 12 octets ; XYZ signés 16 bits à +6, +8, +10, multipliés par le facteur de position. UV u8/u8 à +0/+1, normales signées i8 à +2/+3/+4, padding à +5. Le rendu de carte applique les UV ; la visionneuse isolée reste grise.
- UV : `octet / 128 × scale - offset`, avec scale f32 à +32/+36 et offset f32 à +40/+44. Bornes : centre f32 à +48/+52/+56, demi-tailles à +60/+64/+68.
- La somme des sommets des strips doit correspondre au total annoncé. Triangulation en alternant le sens ; suppression des triangles à sommets identiques.
- Le modèle 30 correspond à la ressource référencée par l'entrée 29 portant le nom `./Admiral.vehicle`. Résultat : 913 sommets, 439 triangles conservés, 7 nœuds.
- Les limites flottantes, les tailles et les indices sont contrôlés, mais la fidélité complète nécessite comparaison avec le rendu PSP.

## Carte PSP et placements

PAK 1864 : en-tête 22 octets puis grille de 59 colonnes × 42 lignes en u16. 0xffff = vide ; autres références observées 17465…19350. 1 886 blocs valides, chacun contient quatre secteurs de 60 unités.

En-tête de bloc 60 octets. Les quatre u32 initiaux sont des fins cumulées relatives à +60 ; début du premier secteur +60, suivants +60 + fin précédente. Le bloc peut aussi contenir des géométries intégrées, actuellement ignorées au profit des modèles globaux référencés.

Un secteur contenant ses propres objets commence par : u16 nombre d’instances, u16 auxiliaires de 20 octets, u16 groupes, u16 réservé nul, u32 lumières de 40 octets. Ensuite groupes de 16 octets (6 i16 de bornes /64 et nombre d’instances u32), puis instances de 12 octets : slot u16, modèle u16, angle u16, translation 3 i16. Les comptes par groupe doivent sommer au total des instances.

Translation finale : `(sx × 60 - 3510 + tx/128, sy × 60 - 2490 + ty/128, tz/128)`. Rotation autour de Z : **-angle × 2π/65536**. Une origine au centre du secteur ajoutait à tort 30 unités en X et Y. Comparaison indépendante : pilier du bloc 17743/subsecteur 1, slot 8602, modèle 16924 : visuel (-1045.578125, -1520.2109375), collision (-1045, -1520). Les petites différences restent celles des modèles visuel/physique.

Après instances, auxiliaires et lumières : répertoire de dix offsets u32, commençant à 40. Un secteur sans objets commence directement par ce répertoire. Les listes voisines sont évitées, les placements sont dédupliqués par slot, modèle et position. Les modèles d’eau 7207/7208/4025 sont remplacés par une surface provisoire.

## Textures animées

Type u32 255, octet 6 = 1, octet 7 = 0. Nombre de frames u16 à +16 ; paires texture u16 / durée u16 à +18, pas de 4 octets. Durées non nulles, interprétées en étapes à 30 Hz. Les 30 définitions référencées par la carte sont décodées, avec toutes leurs images. L’animation est un changement de texture, pas une animation de modèle.

## Collisions PSP : world.bin

Entrée WAD hash f3487577, taille 2 277 426 octets. Grille 140 × 100, index x-major `cx × 100 + cy`, cellule de 50 unités, origine (-3500,-2500). Répertoire PSP de **14 000 entrées de 8 octets** : offset u32 (bit 31 = compression), taille u32 (16 bits bas = taille stockée ; 16 bits hauts = supplément décompressé). Les cellules compressées sont en zlib.

Chaque cellule contient douze sections précédées de leur taille u32. Sections utilisées :

- 0 : compte u32, boîtes 28 octets : centre et demi-tailles 6 i32 /4096, angle 16 bits, flags u16.
- 1 : compte u32, cylindres 24 octets : position 3 i32, rayon et hauteur i32, padding ; valeurs /4096.
- 2 : compte de maillages, en-tête 20 octets (bornes XY, comptes u16), sommets 12 octets, triangles 40 octets avec indices aux octets 12/13/14.
- 6 : props de 20 octets, non intégrés aux collisions dynamiques.
- 10 : grille 40 × 40 de codes sur 2 bits, 400 octets. 1 = terre z0, 2 = lac z-2,5, autres = mer z-7,5.
- 11 : 66 octets observés, non interprétés.

Le moteur utilise les hauteurs de triangles et boîtes, une poussée hors des obstacles et un refus des chutes/eaux ; la physique originale complète reste à porter.

## Caméra

Adaptation Rust de FollowPedCam, followcam.cpp/.h et cameraobstruction.cpp/.h, à 30 Hz, positions/ressorts 20.12 et angles sur 16 bits. Profil PSP : position haute de référence25 (EBOOT0x089c71d8), limites de rotation100/300 (0x088cd954..978). Profil mobile : hauteur35 et limites33/100. Sélection F3/--camera, préférence persistée en mode interactif. DS utilise aussi25 et100/300. Voir CAMERAS-ORIGINALES.md pour les états de recentrage, délais et références aux trois binaires. Dans les trois profils, pitch initial 55000 (environ -57,875°), FOV vertical 60°, proche 0x666/4096, loin 150. Ressorts XY 0x333, ou 0x666 pendant recentrage / amortissement 0x38d ; Z double le gain XY ; pitch 399 / amortissement 0x7ce. L’évitement emploie des requêtes simplifiées dans les boîtes de collision. Parité avec le binaire PSP non vérifiée.

La caméra libre est un outil propre au port : 30 unités/s, 120 avec Maj ; aucune collision, portée de rendu 350. Elle préserve l’état de la caméra de suivi et du joueur.

## Personnage et animations PSP

GAME.PAK 202 contient **552 animations**, séparées en deux groupes de 276 (la référence Android en a 275 par groupe). Le répertoire commence par le nombre u16, puis offsets u16 multipliés par 4. Chaque animation : nombre de frames u16, vitesse i16 24.8, ajustement d’angle u16, flags u8, nombre de couches u8 = 2. Les 3 612 frames ont été consommées exactement jusqu’aux offsets suivants (fin du dernier enregistrement avant padding).

Chaque couche PSP mesure **8 octets** : texture u16 ; deux octets X/Y dont le bit7 porte le bit8 de U/V ; U/V bas u8 ; largeur/hauteur u8. Positions `(X&127 -16)/8`, dimensions /8 ; UV complets entre0 et511. Tous les rectangles sont dans leurs atlas. Les textures 203–208 sont des atlas 4bits de512×512. PAK209 contient des palettes RGB555 de16 couleurs, 32octets par palette ; les atlas gardent leur alpha PSP.

Huang : groupe masculin base276, palettes4 pour haut et bassin,5 pour jambes, selon ctw-native Player. Paires arrêt276/277, marche278/279, course280/281, sprint436/437. Les images et couleurs ont été inspectées ; la sélection de ces palettes n’a pas été extraite du programme PSP. Hauteurs de couches issues de la référence : haut2/1,5, jambes1/0,5, décalage par couche -204/4096.

Animations avancées en24.8 avec les vitesses de la table PSP ; progression liée au palier de déplacement. Les changements de marche/course/sprint conservent la phase. Les animations d’action restent décodées mais sans logique de gameplay associée.

L’entrée transparente est une couleur-clé magenta dans la palette RGB555. Le chargement conserve l’alpha de l’atlas, neutralise le RGB transparent et le rendu utilise un filtrage ponctuel pour éviter les contours roses et la contamination des rectangles voisins.

## Maniement à pied

Adaptation Rust de ctw-native Player::UpdateSpeed/TurnTo/AnimateWalkRunCycle : base819/4096, paliers0/1/2/3, accélération sur8étapes, freinage sur4, rotation limitée à4000unités par étape, coupure de vitesse lors d’un changement de direction supérieur à19000unités hors sprint. Vitesses nominales ~6/12/18unités/s. Intégration17/512 par étape, gravité0x1ED0/4096 par étape à30Hz. Les tests couvrent accélération, marche limitée, freinage, relâchement sprint, demi-tour et chute/atterrissage.

Les contacts reposent encore sur les boîtes/cylindres/triangles PSP, poussées et sous-pas de0,1unité. Ce n’est pas le balayage complet de sphère en virgule fixe du jeu. Pas encore de nage, saut/plongeon, réactions de combat ou interactions avec véhicules.

## Programme PSP et recompilation

EBOOT.BIN : 3 692 496 octets, module CTW, signature `~PSP`, tag `d9160bf0` ; taille ELF déclarée 3 692 160. SHA-256 : `9c8b36ff99e959e17ec8a4c3c9a9f0d11f1e91ac4c2652080e179a6517c92f74`.

BOOT.BIN contient uniquement des zéros. Les 167 ressources OVERLAYS.PAK sont indexées ; 166 se décompressent. Elles ne sont pas analysées comme fonctions exécutables pour le moment.

Le traitement du PRX, les relocations/imports PSP, l'analyse Allegrex/VFPU et la traduction de routines ne sont pas encore implémentés. Le rendu des ressources n'est pas une preuve d'exécution ou de recompilation du jeu.

## Admiral d’essai

PAK29 : countu32=106, fiches312octets. Première fiche : IDmodèleu16 à+2=30, nom à+8=`./Admiral.vehicle`. Dimensionsi16 à+0x28/+0x2a/+0x2c :4528,12536,3362. La formule de demi-tailles XY reprend ctw-native Vehicle::init (`((dim*0x2332>>12)>>1)/4096`), Z=dim/4096. Modèle30 :7nœuds,14matériaux,14strips,913sommets ; textures2 et31. L’assise au sol et la boîte stationnaire sont des adaptations de test, pas la simulation PSP.

## Franchissement : correction par référence PSP

Paire Huang **416/417** = base276 + 0x8c/0x8d ; dispatch EBOOT0x0883d970..97c. 12 frames, rate3840 (24.8), flags0x21. Concordance visuelle avec la vidéo utilisateur à16,4–17,2s,20,6–21,4s et42,3–43,1s. Cycle du prototype0,8s ; ancienne paire424/425 abandonnée pour le franchissement. Le cycle est normalisé sur la trajectoire reconstruite, pas une reproduction de l’opérateur physique MIPS. Voir VIDEO-REFERENCE.md.
