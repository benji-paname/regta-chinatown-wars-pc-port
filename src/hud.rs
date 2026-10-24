// SPDX-License-Identifier: GPL-3.0-only
//! HUD geometry shared by rendering and mouse hit testing; persistent radar preference.
use serde::{Deserialize, Serialize};
pub const FOOTER_Y: f32 = 590.;
pub fn mini(bands: bool) -> [f32; 4] {
    crate::radar::rect(bands)
}
pub const MAP_BUTTON: [f32; 4] = [18., 604., 90., 25.];
pub const ROTATE_BUTTON: [f32; 4] = [114., 604., 145., 25.];
pub const ACTION_BUTTON: [f32; 4] = [450., 598., 220., 31.];
pub const VEHICLE_BUTTON: [f32; 4] = [800., 598., 280., 31.];
pub const PROFILE_BUTTON: [f32; 4] = [280., 10., 150., 31.];
pub const FREE_BUTTON: [f32; 4] = [440., 10., 150., 31.];
pub const LANGUAGE_BUTTON: [f32; 4] = [600., 10., 150., 31.];
pub const FULLSCREEN_BUTTON: [f32; 4] = [760., 10., 165., 31.];
pub const MAP_ROTATE: [f32; 4] = [770., 52., 240., 36.];
pub const MAP_CLOSE: [f32; 4] = [1030., 52., 226., 36.];
pub const MAP_CLEAR: [f32; 4] = [1056., 591., 200., 32.];
pub const MAP_CENTER: [f32; 4] = [24., 591., 200., 32.];
pub const MAP_FIT: [f32; 4] = [240., 591., 200., 32.];
pub const MAP_CITY: [f32; 4] = [456., 591., 200., 32.];
pub const MAP_ZOOM_IN: [f32; 4] = [672., 591., 176., 32.];
pub const MAP_ZOOM_OUT: [f32; 4] = [864., 591., 176., 32.];
pub fn contains(r: [f32; 4], p: [f32; 2]) -> bool {
    p[0] >= r[0] && p[0] < r[0] + r[2] && p[1] >= r[1] && p[1] < r[1] + r[3]
}
/// Original PSP UI resources, read from the user's ISO at runtime.
pub struct Assets {
    pub weapon: crate::texture::Texture,
    pub radar: crate::texture::Texture,
    pub radar_sprites: Vec<crate::radar::Sprite>,
    pub radar_map: crate::radar::Map,
    pub font: crate::texture::Texture,
    pub glyphs: Vec<[u16; 3]>,
    pub font_height: u16,
}
impl Assets {
    pub fn load(
        disc: &mut crate::iso::Disc,
        pak: &crate::pak::Pak,
        wad: &crate::wad::Wad,
    ) -> crate::iso::Result<Self> {
        let font = crate::texture::decode(261, &pak.read(disc, 261)?.0)?;
        let data = wad.read(disc, crate::wad::name_hash("PSP_Hel12x12.bin"))?;
        let height = crate::iso::u16le(&data, 2)?;
        let count = crate::iso::u16le(&data, 0)? as usize;
        if height == 0 || data.len() != 4 + count * 6 {
            return Err("Invalid PSP HUD font".into());
        }
        let mut glyphs = Vec::with_capacity(count);
        for row in data[4..].chunks_exact(6) {
            let g = [
                crate::iso::u16le(row, 0)?,
                crate::iso::u16le(row, 2)?,
                crate::iso::u16le(row, 4)?,
            ];
            if g[1] as usize + g[0] as usize > font.info.width
                || g[2] as usize + height as usize > font.info.height
            {
                return Err("PSP HUD font glyph outside atlas".into());
            }
            glyphs.push(g);
        }
        let radar = crate::texture::decode(378, &pak.read(disc, 378)?.0)?;
        let radar_sprites = crate::radar::sprites(
            &wad.read(disc, crate::wad::name_hash("SS_Radar.bin"))?,
            [radar.info.width, radar.info.height],
        )?;
        Ok(Self {
            weapon: crate::texture::decode(374, &pak.read(disc, 374)?.0)?,
            radar,
            radar_sprites,
            radar_map: crate::radar::Map::load(disc, pak)?,
            font,
            glyphs,
            font_height: height,
        })
    }
}
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub rotate_minimap: bool,
    pub english: bool,
    pub info_bands: bool,
    pub fullscreen: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            rotate_minimap: true,
            english: false,
            info_bands: false,
            fullscreen: false,
        }
    }
}
impl Settings {
    pub fn load(path: &std::path::Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &std::path::Path) -> crate::iso::Result<()> {
        std::fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}
// Every displayed PC binding is paired with its implemented Xbox equivalent,
// regardless of whether a controller is currently connected.
pub type Hint = (&'static str, &'static str, &'static str);
pub const FOOT: &[Hint] = &[
    ("BANDES D INFORMATIONS", "F5", "RB+BACK"),
    ("LANGUE", "F4", "LB+RS"),
    ("DEPLACER", "ZQSD", "LS"),
    ("MARCHER", "CTRL", "LS DOUX"),
    ("SPRINTER", "MAJ", "A"),
    ("ROULER / FRANCHIR", "ESPACE", "X"),
    ("FRAPPER", "J", "B"),
    ("MONTER / DESCENDRE", "E", "Y"),
    ("RECENTRER", "C", "LB"),
    ("PAUSE", "P", "START"),
    ("CARTE GPS", "M", "BACK"),
    ("PDA / MESSAGES", "I", "LB+START"),
    ("CHOIX CAMERA", "F3", "RS CLIC"),
    ("CAMERA LIBRE", "F2", "CROIX HAUT"),
    ("ROTATION RADAR", "G", "CROIX DROITE"),
    ("RETOUR DEPART", "R", "LB+Y"),
    ("COLLISIONS", "F1", "CROIX BAS"),
    ("CAPTURE", "F12", "CROIX GAUCHE"),
    ("PLEIN ECRAN", "F11", "LB+BACK"),
    ("QUITTER", "ECHAP", "START+B"),
];
pub const DRIVE: &[Hint] = &[
    ("BANDES D INFORMATIONS", "F5", "RB+BACK"),
    ("LANGUE", "F4", "LB+RS"),
    ("DIRECTION", "Q / D", "LS"),
    ("ACCELERER", "Z / W", "RT"),
    ("FREIN / RECULER", "S", "LT"),
    ("FREIN A MAIN", "ESPACE", "X"),
    ("DESCENDRE A L ARRET", "E", "Y"),
    ("RECENTRER", "C", "LB"),
    ("PAUSE", "P", "START"),
    ("CARTE GPS", "M", "BACK"),
    ("PDA / MESSAGES", "I", "LB+START"),
    ("CHOIX CAMERA", "F3", "RS CLIC"),
    ("CAMERA LIBRE", "F2", "CROIX HAUT"),
    ("ROTATION RADAR", "G", "CROIX DROITE"),
    ("RETOUR DEPART", "R", "LB+Y"),
    ("COLLISIONS", "F1", "CROIX BAS"),
    ("CAPTURE", "F12", "CROIX GAUCHE"),
    ("PLEIN ECRAN", "F11", "LB+BACK"),
    ("QUITTER", "ECHAP", "START+B"),
];
pub const FREE: &[Hint] = &[
    ("BANDES D INFORMATIONS", "F5", "RB+BACK"),
    ("LANGUE", "F4", "LB+RS"),
    ("VOLER", "ZQSD", "LS"),
    ("REGARDER", "SOURIS+DROIT", "RS"),
    ("MONTER", "ESPACE", "RT"),
    ("DESCENDRE", "CTRL", "LT"),
    ("VOL RAPIDE", "MAJ", "A"),
    ("PAUSE", "P", "START"),
    ("CARTE GPS", "M", "BACK"),
    ("PDA / MESSAGES", "I", "LB+START"),
    ("CHOIX CAMERA", "F3", "RS CLIC"),
    ("RETOUR JOUEUR", "F2", "CROIX HAUT"),
    ("ROTATION RADAR", "G", "CROIX DROITE"),
    ("RETOUR DEPART", "R", "LB+Y"),
    ("COLLISIONS", "F1", "CROIX BAS"),
    ("CAPTURE", "F12", "CROIX GAUCHE"),
    ("PLEIN ECRAN", "F11", "LB+BACK"),
    ("QUITTER", "ECHAP", "START+B"),
];
pub const MAP: &[Hint] = &[
    ("BANDES D INFORMATIONS", "F5", "RB+BACK"),
    ("TOUTE LA VILLE", "V", "LS CLIC"),
    ("LANGUE", "F4", "LB+RS"),
    ("CURSEUR", "FLECHES", "LS"),
    ("DEPLACER CARTE", "CLIC DROIT", "RS"),
    ("DESTINATION", "CLIC / ENTREE", "A"),
    ("RETOUR JEU", "ECHAP", "B"),
    ("ZOOM +", "MOLETTE / +", "RB"),
    ("ZOOM -", "MOLETTE / -", "LB"),
    ("RECENTRER JOUEUR", "ESPACE", "Y"),
    ("VOIR LE TRAJET", "F", "RS CLIC"),
    ("EFFACER GPS", "SUPPR", "X"),
    ("ROTATION RADAR", "G", "CROIX DROITE"),
    ("FERMER CARTE", "M", "BACK"),
    ("CAPTURE", "F12", "CROIX GAUCHE"),
    ("PLEIN ECRAN", "F11", "LB+BACK"),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn radar_preference_survives_reload_and_recovers_from_invalid_file() {
        let path = std::env::temp_dir().join(format!("ctw-hud-{}.json", std::process::id()));
        Settings {
            rotate_minimap: false,
            english: true,
            info_bands: true,
            fullscreen: true,
        }
        .save(&path)
        .unwrap();
        assert!(!Settings::load(&path).rotate_minimap);
        assert!(Settings::load(&path).english);
        assert!(Settings::load(&path).info_bands);
        assert!(Settings::load(&path).fullscreen);
        Settings {
            rotate_minimap: true,
            english: false,
            info_bands: false,
            fullscreen: false,
        }
        .save(&path)
        .unwrap();
        assert!(Settings::load(&path).rotate_minimap);
        assert!(!Settings::load(&path).fullscreen);
        std::fs::write(&path, b"invalid").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        std::fs::remove_file(&path).unwrap();
    }
}
