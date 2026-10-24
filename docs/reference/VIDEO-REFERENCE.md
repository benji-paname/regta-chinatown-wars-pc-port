# Comparaison avec la vidéo PSP — 20 octobre 2026

Référence utilisateur locale : `C:/Users/BENJAMIN/Videos/NVIDIA/PPSSPP/PPSSPP 2026.10.20 - 06.58.54.02.mp4`. Benjamin précise qu’elle est filmée au même endroit que le prototype et confirme les changements après leur présentation.

Durée59,668s, image2560×1440, environ60images/s. SHA-256 : `64481a836123aec6bd6e86eca54a2d4d001d5a4053ef6250d16f2ba79e36b7ce`. Lecture locale avec PyAV19.0.1 ; aucune transmission de la vidéo. Planches de comparaison conservées dans `../../work/video-reference/` : overview.jpg, wall-sequence.jpg, car-sequence.jpg, park-sequence.jpg, candidate-animations.png.

## Animation

Passages utiles : muret vers16,4–17,2s et20,6–21,4s ; voiture vers42,3–43,1s. Huang lève les bras et passe les jambes sur le côté. La comparaison des séquences PSP extraites identifie416/417, et non l’ancien candidat424/425. Douze frames, rate3840, flags0x21 ; cycle du prototype fixé à0,8s.

Confirmation partielle dans EBOOT déchiffré : à0x0883d970/978, le programme sélectionne0x8c/0x8d, soit416/417 pour le corps base276. À0x0883d950, appel de0x0883b7ec avec mode5. Sa table à0x08b3ea50 mène à0x0883bee4 ; cette branche prépare un vecteur puis appelle0x08a62d78. Le sens exact de toutes les grandeurs et l’opérateur physique complet ne sont pas encore établis. Le littéral0x1e000 ne doit notamment pas être interprété comme une hauteur de saut.

Le prototype réutilise la bonne séquence visuelle sur sa trajectoire en trois phases. Les seuils, la trajectoire, les contacts et le comportement sur toutes les formes d’obstacle ne sont donc pas prouvés identiques à la PSP.

## Caméra

Dans le constructeur de cPlayer,0x089c71d8..71dc construit0x19000, soit25 en20.12 ; cette valeur est passée au constructeur caméra0x088cc1dc à0x089c71ec, puis stockée à+0x19c. La hauteur35 du prototype précédent venait de ctw-native.

À0x088cd954..978, les couples diviseur/plafond de rotation sont2/100 en vue libre et3/300 en cas d’obstruction. Le plafond est transmis sans division supplémentaire à0x088cdce4. Le profil mobile conserve la réduction par3 de ctw-native, soit33/100. La branche d’entrée supplémentaire avec plafond910 et les états de recentrage restent à analyser.

La comparaison au même muret confirme une amélioration du cadrage : hauteur relative de Huang et échelle des piliers proches de la vidéo. Le prototype reste en1100×720, la vidéo en2560×1440 ; cette comparaison n’établit pas une parité pixel à pixel. FOV60°, requêtes de collision et branches non portées restent à vérifier.

F3 ou le bouton supérieur sélectionne PSP/mobile sans déplacer Huang. PSP est le défaut ; la préférence interactive est enregistrée dans le dossier des rapports. Les essais déterministes ignorent cette préférence et acceptent `--camera psp|mobile`. F2 reste la caméra libre. Cette section décrit la première correction vidéo ; la version suivante ajoute DS et les états de recentrage issus des trois programmes. Voir CAMERAS-ORIGINALES.md.

## Contrôles après modification

- 44 tests automatisés passent, dont la hauteur et la vitesse de rotation PSP, le changement de profil et la conservation de la caméra libre.
- Muret avec reprise sprint en PSP, toit d’Admiral en PSP, muret en mobile : une action achevée chacun, sans interruption,24 échantillons libres chacun ; animation416 pendant l’action.
- Captures locales : camera-psp.png, camera-mobile.png, vault-psp-mid.png ; rapport de cette étape archivé dans data/build-validation-video.json.
- Fenêtre réelle observée. L’automatisation a détecté une intervention utilisateur avant le clic de changement de caméra ; ce clic et la persistance ne sont pas revendiqués comme tests automatisés accomplis. Benjamin a ensuite confirmé que les changements lui conviennent.

Le fichier ELF, la vidéo et les planches extraites restent des matériaux locaux de recherche. Les adresses ci-dessus concernent exclusivement ULES01347v1.01, ELF SHA-256 `9eaf850f09d743e5ecfc8d1c6a0a8c290399142319b1ef3a732a03d7278bffe0`.
