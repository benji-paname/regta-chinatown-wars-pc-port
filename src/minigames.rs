// SPDX-License-Identifier: GPL-3.0-only
//! GTA Chinatown Wars authentic Minigames Engine matching PSP code overlays.
//! Features:
//! - `sinkingcarapp` (Overlay 138): Sinking car windshield escape minigame (Prologue).
//! - `hotwiringapp` (Overlay 044): Parked vehicle hotwiring (wires, screwdriver, transponder).
//! - `molotovcocktailapp` (Overlay 090): Gas station bottle filling minigame.
//! - `scratchcardapp` (Overlay 131): Liberty City newsstand scratch cards (cash, weapons, properties).
//! - `safecrackingapp` (Overlay 128): Safe cracking minigame with combination tumblers.
//! - `dumpsterweaponstashapp` (Overlay 022): Dumpster searching for hidden loot.

#![allow(dead_code)]

/// Specific minigame type corresponding to a PSP code overlay.
#[derive(Debug, Clone, PartialEq)]
pub enum MinigameKind {
    /// Sinking car escape (Prologue: 4 heavy glass strikes to break windshield).
    SinkingCar {
        hits_done: u8,
        hits_required: u8,
        water_level: f32, // 0.0 to 1.0 (submersion timer)
    },
    /// Vehicle hotwiring ignition bypass.
    Hotwire {
        method: HotwireMethod,
        progress: f32,
    },
    /// Filling Molotov cocktail beer bottles at gas pumps.
    MolotovFilling {
        bottles_filled: u8,
        current_fill: f32,
        is_pumping: bool,
    },
    /// Kiosk lottery scratch cards.
    ScratchCard {
        card_name: &'static str,
        scratched_cells: [bool; 6],
        symbols: [u8; 6], // matching 3 wins prize
        prize_cash: u32,
    },
    /// Safe cracking combination lock.
    SafeCracking {
        combination: [i32; 3],
        current_dial: i32,
        unlocked_tumblers: u8,
    },
    /// Searching alleyway dumpsters.
    DumpsterSearch {
        searched: bool,
        found_weapon: Option<usize>, // weapon type_id
        found_cash: u32,
    },
    /// Fitting and arming a vehicle car bomb (Overlay 010 carbombfittingapp).
    CarBombFitting {
        wires_connected: u8, // 0..=3 (power, ground, detonator switch)
        armed: bool,
    },
    /// Tattoo parlor needle tracing minigame (Overlay 141 tattooingapp).
    Tattooing {
        motif: TattooMotif,
        stencil_progress: f32, // 0.0 to 1.0 (traced lines)
        needle_accuracy: f32,  // 0.0 to 1.0 (precision tracking)
        pain_level: f32,       // 0.0 to 1.0 (flinch/kickout threshold)
        ink_level: f32,        // 0.0 to 1.0 (machine ink supply)
        reloads_done: u8,
    },
    /// Sniper rifle field assembly minigame (Overlay 126 cRifleConstructionApp).
    SniperAssembly {
        current_step: u8, // 0: Receiver, 1: Barrel, 2: Bolt, 3: Scope, 4: Suppressor
        step_progress: f32, // 0.0 to 1.0
        parts_fitted: [bool; 5],
        alignment_error: f32,
    },
    /// Authentic cSearchingBodiesApp (Overlay 133): Searching fallen pedestrian or dead enemy corpse for cash, contraband, and ammo.
    BodySearch {
        ped_index: usize,
        search_timer: f32, // 0.0 to duration
        duration: f32,     // 1.2s authentic search time
        searched: bool,
        looted_cash: u32,
        looted_drug: Option<(crate::drugs::DrugType, u32)>,
        looted_ammo: Option<(u8, u16)>, // (weapon_type_id, rounds)
    },
}

/// Authentic Chinatown Wars tattoo motifs for Triad and gang artwork.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TattooMotif {
    DragonOfBroker,
    KoiOfDukes,
    TigerOfBohan,
    ViperOfAlgonquin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotwireMethod {
    WiresTwist,
    ScrewdriverTurn,
    TransponderKeypad,
}

/// Active interactive minigame session.
#[derive(Debug, Clone, PartialEq)]
pub struct MinigameSession {
    pub kind: MinigameKind,
    pub completed: bool,
    pub failed: bool,
    pub reward_cash: u32,
    pub reward_message: Option<String>,
}

impl MinigameSession {
    pub fn new_sinking_car() -> Self {
        Self {
            kind: MinigameKind::SinkingCar {
                hits_done: 0,
                hits_required: 4,
                water_level: 0.1,
            },
            completed: false,
            failed: false,
            reward_cash: 0,
            reward_message: Some("PARE-BRISE BRISE ! VOUS NAGEZ VERS LA SURFACE".to_string()),
        }
    }

