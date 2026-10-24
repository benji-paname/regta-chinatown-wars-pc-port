// SPDX-License-Identifier: GPL-3.0-only
//! GTA Chinatown Wars authentic Story Missions Engine and Boss Contact State Machine.
//! Integrates mission overlays (ken_a01..zho_c01), objective stages, golden radar blips,
//! comic vignettes briefing cutscenes, cash rewards, and mission pass/fail stats tracking.

#![allow(dead_code)]

use std::fmt;

/// Story contact (Boss) offering missions in Liberty City.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Contact {
    UncleKenny, // Wu "Kenny" Lee (Triads) - Blip "K"
    ChanJaoming, // Chan Jaoming - Blip "C"
    ZhouMing, // Zhou Ming - Blip "Z"
    HsinJaoming, // Hsin Jaoming (Grand Patriarch) - Blip "H"
    WadeHeston, // Detective Wade Heston (LCPD) - Blip "W"
    LesterLeroc, // Lester Leroc (Angels of Death undercover) - Blip "L"
    RudyDAvanzo, // Rudy D'Avanzo (Italian Mob) - Blip "R"
    XinShan, // Xin Shan (PSP exclusive missions) - Blip "X"
}

impl Contact {
    pub fn name(self) -> &'static str {
        match self {
            Contact::UncleKenny => "Wu 'Kenny' Lee",
            Contact::ChanJaoming => "Chan Jaoming",
            Contact::ZhouMing => "Zhou Ming",
            Contact::HsinJaoming => "Hsin Jaoming",
            Contact::WadeHeston => "Wade Heston",
            Contact::LesterLeroc => "Lester Leroc",
            Contact::RudyDAvanzo => "Rudy D'Avanzo",
            Contact::XinShan => "Xin Shan",
        }
    }

    pub fn radar_blip_symbol(self) -> char {
        match self {
            Contact::UncleKenny => 'K',
            Contact::ChanJaoming => 'C',
            Contact::ZhouMing => 'Z',
            Contact::HsinJaoming => 'H',
            Contact::WadeHeston => 'W',
            Contact::LesterLeroc => 'L',
            Contact::RudyDAvanzo => 'R',
            Contact::XinShan => 'X',
        }
    }

    /// World coordinates of contact meeting location.
    pub fn meeting_pos(self) -> [f32; 3] {
        match self {
            Contact::UncleKenny => [485.0, 275.0, 6.0], // Kenny's restaurant (Cerveza Heights)
            Contact::ChanJaoming => [230.0, 260.0, 5.0], // Chan's club (Easton)
            Contact::ZhouMing => [270.0, 510.0, 7.0], // Zhou's penthouse (Middle Park)
            Contact::HsinJaoming => [290.0, 80.0, 5.0], // Hsin's residence (Chinatown)
            Contact::WadeHeston => [160.0, 690.0, 5.0], // Seedy pier (Northwood)
            Contact::LesterLeroc => [650.0, 190.0, 4.0], // Biker clubhouse (Willis)
            Contact::RudyDAvanzo => [200.0, 230.0, 5.0], // Italian cafe (Little Italy)
            Contact::XinShan => [490.0, 290.0, 6.0], // Alleyway in Cerveza Heights
        }
    }
}

impl fmt::Display for Contact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Dynamic mission objective in progress.
#[derive(Debug, Clone, PartialEq)]
pub enum MissionObjective {
    /// Travel to world waypoint.
    GoToLocation {
        target: [f32; 3],
        radius: f32,
        description: &'static str,
    },
    /// Eliminate enemies.
    KillTargets {
        total: u32,
        remaining: u32,
        description: &'static str,
    },
    /// Evade police attention (reach 0 wanted stars).
    LoseWantedLevel {
        initial_stars: u32,
    },
    /// Survive for specified number of frames (30 frames/sec).
    SurviveFrames {
        frames_left: u32,
    },
    /// Complete interactive minigame via overlay.
    CompleteMinigame {
        overlay_name: &'static str,
    },
}

