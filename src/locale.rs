// SPDX-License-Identifier: GPL-3.0-only
use std::sync::atomic::{AtomicBool, Ordering};
static ENGLISH: AtomicBool = AtomicBool::new(false);
pub fn set(english: bool) {
    ENGLISH.store(english, Ordering::Relaxed);
}
pub fn english() -> bool {
    ENGLISH.load(Ordering::Relaxed)
}
pub fn text(s: &str) -> String {
    translate(s, english())
}
pub fn translate(s: &str, english: bool) -> String {
    if !english {
        return s.into();
    }
    let exact = match s {
        "BANDES D INFORMATIONS" => "INFORMATION BARS",
        "TOUTE LA VILLE" => "WHOLE CITY",
        "LS CLIC" => "LS CLICK",
        "CARTE DE LIBERTY CITY" => "LIBERTY CITY MAP",
        "VERSION DE DEMONSTRATION" => "DEMONSTRATION BUILD",
        "DEPLACER" => "MOVE",
        "ZQSD" => "WASD",
        "MARCHER" => "WALK",
        "LS DOUX" => "LS GENTLY",
        "SPRINTER" => "SPRINT",
        "MAJ" => "SHIFT",
        "ROULER / FRANCHIR" => "ROLL / VAULT",
        "ESPACE" => "SPACE",
        "FRAPPER" => "ATTACK",
        "MONTER / DESCENDRE" => "ENTER / EXIT",
        "RECENTRER" => "RECENTER",
        "CARTE GPS" => "GPS MAP",
        "CHOIX CAMERA" => "CAMERA MODE",
        "RS CLIC" => "RS CLICK",
        "CAMERA LIBRE" => "FREE CAMERA",
        "CROIX HAUT" => "DPAD UP",
        "CROIX BAS" => "DPAD DOWN",
        "CROIX GAUCHE" => "DPAD LEFT",
        "CROIX DROITE" => "DPAD RIGHT",
        "ROTATION RADAR" => "RADAR ROTATION",
        "RETOUR DEPART" => "RESET DEMO",
        "CAPTURE" => "SCREENSHOT",
        "QUITTER" => "QUIT",
        "ECHAP" => "ESC",
        "DIRECTION" => "STEER",
        "Q / D" => "A / D",
        "ACCELERER" => "ACCELERATE",
        "Z / W" => "W",
        "FREIN / RECULER" => "BRAKE / REVERSE",
        "FREIN A MAIN" => "HANDBRAKE",
        "DESCENDRE A L ARRET" => "EXIT WHEN STOPPED",
        "VOLER" => "FLY",
        "REGARDER" => "LOOK",
        "SOURIS+DROIT" => "MOUSE+RMB",
        "MONTER" => "ASCEND",
        "DESCENDRE" => "DESCEND",
        "VOL RAPIDE" => "FLY FASTER",
        "RETOUR JOUEUR" => "FOLLOW PLAYER",
        "CURSEUR" => "CURSOR",
        "FLECHES" => "ARROWS",
        "DEPLACER CARTE" => "PAN MAP",
        "CLIC DROIT" => "RIGHT CLICK",
        "CLIC / ENTREE" => "CLICK / ENTER",
        "RETOUR JEU" => "BACK TO GAME",
        "MOLETTE / +" => "WHEEL / +",
        "MOLETTE / -" => "WHEEL / -",
        "RECENTRER JOUEUR" => "CENTER ON PLAYER",
        "VOIR LE TRAJET" => "FIT ROUTE",
        "EFFACER GPS" => "CLEAR GPS",
        "SUPPR" => "DELETE",
        "FERMER CARTE" => "CLOSE MAP",
        "RADAR : ROTATION" => "RADAR: ROTATING",
        "RADAR : NORD FIXE" => "RADAR: NORTH UP",
        "ROULADE EN COURS" => "ROLLING",
        "FRANCHISSEMENT" => "VAULTING",
        "FRANCHIR" => "VAULT",
        "ROULADE" => "ROLL",
        "FREINE POUR DESCENDRE" => "STOP TO EXIT",
        "DESCENDRE DU VEHICULE" => "EXIT VEHICLE",
        "MONTER DANS LE VEHICULE" => "ENTER VEHICLE",
        "DESTINATION ATTEINTE" => "DESTINATION REACHED",
        "ITINERAIRE GPS" => "GPS ROUTE",
        "HORS COMBAT" => "KNOCKED OUT",
        "RECOMMENCER" => "RESTART",
        "CHOISIS UNE ROUTE POUR TE GUIDER" => "CHOOSE A ROAD FOR DIRECTIONS",
        "ROTATION RADAR : ON" => "RADAR ROTATION: ON",
        "ROTATION RADAR : OFF" => "RADAR ROTATION: OFF",
        "RETOUR AU JEU" => "BACK TO GAME",
        "EFFACER LE GPS" => "CLEAR GPS",
        "CHOISIS UNE DESTINATION" => "CHOOSE A DESTINATION",
        "DESTINATION EFFACEE" => "DESTINATION CLEARED",
        "AUCUNE ROUTE A CET ENDROIT" => "NO ROAD AT THIS LOCATION",
        "ITINERAIRE INDISPONIBLE" => "ROUTE UNAVAILABLE",
        "ITINERAIRE ACTIF" => "ROUTE ACTIVE",
        "LANGUE" => "LANGUAGE",
        "FRANCAIS" => "FRENCH",
        "PLEIN ECRAN" => "FULLSCREEN",
        "FENETRE" => "WINDOWED",
        "UNITES/S" => "UNITS/S",
        _ => s,
    };
    if exact != s {
        return exact.into();
    }
    s.replace("REJOINDRE LA ROUTE", "JOIN THE ROAD")
        .replace("CONTINUER", "CONTINUE")
        .replace("DEMI-TOUR DANS", "U-TURN IN")
        .replace("A GAUCHE DANS", "TURN LEFT IN")
        .replace("A DROITE DANS", "TURN RIGHT IN")
        .replace("DISTANCE RESTANTE", "DISTANCE REMAINING")
        .replace("TRAJET  ", "ROUTE  ")
        .replace("MAR ", "TUE ")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_gps_values_and_signature_survive_language_switch() {
        assert_eq!(translate("A GAUCHE DANS 42 M", true), "TURN LEFT IN 42 M");
        assert_eq!(
            translate("REJOINDRE LA ROUTE - 17 M", true),
            "JOIN THE ROAD - 17 M"
        );
        assert_eq!(
            translate("DISTANCE RESTANTE  1.25 KM", true),
            "DISTANCE REMAINING  1.25 KM"
        );
        assert_eq!(translate("by Benji Paname", true), "by Benji Paname");
        assert_eq!(translate("A GAUCHE DANS 42 M", false), "A GAUCHE DANS 42 M");
    }
}