    pub fn new_hotwire(method: HotwireMethod) -> Self {
        Self {
            kind: MinigameKind::Hotwire { method, progress: 0.0 },
            completed: false,
            failed: false,
            reward_cash: 0,
            reward_message: Some("MOTEUR DEMARRE !".to_string()),
        }
    }

    pub fn new_molotov() -> Self {
        Self {
            kind: MinigameKind::MolotovFilling {
                bottles_filled: 0,
                current_fill: 0.0,
                is_pumping: false,
            },
            completed: false,
            failed: false,
            reward_cash: 0,
            reward_message: Some("COCKTAILS MOLOTOV FABRIQUES !".to_string()),
        }
    }

    pub fn new_scratch_card(name: &'static str, prize: u32) -> Self {
        Self {
            kind: MinigameKind::ScratchCard {
                card_name: name,
                scratched_cells: [false; 6],
                symbols: [1, 2, 1, 3, 1, 2], // 3 matching '1's wins!
                prize_cash: prize,
            },
            completed: false,
            failed: false,
            reward_cash: 0,
            reward_message: None,
        }
    }

    pub fn new_safe_cracking(comb: [i32; 3]) -> Self {
        Self {
            kind: MinigameKind::SafeCracking {
                combination: comb,
                current_dial: 0,
                unlocked_tumblers: 0,
            },
            completed: false,
            failed: false,
            reward_cash: 2500,
            reward_message: Some("COFFRE-FORT DEVERROUILLE ! +$2500".to_string()),
        }
    }

    pub fn new_dumpster(weapon: Option<usize>, cash: u32) -> Self {
        Self {
            kind: MinigameKind::DumpsterSearch {
                searched: false,
                found_weapon: weapon,
                found_cash: cash,
            },
            completed: false,
            failed: false,
            reward_cash: cash,
            reward_message: None,
        }
    }

    pub fn new_car_bomb() -> Self {
        Self {
            kind: MinigameKind::CarBombFitting {
                wires_connected: 0,
                armed: false,
            },
            completed: false,
            failed: false,
            reward_cash: 500,
            reward_message: None,
        }
    }

    pub fn new_tattoo(motif: TattooMotif) -> Self {
        let reward = match motif {
            TattooMotif::DragonOfBroker => 1200,
            TattooMotif::KoiOfDukes => 800,
            TattooMotif::TigerOfBohan => 950,
            TattooMotif::ViperOfAlgonquin => 1500,
        };
        Self {
            kind: MinigameKind::Tattooing {
                motif,
                stencil_progress: 0.0,
                needle_accuracy: 1.0,
                pain_level: 0.0,
                ink_level: 1.0,
                reloads_done: 0,
            },
            completed: false,
            failed: false,
            reward_cash: reward,
            reward_message: None,
        }
    }

    pub fn new_sniper_assembly() -> Self {
        Self {
            kind: MinigameKind::SniperAssembly {
                current_step: 0,
                step_progress: 0.0,
                parts_fitted: [false; 5],
                alignment_error: 0.5,
            },
            completed: false,
            failed: false,
            reward_cash: 1000,
            reward_message: Some("FUSIL DE PRECISION ASSEMBLE ! Arme prete a l'emploi (+1000$)".to_string()),
        }
    }

    pub fn new_body_search(
        ped_index: usize,
        cash: u32,
        drug: Option<(crate::drugs::DrugType, u32)>,
        ammo: Option<(u8, u16)>,
    ) -> Self {
        let mut msg = format!("CORPS FOUILLÉ : +${cash}");
        if let Some((d, q)) = drug {
            msg.push_str(&format!(" & {q}x {}", d.name()));
        }
        if let Some((w, a)) = ammo {
            msg.push_str(&format!(" & Arme #{w} (+{a})"));
        }
        Self {
            kind: MinigameKind::BodySearch {
                ped_index,
                search_timer: 0.0,
                duration: 1.2,
                searched: false,
                looted_cash: cash,
                looted_drug: drug,
                looted_ammo: ammo,
            },
            completed: false,
            failed: false,
            reward_cash: cash,
            reward_message: Some(msg),
        }
    }

    pub fn body_search_progress(&self) -> f32 {
        match &self.kind {
            MinigameKind::BodySearch { search_timer, duration, .. } => {
                (search_timer / duration).clamp(0.0, 1.0)
            }
            _ => 0.0,
        }
    }

    pub fn body_search_ped_index(&self) -> Option<usize> {
        match &self.kind {
            MinigameKind::BodySearch { ped_index, .. } => Some(*ped_index),
            _ => None,
        }
    }

    pub fn body_search_loot(&self) -> Option<(u32, Option<(crate::drugs::DrugType, u32)>, Option<(u8, u16)>)> {
        match &self.kind {
            MinigameKind::BodySearch { looted_cash, looted_drug, looted_ammo, .. } => {
                Some((*looted_cash, *looted_drug, *looted_ammo))
            }
            _ => None,
        }
    }