/// Mission status state.
#[derive(Debug, Clone, PartialEq)]
pub enum MissionState {
    NotStarted,
    Available,
    Briefing {
        vignette_id: u32,
    },
    InProgress {
        current_step: usize,
        objective: MissionObjective,
    },
    Passed {
        reward_cash: u32,
    },
    Failed {
        reason: &'static str,
    },
}

/// Static description of a story mission matching an overlay.
#[derive(Debug, Clone)]
pub struct MissionDef {
    pub id: u32,
    pub overlay_name: &'static str,
    pub title: &'static str,
    pub contact: Contact,
    pub reward_cash: u32,
    pub steps: Vec<MissionObjective>,
}

/// Mission Engine orchestrating story progression.
pub struct MissionEngine {
    pub catalog: Vec<MissionDef>,
    pub current_mission_idx: Option<usize>,
    pub state: MissionState,
    pub passed_count: u32,
    pub failed_count: u32,
}

impl Default for MissionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MissionEngine {
    pub fn new() -> Self {
        // Authentic prologue and opening story missions
        let catalog = vec![
            MissionDef {
                id: 1,
                overlay_name: "ken_a01",
                title: "Yu Jian",
                contact: Contact::UncleKenny,
                reward_cash: 50,
                steps: vec![
                    MissionObjective::CompleteMinigame { overlay_name: "sinkingcarapp" },
                    MissionObjective::GoToLocation {
                        target: [485.0, 275.0, 6.0],
                        radius: 8.0,
                        description: "Drive to Uncle Kenny's restaurant in Cerveza Heights",
                    },
                ],
            },
            MissionDef {
                id: 2,
                overlay_name: "ken_a02",
                title: "Pursuit Farce",
                contact: Contact::UncleKenny,
                reward_cash: 100,
                steps: vec![
                    MissionObjective::LoseWantedLevel { initial_stars: 2 },
                    MissionObjective::GoToLocation {
                        target: [480.0, 290.0, 6.0],
                        radius: 6.0,
                        description: "Head to your new Safehouse in Cerveza Heights",
                    },
                ],
            },
            MissionDef {
                id: 3,
                overlay_name: "ken_a03",
                title: "Under the Gun",
                contact: Contact::UncleKenny,
                reward_cash: 150,
                steps: vec![
                    MissionObjective::KillTargets {
                        total: 3,
                        remaining: 3,
                        description: "Take out the thugs attacking the alley",
                    },
                ],
            },
            MissionDef {
                id: 4,
                overlay_name: "hes_a01",
                title: "The Wheelman",
                contact: Contact::WadeHeston,
                reward_cash: 500,
                steps: vec![
                    MissionObjective::CompleteMinigame { overlay_name: "hotwiringapp" },
                    MissionObjective::GoToLocation {
                        target: [160.0, 690.0, 5.0],
                        radius: 7.0,
                        description: "Deliver the sports car to Heston's lockup",
                    },
                ],
            },
        ];

        Self {
            catalog,
            current_mission_idx: None,
            state: MissionState::NotStarted,
            passed_count: 0,
            failed_count: 0,
        }
    }

