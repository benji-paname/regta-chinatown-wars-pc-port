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
            _ => {}
        }
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
}