    /// Replenish tattoo machine ink reservoir from cap.
    pub fn dip_ink(&mut self) {
        if let MinigameKind::Tattooing { ink_level, pain_level, reloads_done, .. } = &mut self.kind {
            *ink_level = 1.0;
            *pain_level = (*pain_level - 0.10).max(0.0);
            *reloads_done += 1;
        }
    }

    /// Traces needle stroke along or off the stencil contour.
    pub fn apply_needle_stroke(&mut self, on_stencil: bool) -> bool {
        if self.completed || self.failed {
            return self.completed;
        }

        let reward = self.reward_cash;
        if let MinigameKind::Tattooing { stencil_progress, needle_accuracy, pain_level, ink_level, .. } = &mut self.kind {
            if *ink_level <= 0.0 {
                return false; // Out of ink, must dip!
            }

            if on_stencil {
                *stencil_progress = (*stencil_progress + 0.20).min(1.0);
                *ink_level = (*ink_level - 0.15).max(0.0);
                *needle_accuracy = (*needle_accuracy * 0.95 + 0.05).min(1.0);
                *pain_level = (*pain_level + 0.06).min(1.0);
            } else {
                // Needle slip outside stencil line
                *needle_accuracy = (*needle_accuracy * 0.80).max(0.1);
                *pain_level = (*pain_level + 0.35).min(1.0);
            }

            if *pain_level >= 1.0 {
                self.failed = true;
                self.reward_message = Some("TATOUAGE ÉCHOUÉ ! Le client s'est enfui à cause de la douleur.".into());
                return false;
            }

            if *stencil_progress >= 1.0 && *needle_accuracy >= 0.70 {
                self.completed = true;
                self.reward_message = Some(format!(
                    "TATOUAGE TERMINÉ AVEC SUCCÈS ! Précision : {:.0}% (+${})",
                    *needle_accuracy * 100.0,
                    reward
                ));
            }
        }

        self.completed
    }

    /// Primary interaction action (strike glass, twist wire, pump fuel, scratch cell, turn dial).
    pub fn trigger_action(&mut self) -> bool {
        if self.completed || self.failed {
            return self.completed;
        }

        match &mut self.kind {
            MinigameKind::SinkingCar { hits_done, hits_required, .. } => {
                *hits_done += 1;
                if *hits_done >= *hits_required {
                    self.completed = true;
                }
            }
            MinigameKind::Hotwire { progress, .. } => {
                *progress += 0.35;
                if *progress >= 1.0 {
                    self.completed = true;
                }
            }
            MinigameKind::MolotovFilling { current_fill, bottles_filled, is_pumping } => {
                *is_pumping = !*is_pumping;
                if *current_fill >= 0.85 && *current_fill <= 1.05 {
                    *bottles_filled += 1;
                    *current_fill = 0.0;
                    *is_pumping = false;
                    if *bottles_filled >= 3 {
                        self.completed = true;
                    }
                }
            }
            MinigameKind::ScratchCard { scratched_cells, symbols, prize_cash, .. } => {
                // Scratch next unopened cell
                if let Some(cell) = scratched_cells.iter_mut().find(|c| !**c) {
                    *cell = true;
                }
                // Check if 3 matching symbols revealed
                let mut counts = [0u8; 10];
                for i in 0..6 {
                    if scratched_cells[i] {
                        let sym = symbols[i] as usize;
                        if sym < counts.len() {
                            counts[sym] += 1;
                            if counts[sym] >= 3 {
                                self.completed = true;
                                self.reward_cash = *prize_cash;
                                self.reward_message = Some(format!("TICKET GAGNANT ! +${}", prize_cash));
                                return true;
                            }
                        }
                    }
                }
                if scratched_cells.iter().all(|&c| c) {
                    self.failed = true;
                    self.reward_message = Some("TICKET PERDANT".to_string());
                }
            }
            MinigameKind::SafeCracking { combination, current_dial, unlocked_tumblers } => {
                let target = combination[*unlocked_tumblers as usize];
                if *current_dial == target {
                    *unlocked_tumblers += 1;
                    if *unlocked_tumblers >= 3 {
                        self.completed = true;
                    }
                }
            }
            MinigameKind::DumpsterSearch { searched, found_weapon, found_cash } => {
                *searched = true;
                self.completed = true;
                let msg = match found_weapon {
                    Some(w) => format!("TROUVE DANS LA BENNE: ARME ID {} ET ${} !", w, found_cash),
                    None => format!("TROUVE DANS LA BENNE: ${}", found_cash),
                };
                self.reward_message = Some(msg);
            }
            MinigameKind::CarBombFitting { wires_connected, armed } => {
                *wires_connected += 1;
                if *wires_connected >= 3 {
                    *armed = true;
                    self.completed = true;
                    self.reward_message = Some("BOMBE AUTOMOBILE AMORCÉE ! Le véhicule explosera au démarrage (+500$)".into());
                }
            }
            MinigameKind::Tattooing { ink_level, .. } => {
                if *ink_level <= 0.0 {
                    self.dip_ink();
                } else {
                    self.apply_needle_stroke(true);
                }
            }
            MinigameKind::SniperAssembly { current_step, step_progress, parts_fitted, alignment_error } => {
                if *current_step < 5 {
                    *step_progress += 0.35;
                    *alignment_error = (*alignment_error * 0.7).max(0.0);
                    if *step_progress >= 1.0 {
                        parts_fitted[*current_step as usize] = true;
                        *current_step += 1;
                        *step_progress = 0.0;
                        *alignment_error = 0.4;
                        if *current_step >= 5 {
                            self.completed = true;
                        }
                    }
                }
            }
            MinigameKind::BodySearch { search_timer, duration, searched, .. } => {
                *search_timer = *duration;
                *searched = true;
                self.completed = true;
            }
        }

        self.completed
    }

