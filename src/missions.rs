// SPDX-License-Identifier: GPL-3.0-only
//! GTA Chinatown Wars authentic Story Missions Engine, Cutscenes Staging & Respawn System.
//! Integrates all 8 Boss storylines (Uncle Kenny, Chan, Zhou, Hsin, Heston, Lester, Rudy, Xin),
//! comic-book cinematic vignettes with dialogue, camera staging positions, and
//! hospital ("WASTED") / police station ("BUSTED") respawn handling.

#![allow(dead_code)]

use std::fmt;

/// Story contact (Boss) offering missions in Liberty City.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Contact {
    UncleKenny,  // Wu "Kenny" Lee (Triads) - Blip "K"
    ChanJaoming, // Chan Jaoming - Blip "C"
    ZhouMing,    // Zhou Ming - Blip "Z"
    HsinJaoming, // Hsin Jaoming (Grand Patriarch) - Blip "H"
    WadeHeston,  // Detective Wade Heston (LCPD) - Blip "W"
    LesterLeroc, // Lester Leroc (Angels of Death undercover) - Blip "L"
    RudyDAvanzo, // Rudy D'Avanzo (Italian Mob) - Blip "R"
    XinShan,     // Xin Shan (PSP exclusive storyline) - Blip "X"
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
            Contact::UncleKenny => [485.0, 275.0, 6.0],  // Kenny's restaurant (Cerveza Heights)
            Contact::ChanJaoming => [230.0, 260.0, 5.0], // Chan's club (Easton)
            Contact::ZhouMing => [270.0, 510.0, 7.0],    // Zhou's penthouse (Middle Park)
            Contact::HsinJaoming => [290.0, 80.0, 5.0],   // Hsin's residence (Chinatown)
            Contact::WadeHeston => [160.0, 690.0, 5.0],  // Seedy pier (Northwood)
            Contact::LesterLeroc => [650.0, 190.0, 4.0], // Biker clubhouse (Willis)
            Contact::RudyDAvanzo => [200.0, 230.0, 5.0], // Italian cafe (Little Italy)
            Contact::XinShan => [490.0, 290.0, 6.0],     // Alleyway in Cerveza Heights
        }
    }
}

impl fmt::Display for Contact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Character speaker in comic vignettes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
    HuangLee,
    UncleKenny,
    ChanJaoming,
    ZhouMing,
    HsinJaoming,
    WadeHeston,
    LesterLeroc,
    RudyDAvanzo,
    XinShan,
    LingShan,
    TriadGoon,
    PoliceOfficer,
}

impl Speaker {
    pub fn display_name(self) -> &'static str {
        match self {
            Speaker::HuangLee => "Huang Lee",
            Speaker::UncleKenny => "Uncle Kenny",
            Speaker::ChanJaoming => "Chan Jaoming",
            Speaker::ZhouMing => "Zhou Ming",
            Speaker::HsinJaoming => "Hsin Jaoming",
            Speaker::WadeHeston => "Det. Heston",
            Speaker::LesterLeroc => "Lester Leroc",
            Speaker::RudyDAvanzo => "Rudy D'Avanzo",
            Speaker::XinShan => "Xin Shan",
            Speaker::LingShan => "Ling Shan",
            Speaker::TriadGoon => "Triad Soldier",
            Speaker::PoliceOfficer => "LCPD Officer",
        }
    }
}

/// Single comic panel inside a story cutscene briefing.
#[derive(Debug, Clone, PartialEq)]
pub struct ComicPanel {
    pub speaker: Speaker,
    pub dialogue_en: &'static str,
    pub dialogue_fr: &'static str,
    pub camera_focus: [f32; 3],
    pub camera_zoom: f32,
    pub duration_frames: u32,
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
    /// Escort or tail a contact vehicle.
    TailVehicle {
        target_pos: [f32; 3],
        min_distance: f32,
        max_distance: f32,
        description: &'static str,
    },
}

