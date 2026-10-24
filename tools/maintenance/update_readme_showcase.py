# SPDX-License-Identifier: GPL-3.0-only
with open('README.md', 'r', encoding='utf-8') as f:
    text = f.read()

header = """<p align="center">
  <img src="docs/assets/logo.svg" alt="ReGTA CTW Official Logo" width="720"/>
</p>

# ReGTA CTW — ReGTA CTW PSP Recomp

[![Version](https://img.shields.io/badge/version-0.1.4--alpha-orange)](CHANGELOG.md)
[![Tests](https://img.shields.io/badge/tests-152%20passed%20%7C%200%20warnings-brightgreen)](docs/STATUS.md)
[![Platform](https://img.shields.io/badge/platform-Windows%20x64-blue)](#build-and-run)
[![Language](https://img.shields.io/badge/runtime-Rust-dea584)](#repository-layout)
[![License](https://img.shields.io/badge/license-GPL--3.0--only-blue)](LICENSE)
[![Donate Tipeee](https://img.shields.io/badge/Tipeee-Soutenir%20le%20Projet-red?logo=tipeee)](https://fr.tipeee.com/benji-paname/)
[![Donate PayPal](https://img.shields.io/badge/PayPal.Me-Faire%20un%20Don-blue?logo=paypal)](https://paypal.me/BenjiLsg)

**ReGTA CTW PSP Recomp** — portage et recompilation statique native en pur Rust de *Grand Theft Auto: Chinatown Wars* PSP (version Europe **ULES-01347, version 1.01**).

**par Benji Paname**

> 🛑 **Note importante de distribution** : Ce projet est actuellement en phase de développement fermé (Pré-Alpha v0.1.4). **Aucun fichier binaire exécutable, aucune ROM ni aucun asset propriétaire n'est téléchargeable pour le moment.** Le code source de recherche est documenté en toute transparence. Vous pouvez soutenir activement l'accélération du développement via les liens de don ci-dessous !

---

## 💛 Soutenir le Projet & Accélérer le Développement

Pour permettre à **Benji Paname** de consacrer un maximum de temps au reverse engineering, à l'analyse du code Allegrex MIPS et à l'acquisition du matériel de test :

- 🧡 **[Faire un don ou s'abonner sur Tipeee (Mensuel / Suivi)](https://fr.tipeee.com/benji-paname/)**
- 💙 **[Don direct immédiat via PayPal.Me](https://paypal.me/BenjiLsg)**

---

ReGTA CTW vise à reconstruire le comportement du jeu PSP en code natif Windows x64 pur. Il lit les données du jeu directement depuis l'ISO originale de l'utilisateur sans émuler le CPU PSP (0% d'overhead d'émulation). Le projet s'appuie sur les recherches d'ingénierie inverse et la décompression des 166 code overlays du conteneur `OVERLAYS.PAK`.

🌐 **Site vitrine officiel (GitHub Pages)** : Consultez [`docs/index.html`](docs/index.html) pour la vitrine interactive complète avec le logo officiel, la matrice des systèmes et la feuille de route.

"""

idx = text.find('## Build and run')
if idx != -1:
    new_text = header + text[idx:]
    with open('README.md', 'w', encoding='utf-8') as f:
        f.write(new_text)
    print("README.md successfully updated!")
else:
    print("Marker '## Build and run' not found.")