    /// Advances time-based minigame physics (e.g. rising water level in sinking car).
    pub fn tick(&mut self, dt: f32) {
        if self.completed || self.failed {
            return;
        }

        match &mut self.kind {
            MinigameKind::SinkingCar { water_level, .. } => {
                *water_level += dt * 0.04;
                if *water_level >= 1.0 {
                    self.failed = true;
                    self.reward_message = Some("NOYADE : VOUS N'AVEZ PAS BRISÉ LE PARE-BRISE A TEMPS".to_string());
                }
            }
            MinigameKind::MolotovFilling { current_fill, is_pumping, .. } => {
                if *is_pumping {
                    *current_fill += dt * 0.6;
                    if *current_fill > 1.15 {
                        // Overflow! Reset bottle
                        *current_fill = 0.0;
                        *is_pumping = false;
                    }
                }
            }
            MinigameKind::Tattooing { pain_level, .. } => {
                *pain_level = (*pain_level - dt * 0.15).max(0.0);
            }
            MinigameKind::SniperAssembly { alignment_error, .. } => {
                *alignment_error = (*alignment_error + dt * 0.05).min(1.0);
            }
            MinigameKind::BodySearch { search_timer, duration, searched, .. } => {
                *search_timer += dt;
                if *search_timer >= *duration {
                    *searched = true;
                    self.completed = true;
                }
            }
            _ => {}
        }
    }
}

// ============================================================================
// Authentic Chinatown Wars Ammu-Nation Online Store (cAmmozoneApp, Overlay 001)
// ============================================================================

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmmozoneCategory {
    Handguns = 0,
    Shotguns = 1,
    SubmachineGuns = 2,
    AssaultRifles = 3,
    HeavyWeapons = 4,
    Explosives = 5,
    BodyArmor = 6,
}