/// Mission status state.
#[derive(Debug, Clone, PartialEq)]
pub enum MissionState {
    NotStarted,
    Available,
    Briefing {
        vignette_id: u32,
        panel_idx: usize,
        elapsed_frames: u32,
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
    pub vignette: Vec<ComicPanel>,
    pub steps: Vec<MissionObjective>,
}

/// Hospital respawn centers in Liberty City.
pub const HOSPITALS: [(&str, [f32; 3]); 3] = [
    ("Hope Medical Center (Algonquin)", [180.0, 310.0, 5.0]),
    ("Schottler Medical Center (Broker)", [490.0, 180.0, 6.0]),
    ("Cerveza Heights Medical (Dukes)", [510.0, 310.0, 6.0]),
];

/// Police station respawn stations in Liberty City.
pub const POLICE_STATIONS: [(&str, [f32; 3]); 3] = [
    ("East Island City Police (Dukes)", [520.0, 260.0, 6.0]),
    ("Lower Easton Police (Algonquin)", [240.0, 220.0, 5.0]),
    ("Fortside Police (Bohan)", [380.0, 540.0, 7.0]),
];

/// Player fate handling for WASTED and BUSTED states.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerFate {
    Healthy,
    Wasted {
        timer: f32,
        hospital_idx: usize,
        fee: u32,
    },
    Busted {
        timer: f32,
        station_idx: usize,
        bail: u32,
        contraband_seized: bool,
    },
}

/// Finds the nearest hospital for respawn after death.
pub fn find_nearest_hospital(pos: [f32; 3]) -> (usize, &'static str, [f32; 3]) {
    let mut best_idx = 0;
    let mut best_dist_sq = f32::MAX;
    for (idx, &(_name, hpos)) in HOSPITALS.iter().enumerate() {
        let dx = pos[0] - hpos[0];
        let dy = pos[1] - hpos[1];
        let dist_sq = dx * dx + dy * dy;
        if dist_sq < best_dist_sq {
            best_dist_sq = dist_sq;
            best_idx = idx;
        }
    }
    let (name, hpos) = HOSPITALS[best_idx];
    (best_idx, name, hpos)
}

/// Finds the nearest police station for respawn after arrest.
pub fn find_nearest_police_station(pos: [f32; 3]) -> (usize, &'static str, [f32; 3]) {
    let mut best_idx = 0;
    let mut best_dist_sq = f32::MAX;
    for (idx, &(_name, spos)) in POLICE_STATIONS.iter().enumerate() {
        let dx = pos[0] - spos[0];
        let dy = pos[1] - spos[1];
        let dist_sq = dx * dx + dy * dy;
        if dist_sq < best_dist_sq {
            best_dist_sq = dist_sq;
            best_idx = idx;
        }
    }
    let (name, spos) = POLICE_STATIONS[best_idx];
    (best_idx, name, spos)
}

/// Door side determination for vehicle entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorSide {
    Driver,
    Passenger,
}

/// Computes the nearest vehicle door for the player to enter.
pub fn calculate_nearest_door(
    player_pos: [f32; 2],
    car_pos: [f32; 2],
    car_heading: f32,
    half_width: f32,
) -> (DoorSide, [f32; 2]) {
    // Normal perpendicular to car heading: left is driver, right is passenger
    let side_normal = [car_heading.cos(), car_heading.sin()];
    let driver_door = [
        car_pos[0] - side_normal[0] * half_width,
        car_pos[1] - side_normal[1] * half_width,
    ];
    let passenger_door = [
        car_pos[0] + side_normal[0] * half_width,
        car_pos[1] + side_normal[1] * half_width,
    ];

    let dx_drv = player_pos[0] - driver_door[0];
    let dy_drv = player_pos[1] - driver_door[1];
    let dist_drv = dx_drv * dx_drv + dy_drv * dy_drv;

    let dx_pass = player_pos[0] - passenger_door[0];
    let dy_pass = player_pos[1] - passenger_door[1];
    let dist_pass = dx_pass * dx_pass + dy_pass * dy_pass;

    if dist_drv <= dist_pass {
        (DoorSide::Driver, driver_door)
    } else {
        (DoorSide::Passenger, passenger_door)
    }
}

/// Siren state for emergency vehicles.
#[derive(Debug, Clone, PartialEq)]
pub struct SirenState {
    pub active: bool,
    pub is_emergency_vehicle: bool,
    pub flash_phase: f32,
    pub sound_loop_tick: u32,
}

