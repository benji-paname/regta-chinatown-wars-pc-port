# Catalogue Officiel des 166 Code Overlays PSP (ULES-01347 / ULUS-10490)

Grand Theft Auto: Chinatown Wars PSP charge son code exécutable additionnel dynamiquement depuis `PSP_GAME/USRDIR/WAD/OVERLAY/CTW_PSP_UMD_MASTER_SCEE/OVERLAYS.PAK`.

Format de stockage : conteneur de blocs `20KP`, compression zlib `10SA`, exécutable ELF MIPS 32 bits (type `0xFFA8`) avec section unique `.overlay_<nom>` allouée à une adresse fixe.

## Répartition des 2 Slots Mémoire PSP

| Slot | Adresse Base | Capacité Max | Nombre | Usage |
|---|---|---|---|---|
| **Slot 0** | `0x08b87460` | 172 Ko | 114 | Missions d'histoire, missions secondaires, modes de jeu |
| **Slot 1** | `0x08bb2480` | 87 Ko | 52 | Applications PDA, HUD, minijeux tactiles/stick, interfaces |

---

## Liste Complète des 166 Code Overlays Extraits

| ID | Nom de l'Overlay | Slot Mémoire | Taille Compressée | Taille Décompressée | Description / Usage Authentique |
|:---:|:---|:---:|:---:|:---:|:---|
| 001 | `ammozoneapp` | Slot 1 (App) | 17 285 o | 42 716 o | Boutique d'armes en ligne Ammu-Nation sur le PDA |
| 002 | `autosavequeryapp` | Slot 1 (App) | 1 604 o | 4 532 o | Dialogue de confirmation de sauvegarde automatique |
| 003 | `bik_b01` | Slot 0 (Mission) | 45 058 o | 131 076 o | Mission Biker / Angels of Death 1 |
| 004 | `bik_b02` | Slot 0 (Mission) | 28 848 o | 84 380 o | Mission Biker / Angels of Death 2 |
| 005 | `bik_b03` | Slot 0 (Mission) | 28 735 o | 78 972 o | Mission Biker / Angels of Death 3 |
| 006 | `bik_b04` | Slot 0 (Mission) | 37 055 o | 104 428 o | Mission Biker / Angels of Death 4 |
| 007 | `bombdisposalapp` | Slot 1 (App) | 7 066 o | 17 000 o | Minijeu de désamorçage de bombe |
| 008 | `bombplantingapp` | Slot 1 (App) | 4 013 o | 8 552 o | Minijeu de pose d'explosif C4 / détonateur |
| 009 | `bugplantingapp` | Slot 1 (App) | 2 779 o | 7 168 o | Minijeu d'installation de micro-espion |
| 010 | `carbombfittingapp` | Slot 1 (App) | 2 921 o | 9 140 o | Minijeu de piégeage de véhicule |
| 011 | `caryard` | Slot 0 (Mission) | 7 775 o | 19 884 o | Concessionnaire Boabo Auto Merchant |
| 012 | `charviewer` | Slot 0 (Mission) | 1 401 o | 2 536 o | Visualiseur de personnages de test |
| 013 | `chinesedelivery` | Slot 0 (Mission) | 19 482 o | 52 276 o | Job secondaire : livraison de nouilles à scooter |
| 014 | `contactsapp` | Slot 1 (App) | 3 100 o | 6 028 o | Carnet d'adresses des contacts du PDA |
| 015 | `controlconfigapp` | Slot 1 (App) | 5 271 o | 10 220 o | Menu de configuration des commandes PSP |
| 016 | `deatharrestapp` | Slot 1 (App) | 894 o | 1 552 o | Écran Busted / Wasted |
| 017 | `deedapp` | Slot 1 (App) | 2 039 o | 3 800 o | Gestion des actes de propriété immobilière |
| 018 | `desktopex` | Slot 0 (Mission) | 4 541 o | 10 684 o | Bureau système PSP étendu |
| 019 | `dlcnotifyapp` | Slot 1 (App) | 706 o | 1 360 o | Notifications de contenu téléchargeable |
| 020 | `drugvanslashapp` | Slot 1 (App) | 5 084 o | 9 192 o | Minijeu de découpe des portes de fourgon de drogue |
| 021 | `dummyapp` | Slot 1 (App) | 1 772 o | 3 684 o | Application factice réservée |
| 022 | `dumpsterweaponstashapp` | Slot 1 (App) | 16 286 o | 39 744 o | Minijeu de fouille des bennes à ordures rouges/vertes |
| 023 | `emailapp` | Slot 1 (App) | 15 517 o | 35 556 o | Messagerie PDA (scénario, alertes pénuries/surplus) |
| 024 | `endgame` | Slot 0 (Mission) | 1 728 o | 4 852 o | Épilogue et générique de fin |
| 025 | `enginesabotage` | Slot 0 (Mission) | 6 367 o | 17 080 o | Sabotage mécanique de moteurs |
| 026 | `environmentsetups` | Slot 0 (Mission) | 8 585 o | 32 944 o | Configurations d'ambiance et météo scénarisées |
| 027 | `explodingpolicevansapp` | Slot 1 (App) | 5 533 o | 12 312 o | Minijeu d'évasion de fourgon cellulaire |
| 028 | `firefighter` | Slot 0 (Mission) | 26 636 o | 79 584 o | Missions d'autopompe de pompier |
| 029 | `friendlistapp` | Slot 1 (App) | 5 003 o | 11 416 o | Liste d'amis multijoueur Ad-Hoc PSP |
| 030-040 | `hes_a01 .. hes_c01b` | Slot 0 (Mission) | ~30 Ko | ~80 Ko | 11 missions du Détective Wade Heston (LCPD) |
| 041 | `hobotron` | Slot 0 (Mission) | 12 444 o | 38 592 o | Système des fouilles de poubelles |
| 042-043 | `hostscreenpspapp` | Slot 1 (App) | ~5 Ko | ~12 Ko | Salons multijoueur Ad-Hoc PSP |
| 044 | `hotwiringapp` | Slot 1 (App) | 12 173 o | 27 368 o | Minijeu de démarrage aux fils / tournevis / transpondeur |
| 045 | `hudapp` | Slot 1 (App) | 20 482 o | 49 704 o | Interface tête haute PSP d'origine |
| 046 | `internettradingapp_psp` | Slot 1 (App) | 6 903 o | 15 472 o | Terminal boursier / commerce en ligne |
| 047-053 | `jao_a01 .. jao_a07` | Slot 0 (Mission) | ~25 Ko | ~65 Ko | 7 missions de Chan Jaoming |
| 054-067 | `ken_a01 .. ken_b04` | Slot 0 (Mission) | ~25 Ko | ~70 Ko | 14 missions de Wu Kenny Lee (Triades) |
| 068-077 | `les_a01 .. les_b03` | Slot 0 (Mission) | ~30 Ko | ~85 Ko | 10 missions de Lester Leroc (infiltré Angels of Death) |
| 078-084 | `libertygames` | Slot 0 (Mission) | ~15 Ko | ~40 Ko | Épreuves olympiques du parc de Liberty City |
| 085 | `lockup` | Slot 0 (Mission) | 4 489 o | 11 392 o | Garages de stockage de véhicules |
| 086-088 | `med_a01 .. med_b03` | Slot 0 (Mission) | ~20 Ko | ~55 Ko | Missions ambulance paramédic |
| 089 | `metalfinderapp` | Slot 1 (App) | 14 067 o | 34 812 o | Détecteur de métaux sur la plage |
| 090 | `molotovcocktailapp` | Slot 1 (App) | 9 864 o | 24 064 o | Minijeu de remplissage de cocktails molotov à la pompe |
| 091 | `monumentapp` | Slot 1 (App) | 4 947 o | 11 064 o | Lions de Fo / Statues cachées |
| 092 | `multiplayerapp` | Slot 1 (App) | 16 029 o | 41 844 o | Gestionnaire multijoueur Ad-Hoc PSP |
| 093 | `noodleexchangeapp` | Slot 1 (App) | 7 514 o | 16 148 o | Bourse aux nouilles |
| 094 | `paramedicapp` | Slot 1 (App) | 11 601 o | 23 836 o | Interface de défibrillateur / soins |
| 095 | `paynsprayapp` | Slot 1 (App) | 8 518 o | 21 000 o | Minijeu d'atelier de peinture Pay 'n' Spray |
| 096 | `pdaapp` | Slot 1 (App) | 24 532 o | 62 108 o | Menu racine du PDA de Huang Lee |
| 097 | `pipebombplantingapp` | Slot 1 (App) | 4 233 o | 9 124 o | Minijeu de fabrication et pose de tuyaux explosifs |
| 098 | `policebribeapp` | Slot 1 (App) | 3 147 o | 7 004 o | Pots-de-vin à la police pour effacer les étoiles |
| 099 | `quicktradingapp` | Slot 1 (App) | 19 865 o | 44 948 o | Interface transactionnelle de deal de rue avec les 81 dealers |
| 100-112 | `rac_a01 .. races_north` | Slot 0 (Mission) | ~22 Ko | ~60 Ko | Courses de rue illégales de Liberty City |
| 115 | `radioapp` | Slot 1 (App) | 6 450 o | 11 972 o | Sélecteur de stations de radio |
| 116 | `rampage` | Slot 0 (Mission) | 20 643 o | 56 796 o | Carnages / paquetages de destruction |
| 117 | `replaylevelapp` | Slot 1 (App) | 11 892 o | 23 640 o | Tableau de rejouabilité des missions au panneau d'affichage |
| 118 | `restartengineapp` | Slot 1 (App) | 4 286 o | 8 380 o | Redémarrage de moteur calé |
| 119 | `rifleconstructionapp` | Slot 1 (App) | 5 480 o | 15 016 o | Minijeu d'assemblage du fusil de sniper dans sa mallette |
| 120-127 | `rnd_ch01 .. rnd_ch08` | Slot 0 (Mission) | ~18 Ko | ~50 Ko | 8 rencontres aléatoires de rue authentiques |
| 128 | `safecrackingapp` | Slot 1 (App) | 6 429 o | 14 032 o | Minijeu de crochetage de coffre-fort au stéthoscope |
| 129 | `safehouseapp` | Slot 1 (App) | 36 115 o | 89 104 o | Intérieur de planque (lit, boîte de réserve, garde-robe) |
| 130 | `safehousesalehandler` | Slot 1 (App) | 3 575 o | 9 140 o | Gestionnaire d'achat immobilier des 21 planques |
| 131 | `scratchcardapp` | Slot 1 (App) | 10 492 o | 24 480 o | Minijeu de grattage des tickets de loterie aux kiosques |
| 134 | `searchingbodiesapp` | Slot 1 (App) | 4 944 o | 11 268 o | Minijeu de fouille des cadavres |
| 135 | `settingsapp` | Slot 1 (App) | 4 635 o | 12 100 o | Paramètres audio, affichage et options |
| 136 | `shootingrange` | Slot 0 (Mission) | 35 921 o | 119 916 o | Stand de tir d'Ammu-Nation |
| 138 | `sinkingcarapp` | Slot 1 (App) | 12 484 o | 28 016 o | Minijeu de bris de pare-brise sous l'eau (Prologue) |
| 139 | `smashelectroniclockapp` | Slot 1 (App) | 6 789 o | 16 920 o | Minijeu de piratage de clavier numérique de sécurité |
| 141 | `tattooingapp` | Slot 1 (App) | 8 656 o | 34 496 o | Minijeu d'application de tatouage à l'aiguille |
| 143 | `taxi` | Slot 0 (Mission) | 12 387 o | 32 004 o | Job de chauffeur de taxi de Liberty City |
| 144-148 | `timetrial01 .. 05` | Slot 0 (Mission) | ~24 Ko | ~65 Ko | Contre-la-montre sur circuits fermés |
| 149 | `tradestatsapp` | Slot 1 (App) | 21 644 o | 47 784 o | Graphiques des cours et historiques des 6 drogues |
| 152 | `vigilante` | Slot 0 (Mission) | 21 179 o | 53 744 o | Missions d'autodéfense en voiture de police |
| 156-157 | `xin_01 .. xin_02` | Slot 0 (Mission) | ~12 Ko | ~30 Ko | Missions exclusives PSP : Xin Shan |
| 158-166 | `zho_a01 .. zho_c01` | Slot 0 (Mission) | ~30 Ko | ~85 Ko | 9 missions de Zhou Ming |
