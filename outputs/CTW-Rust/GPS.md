# GPS et minicarte — 22 octobre 2026

Mise à jour du 24 octobre : voir RADAR-PSP.md pour l'emplacement et la géométrie d'origine, la transparence, les jauges vie/armure et la comparaison vidéo. Les symboles et la police utilisent en priorité le pack décrit dans TEXTURES-HD.md. La flèche du joueur est blanche/grise. Validation actuelle : 81 tests et 40 scénarios, dont les huit scénarios GPS.

Le GPS fonctionne sur le réseau routier extrait de l'ISO PSP. Il ne pilote pas le personnage : choisir une destination puis suivre le tracé jaune. Le marqueur vert représente Huang et son orientation ; le losange rose représente la destination, vert après l'arrivée. Hors du champ de la minicarte, le marqueur reste au bord. La grande carte garde le nord en haut. Le radar peut tourner avec la caméra ou rester au nord : G / croix droite, préférence mémorisée. F4 / LB+clic RS change français/anglais. Les trois profils caméra sont préservés.

## Utiliser

- M, Back sur manette ou clic sur la minicarte : ouvrir/fermer la carte. Le déplacement est suspendu pendant sa consultation.
- Clic sur la carte : destination, replacée sur la route autorisée la plus proche dans un rayon de 100 unités. Le marqueur indique le point réellement retenu.
- Manette : stick gauche pour déplacer le curseur, A pour choisir. Au clavier : flèches et Entrée. Atteindre le bord fait défiler la carte.
- Molette, +/− ou LB/RB : zoom. Clic droit maintenu : déplacer la carte.
- Espace/Y : recentrer sur Huang. F/clic stick droit : cadrer tout l'itinéraire.
- Suppr/X ou bouton « Effacer le GPS » : supprimer la destination.
- Échap/B, M/Back ou bouton « Retour au jeu » : reprendre. Échap ne quitte le jeu que lorsque la carte est fermée.

La destination est mémorisée dans `data/gps-settings.json`, sans modifier le réglage de caméra. En jeu, la minicarte suit Huang même si la caméra libre est activée. Les commandes de roulade, sprint et caméra sont réservées à leurs fonctions de carte lorsque celle-ci est ouverte.

`Essayer-GPS.cmd` ouvre un exemple de trajet. Le lancement normal conserve une minicarte même sans destination. Options de diagnostic : `--gps X Y`, `--gps-map` ; les essais déterministes ne lisent ni n'écrivent la préférence GPS de l'utilisateur.

## Données PSP et calcul

`ROM.WAD/ai.bin` ULES01347 : 28 312 octets, 4 108 liaisons et 2 008 points. SHA256 : `51c7d3b7041cb2573f9968cef8d32a045b89ad1b31a33040412353df6bed7023`.

En-tête : nombre de liens, nombre de points, zéro, position de la table des points. Liens u16 à +16 ; points de 10 octets à +8 232 : premier lien u16, flags u16, X/Y i16 en huitièmes d'unité, Z i8 en demi-unités, octet de couverture. Le nombre de voisins est `(flags >> 2) & 7`. Lecture vérifiée sur cette ISO ; format informé par `ctw-native roads.cpp/.h`, © 2026 Kryo.to, GPL-3.0.

Le calcul Rust utilise Dijkstra, avec départ et destination projetés sur les segments. Il respecte le sens des liaisons. Les voies maritimes (Z < −2) et réservées (flag 0x800) sont exclues : 3 136 liaisons terrestres utilisables. Une destination séparée du réseau accessible reçoit « Itinéraire indisponible » ; un clic trop loin d'une voie reçoit « Aucune route à cet endroit ». Aucun trait direct ne remplace un itinéraire manquant.

Recalcul au plus toutes les 0,5 seconde lorsque Huang s'est déplacé de plus de 3 unités depuis le dernier calcul. Distance restante, prochain virage et arrivée à moins de 6 unités. Ces seuils et l'algorithme sont propres au prototype : ils ne sont pas présentés comme le code GPS original PSP. Le tronçon d'approche en pointillés indique seulement comment rejoindre le réseau ; il ne garantit pas un chemin piéton libre de collisions à travers la cour ou le bâtiment voisin.

Depuis le23octobre, le fond utilise les **40 tuiles originales PSP**, PAK332–371, référencées par l'annuaire331. L'en-tête PSP est de8octets ; le format mobile à14octets ne s'applique pas. Les anciennes conclusions sur l'absence de tuiles étaient dues au mauvais index. Le tracé GPS Rust reste distinct du programme original. Il fonctionne à pied et en voiture ; missions et commerces attendent leurs systèmes.

La signature « by Benji Paname » est affichée en haut à gauche. Tous les textes du HUD/GPS sont commutables français/anglais. La destination reste dans gps-settings.json ; langue et rotation dans hud-settings.json.

## Vérifications

76 tests réussis, dont six GPS : tables invalides, sens uniques et projections, voies maritimes/réservées, destination non finie, chemin inaccessible, choix du trajet le plus court, diminution de la distance entre deux recalculs, recalcul/arrivée/effacement, conversion écran/monde, zoom et découpage des traits.

Huit scénarios sur la vraie carte, voir `data/gps-validation.json` : carte ouverte sans tick de déplacement, trajet local, recalcul en mouvement, traversée de ville d'environ 3 851 unités / 54 points, destination inaccessible, arrivée au départ et après déplacement, destination hors réseau, roulade conservée avec GPS actif. Les scénarios local/carte partagent le même essai ; huit exécutions au total. Tous les segments tracés ont été vérifiés indépendamment contre les liaisons terrestres dirigées de `ai.bin`. Aucun modèle PSP en erreur.

Captures du HUD et de la grande carte inspectées. Le contrôle interactif par l'outil Windows n'a pas pu démarrer (`failed to write kernel assets`, chemin introuvable, y compris après réinitialisation). Les clics/touches en fenêtre et la manipulation d'une manette physique restent donc à confirmer ; aucune validation interactive n'est revendiquée.

Reproduction : `work/validate-gps.py` à la racine de la conversation, utilisant le binaire construit et l'ISO privée. Aucun contenu du jeu à redistribuer.

## Présentation PSP du 23 octobre

Radar et grande carte utilisent le même fond original et les mêmes coordonnées. Les points des véhicules et des passants ont été retirés du radar comme dans la référence PSP. La vie est liée à la santé réelle de la scène. **F5 / RB+Back** masque les bandes de commandes ; **F4 / LB+clic RS** change la langue. **V / clic LS** cadre toute la ville ; F/clic RS cadre le trajet. Boutons de recentrage, vue de ville, zoom et suppression utilisables à la souris. Les options `--info-bands on|off` et `--gps-city` servent aux captures et au lancement.