    /// Triggers mission start when player enters contact beacon.
    pub fn start_mission(&mut self, mission_id: u32) -> Result<(), &'static str> {
        let idx = self.catalog.iter().position(|m| m.id == mission_id).ok_or("Mission not found")?;
        self.current_mission_idx = Some(idx);
        self.state = MissionState::Briefing { vignette_id: mission_id };
        Ok(())
    }

    /// Advances briefing vignette to gameplay.
    pub fn begin_gameplay(&mut self) -> Result<(), &'static str> {
        let idx = self.current_mission_idx.ok_or("No mission active")?;
        let mission = &self.catalog[idx];
        let first_step = mission.steps.first().cloned().ok_or("Mission has no steps")?;

        self.state = MissionState::InProgress {
            current_step: 0,
            objective: first_step,
        };
        Ok(())
    }

    /// Progresses current objective stage or passes the mission if final step completed.
    pub fn advance_objective(&mut self, player_cash: &mut u32) -> bool {
        let idx = match self.current_mission_idx {
            Some(i) => i,
            None => return false,
        };

        let (next_step_idx, is_final) = match &self.state {
            MissionState::InProgress { current_step, .. } => {
                let next = current_step + 1;
                let mission = &self.catalog[idx];
                if next >= mission.steps.len() {
                    (next, true)
                } else {
                    (next, false)
                }
            }
            _ => return false,
        };

        if is_final {
            let reward = self.catalog[idx].reward_cash;
            *player_cash = player_cash.saturating_add(reward);
            self.state = MissionState::Passed { reward_cash: reward };
            self.passed_count += 1;
            true
        } else {
            let next_obj = self.catalog[idx].steps[next_step_idx].clone();
            self.state = MissionState::InProgress {
                current_step: next_step_idx,
                objective: next_obj,
            };
            false
        }
    }

    /// Fails the current mission with a stated reason.
    pub fn fail_mission(&mut self, reason: &'static str) {
        if self.current_mission_idx.is_some() {
            self.state = MissionState::Failed { reason };
            self.failed_count += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_contacts_and_radar_symbols() {
        assert_eq!(Contact::UncleKenny.radar_blip_symbol(), 'K');
        assert_eq!(Contact::ChanJaoming.radar_blip_symbol(), 'C');
        assert_eq!(Contact::ZhouMing.radar_blip_symbol(), 'Z');
        assert_eq!(Contact::HsinJaoming.radar_blip_symbol(), 'H');
        assert_eq!(Contact::WadeHeston.radar_blip_symbol(), 'W');

        let ken_pos = Contact::UncleKenny.meeting_pos();
        assert!(ken_pos[0] > 0.0 && ken_pos[1] > 0.0);
    }

    #[test]
    fn mission_progression_and_cash_reward() {
        let mut engine = MissionEngine::new();
        let mut player_cash = 200u32;

        // Start mission 1: Yu Jian (Uncle Kenny)
        assert!(engine.start_mission(1).is_ok());
        assert_eq!(engine.state, MissionState::Briefing { vignette_id: 1 });

        // Start gameplay (step 0: sinking car minigame)
        assert!(engine.begin_gameplay().is_ok());
        match &engine.state {
            MissionState::InProgress { current_step, objective } => {
                assert_eq!(*current_step, 0);
                assert_eq!(*objective, MissionObjective::CompleteMinigame { overlay_name: "sinkingcarapp" });
            }
            _ => panic!("Expected InProgress state"),
        }

        // Complete step 0 -> advances to step 1 (Drive to restaurant)
        assert!(!engine.advance_objective(&mut player_cash));
        match &engine.state {
            MissionState::InProgress { current_step, .. } => assert_eq!(*current_step, 1),
            _ => panic!("Expected step 1 InProgress"),
        }

        // Complete final step -> Mission Passed and cash rewarded (+50$)
        assert!(engine.advance_objective(&mut player_cash));
        assert_eq!(player_cash, 250);
        assert_eq!(engine.state, MissionState::Passed { reward_cash: 50 });
        assert_eq!(engine.passed_count, 1);
    }

    #[test]
    fn mission_failure_tracks_stats() {
        let mut engine = MissionEngine::new();
        let cash = 0u32;

        engine.start_mission(2).unwrap();
        engine.begin_gameplay().unwrap();
        engine.fail_mission("Huang was wasted by police");

        assert_eq!(engine.state, MissionState::Failed { reason: "Huang was wasted by police" });
        assert_eq!(engine.failed_count, 1);
        assert_eq!(cash, 0);
    }
}
