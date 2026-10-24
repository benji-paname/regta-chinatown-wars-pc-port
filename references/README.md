# Références Techniques et Projets Liés (ReGTA CTW)

Ce dossier regroupe les dépôts et sources d'ingénierie inverse servant de vérité terrain absolue pour la recompilation statique et le portage natif PC de *Grand Theft Auto: Chinatown Wars* PSP (ULES-01347 / ULUS-10490).

---

### 1. `chinatown-wars-psp-recomp` (highcmd)
- **Origine** : [highcmd/chinatown-wars-psp-recomp](https://github.com/highcmd/chinatown-wars-psp-recomp)
- **Nature** : Recompilateur statique MIPS Allegrex -> C++ natif (Linux/Windows x64) avec runtime PSP HLE.
- **Points clés pour notre port Rust** :
  - **Structure des 167 Code Overlays** (`OVERLAYS.PAK`) : 114 overlays de missions/modes (`0x08b87460`) et 52 overlays d'apps/minijeux (`0x08bb2480`).
  - **Désassemblage & Adresses exactes** : 12 222 fonctions de l'EBOOT et 22 579 fonctions d'overlays cartographiées dans `data/functions.json`.
  - **Moteur de streaming de Liberty City** : grille 3x3 de tuiles de 60 unités à `0x08C12000` (`F_089F4754`).
  - **Caméra & Matrice de vue** : matrice à `0x088559AC`, pitch à `0x12C`, hauteur à `+0x108`.
  - **Audio ATRAC3+ & Radio** : ring buffer et démarrage mid-song (`SetHalfwayBufferAndGetID`).
  - **Rendu GE OpenGL** : gestion des passes de stencil pour les ombres et du bloom.

---

### 2. `psp-re` (PSP Reverse Engineering HQ)
- **Origine** : [psp-re/psp-re.github.io](https://github.com/psp-re/psp-re.github.io)
- **Nature** : Hub communautaire de référence sur l'architecture PSP, les syscalls Sony, et le reversing MIPS.
- **Ressources** :
  - Tables de NIDs (NID cracking).
  - Scripts Ghidra Allegrex et archives de types de données C (`pspsdk.gdt`, `uofw.gdt`).
  - Documentation du décryptage PRX/EBOOT et des registres matériels (VFPU, GE, SAS).

---

### 3. `upstream-ctw` (work/upstream-ctw - Kryo.to)
- **Origine** : Port C++ Windows/SDL2 de la version Android 4.4.243 (GPL-3.0).
- **Rôle** : Référence des classes de gameplay haut niveau (`peds.cpp`, `weapons.cpp`, `traffic.cpp`, `stats.cpp`, `explosions.cpp`).