#[allow(dead_code)]
impl AmmozoneCategory {
    pub const ALL: [AmmozoneCategory; 7] = [
        AmmozoneCategory::Handguns,
        AmmozoneCategory::Shotguns,
        AmmozoneCategory::SubmachineGuns,
        AmmozoneCategory::AssaultRifles,
        AmmozoneCategory::HeavyWeapons,
        AmmozoneCategory::Explosives,
        AmmozoneCategory::BodyArmor,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            AmmozoneCategory::Handguns => "ARMES DE POING",
            AmmozoneCategory::Shotguns => "FUSILS & POMPES",
            AmmozoneCategory::SubmachineGuns => "MITRAILLETTES",
            AmmozoneCategory::AssaultRifles => "FUSILS D'ASSAUT",
            AmmozoneCategory::HeavyWeapons => "ARMES LOURDES",
            AmmozoneCategory::Explosives => "EXPLOSIFS & GRENADES",
            AmmozoneCategory::BodyArmor => "PROTECTIONS KEVLAR",
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct AmmozoneItem {
    pub id: usize,
    pub name: &'static str,
    pub category: AmmozoneCategory,
    pub type_id: u8,
    pub price: u32,
    pub ammo_pack: u16,
    pub damage_stars: u8,
    pub fire_rate_stars: u8,
    pub description: &'static str,
}

#[allow(dead_code)]
pub const AMMOZONE_CATALOG: &[AmmozoneItem] = &[
    AmmozoneItem { id: 0, name: "Pistolet 9mm", category: AmmozoneCategory::Handguns, type_id: 2, price: 500, ammo_pack: 50, damage_stars: 2, fire_rate_stars: 3, description: "Arme de poing standard, fiable et économique." },
    AmmozoneItem { id: 1, name: "Pistolets Doubles (.45)", category: AmmozoneCategory::Handguns, type_id: 5, price: 1200, ammo_pack: 60, damage_stars: 3, fire_rate_stars: 4, description: "Puissance de feu doublée pour les fusillades rapprochées." },
    AmmozoneItem { id: 2, name: "Desert Eagle .50", category: AmmozoneCategory::Handguns, type_id: 2, price: 1800, ammo_pack: 35, damage_stars: 4, fire_rate_stars: 2, description: "Calibre lourd capable de neutraliser en un coup." },
    AmmozoneItem { id: 3, name: "Fusil à pompe Pump-Action", category: AmmozoneCategory::Shotguns, type_id: 4, price: 1200, ammo_pack: 30, damage_stars: 4, fire_rate_stars: 2, description: "Dévastateur à courte portée avec gerbe de plombs." },
    AmmozoneItem { id: 4, name: "Fusil à canon scié", category: AmmozoneCategory::Shotguns, type_id: 4, price: 950, ammo_pack: 25, damage_stars: 4, fire_rate_stars: 3, description: "Compact et dissimulable pour les règlements de compte." },
    AmmozoneItem { id: 5, name: "Micro-SMG (Uzi)", category: AmmozoneCategory::SubmachineGuns, type_id: 6, price: 2000, ammo_pack: 120, damage_stars: 2, fire_rate_stars: 5, description: "Cadence de tir ultra-rapide en tir continu." },
    AmmozoneItem { id: 6, name: "SMG Tactique (MP5)", category: AmmozoneCategory::SubmachineGuns, type_id: 6, price: 2600, ammo_pack: 120, damage_stars: 3, fire_rate_stars: 4, description: "Précision chirurgicale et groupement de tirs optimal." },
    AmmozoneItem { id: 7, name: "Fusil d'assaut AK-47", category: AmmozoneCategory::AssaultRifles, type_id: 7, price: 3500, ammo_pack: 150, damage_stars: 4, fire_rate_stars: 4, description: "Le classique mondial, puissance et pénétration assurées." },
    AmmozoneItem { id: 8, name: "Lance-roquettes RPG", category: AmmozoneCategory::HeavyWeapons, type_id: 10, price: 10000, ammo_pack: 8, damage_stars: 5, fire_rate_stars: 1, description: "Roquette explosive capable de détruire véhicules et hélicoptères." },
    AmmozoneItem { id: 9, name: "Lance-flammes", category: AmmozoneCategory::HeavyWeapons, type_id: 11, price: 8000, ammo_pack: 200, damage_stars: 4, fire_rate_stars: 5, description: "Projection de carburant enflammé nettoyant toute zone." },
    AmmozoneItem { id: 10, name: "Minigun Rotatif", category: AmmozoneCategory::HeavyWeapons, type_id: 10, price: 15000, ammo_pack: 300, damage_stars: 5, fire_rate_stars: 5, description: "6 canons rotatifs délivrant un déluge d'acier continu." },
    AmmozoneItem { id: 11, name: "Grenades à fragmentation (x5)", category: AmmozoneCategory::Explosives, type_id: 13, price: 800, ammo_pack: 5, damage_stars: 5, fire_rate_stars: 2, description: "Explosif défensif à retardateur 2.5 secondes." },
    AmmozoneItem { id: 12, name: "Cocktails Molotov (x5)", category: AmmozoneCategory::Explosives, type_id: 14, price: 650, ammo_pack: 5, damage_stars: 4, fire_rate_stars: 2, description: "Bouteille incendiaire créant une nappe de feu au sol." },
    AmmozoneItem { id: 13, name: "Gilet Pare-balles Kevlar", category: AmmozoneCategory::BodyArmor, type_id: 99, price: 500, ammo_pack: 1, damage_stars: 0, fire_rate_stars: 0, description: "Protection balistique absorbant les tirs ennemis (100%)." },
];

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct CartEntry {
    pub item_id: usize,
    pub quantity: u32,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct AmmozoneApp {
    pub current_category: AmmozoneCategory,
    pub selected_item_idx: usize,
    pub cart: Vec<CartEntry>,
    pub delivery_in_progress: bool,
    pub delivery_timer: f32,
    pub total_delivered_orders: u32,
}

impl Default for AmmozoneApp {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl AmmozoneApp {
    pub fn new() -> Self {
        Self {
            current_category: AmmozoneCategory::Handguns,
            selected_item_idx: 0,
            cart: Vec::new(),
            delivery_in_progress: false,
            delivery_timer: 0.0,
            total_delivered_orders: 0,
        }
    }

    pub fn switch_category(&mut self, cat: AmmozoneCategory) {
        self.current_category = cat;
        self.selected_item_idx = 0;
    }

    pub fn next_category(&mut self) {
        let cur = self.current_category as usize;
        let next = (cur + 1) % AmmozoneCategory::ALL.len();
        self.current_category = AmmozoneCategory::ALL[next];
        self.selected_item_idx = 0;
    }

    pub fn prev_category(&mut self) {
        let cur = self.current_category as usize;
        let prev = (cur + AmmozoneCategory::ALL.len() - 1) % AmmozoneCategory::ALL.len();
        self.current_category = AmmozoneCategory::ALL[prev];
        self.selected_item_idx = 0;
    }

    pub fn filtered_items(&self) -> Vec<&'static AmmozoneItem> {
        AMMOZONE_CATALOG.iter().filter(|i| i.category == self.current_category).collect()
    }

    pub fn next_item(&mut self) {
        let count = self.filtered_items().len();
        if count > 0 {
            self.selected_item_idx = (self.selected_item_idx + 1) % count;
        }
    }

    pub fn prev_item(&mut self) {
        let count = self.filtered_items().len();
        if count > 0 {
            self.selected_item_idx = (self.selected_item_idx + count - 1) % count;
        }
    }

    pub fn current_item(&self) -> Option<&'static AmmozoneItem> {
        self.filtered_items().get(self.selected_item_idx).copied()
    }

    pub fn add_to_cart(&mut self, item_id: usize, qty: u32) {
        if let Some(entry) = self.cart.iter_mut().find(|e| e.item_id == item_id) {
            entry.quantity = entry.quantity.saturating_add(qty);
        } else {
            self.cart.push(CartEntry { item_id, quantity: qty });
        }
    }

    pub fn remove_from_cart(&mut self, item_id: usize) {
        self.cart.retain(|e| e.item_id != item_id);
    }

    pub fn clear_cart(&mut self) {
        self.cart.clear();
    }

    pub fn cart_total(&self) -> u32 {
        self.cart.iter().map(|e| {
            let price = AMMOZONE_CATALOG.iter().find(|i| i.id == e.item_id).map(|i| i.price).unwrap_or(0);
            price * e.quantity
        }).sum()
    }

    pub fn checkout(&mut self, player_cash: &mut u32) -> Result<u32, &'static str> {
        if self.cart.is_empty() {
            return Err("Panier vide");
        }
        let total = self.cart_total();
        if *player_cash < total {
            return Err("Fonds insuffisants");
        }
        *player_cash -= total;
        self.delivery_in_progress = true;
        self.delivery_timer = 45.0;
        self.cart.clear();
        Ok(total)
    }

    pub fn update(&mut self, dt: f32) -> bool {
        if self.delivery_in_progress {
            self.delivery_timer -= dt;
            if self.delivery_timer <= 0.0 {
                self.delivery_in_progress = false;
                self.total_delivered_orders += 1;
                return true;
            }
        }
        false
    }

    pub fn is_delivering(&self) -> bool {
        self.delivery_in_progress
    }

    pub fn delivery_remaining_seconds(&self) -> u32 {
        self.delivery_timer.max(0.0).ceil() as u32
    }
}

// ============================================================================
// Authentic Chinatown Wars Full-Screen Interactive GPS Map (cMapApp)
// ============================================================================

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MapApp {
    pub cursor_pos: [f32; 2],
    pub pan_offset: [f32; 2],
    pub zoom_level: f32,
    pub filter_dealers: bool,
    pub filter_safehouses: bool,
    pub filter_contacts: bool,
    pub filter_stunts: bool,
    pub filter_cameras: bool,
    pub user_waypoint: Option<[f32; 3]>,
    pub gps_route: Vec<[f32; 2]>,
}

impl Default for MapApp {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl MapApp {
    pub fn new() -> Self {
        Self {
            cursor_pos: [250.0, 250.0],
            pan_offset: [0.0, 0.0],
            zoom_level: 1.0,
            filter_dealers: true,
            filter_safehouses: true,
            filter_contacts: true,
            filter_stunts: true,
            filter_cameras: true,
            user_waypoint: None,
            gps_route: Vec::new(),
        }
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom_level = zoom.clamp(0.6, 3.0);
    }

