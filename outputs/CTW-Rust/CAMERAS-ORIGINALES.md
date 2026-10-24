# Caméras PSP, mobile et DS — 21 octobre 2026

F3 ou le bouton supérieur parcourt **PSP → Mobile → DS → PSP**. Le choix est enregistré dans `data/camera-settings.json`. Huang ne change pas de position. Maintenir **C** demande un recentrage ; F2 conserve la caméra libre. `--camera psp|mobile|ds` impose le profil au lancement.

Le suivi à pied et les états de recentrage ont été adaptés en Rust après lecture des trois programmes fournis. Il ne s'agit pas seulement de trois hauteurs différentes. **La parité complète avec les trois jeux originaux n'est pas encore validée.**

## Comportements intégrés

| Profil | Hauteur de référence | Suivi ordinaire, plafond par tick | Recentrage automatique à pied |
|---|---:|---:|---|
| PSP | 25 | 100, ou 300 si vue obstruée | Demande après 10 ticks de sprint accumulés |
| Mobile | 35, échelle appareil 1 | 33, ou 100 si vue obstruée | Après 30 ticks ; direction relative >5000 unités en valeur absolue, même signe, sprint maintenu |
| DS | 25 | 100, ou 300 si vue obstruée | Pas de compteur PSP retrouvé dans le contrôleur étudié ; commande explicite |

Les compteurs sont consultés avant leur mise à jour : demandes aux ticks 11 et 31 lorsque les conditions sont continues. Simulation du port à 30 Hz. La demande automatique PSP/mobile utilise une rotation amortie distincte du suivi ordinaire, avec engagement du côté du virage au-delà d'environ 120°. Le recentrage explicite suit une autre branche, accélérée pour les grands demi-tours ; sur DS cette accélération nécessite aussi un déplacement. Le ressort de position devient plus rapide pendant le recentrage. La fin du recentrage et la remise à zéro au déplacement suivent les branches identifiées hors cinématiques.

## Repères dans les programmes fournis

Adresses virtuelles, pas offsets des fichiers. Les noms PSP/DS sont des identifications par analyse ; le programme Android conserve des symboles. Les scripts et désassemblages de recherche restent dans `../../work/original-cameras/` et `../../work/psp-analysis/`.

- **PSP ULES01347 1.01** : hauteur 25 à `0x089c71d8`. `Update` à `0x088cd17c`, promotion du mode à `0x088cd868`, branche recentrage à `0x088cd91c`. `TryToFaceAngle` à `0x088cdce4`, mode automatique à `0x088ce010`, explicite à `0x088ce098`. Compteur du contrôleur à +0x41 : seuil 10 à `0x089cf36c`, demande yoke+0xb2 à `0x089cff50`, incrémentation à `0x089d0398`. Voir `psp-recenter.txt`.
- **APK fourni, ARM64** : `FollowPedCam::Update` à `0x72dfc0`, demande yoke+0xc6 à `0x72e228`, `TryToFaceAngle` à `0x72e578`. Division de vitesse par 3 à `0x72dc48`, hors cinématiques. `cPlayerOnFoot::IsToAutoCentre` à `0x71adc0`, compteur +0x5d ; contraintes du stick à `0x71bd48..9c`. Voir `android-followcam.txt` et `android-onfoot.txt`. `GetCamHeightScale` à `0x79fd08` distingue 1 et 0,72 selon la classe d'appareil ; le port conserve 1 et la hauteur de base 35 issue de ctw-native. L'équivalence avec chaque téléphone/tablette n'est pas validée.
- **DS YGXP, ARM9 Thumb** : constructeur à `0x020c63d4`, hauteur 25 passée à `0x02034358..62`. `Update` à `0x020c6754`, demande yoke+0x60 à `0x020c69a2`, état unique caméra+0x120. `TryToFaceAngle` à `0x020c6cb0`, garde de déplacement pour grands angles à `0x020c6e3a`. Commande 0x11 dans le contrôleur à `0x02075848`, écritures yoke+0x60 à `0x02075918` et `0x02075a3c`. Voir `ds-height.txt`, `ds-controller.txt`, `ds-update.txt`, `ds-facing.txt`.

## Empreintes SHA-256

- ISO PSP : `8c62ef8d7263ff0d42db40901927de4e86afd02f3d3450c3a96d0e2bc8d5168e`
- ELF PSP déchiffré : `9eaf850f09d743e5ecfc8d1c6a0a8c290399142319b1ef3a732a03d7278bffe0`
- ROM DS : `0d58799e2fada7690a105bc50029b272f2e373838b19e2c6f0efae09f12cb571`
- ARM9 DS : `b45dcf216dfbbc3d0dcd7ad16787442a93d29db70f663fab5ecedc92975f1a5d`
- APK fourni : `59ffd174af91ca068d03164210583559a6e46d572b63d5d01d4c37719a558979`
- libGame.so ARM64 : `c75e300a66317291c6afbeb4df6e0e0f9d8aef5d6ecd7bd43804a4e675d10936`

APK et ROM DS lus statiquement, sans installation ni exécution. Leur présence n'est pas nécessaire au lancement du port : la carte et Huang restent issus de l'ISO PSP. Le nom de l'APK ne prouve pas qu'il s'agit d'une distribution officielle non modifiée. Les originaux et extractions restent locaux, hors publication du code.

## Validation et limites

48 tests passent, dont quatre nouveaux contrôles : délais et conditions PSP/mobile, demi-tour et comportement DS, recentrage à l'arrêt, retour au repos et indépendance de la caméra libre. Trois essais de sprint sur la carte PSP, un par profil, se terminent sans erreur de modèle ; les captures `recenter-psp.png`, `recenter-mobile.png` et `recenter-ds.png` ont été inspectées. Leurs trajectoires diffèrent puisque les directions sont relatives à la caméra : leurs angles finaux ne sont pas une comparaison directe de vitesse.

Le muret en sprint PSP et la montée sur l'Admiral avec caméra DS terminent chacun une action, sans interruption ni pénétration détectée sur 24 échantillons. La fenêtre interactive a été observée en profil mobile ; une intervention utilisateur a interrompu le clic automatisé. Un choix DS a ensuite été constaté dans le fichier de préférence de cette session. Cela ne constitue pas une validation automatisée complète du cycle F3 et du rechargement au lancement.

Restent à porter/vérifier : balayages de collision de caméra d'origine, branches armes/visée, cinématiques, véhicules, initialisation et transitions exactes, cadrage/projection selon écran, échelle des appareils mobiles. Les appels de collision sont encore simplifiés ; les sinusoïdes passent par des flottants. Aucune comparaison image par image avec DS/mobile ni trace instrumentée des trois originaux ne démontre une parité intégrale. Le bouton désigne le profil de suivi, pas une émulation du matériel ni un portage complet de chaque version.
