# Textures améliorées prioritaires

Le dossier `CW-Texture-Pack-master` fourni par Benjamin a été importé localement. **47 images améliorées remplacent 48 ressources PSP** ; une image de cinématique correspond à deux ressources identiques. Le jeu charge les versions améliorées en priorité, puis les images de l'ISO lorsqu'aucun remplacement vérifié n'existe.

Les remplacements incluent les polices, le radar, les armes et boutons d'interface, des objets, panneaux, menus, cinématiques et mini-jeux. Les ressources des fonctions encore absentes du prototype sont préparées ; elles ne rendent pas ces fonctions jouables à elles seules. Au point de départ actuel, le radar, le poing et la police du HUD sont effectivement remplacés.

## Sélection des textes et doublons

Les atlas numérotés `Text/246.png` à `Text/250.png` et `Text/259.png` à `Text/261.png`, en **4096 × 4096**, correspondent aux empreintes PSP de l'index. Ils sont prioritaires. Les variantes plus petites `Text_1.png` à `Text_4.png`, non référencées, sont écartées pour éviter de superposer ou de substituer des dispositions différentes. Les mesures des caractères proviennent toujours des fichiers PSP ; les textes ne deviennent pas huit fois plus grands à l'écran.

Le symbole monétaire vert de `Props/Cash.png` remplace la ressource verte 275, pas la variante bleue 309 qui partage les mêmes indices de pixels. Les images Armor, Bat, Micro_SMG, Pistol et Tracking, non référencées, ont été comparées visuellement aux ressources PSP pour retrouver leur correspondance. Trois chemins de mini-jeux incorrects dans l'index ont été résolus vers les fichiers présents.

`Text/Loading_Text.png` est conservé dans le pack source : son équivalent n'a pas été identifié dans les textures chargées du prototype. Il n'est pas appliqué à une police arbitraire.

## Contenu absent

L'index du pack cite **2303 chemins PNG absents du dossier fourni**. Les répertoires Map, Pedestrians et Vehicles sont absents : la carte géographique, Huang, les passants et les voitures utilisent donc leurs images PSP. La **bordure et les symboles du radar**, eux, disposent bien d'un remplacement HD. Aucun agrandissement automatique inventé n'est substitué aux fichiers manquants.

## Chargement

Le pack préparé est dans `data/texture-pack/`. Le dossier du Bureau peut être déplacé après import : le jeu utilise sa copie locale convertie sans perte en RGBA compressé. Aucun Python n'est requis pour jouer. `manifest.json` conserve les correspondances, dimensions et empreintes des originaux et des PNG. Le chargeur vérifie l'original, sa palette et le fichier amélioré avant usage, conserve le canal alpha et laisse les données PSP utilisées par la simulation intactes.

Les lanceurs habituels et `CTW-Rust.exe` activent automatiquement le pack. `--original-textures` sert à comparer avec l'ISO seule ; `--texture-pack chemin` sélectionne un autre dossier **préparé au même format**, pas directement un dossier PPSSPP brut.

L'import repose sur l'index local et les empreintes de texels compatibles avec [TextureDecoder de PPSSPP](https://github.com/hrydgard/ppsspp/blob/master/GPU/Common/TextureDecoder.cpp). Les clés de palettes et dimensions ont été vérifiées avec [TextureCacheCommon](https://github.com/hrydgard/ppsspp/blob/master/GPU/Common/TextureCacheCommon.h). Le README du pack fourni est conservé dans `data/texture-pack/PACK-README.md`.
