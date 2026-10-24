<!-- Jalon 2026-10-24 10:42 +02:00 -->
Sources actives dans src/ a la racine. Workspace Cargo racine, manifeste compatible outputs/CTW-Rust/Cargo.toml. Un cache target/. Compile avec -D warnings : 119 tests passes, 0 echec, 0 avertissement. Commande de reprise : .\Compiler.ps1 -Test depuis la racine. Nettoyage pas encore execute. Prochain travail : tests sans tables privees, docs publiques, nettoyage verifie. Premier commit 7d3d173.

<!-- Restructuration en cours, 2026-10-24 10:38 +02:00 -->
Baseline confirmee : 119 tests reussis, 0 echec, aucun avertissement dans work/restructure-baseline.log. Aucune restructuration du code effectuee a ce jalon. Reprise : depuis la racine, lancer .\outputs\CTW-Rust\Compiler.ps1 -Test. Prochaine etape : creer le commit initial, puis migrer les sources et rendre les tests independants des tables locales Android. Ne pas confondre ces tables avec des donnees PSP validees.

# Guide de Reprise Immédiate pour Modèle d'IA & Développeur
## Projet : Grand Theft Auto: Chinatown Wars PSP (ULES01347 1.01) — Recompilation Statique Rust PC

> [!IMPORTANT]
> **RÈGLE FONDAMENTALE DU PROJET (Consigne Formelle de Benjamin)** :
> Ce projet est une **recompilation statique native 100% fidèle** de *Grand Theft Auto: Chinatown Wars* PSP (version Europe ULES01347 1.01) en Rust pour PC (Windows x64), selon les standards stricts de projets reconnus comme *Skate3recomp*, *Forza Horizon 1 - ReImagined* et *FH2 Azimuth*.
> **Rien ne doit être inventé ni approximé**. Chaque comportement, structure de données, équation en virgule fixe, table binaire, IA ou échantillon sonore provient directement des binaires originaux du jeu et des sources C++ de référence décompilées (`work/upstream-ctw/port/game/`).
> Règle de compilation : **0 avertissement (`#[warn(dead_code)]`, `#[warn(unused)]`), 100% de tests unitaires réussis** via `Compiler.ps1 -Test`.
> Format des réponses : En français, avec des liens cliquables GitHub markdown utilisant impérativement le schéma `file:///` pour tous les fichiers et symboles de code.

---

### 1. Démarrage Rapide en 30 Secondes

#### Compiler et exécuter tous les tests unitaires :
```powershell
Set-Location 'C:\Users\BENJAMIN\Documents\Codex\2026-10-17\tu-voit-le-projet-vcs-rust\outputs\CTW-Rust'
.\Compiler.ps1 -Test
```
> **Résultat attendu** : `119 passed; 0 failed; 0 ignored; finished in 0.08s` (0 warning, 0 error).

#### Exécuter le moteur en mode headless avec capture d'image bitmap :
```powershell
.\CTW-Rust.exe --iso "F:\JEUX EMULATION\PSP GAME\Grand Theft Auto - Chinatown Wars (Europe) (En,Fr,De,Es,It).iso" --headless --frames 60 --capture run_combat_test.bmp
```

#### Lancer le jeu en mode fenêtré interactif complet :
```powershell
.\CTW-Rust.exe --iso "F:\JEUX EMULATION\PSP GAME\Grand Theft Auto - Chinatown Wars (Europe) (En,Fr,De,Es,It).iso"
```

#### Lancer l'émulateur PPSSPP original de référence (comparaison visuelle 1:1) :
```powershell
Start-Process "C:\Users\BENJAMIN\Desktop\Grand Theft Auto_ Chinatown Wars PSP OG.lnk"
```

---

### 2. État d'Avancement des Systèmes (119/119 Tests Verts)

1. **Moteur d'Explosions & Débris (`src/explosions.rs`, `explosions.cpp`, `vehicledebris.cpp`)** :
   - Formule exacte de falloff en virgule fixe Q12 d'après `AffectEntities`.
   - Dégâts réduits pour le joueur avec knockdown, souffle balistique sur les piétons, et projection ascendante 3D des véhicules le long de l'axe vertical Z (`direction[2] = 4096`).
   - Débris de carrosserie détachables (`createDebris`, portes, coffre, capot) avec dynamique quaternionique, friction et rebond au sol.
   - Particules multi-couches : flash additif expansif blanc/jaune (`GL_SRC_ALPHA, GL_ONE`), boules de feu freinées par l'air (`0x199`), fumée noire volumétrique et étincelles paraboliques.