impl SirenState {
    pub fn new(is_emergency: bool) -> Self {
        Self {
            active: false,
            is_emergency_vehicle: is_emergency,
            flash_phase: 0.0,
            sound_loop_tick: 0,
        }
    }

    pub fn toggle(&mut self) -> bool {
        if self.is_emergency_vehicle {
            self.active = !self.active;
            self.active
        } else {
            false
        }
    }

    pub fn update(&mut self, dt: f32) {
        if self.active {
            self.flash_phase = (self.flash_phase + dt * 6.0).rem_euclid(std::f32::consts::TAU);
            self.sound_loop_tick = self.sound_loop_tick.wrapping_add(1);
        }
    }
}

/// Mission Engine orchestrating story progression across all 8 boss contacts.
pub struct MissionEngine {
    pub catalog: Vec<MissionDef>,
    pub current_mission_idx: Option<usize>,
    pub state: MissionState,
    pub passed_count: u32,
    pub failed_count: u32,
    pub player_fate: PlayerFate,
}

impl Default for MissionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MissionEngine {
    pub fn new() -> Self {
        let mut catalog = Vec::new();

        // 1. UNCLE KENNY MISSIONS
        catalog.push(MissionDef {
            id: 1,
            overlay_name: "ken_a01",
            title: "Yu Jian",
            contact: Contact::UncleKenny,
            reward_cash: 50,
            vignette: vec![
                ComicPanel {
                    speaker: Speaker::UncleKenny,
                    dialogue_en: "Huang! You lost Yu Jian, the family sword?! My honor is ruined!",
                    dialogue_fr: "Huang ! Tu as perdu Yu Jian, l'épée familiale ?! Mon honneur est ruiné !",
                    camera_focus: [485.0, 275.0, 6.0],
                    camera_zoom: 1.2,
                    duration_frames: 120,
                },
                ComicPanel {
                    speaker: Speaker::HuangLee,
                    dialogue_en: "I was ambushed at the airport! They shot me and left me to drown.",
                    dialogue_fr: "J'ai été pris en embuscade à l'aéroport ! Ils m'ont tiré dessus et jeté à l'eau.",
                    camera_focus: [484.0, 276.0, 6.0],
                    camera_zoom: 1.4,
                    duration_frames: 120,
                },
            ],
            steps: vec![
                MissionObjective::CompleteMinigame { overlay_name: "sinkingcarapp" },
                MissionObjective::GoToLocation {
                    target: [485.0, 275.0, 6.0],
                    radius: 8.0,
                    description: "Drive to Uncle Kenny's restaurant in Cerveza Heights",
                },
            ],
        });

        catalog.push(MissionDef {
            id: 2,
            overlay_name: "ken_a02",
            title: "Pursuit Farce",
            contact: Contact::UncleKenny,
            reward_cash: 100,
            vignette: vec![
                ComicPanel {
                    speaker: Speaker::UncleKenny,
                    dialogue_en: "The cops are crawling all over Broker. Shake them off before you bring heat here!",
                    dialogue_fr: "Les flics grouillent dans Broker. Sème-les avant d'attirer des ennuis ici !",
                    camera_focus: [485.0, 275.0, 6.0],
                    camera_zoom: 1.1,
                    duration_frames: 120,
                },
            ],
            steps: vec![
                MissionObjective::LoseWantedLevel { initial_stars: 2 },
                MissionObjective::GoToLocation {
                    target: [480.0, 290.0, 6.0],
                    radius: 6.0,
                    description: "Head to your new Safehouse in Cerveza Heights",
                },
            ],
        });

        catalog.push(MissionDef {
            id: 3,
            overlay_name: "ken_a03",
            title: "Under the Gun",
            contact: Contact::UncleKenny,
            reward_cash: 150,
            vignette: vec![
                ComicPanel {
                    speaker: Speaker::LingShan,
                    dialogue_en: "Keep your guard up, Huang. In Liberty City, shooting comes before talking.",
                    dialogue_fr: "Reste sur tes gardes, Huang. À Liberty City, on tire d'abord et on parle ensuite.",
                    camera_focus: [485.0, 275.0, 6.0],
                    camera_zoom: 1.3,
                    duration_frames: 120,
                },
            ],
            steps: vec![
                MissionObjective::KillTargets {
                    total: 3,
                    remaining: 3,
                    description: "Take out the thugs attacking the alley",
                },
            ],
        });

        catalog.push(MissionDef {
            id: 4,
            overlay_name: "ken_a04",
            title: "Payback",
            contact: Contact::UncleKenny,
            reward_cash: 200,
            vignette: vec![
                ComicPanel {
                    speaker: Speaker::UncleKenny,
                    dialogue_en: "Those Spanish Lords insulted our turf. Go burn their vehicles down!",
                    dialogue_fr: "Ces Spanish Lords ont insulté notre territoire. Va brûler leurs caisses !",
                    camera_focus: [485.0, 275.0, 6.0],
                    camera_zoom: 1.2,
                    duration_frames: 120,
                },
            ],
            steps: vec![
                MissionObjective::KillTargets {
                    total: 4,
                    remaining: 4,
                    description: "Destroy the Spanish Lords' cars and crews",
                },
            ],
        });

        // 2. CHAN JAOMING MISSIONS
        catalog.push(MissionDef {
            id: 5,
            overlay_name: "cha_a01",
            title: "Pimp His Ride",
            contact: Contact::ChanJaoming,
            reward_cash: 300,
            vignette: vec![
                ComicPanel {
                    speaker: Speaker::ChanJaoming,
                    dialogue_en: "My rival is entering that championship race. Sabotage his engine, make it subtle!",
                    dialogue_fr: "Mon rival s'est inscrit à la course. Sabote son moteur, mais fais ça discrètement !",
                    camera_focus: [230.0, 260.0, 5.0],
                    camera_zoom: 1.2,
                    duration_frames: 120,
                },
            ],
            steps: vec![
                MissionObjective::CompleteMinigame { overlay_name: "hotwiringapp" },
                MissionObjective::GoToLocation {
                    target: [230.0, 260.0, 5.0],
                    radius: 7.0,
                    description: "Deliver the sabotaged sports car back to the club",
                },
            ],
        });

        catalog.push(MissionDef {
            id: 6,
            overlay_name: "cha_a02",
            title: "Whack the Racers",
            contact: Contact::ChanJaoming,
            reward_cash: 450,
            vignette: vec![
                ComicPanel {
                    speaker: Speaker::ChanJaoming,
                    dialogue_en: "Ram the competitors off the road so I win the golden cup!",
                    dialogue_fr: "Éjecte les concurrents hors de la route pour que je remporte la coupe dorée !",
                    camera_focus: [230.0, 260.0, 5.0],
                    camera_zoom: 1.2,
                    duration_frames: 120,
                },
            ],
            steps: vec![
                MissionObjective::TailVehicle {
                    target_pos: [240.0, 270.0, 5.0],
                    min_distance: 5.0,
                    max_distance: 40.0,
                    description: "Follow and ram Chan's rival racers",
                },
            ],
        });

        // 3. ZHOU MING MISSIONS
        catalog.push(MissionDef {
            id: 7,
            overlay_name: "zho_a01",
            title: "Driven to Destruction",
            contact: Contact::ZhouMing,
            reward_cash: 400,
            vignette: vec![
                ComicPanel {
                    speaker: Speaker::ZhouMing,
                    dialogue_en: "Drive me through the enemy blockade, Huang. I want to see them crumble under my fire!",
                    dialogue_fr: "Conduis-moi à travers le barrage ennemi, Huang. Je veux les voir tomber sous mes tirs !",
                    camera_focus: [270.0, 510.0, 7.0],
                    camera_zoom: 1.3,
                    duration_frames: 120,
                },
            ],
            steps: vec![
                MissionObjective::KillTargets {
                    total: 5,
                    remaining: 5,
                    description: "Eliminate rival syndicate ambushers",
                },
            ],
        });

        // 4. HSIN JAOMING (GRAND PATRIARCH) MISSIONS
        catalog.push(MissionDef {
            id: 8,
            overlay_name: "hsi_a01",
            title: "The New Syndicate",
            contact: Contact::HsinJaoming,
            reward_cash: 1000,
            vignette: vec![
                ComicPanel {
                    speaker: Speaker::HsinJaoming,
                    dialogue_en: "A rat is leaking our operations to the Feds. Find who betrayed the Triads!",
                    dialogue_fr: "Une taupe balance nos opérations aux Fédéraux. Découvre qui a trahi les Triades !",
                    camera_focus: [290.0, 80.0, 5.0],
                    camera_zoom: 1.4,
                    duration_frames: 150,
                },
            ],
            steps: vec![
                MissionObjective::GoToLocation {
                    target: [280.0, 100.0, 5.0],
                    radius: 6.0,
                    description: "Meet Hsin's informant in Chinatown",
                },
                MissionObjective::LoseWantedLevel { initial_stars: 3 },
            ],
        });

        // 5. WADE HESTON (LCPD) MISSIONS
        catalog.push(MissionDef {
            id: 9,
            overlay_name: "hes_a01",
            title: "The Wheelman",
            contact: Contact::WadeHeston,
            reward_cash: 500,
            vignette: vec![
                ComicPanel {
                    speaker: Speaker::WadeHeston,
                    dialogue_en: "You help me clear my badge with Internal Affairs, and I'll find who stole your sword.",
                    dialogue_fr: "Aide-moi à blanchir mon insigne auprès de la police des polices, et je trouverai ton épée.",
                    camera_focus: [160.0, 690.0, 5.0],
                    camera_zoom: 1.2,
                    duration_frames: 120,
                },
            ],
            steps: vec![
                MissionObjective::CompleteMinigame { overlay_name: "hotwiringapp" },
                MissionObjective::GoToLocation {
                    target: [160.0, 690.0, 5.0],
                    radius: 7.0,
                    description: "Deliver the sports car to Heston's lockup",
                },
            ],
        });

        // 6. LESTER LEROC (ANGELS OF DEATH) MISSIONS
        catalog.push(MissionDef {
            id: 10,
            overlay_name: "les_a01",
            title: "The Biker Initiation",
            contact: Contact::LesterLeroc,
            reward_cash: 600,
            vignette: vec![
                ComicPanel {
                    speaker: Speaker::LesterLeroc,
                    dialogue_en: "I'm deep undercover with the Angels of Death. Ride with me and make me look tough!",
                    dialogue_fr: "Je suis infiltré chez les Angels of Death. Roule avec moi et fais-moi passer pour un dur !",
                    camera_focus: [650.0, 190.0, 4.0],
                    camera_zoom: 1.2,
                    duration_frames: 120,
                },
            ],
            steps: vec![
                MissionObjective::TailVehicle {
                    target_pos: [660.0, 200.0, 4.0],
                    min_distance: 3.0,
                    max_distance: 30.0,
                    description: "Escort Lester's chopper without falling behind",
                },
            ],
        });

        // 7. RUDY D'AVANZO (ITALIAN MOB) MISSIONS
        catalog.push(MissionDef {
            id: 11,
            overlay_name: "rud_a01",
            title: "Grave Danger",
            contact: Contact::RudyDAvanzo,
            reward_cash: 750,
            vignette: vec![
                ComicPanel {
                    speaker: Speaker::RudyDAvanzo,
                    dialogue_en: "The Messina family thinks they own Little Italy. Let's dig up some dirt on them.",
                    dialogue_fr: "La famille Messina croit posséder Little Italy. Déterrons des secrets compromettants.",
                    camera_focus: [200.0, 230.0, 5.0],
                    camera_zoom: 1.3,
                    duration_frames: 120,
                },
            ],
            steps: vec![
                MissionObjective::GoToLocation {
                    target: [210.0, 240.0, 5.0],
                    radius: 6.0,
                    description: "Retrieve Rudy's confidential blackmail ledger",
                },
            ],
        });

        // 8. XIN SHAN (PSP EXCLUSIVE STORYLINE)
        catalog.push(MissionDef {
            id: 12,
            overlay_name: "xin_a01",
            title: "Deadly Arrival",
            contact: Contact::XinShan,
            reward_cash: 1200,
            vignette: vec![
                ComicPanel {
                    speaker: Speaker::XinShan,
                    dialogue_en: "Huang! I am Ling's brother. She spoke of you before she died... We must take revenge.",
                    dialogue_fr: "Huang ! Je suis le frère de Ling. Elle m'a parlé de toi avant de mourir... Vengeons-la.",
                    camera_focus: [490.0, 290.0, 6.0],
                    camera_zoom: 1.4,
                    duration_frames: 150,
                },
            ],
            steps: vec![
                MissionObjective::KillTargets {
                    total: 6,
                    remaining: 6,
                    description: "Ambush and take out Ling's assassins",
                },
            ],
        });

        Self {
            catalog,
            current_mission_idx: None,
            state: MissionState::NotStarted,
            passed_count: 0,
            failed_count: 0,
            player_fate: PlayerFate::Healthy,
        }
    }