    pub fn zoom_in(&mut self) {
        self.set_zoom(self.zoom_level + 0.25);
    }

    pub fn zoom_out(&mut self) {
        self.set_zoom(self.zoom_level - 0.25);
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.pan_offset[0] += dx / self.zoom_level;
        self.pan_offset[1] += dy / self.zoom_level;
        self.cursor_pos[0] += dx / self.zoom_level;
        self.cursor_pos[1] += dy / self.zoom_level;
    }

    pub fn center_on(&mut self, pos: [f32; 2]) {
        self.cursor_pos = pos;
        self.pan_offset = [0.0, 0.0];
    }

    pub fn place_waypoint(&mut self, pos: [f32; 3]) {
        self.user_waypoint = Some(pos);
    }

    pub fn clear_waypoint(&mut self) {
        self.user_waypoint = None;
        self.gps_route.clear();
    }

    pub fn waypoint(&self) -> Option<[f32; 3]> {
        self.user_waypoint
    }

    pub fn toggle_dealers(&mut self) {
        self.filter_dealers = !self.filter_dealers;
    }

    pub fn toggle_safehouses(&mut self) {
        self.filter_safehouses = !self.filter_safehouses;
    }

    pub fn toggle_contacts(&mut self) {
        self.filter_contacts = !self.filter_contacts;
    }

