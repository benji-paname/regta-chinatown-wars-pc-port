# 🎬 Kit Vidéo YouTube & Promotion — ReGTA Chinatown Wars PC Port

Guide complet et clé en main pour enregistrer, monter, publier et promouvoir la première vidéo YouTube officielle du projet afin de captiver la communauté et inviter au soutien financier.

---

## 1. 🎯 Titres Optimisés pour l'Algorithme YouTube (Fort CTR)

- **Option 1 (Recommandée — Fort impact de curiosité)** :  
  `J'ai recréé GTA Chinatown Wars sur PC en RUST ! (Sans Émulateur)`
- **Option 2 (Technique & Passionné)** :  
  `GTA Chinatown Wars en 60 FPS Natif sur PC : Le Projet ReGTA`
- **Option 3 (Communauté GTA)** :  
  `Ce jeu culte de la PSP tourne enfin nativement sur PC ! (ReGTA Chinatown Wars)`

---

## 2. 🖼️ Kit Miniature (Thumbnail) — Composition Recommandée

Une bonne miniature YouTube doit être lisible en petit format sur smartphone :
1. **Arrière-plan** : Une capture d'écran de gameplay saisissante (Huang Lee en dérapage avec l'Admiral ou en moto, ou une fusillade avec les gyrophares de police).
2. **Élément Central / Droit** : Le logo officiel transparent [`logo-transparent.png`](assets/logo-transparent.png) ou [`logo-ctw-transparent.png`](assets/logo-ctw-transparent.png) avec un léger contour blanc/doré ou lueur rouge.
3. **Texte Court & Massif (3 mots max)** :
   - `SUR PC !` (en jaune or Chinatown Wars `#FFD700`)
   - `60 FPS RUST` (en badge rouge ou blanc)
4. Les fichiers PNG haute définition (1800×1000 et 1920×520) sont disponibles dans :
   [`outputs/CTW-Rust/branding/`](../outputs/CTW-Rust/branding/) et [`docs/assets/`](assets/).

---

## 3. 🎙️ Script & Storyboard Minute par Minute (Durée : ~4 min 30)

### [0:00 - 0:35] L'Accroche Visuelle (Hook)
- **Visuel** : Écran noir pendant 2 secondes avec le carillon audio du PDA (`play_pda_notification`). Soudain, cut direct sur du gameplay ultra fluide en plein écran : Huang qui sprinte, fait une roulade évasive par-dessus un muret, vole une voiture et démarre en trombe.
- **Voix off (Benjamin)** :  
  > *« Vous vous souvenez de GTA Chinatown Wars sur PSP et Nintendo DS ? Ce chef-d'œuvre en vue du dessus avec ses guerres de triades et son ambiance de bande dessinée. Pendant plus de 15 ans, il est resté coincé sur consoles portables. Aujourd'hui, je vous présente ReGTA : le portage PC natif de Chinatown Wars, développé de zéro en Rust, qui tourne à 60 FPS sans aucun émulateur ! »*

### [0:35 - 1:20] Pourquoi Rust et pas un Émulateur ?
- **Visuel** : Comparaison côte à côte : à gauche un émulateur PSP avec ses saccades et ses bandes noires, à droite le projet ReGTA en plein écran 1080p/4K 60 FPS avec contrôles clavier/souris et manette Xbox réactifs.
- **Voix off** :  
  > *« Pourquoi ne pas simplement utiliser un émulateur ? Parce qu'un émulateur gaspille de la puissance CPU pour imiter l'architecture de la PSP. Ici, nous faisons de l'ingénierie inverse et de la recompilation statique : le code du jeu est reconstruit en Rust natif. Le moteur lit directement les textures, les modèles 3D et la géométrie de votre propre ISO de PSP originale, pour une fluidité absolue et une consommation minimale. »*

### [1:20 - 2:45] Démonstration : Ce qui fonctionne déjà dans le prototype
- **Visuel** : Enchaînement rapide de séquences de gameplay illustrant les mécaniques complètes :
  1. **Physiques & Conduite** : Dérapages, frein à main, motos qui se cabrent, suspensions dynamiques issues des données originales de Pak29.
  2. **Mouvements d'Huang** : Marche, sprint, roulade contextuelle pour éviter les voitures et franchissement d'obstacles (vaulting).
  3. **Système de Combat & Armes** : Pistolet, fusil à pompe, lance-roquettes, dispersion des tirs et impacts balistiques réactifs.
  4. **Le PDA interactif** : Ouvrir le PDA, consulter les emails de l'histoire, la boutique Ammu-Nation avec livraison, le carnet de contacts des 8 boss avec GPS automatique.
  5. **Les Missions Secondaires** : Le taxi avec compteur et pourboires, les pompiers à la lance à eau, les courses d'autodéfense de police.
  6. **Les Défis** : Les 100 caméras de sécurité LCPD destructibles, les 30 sauts uniques USJ et les carnages à munitions infinies.
- **Voix off** :  
  > *« À ce stade, nous avons déjà indexé les 166 overlays de code de la PSP et validé 152 tests unitaires avec zéro warning. La ville de Liberty City vit déjà : le trafic roule, les piétons marchent sur leurs réseaux de trottoirs, et le PDA est déjà pleinement fonctionnel. »*