    /// Triggers mission start and opens comic vignette briefing.
    pub fn start_mission(&mut self, mission_id: u32) -> Result<(), &'static str> {
        let idx = self.catalog.iter().position(|m| m.id == mission_id).ok_or("Mission not found")?;
        self.current_mission_idx = Some(idx);
        self.state = MissionState::Briefing {
            vignette_id: mission_id,
            panel_idx: 0,
            elapsed_frames: 0,
        };
        Ok(())
    }

    /// Progresses cutscene vignette frame-by-frame.
    pub fn update_vignette(&mut self) -> bool {
        let idx = match self.current_mission_idx {
            Some(i) => i,
            None => return false,
        };

        if let MissionState::Briefing { panel_idx, elapsed_frames, .. } = &mut self.state {
            let mission = &self.catalog[idx];
            if mission.vignette.is_empty() {
                let _ = self.begin_gameplay();
                return true;
            }

            *elapsed_frames += 1;
            let current_panel = &mission.vignette[*panel_idx];
            if *elapsed_frames >= current_panel.duration_frames {
                if *panel_idx + 1 < mission.vignette.len() {
                    *panel_idx += 1;
                    *elapsed_frames = 0;
                } else {
                    let _ = self.begin_gameplay();
                    return true;
                }
            }
            false
        } else {
            false
        }
    }

    /// Skips current briefing cutscene and launches gameplay immediately.
    pub fn skip_vignette(&mut self) -> Result<(), &'static str> {
        self.begin_gameplay()
    }

    /// Advances briefing vignette to active gameplay.
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

    /// Triggers player death ("WASTED"), deducts medical bill, fails active mission.
    pub fn trigger_wasted(&mut self, player_pos: [f32; 3], player_cash: &mut u32) -> [f32; 3] {
        let (hospital_idx, _, respawn_pos) = find_nearest_hospital(player_pos);
        let fee = (*player_cash / 10).min(100);
        *player_cash = player_cash.saturating_sub(fee);

        self.player_fate = PlayerFate::Wasted {
            timer: 3.5,
            hospital_idx,
            fee,
        };

        if matches!(self.state, MissionState::InProgress { .. } | MissionState::Briefing { .. }) {
            self.fail_mission("Huang was wasted");
        }

        respawn_pos
    }

    /// Triggers player arrest ("BUSTED"), deducts bail, confiscates weapons, fails active mission.
    pub fn trigger_busted(
        &mut self,
        player_pos: [f32; 3],
        player_cash: &mut u32,
        weapons: &mut Vec<u32>,
    ) -> [f32; 3] {
        let (station_idx, _, respawn_pos) = find_nearest_police_station(player_pos);
        let bail = (*player_cash / 10).min(200);
        *player_cash = player_cash.saturating_sub(bail);
        weapons.clear(); // Police seize all firearms

        self.player_fate = PlayerFate::Busted {
            timer: 3.5,
            station_idx,
            bail,
            contraband_seized: true,
        };

        if matches!(self.state, MissionState::InProgress { .. } | MissionState::Briefing { .. }) {
            self.fail_mission("Huang was busted by the LCPD");
        }

        respawn_pos
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
        assert_eq!(Contact::LesterLeroc.radar_blip_symbol(), 'L');
        assert_eq!(Contact::RudyDAvanzo.radar_blip_symbol(), 'R');
        assert_eq!(Contact::XinShan.radar_blip_symbol(), 'X');

        let ken_pos = Contact::UncleKenny.meeting_pos();
        assert!(ken_pos[0] > 0.0 && ken_pos[1] > 0.0);
    }

    #[test]
    fn comic_vignette_briefing_progression_and_skipping() {
        let mut engine = MissionEngine::new();
        // Start mission 1: Yu Jian (Uncle Kenny)
        assert!(engine.start_mission(1).is_ok());

        // Check briefing state
        match engine.state {
            MissionState::Briefing { vignette_id, panel_idx, .. } => {
                assert_eq!(vignette_id, 1);
                assert_eq!(panel_idx, 0);
            }
            _ => panic!("Expected Briefing state"),
        }

        // Test skipping briefing cutscene
        assert!(engine.skip_vignette().is_ok());
        match engine.state {
            MissionState::InProgress { current_step, .. } => assert_eq!(current_step, 0),
            _ => panic!("Expected InProgress after skipping vignette"),
        }
    }

    #[test]
    fn full_story_campaign_catalog_all_8_bosses() {
        let engine = MissionEngine::new();
        assert_eq!(engine.catalog.len(), 12, "Includes all 12 core story missions across 8 contacts");

        let contacts_present: std::collections::HashSet<_> =
            engine.catalog.iter().map(|m| m.contact).collect();
        assert_eq!(contacts_present.len(), 8, "All 8 bosses represented in mission campaign");
    }

    #[test]
    fn nearest_hospital_and_police_station_selection() {
        // Position near Broker
        let pos_broker = [480.0, 190.0, 6.0];
        let (h_idx, h_name, _) = find_nearest_hospital(pos_broker);
        assert_eq!(h_idx, 1);
        assert!(h_name.contains("Schottler"));

        // Position near Algonquin
        let pos_algonquin = [170.0, 320.0, 5.0];
        let (h_idx_alg, h_name_alg, _) = find_nearest_hospital(pos_algonquin);
        assert_eq!(h_idx_alg, 0);
        assert!(h_name_alg.contains("Hope"));

        let (p_idx, p_name, _) = find_nearest_police_station(pos_broker);
        assert_eq!(p_idx, 0);
        assert!(p_name.contains("East Island City"));
    }

    #[test]
    fn wasted_and_busted_player_consequences() {
        let mut engine = MissionEngine::new();
        let mut cash = 500u32;
        let mut weapons = vec![1, 2, 3]; // Pistol, Shotgun, SMG

        // Trigger active mission
        engine.start_mission(1).unwrap();
        engine.skip_vignette().unwrap();

        // Player dies -> Wasted
        let respawn = engine.trigger_wasted([480.0, 190.0, 6.0], &mut cash);
        assert_eq!(cash, 450, "10% medical fee deducted (max 100$)");
        assert_eq!(engine.state, MissionState::Failed { reason: "Huang was wasted" });
        assert_eq!(respawn, HOSPITALS[1].1, "Respawns at Schottler Medical");

        // Player arrested -> Busted
        let mut cash_busted = 1000u32;
        let respawn_cop = engine.trigger_busted([510.0, 250.0, 6.0], &mut cash_busted, &mut weapons);
        assert_eq!(cash_busted, 900, "Bail deducted");
        assert!(weapons.is_empty(), "Contraband weapons confiscated by police");
        assert_eq!(respawn_cop, POLICE_STATIONS[0].1, "Respawns at East Island City Police");
    }

    #[test]
    fn vehicle_nearest_door_and_siren_toggle() {
        let car_pos = [100.0, 100.0];
        let car_heading = 0.0; // facing North, sides are West (-x) and East (+x)
        let half_width = 1.0;

        // Player stands to the left (Driver side)
        let player_left = [98.0, 100.0];
        let (side, door_pos) = calculate_nearest_door(player_left, car_pos, car_heading, half_width);
        assert_eq!(side, DoorSide::Driver);
        assert!((door_pos[0] - 99.0).abs() < 1e-4);

        // Player stands to the right (Passenger side)
        let player_right = [102.0, 100.0];
        let (side_pass, _) = calculate_nearest_door(player_right, car_pos, car_heading, half_width);
        assert_eq!(side_pass, DoorSide::Passenger);

        // Siren toggle test
        let mut siren_cop = SirenState::new(true);
        assert!(!siren_cop.active);
        assert!(siren_cop.toggle());
        assert!(siren_cop.active);
        siren_cop.update(0.1);
        assert!(siren_cop.sound_loop_tick > 0);

        let mut siren_civilian = SirenState::new(false);
        assert!(!siren_civilian.toggle(), "Civilian vehicles cannot turn on sirens");
    }
}