2. **Police & Takedowns (`src/police.rs`, `wanted_tables.bin`)** :
   - Mécanique authentique de Chinatown Wars : détruire les voitures de poursuite policière (en les écrasant contre des obstacles) valide un takedown et fait baisser l'indice de recherche.
   - 1 à 6 étoiles avec compteurs de criminalité et IA de poursuite avec sirènes stroboscopiques.

3. **Arsenal Complet 28 Armes & Ciblage Auto (`src/weapons.rs`, `weapon_tables.bin`)** :
   - 28 armes issues de `cWeaponInfo::msWeaponInfos` réparties sur 11 slots d'inventaire.
   - Gestion des chargeurs, cadence de tir, et rechargement temporisé.
   - Cône de visée automatique $\pm 60^\circ$ jusqu'à 40 m avec réticule dynamique coloré.
   - Sélection d'armes avec `Tab`, molette de la souris, ou `D-Pad Gauche / Droite`.

4. **Système de Radio PSP Authentique (`src/radio.rs`, `radio_tables.bin`)** :
   - Table binaire authentique `radio_tables.bin` (Format `CTWRAD2\0`, 11 stations officielles, 33 flux audio MP3 de 40 octets, sprites de volume).
   - Gestionnaire `RadioManager` implémentant le cyclage séquentiel des stations (0: Truthwave, 1: Radio Broker, 2: Tortoise, 3: Turntables on the Hudson, 4: The Prairie Cartel, 5: DJ Khalil, 6: Tortoise Special, 7: Wonderwheel, 8: Anvil, 9: SinoWav FM, 10: DFA Records, 11: Radio Éteinte).
   - Bandeau HUD stylisé Chinatown Wars au volant (`rx = W/2 - 130`, typographie dorée) et touches de commande au volant (`N`/`,`/`D-Pad Haut`, `B`/`;`/`D-Pad Bas`).

5. **Météo Dynamique & Éclairage Jour/Nuit (`src/weather.rs`, `weather_tables.bin`)** :
   - Parseur binaire de `weather_tables.bin` (Format `CTWWTHR1`, matrice 8x8 de probabilités cumulées de `cTimeCycle::msWeatherChanceTable`, 5 échantillons de tonnerre : 815..819).
   - Gestionnaire `WeatherManager` gérant les 8 types de météo (Ensoleillé, Très ensoleillé, Nuageux, Couvert, Pluie, Orage, Brumeux, Flash).
   - Cycle circadien 24h avec éclairage ambiant (pic à midi, nuit bleutée).
   - Orage authentique (`cTimeCycle::UpdateLightning`) : éclair stroboscopique plein écran avec tonnerre retardé simulant la vitesse du son.

6. **Statistiques Authentiques PSP & Sauvegarde Binaire (`src/stats.rs`, `stats.h`, `stats.cpp`)** :
   - Structure binaire exacte de la section de sauvegarde PSP (`StatsData`, 0xFC = 252 octets).
   - Mots de 64 bits bit-packés avec wrapping arithmétique strict (12 bits, 8 bits, 20 bits pour `word_c0`, `word_c8`, `word_d0`, `word_d8`).
   - Calcul authentique du rang criminel (`calc_criminal_rating`) avec mapping des textes GXT (0x5D2 à 0x617).
   - Valeur média (`calc_media_value`), précision de tir Q12 (`calc_accuracy`), arme préférée, stations radio les plus/moins écoutées.
   - Suivi complet de la progression de sauvegarde `SaveProgress` (30 cascades uniques USJ, 100 caméras de sécurité, 21 planques, 81 revendeurs de drogue, 6 drogues, 8 rencontres aléatoires, 16 courses de rue).

---

### 3. Feuille de Route pour la Suite Immédiate

1. **Système de Planques & Commerce de Marchandises (`src/safehouse.rs`, `src/drugs.rs`)** :
   - Les 6 drogues authentiques : Heroin, Coke, Weed, Acid, Ecstasy, Downers.
   - Les 81 dealers dispersés dans Liberty City avec préférences de gangs et cours fluctuants.
   - Stash box illimitée dans les 21 planques pour protéger la marchandise des arrestations (Busted).
   - Messages du PDA (alertes de surstock ou de pénurie).

2. **Moteur de Missions & Scripting (`src/missions.rs`, `gameplayapi.cpp`)** :
   - Donneurs d'ordres narratifs : Kenny Lee, Chan Jaoming, Zhou Ming, Hsin Jaoming, Wade Heston.
   - Blips de contact dorés sur le radar et marqueurs d'objectifs au sol.
   - Cinématiques en vignettes de comics avec textes DS_GXT.

3. **Intégration du Sauveur de Progression (`src/savegame.rs`)** :
   - Écriture/Lecture du bloc de sauvegarde PSP de 0xFC octets pour conserver les records et statistiques du joueur.