### [2:45 - 3:30] La Roadmap : Ce qui arrive très vite
- **Visuel** : Aperçu des prochaines fonctionnalités en cours de développement (les minijeux tactiles de précision comme le tatouage à l'aiguille, le piégeage de véhicule sous le capot, les coffres-forts, et les 16 circuits de courses de rue illégales).
- **Voix off** :  
  > *« L'objectif final est simple : offrir l'expérience intégrale de Chinatown Wars sur PC avec la campagne complète, les sauvegardes, des packs de textures HD et le support multi-écrans. C'est un travail titanesque qui demande des centaines d'heures de recherche approfondie. »*

### [3:30 - 4:30] Appel à la Communauté & Financement (Call to Action)
- **Visuel** : Capture du site officiel [https://benji-paname.github.io/regta-chinatown-wars-pc-port/](https://benji-paname.github.io/regta-chinatown-wars-pc-port/) avec le logo officiel et les boutons de soutien Tipeee et PayPal.
- **Voix off** :  
  > *« Ce projet est développé de manière totalement transparente et bénévole. Pour m'aider à y consacrer un maximum de temps et accélérer la sortie d'une première version jouable pour tout le monde, j'ai mis en place une page Tipeee et un lien PayPal. Même un don de 2 ou 5 euros permet d'acheter du matériel de test et fait une différence énorme sur la vitesse de développement.*  
  > *Vous retrouverez tous les liens en description : le site officiel, le GitHub pour suivre les commits au jour le jour, et les pages de dons. N'hésitez pas à liker la vidéo, à vous abonner et à partager avec tous les passionnés de GTA. On se retrouve très vite pour le prochain devlog ! »*

---

## 4. 📋 Description YouTube Prête à Copier-Coller

```markdown
🎮 ReGTA : Le retour de Grand Theft Auto: Chinatown Wars sur PC en Rust natif !

Après des mois de reverse engineering et de développement intensif, voici les premières images de ReGTA Chinatown Wars PC Port, un projet indépendant visant à recréer l'expérience culte de la PSP nativement sur PC sans aucun émulateur.

🔗 LIENS OFFICIELS DU PROJET :
🌐 Site Web Officiel : https://benji-paname.github.io/regta-chinatown-wars-pc-port/
💻 Code Source sur GitHub : https://github.com/benji-paname/regta-chinatown-wars-pc-port
☕ Soutenir le projet sur Tipeee : https://fr.tipeee.com/benji-paname/
💳 Soutenir via PayPal : https://paypal.me/BenjiLsg

⏱️ TIMESTAMPS :
0:00 - Introduction & Démo en direct
0:35 - Pourquoi Rust & Pas d'émulateur ?
1:20 - Gameplay : Conduite, Physique & Combats
1:55 - Le PDA interactif, GPS & Économie
2:25 - Services d'urgence & Collectibles (100 caméras)
2:50 - Roadmap & Prochaines fonctionnalités (Tatouages, Courses de rue)
3:30 - Comment soutenir le projet pour accélérer sa sortie !

⚠️ DISCLAIMER JURIDIQUE / PRE-ALPHA :
ReGTA est un projet de recherche fan indépendant à but non lucratif, non affilié à Rockstar Games ou Take-Two Interactive. Aucun fichier propriétaire, ROM, ISO ou exécutable du jeu n'est distribué ni disponible au téléchargement. Le moteur nécessite que l'utilisateur fournisse sa propre copie originale du jeu PSP (ULES-01347). Le soutien financier est facultatif et aide au développement technique ; il ne constitue pas un achat de jeu ni une garantie de date de sortie.

#GTA #ChinatownWars #Rust #PCGaming #Recomp #ReverseEngineering #PSP #Devlog
```

---

## 5. 🏷️ Mots-Clés / Tags YouTube Optimisés (SEO)

```text
GTA Chinatown Wars, GTA Chinatown Wars PC, GTA Chinatown Wars PC Port, ReGTA, Recomp, GTA Recompiled, Chinatown Wars Remaster, GTA PSP PC, Rust gamedev, Reverse engineering GTA, Grand Theft Auto Chinatown Wars, Huang Lee, GTA PC 60fps, Liberty City, GTA fangame, GTA modding, GTA 2026
```

---

## 6. 🎥 Comment Enregistrer Facilement tes Clips de Démo avec OBS

1. **Lancement Plein Écran Fluide** :
   Double-clique sur :
   `C:\Users\BENJAMIN\Documents\Codex\2026-10-17\tu-voit-le-projet-vcs-rust\outputs\CTW-Rust\Jouer-Plein-Ecran.cmd`
2. **Réglages OBS Studio** :
   - Mode : **Capture de jeu** (sélectionner `CTW-Rust.exe`) ou **Capture d'écran**.
   - Résolution : 1920×1080 (1080p) ou 2560×1440 à 60 FPS.
   - Bitrate recommandé : 12 000 à 16 000 Kbps (CBR / NVENC si carte NVIDIA).
3. **Touches et Raccourcis Utiles en Jeu** :
   - `Z / Q / S / D` ou Stick gauche : Déplacement & Direction
   - `Espace` ou Bouton A : Sprint / Roulade évasive
   - `E` ou Bouton Y : Entrer/Sortir d'un véhicule, fouiller une benne
   - `F` ou Gâchette : Tirer / Frapper
   - `Tab` ou Select : Ouvrir le PDA
   - `F1` : Bascule caméra (PSP / Mobile / DS)
   - `F2` : Minimap plein écran / rotation