    pub fn toggle_stunts(&mut self) {
        self.filter_stunts = !self.filter_stunts;
    }

    pub fn toggle_cameras(&mut self) {
        self.filter_cameras = !self.filter_cameras;
    }

    pub fn set_gps_route(&mut self, path: Vec<[f32; 2]>) {
        self.gps_route = path;
    }

    pub fn zoom(&self) -> f32 {
        self.zoom_level
    }

    pub fn cursor(&self) -> [f32; 2] {
        self.cursor_pos
    }

    pub fn route_points_count(&self) -> usize {
        self.gps_route.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sinking_car_window_break_progression() {
        let mut game = MinigameSession::new_sinking_car();
        assert!(!game.completed);

        // Strike windshield 3 times: not yet broken
        assert!(!game.trigger_action());
        assert!(!game.trigger_action());
        assert!(!game.trigger_action());
        assert!(!game.completed);

        // 4th strike: breaks glass and escapes!
        assert!(game.trigger_action());
        assert!(game.completed);
        assert!(game.reward_message.is_some());
    }

    #[test]
    fn hotwire_ignition_completion() {
        let mut game = MinigameSession::new_hotwire(HotwireMethod::WiresTwist);
        assert!(!game.completed);

        game.trigger_action();
        game.trigger_action();
        assert!(!game.completed);

        game.trigger_action();
        assert!(game.completed);
    }

    #[test]
    fn scratchcard_lottery_match_three() {
        let mut card = MinigameSession::new_scratch_card("I'm Rich", 500);
        // Scratch cells revealing symbols [1, 2, 1, 3, 1, 2]
        // 1st cell (sym 1)
        assert!(!card.trigger_action());
        // 2nd cell (sym 2)
        assert!(!card.trigger_action());
        // 3rd cell (sym 1)
        assert!(!card.trigger_action());
        // 4th cell (sym 3)
        assert!(!card.trigger_action());
        // 5th cell (sym 1) -> 3rd '1' matched! Wins $500
        assert!(card.trigger_action());
        assert!(card.completed);
        assert_eq!(card.reward_cash, 500);
    }

    #[test]
    fn safe_cracking_combination_progress() {
        let mut safe = MinigameSession::new_safe_cracking([15, 42, 88]);
        assert!(!safe.completed);

        // 1st tumbler
        if let MinigameKind::SafeCracking { current_dial, .. } = &mut safe.kind {
            *current_dial = 15;
        }
        assert!(!safe.trigger_action());

        // 2nd tumbler
        if let MinigameKind::SafeCracking { current_dial, .. } = &mut safe.kind {
            *current_dial = 42;
        }
        assert!(!safe.trigger_action());

        // 3rd tumbler
        if let MinigameKind::SafeCracking { current_dial, .. } = &mut safe.kind {
            *current_dial = 88;
        }
        assert!(safe.trigger_action());
        assert!(safe.completed);
        assert_eq!(safe.reward_cash, 2500);
    }

    #[test]
    fn car_bomb_fitting_wiring_and_arming() {
        let mut bomb = MinigameSession::new_car_bomb();
        assert!(!bomb.completed);

        assert!(!bomb.trigger_action());
        assert!(!bomb.trigger_action());
        assert!(bomb.trigger_action());
        assert!(bomb.completed);
        if let MinigameKind::CarBombFitting { wires_connected, armed } = bomb.kind {
            assert_eq!(wires_connected, 3);
            assert!(armed);
        } else {
            panic!("Wrong minigame kind");
        }
    }

    #[test]
    fn tattoo_parlor_needle_tracing_and_completion() {
        let mut tat = MinigameSession::new_tattoo(TattooMotif::DragonOfBroker);
        assert!(!tat.completed);
        assert!(!tat.failed);

        // Apply needle strokes on stencil with ink dips
        for _ in 0..5 {
            if let MinigameKind::Tattooing { ink_level, .. } = tat.kind {
                if ink_level <= 0.0 {
                    tat.dip_ink();
                }
            }
            tat.apply_needle_stroke(true);
        }

        // Final ink dip and finishing pass
        if let MinigameKind::Tattooing { ink_level, .. } = tat.kind {
            if ink_level <= 0.0 {
                tat.dip_ink();
            }
        }
        tat.apply_needle_stroke(true);

        assert!(tat.completed);
        assert_eq!(tat.reward_cash, 1200);
        assert!(tat.reward_message.unwrap().contains("TATOUAGE TERMINÉ"));
    }

    #[test]
    fn tattoo_parlor_needle_slip_causes_pain_and_failure() {
        let mut tat = MinigameSession::new_tattoo(TattooMotif::ViperOfAlgonquin);
        assert!(!tat.failed);

        // Slip multiple times off stencil line: sharp pain spike
        tat.apply_needle_stroke(false);
        tat.apply_needle_stroke(false);
        tat.apply_needle_stroke(false); // pain reaches 1.05 >= 1.0!

        assert!(tat.failed);
        assert!(!tat.completed);
        assert!(tat.reward_message.unwrap().contains("Le client s'est enfui"));
    }

    #[test]
    fn sniper_rifle_assembly_c_rifle_construction_app() {
        let mut game = MinigameSession::new_sniper_assembly();
        assert!(!game.completed);
        assert_eq!(game.reward_cash, 1000);

        // Advance through the 5 assembly stages (3 actions per part)
        for step in 0..5 {
            if let MinigameKind::SniperAssembly { current_step, .. } = game.kind {
                assert_eq!(current_step, step);
            }
            game.trigger_action();
            game.trigger_action();
            game.trigger_action();
        }

        assert!(game.completed);
        if let MinigameKind::SniperAssembly { parts_fitted, .. } = game.kind {
            assert_eq!(parts_fitted, [true; 5], "All 5 sniper rifle components fitted");
        }
        assert!(game.reward_message.unwrap().contains("FUSIL DE PRECISION ASSEMBLE"));
    }

    #[test]
    fn body_searching_c_searching_bodies_app() {
        let mut session = MinigameSession::new_body_search(
            4,
            120,
            Some((crate::drugs::DrugType::Coke, 2)),
            Some((6, 45)),
        );
        assert!(!session.completed);
        assert_eq!(session.body_search_ped_index(), Some(4));
        assert_eq!(session.body_search_progress(), 0.0);

        // Simulation ticks forward 0.6s
        session.tick(0.6);
        assert!(!session.completed);
        let prog = session.body_search_progress();
        assert!(prog >= 0.49 && prog <= 0.51, "Progress at 50%");

        // Simulation completes remaining 0.65s
        session.tick(0.65);
        assert!(session.completed);
        assert_eq!(session.body_search_progress(), 1.0);

        let loot = session.body_search_loot();
        assert_eq!(loot, Some((120, Some((crate::drugs::DrugType::Coke, 2)), Some((6, 45)))));
    }

    #[test]
    fn ammozone_app_cart_checkout_and_delivery() {
        let mut app = AmmozoneApp::new();
        assert_eq!(app.current_category, AmmozoneCategory::Handguns);
        assert!(!app.is_delivering());

        // Category switching
        app.next_category();
        assert_eq!(app.current_category, AmmozoneCategory::Shotguns);
        app.prev_category();
        assert_eq!(app.current_category, AmmozoneCategory::Handguns);

        // Add 2 Pistols (ID 0, $500 ea) to cart
        app.add_to_cart(0, 2);
        assert_eq!(app.cart_total(), 1000);

        // Add 1 AK-47 (ID 7, $3500)
        app.add_to_cart(7, 1);
        assert_eq!(app.cart_total(), 4500);

        // Checkout with insufficient funds
        let mut cash = 3000u32;
        assert!(app.checkout(&mut cash).is_err());
        assert_eq!(cash, 3000);

        // Checkout with sufficient funds
        cash = 6000;
        let checkout_res = app.checkout(&mut cash);
        assert!(checkout_res.is_ok());
        assert_eq!(cash, 1500);
        assert!(app.is_delivering());
        assert!(app.delivery_remaining_seconds() > 0);

        // Advance delivery timer to completion
        assert!(!app.update(20.0));
        assert!(app.is_delivering());
        assert!(app.update(26.0), "Delivery arrives at safehouse!");
        assert!(!app.is_delivering());
        assert_eq!(app.total_delivered_orders, 1);
    }

    #[test]
    fn map_app_navigation_and_waypoints() {
        let mut map = MapApp::new();
        assert_eq!(map.zoom(), 1.0);

        // Zoom controls
        map.zoom_in();
        assert_eq!(map.zoom(), 1.25);
        map.zoom_out();
        assert_eq!(map.zoom(), 1.0);

        // Pan map
        map.pan(50.0, -25.0);
        assert!(map.cursor()[0] > 250.0);

        // Place and clear waypoint
        assert!(map.waypoint().is_none());
        map.place_waypoint([485.0, 275.0, 6.0]);
        assert_eq!(map.waypoint(), Some([485.0, 275.0, 6.0]));
        map.clear_waypoint();
        assert!(map.waypoint().is_none());

        // Toggle filters
        assert!(map.filter_dealers);
        map.toggle_dealers();
        assert!(!map.filter_dealers);
    }
}
