// SPDX-License-Identifier: GPL-3.0-only
//! GTA Chinatown Wars authentic Street Races Engine (Overlay 140 streetrace).
//! Implements all 16 street races and time trials across Broker, Dukes, Bohan,
//! and Algonquin with checkpoint detection, multi-lap timing, dynamic AI rival
//! racers with rubber-banding, real-time position ranking (1st-4th), medal
//! thresholds (Bronze/Silver/Gold), cash prize awards, and best time logging
//! in SaveProgress::race_best_time[16] (IDs 0x4C..0x5B).

#![allow(dead_code)]

use crate::stats::{SaveProgress, Stats};

/// Checkpoint capture radius in world coordinate units.
pub const CHECKPOINT_RADIUS: f32 = 14.0;
/// Abandoned vehicle timeout in seconds before race is forfeited.
pub const OUT_OF_VEHICLE_TIMEOUT: f32 = 10.0;

/// Static definition of an official Liberty City street race circuit.
#[derive(Debug, Clone)]
pub struct RaceCircuit {
    pub id: u8,
    pub name: &'static str,
    pub borough: &'static str,
    pub district: &'static str,
    pub laps: u8,
    pub entry_fee: u32,
    pub reward_cash: u32,
    pub target_gold_s: u32,
    pub target_silver_s: u32,
    pub target_bronze_s: u32,
    pub checkpoints: &'static [[f32; 3]],
    pub rival_models: &'static [&'static str],
}

impl RaceCircuit {
    /// Total circuit length for one lap in world units.
    pub fn lap_length(&self) -> f32 {
        if self.checkpoints.len() < 2 {
            return 100.0;
        }
        let mut total = 0.0;
        for i in 0..self.checkpoints.len() {
            let next = (i + 1) % self.checkpoints.len();
            let p1 = self.checkpoints[i];
            let p2 = self.checkpoints[next];
            let dx = p2[0] - p1[0];
            let dy = p2[1] - p1[1];
            total += (dx * dx + dy * dy).sqrt();
        }
        total
    }
}

/// Simulated AI rival racer on the circuit.
#[derive(Debug, Clone)]
pub struct RivalRacer {
    pub name: &'static str,
    pub model: &'static str,
    pub lap: u8,
    pub checkpoint_idx: usize,
    pub pos: [f32; 3],
    pub speed: f32,
    pub total_progress: f32, // continuous progress along circuit
}

impl RivalRacer {
    pub fn new(name: &'static str, model: &'static str, start_pos: [f32; 3], base_speed: f32) -> Self {
        Self {
            name,
            model,
            lap: 1,
            checkpoint_idx: 0,
            pos: start_pos,
            speed: base_speed,
            total_progress: 0.0,
        }
    }

    /// Advance AI racer along circuit checkpoints.
    pub fn advance(&mut self, dt: f32, circuit: &RaceCircuit, player_progress: f32) {
        if self.lap > circuit.laps {
            return;
        }

        // Authentic Chinatown Wars rubber-banding:
        // Rivals adjust pace dynamically to keep races exciting and close
        let diff = player_progress - self.total_progress;
        let speed_mult = if diff > 50.0 {
            1.15 // Player leading: rivals accelerate
        } else if diff < -50.0 {
            0.88 // Player far behind: rivals ease off
        } else {
            1.00
        };

        let current_speed = self.speed * speed_mult;
        let step = current_speed * dt;
        self.total_progress += step;

        let target_cp = circuit.checkpoints[self.checkpoint_idx];
        let dx = target_cp[0] - self.pos[0];
        let dy = target_cp[1] - self.pos[1];
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= step.max(5.0) {
            // Reached checkpoint
            self.pos = target_cp;
            self.checkpoint_idx += 1;
            if self.checkpoint_idx >= circuit.checkpoints.len() {
                self.checkpoint_idx = 0;
                self.lap += 1;
            }
        } else {
            let u = [dx / dist, dy / dist];
            self.pos[0] += u[0] * step;
            self.pos[1] += u[1] * step;
            self.pos[2] = target_cp[2];
        }
    }
}

/// Current state of a street race session.
#[derive(Debug, Clone, PartialEq)]
pub enum RaceState {
    /// 3-second countdown before green light (3.. 2.. 1.. GO!)
    Countdown(f32),
    /// Active racing
    Racing,
    /// Finished all laps with final rank and total elapsed time
    Finished { rank: u8, time_ms: u32, reward: u32 },
    /// Failed or forfeited race
    Failed { reason: &'static str },
}

/// Active race session tracking player, rivals, checkpoints, and timing.
#[derive(Debug, Clone)]
pub struct RaceSession {
    pub circuit: RaceCircuit,
    pub state: RaceState,
    pub lap: u8,
    pub checkpoint_idx: usize,
    pub elapsed_ms: u32,
    pub lap_times_ms: Vec<u32>,
    pub rivals: Vec<RivalRacer>,
    pub player_rank: u8,
    pub out_of_car_timer: f32,
    pub banner_message: Option<(String, f32)>,
    pub last_lap_start_ms: u32,
}

impl RaceSession {
    pub fn new(circuit: RaceCircuit) -> Self {
        let start_p = circuit.checkpoints[0];
        // Generate 3 authentic rival street racers
        let rivals = vec![
            RivalRacer::new(
                "Tommy 'Nitro'",
                circuit.rival_models.first().copied().unwrap_or("STREETRACER1"),
                [start_p[0] - 3.0, start_p[1] - 4.0, start_p[2]],
                28.0,
            ),
            RivalRacer::new(
                "Viper Chen",
                circuit.rival_models.get(1).copied().unwrap_or("STREETRACER2"),
                [start_p[0] + 3.0, start_p[1] - 5.0, start_p[2]],
                27.2,
            ),
            RivalRacer::new(
                "Speedy Gomez",
                circuit.rival_models.get(2).copied().unwrap_or("STREETRACER3"),
                [start_p[0], start_p[1] - 8.0, start_p[2]],
                26.5,
            ),
        ];

        Self {
            circuit,
            state: RaceState::Countdown(3.0),
            lap: 1,
            checkpoint_idx: 0,
            elapsed_ms: 0,
            lap_times_ms: Vec::new(),
            rivals,
            player_rank: 1,
            out_of_car_timer: 0.0,
            banner_message: Some(("DÉPART DANS 3...".into(), 1.0)),
            last_lap_start_ms: 0,
        }
    }

    /// Primary update cycle per game tick.
    pub fn update(
        &mut self,
        dt: f32,
        player_pos: [f32; 3],
        player_in_vehicle: bool,
        save: &mut SaveProgress,
        cash: &mut u32,
    ) {
        match &mut self.state {
            RaceState::Countdown(time_left) => {
                *time_left -= dt;
                if *time_left <= 0.0 {
                    self.state = RaceState::Racing;
                    self.banner_message = Some(("PARTEZ ! GO !".into(), 1.5));
                } else if *time_left <= 1.0 {
                    self.banner_message = Some(("DÉPART DANS 1...".into(), 1.0));
                } else if *time_left <= 2.0 {
                    self.banner_message = Some(("DÉPART DANS 2...".into(), 1.0));
                }
            }
            RaceState::Racing => {
                // Abandoned vehicle check
                if !player_in_vehicle {
                    self.out_of_car_timer += dt;
                    if self.out_of_car_timer >= OUT_OF_VEHICLE_TIMEOUT {
                        self.state = RaceState::Failed {
                            reason: "COURSE FORFAITE ! Véhicule abandonné trop longtemps.",
                        };
                        self.banner_message = Some((
                            "COURSE FORFAITE : Véhicule abandonné !".into(),
                            3.5,
                        ));
                        return;
                    } else {
                        let remain = (OUT_OF_VEHICLE_TIMEOUT - self.out_of_car_timer).ceil() as u32;
                        self.banner_message = Some((
                            format!("RETOURNEZ AU VÉHICULE ! ({}s)", remain),
                            0.2,
                        ));
                    }
                } else {
                    self.out_of_car_timer = 0.0;
                }

                self.elapsed_ms = self.elapsed_ms.saturating_add((dt * 1000.0) as u32);

                // Check distance to current checkpoint
                let target_cp = self.circuit.checkpoints[self.checkpoint_idx];
                let dx = target_cp[0] - player_pos[0];
                let dy = target_cp[1] - player_pos[1];
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= CHECKPOINT_RADIUS {
                    self.checkpoint_idx += 1;
                    if self.checkpoint_idx >= self.circuit.checkpoints.len() {
                        // Lap completed
                        let lap_time = self.elapsed_ms.saturating_sub(self.last_lap_start_ms);
                        self.lap_times_ms.push(lap_time);
                        self.last_lap_start_ms = self.elapsed_ms;
                        self.checkpoint_idx = 0;
                        self.lap += 1;

                        if self.lap > self.circuit.laps {
                            // Race completed!
                            self.finish_race(save, cash);
                            return;
                        } else {
                            self.banner_message = Some((
                                format!("TOUR {}/{} !", self.lap, self.circuit.laps),
                                2.0,
                            ));
                        }
                    }
                }

                // Compute player's total progress along circuit
                let lap_len = self.circuit.lap_length();
                let player_total_progress = (self.lap.saturating_sub(1) as f32) * lap_len
                    + (self.checkpoint_idx as f32 * (lap_len / self.circuit.checkpoints.len() as f32))
                    + (CHECKPOINT_RADIUS - dist.min(CHECKPOINT_RADIUS));

                // Update rival AI racers
                let mut rivals_ahead = 0;
                for rival in &mut self.rivals {
                    rival.advance(dt, &self.circuit, player_total_progress);
                    if rival.total_progress > player_total_progress {
                        rivals_ahead += 1;
                    }
                }
                self.player_rank = (rivals_ahead + 1) as u8;
            }
            RaceState::Finished { .. } | RaceState::Failed { .. } => {}
        }
    }

    /// Evaluates final ranking, awards prizes, updates stats and commits record.
    fn finish_race(&mut self, save: &mut SaveProgress, cash: &mut u32) {
        let rank = self.player_rank;
        let time_ms = self.elapsed_ms;
        let time_seconds = (time_ms / 1000).max(1);

        let reward = if rank == 1 {
            self.circuit.reward_cash
        } else if rank == 2 {
            self.circuit.reward_cash / 3
        } else if rank == 3 {
            self.circuit.reward_cash / 6
        } else {
            0
        };

        *cash = cash.saturating_add(reward);

        // Commit best time to SaveProgress (0x4C + circuit_id)
        if rank == 1 {
            Stats::log_race_time(save, 0x4C + self.circuit.id as usize, time_seconds);
        }

        let medal = if time_seconds <= self.circuit.target_gold_s {
            "MÉDAILLE D'OR"
        } else if time_seconds <= self.circuit.target_silver_s {
            "MÉDAILLE D'ARGENT"
        } else if time_seconds <= self.circuit.target_bronze_s {
            "MÉDAILLE DE BRONZE"
        } else {
            "QUALIFIÉ"
        };

        let msg = if rank == 1 {
            format!(
                "VICTOIRE ! 1ÈRE PLACE — {} — Temps : {:.2}s (+${})",
                medal,
                time_ms as f32 / 1000.0,
                reward
            )
        } else {
            format!(
                "COURSE TERMINÉE — {}e PLACE — Temps : {:.2}s (+${})",
                rank,
                time_ms as f32 / 1000.0,
                reward
            )
        };

        self.banner_message = Some((msg, 4.0));
        self.state = RaceState::Finished { rank, time_ms, reward };
    }

    /// Current target checkpoint world position.
    pub fn current_checkpoint(&self) -> Option<[f32; 3]> {
        if self.state == RaceState::Racing || matches!(self.state, RaceState::Countdown(_)) {
            self.circuit.checkpoints.get(self.checkpoint_idx).copied()
        } else {
            None
        }
    }

    /// Next following checkpoint for radar projection.
    pub fn next_checkpoint(&self) -> Option<[f32; 3]> {
        if self.state == RaceState::Racing {
            let next_idx = (self.checkpoint_idx + 1) % self.circuit.checkpoints.len();
            self.circuit.checkpoints.get(next_idx).copied()
        } else {
            None
        }
    }

    /// Radar waypoint blip: returns position and RGB color.
    pub fn current_radar_blip(&self) -> Option<([f32; 3], [u8; 3])> {
        self.current_checkpoint().map(|pos| (pos, [255, 215, 0])) // Imperial Chinatown gold
    }

    /// Format rank display string ("1st / 4", "2nd / 4", etc.).
    pub fn rank_display(&self) -> String {
        let suffix = match self.player_rank {
            1 => "1er",
            2 => "2e",
            3 => "3e",
            _ => "4e",
        };
        format!("{}/4", suffix)
    }

    /// Format elapsed timer as MM:SS.ms.
    pub fn timer_display(&self) -> String {
        let total_sec = self.elapsed_ms / 1000;
        let mins = total_sec / 60;
        let secs = total_sec % 60;
        let ms = (self.elapsed_ms % 1000) / 10;
        format!("{:02}:{:02}.{:02}", mins, secs, ms)
    }

    /// Format lap counter ("TOUR 1/2").
    pub fn lap_display(&self) -> String {
        format!("TOUR {}/{}", self.lap.min(self.circuit.laps), self.circuit.laps)
    }
}

/// Official Catalog of all 16 Liberty City Street Races and Time Trials.
pub struct RaceManager {
    pub circuits: Vec<RaceCircuit>,
}

impl Default for RaceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RaceManager {
    pub fn new() -> Self {
        let circuits = vec![
            // 0: Broker - Transporterror (BOABO Industrial docks)
            RaceCircuit {
                id: 0,
                name: "Transporterror",
                borough: "Broker",
                district: "BOABO",
                laps: 2,
                entry_fee: 100,
                reward_cash: 1000,
                target_gold_s: 48,
                target_silver_s: 56,
                target_bronze_s: 65,
                checkpoints: &[
                    [360.0, 180.0, 5.0],
                    [390.0, 240.0, 5.0],
                    [440.0, 260.0, 5.0],
                    [470.0, 200.0, 5.0],
                    [430.0, 150.0, 5.0],
                    [370.0, 140.0, 5.0],
                ],
                rival_models: &["STREETRACER1", "STREETRACER2", "STREETRACER3"],
            },
            // 1: Dukes - Midtown Mayhem (Meadow Hills park circuit)
            RaceCircuit {
                id: 1,
                name: "Midtown Mayhem",
                borough: "Dukes",
                district: "Meadow Hills",
                laps: 2,
                entry_fee: 150,
                reward_cash: 1200,
                target_gold_s: 52,
                target_silver_s: 60,
                target_bronze_s: 70,
                checkpoints: &[
                    [720.0, 360.0, 5.0],
                    [760.0, 410.0, 5.0],
                    [790.0, 370.0, 5.0],
                    [750.0, 310.0, 5.0],
                    [690.0, 320.0, 5.0],
                ],
                rival_models: &["STREETRACER2", "STREETRACER4", "STREETRACER5"],
            },
            // 2: Broker - South Boulevard Boogie
            RaceCircuit {
                id: 2,
                name: "South Boulevard Boogie",
                borough: "Broker",
                district: "South Slopes",
                laps: 2,
                entry_fee: 200,
                reward_cash: 1400,
                target_gold_s: 50,
                target_silver_s: 58,
                target_bronze_s: 68,
                checkpoints: &[
                    [510.0, -10.0, 6.0],
                    [540.0, 50.0, 6.0],
                    [580.0, 30.0, 6.0],
                    [550.0, -40.0, 6.0],
                    [490.0, -50.0, 6.0],
                ],
                rival_models: &["STREETRACER3", "STREETRACER1", "STREETRACER2"],
            },
            // 3: Broker - Beach Blitz (Firefly Island boardwalk sprint)
            RaceCircuit {
                id: 3,
                name: "Beach Blitz",
                borough: "Broker",
                district: "Firefly Island",
                laps: 1,
                entry_fee: 250,
                reward_cash: 1600,
                target_gold_s: 42,
                target_silver_s: 48,
                target_bronze_s: 58,
                checkpoints: &[
                    [310.0, -90.0, 4.0],
                    [360.0, -120.0, 4.0],
                    [420.0, -140.0, 4.0],
                    [490.0, -110.0, 4.0],
                    [540.0, -70.0, 4.0],
                ],
                rival_models: &["STREETRACER5", "STREETRACER1 MOD", "STREETRACER4"],
            },
            // 4: Bohan - Boulevard Boogie
            RaceCircuit {
                id: 4,
                name: "Boulevard Boogie",
                borough: "Bohan",
                district: "Boulevard",
                laps: 2,
                entry_fee: 200,
                reward_cash: 1500,
                target_gold_s: 55,
                target_silver_s: 64,
                target_bronze_s: 75,
                checkpoints: &[
                    [390.0, 930.0, 5.0],
                    [430.0, 980.0, 5.0],
                    [480.0, 970.0, 5.0],
                    [460.0, 910.0, 5.0],
                    [410.0, 890.0, 5.0],
                ],
                rival_models: &["STREETRACER1", "STREETRACER3", "STREETRACER5"],
            },
            // 5: Bohan - Industrial Drift
            RaceCircuit {
                id: 5,
                name: "Industrial Drift",
                borough: "Bohan",
                district: "Industrial",
                laps: 3,
                entry_fee: 300,
                reward_cash: 1800,
                target_gold_s: 65,
                target_silver_s: 75,
                target_bronze_s: 88,
                checkpoints: &[
                    [560.0, 890.0, 5.0],
                    [600.0, 930.0, 5.0],
                    [630.0, 880.0, 5.0],
                    [590.0, 840.0, 5.0],
                    [540.0, 850.0, 5.0],
                ],
                rival_models: &["STREETRACER2", "STREETRACER1 MOD", "STREETRACER4"],
            },
            // 6: Dukes - Bridge Burner
            RaceCircuit {
                id: 6,
                name: "Bridge Burner",
                borough: "Dukes",
                district: "East Island City",
                laps: 2,
                entry_fee: 350,
                reward_cash: 2000,
                target_gold_s: 58,
                target_silver_s: 68,
                target_bronze_s: 80,
                checkpoints: &[
                    [530.0, 420.0, 5.0],
                    [580.0, 490.0, 5.0],
                    [620.0, 470.0, 5.0],
                    [570.0, 390.0, 5.0],
                    [510.0, 380.0, 5.0],
                ],
                rival_models: &["STREETRACER4", "STREETRACER5", "STREETRACER1"],
            },
            // 7: Bohan - Alleyway Dash
            RaceCircuit {
                id: 7,
                name: "Alleyway Dash",
                borough: "Bohan",
                district: "Chase Point",
                laps: 2,
                entry_fee: 300,
                reward_cash: 1900,
                target_gold_s: 49,
                target_silver_s: 57,
                target_bronze_s: 68,
                checkpoints: &[
                    [440.0, 810.0, 4.0],
                    [480.0, 840.0, 4.0],
                    [500.0, 790.0, 4.0],
                    [460.0, 760.0, 4.0],
                    [420.0, 780.0, 4.0],
                ],
                rival_models: &["STREETRACER3", "STREETRACER2", "STREETRACER1 MOD"],
            },
            // 8: Algonquin - Star Junction GP
            RaceCircuit {
                id: 8,
                name: "Star Junction GP",
                borough: "Algonquin",
                district: "Star Junction",
                laps: 2,
                entry_fee: 400,
                reward_cash: 2200,
                target_gold_s: 60,
                target_silver_s: 70,
                target_bronze_s: 82,
                checkpoints: &[
                    [190.0, 420.0, 7.0],
                    [220.0, 480.0, 7.0],
                    [250.0, 450.0, 7.0],
                    [210.0, 380.0, 7.0],
                    [170.0, 390.0, 7.0],
                ],
                rival_models: &["STREETRACER1 MOD", "STREETRACER5", "STREETRACER2"],
            },
            // 9: Algonquin - Algonquin Expressway
            RaceCircuit {
                id: 9,
                name: "Algonquin Expressway",
                borough: "Algonquin",
                district: "Middle Park East",
                laps: 2,
                entry_fee: 450,
                reward_cash: 2400,
                target_gold_s: 68,
                target_silver_s: 78,
                target_bronze_s: 92,
                checkpoints: &[
                    [260.0, 520.0, 8.0],
                    [290.0, 600.0, 8.0],
                    [310.0, 570.0, 8.0],
                    [270.0, 480.0, 8.0],
                    [230.0, 490.0, 8.0],
                ],
                rival_models: &["STREETRACER5", "STREETRACER4", "STREETRACER1 MOD"],
            },
            // 10: Algonquin - Castle Gardens Sprint
            RaceCircuit {
                id: 10,
                name: "Castle Gardens Sprint",
                borough: "Algonquin",
                district: "Castle Gardens",
                laps: 1,
                entry_fee: 500,
                reward_cash: 2500,
                target_gold_s: 44,
                target_silver_s: 52,
                target_bronze_s: 62,
                checkpoints: &[
                    [280.0, 110.0, 5.0],
                    [250.0, 60.0, 5.0],
                    [200.0, 50.0, 5.0],
                    [160.0, 90.0, 5.0],
                    [220.0, 130.0, 5.0],
                ],
                rival_models: &["STREETRACER1 MOD", "STREETRACER2", "STREETRACER3"],
            },
            // 11: Algonquin - Meat Quarter Dash
            RaceCircuit {
                id: 11,
                name: "Meat Quarter Dash",
                borough: "Algonquin",
                district: "Meat Quarter",
                laps: 2,
                entry_fee: 400,
                reward_cash: 2300,
                target_gold_s: 53,
                target_silver_s: 62,
                target_bronze_s: 74,
                checkpoints: &[
                    [140.0, 340.0, 5.0],
                    [160.0, 390.0, 5.0],
                    [190.0, 370.0, 5.0],
                    [170.0, 310.0, 5.0],
                    [130.0, 320.0, 5.0],
                ],
                rival_models: &["STREETRACER4", "STREETRACER3", "STREETRACER5"],
            },
            // 12: Algonquin - Suffolk Circuit
            RaceCircuit {
                id: 12,
                name: "Suffolk Circuit",
                borough: "Algonquin",
                district: "Suffolk",
                laps: 2,
                entry_fee: 450,
                reward_cash: 2500,
                target_gold_s: 56,
                target_silver_s: 65,
                target_bronze_s: 78,
                checkpoints: &[
                    [170.0, 190.0, 6.0],
                    [200.0, 240.0, 6.0],
                    [230.0, 220.0, 6.0],
                    [190.0, 160.0, 6.0],
                    [150.0, 170.0, 6.0],
                ],
                rival_models: &["STREETRACER1", "STREETRACER1 MOD", "STREETRACER5"],
            },
            // 13: Algonquin - Heliport Rush
            RaceCircuit {
                id: 13,
                name: "Heliport Rush",
                borough: "Algonquin",
                district: "Lower Easton",
                laps: 2,
                entry_fee: 500,
                reward_cash: 2600,
                target_gold_s: 58,
                target_silver_s: 67,
                target_bronze_s: 80,
                checkpoints: &[
                    [220.0, 250.0, 5.0],
                    [260.0, 300.0, 5.0],
                    [290.0, 280.0, 5.0],
                    [250.0, 220.0, 5.0],
                    [200.0, 230.0, 5.0],
                ],
                rival_models: &["STREETRACER2", "STREETRACER4", "STREETRACER1 MOD"],
            },
            // 14: Algonquin - Times Square Shakedown
            RaceCircuit {
                id: 14,
                name: "Times Square Shakedown",
                borough: "Algonquin",
                district: "Triangle",
                laps: 3,
                entry_fee: 550,
                reward_cash: 2800,
                target_gold_s: 72,
                target_silver_s: 84,
                target_bronze_s: 98,
                checkpoints: &[
                    [180.0, 450.0, 7.0],
                    [220.0, 510.0, 7.0],
                    [240.0, 480.0, 7.0],
                    [200.0, 410.0, 7.0],
                    [160.0, 430.0, 7.0],
                ],
                rival_models: &["STREETRACER5", "STREETRACER1 MOD", "STREETRACER3"],
            },
            // 15: Cross-City - Liberty City Grand Prix (Ultimate Endurance)
            RaceCircuit {
                id: 15,
                name: "Liberty City Grand Prix",
                borough: "Cross-City",
                district: "Algonquin & Broker",
                laps: 1,
                entry_fee: 1000,
                reward_cash: 5000,
                target_gold_s: 110,
                target_silver_s: 130,
                target_bronze_s: 155,
                checkpoints: &[
                    [280.0, 110.0, 5.0],
                    [220.0, 250.0, 5.0],
                    [190.0, 420.0, 7.0],
                    [260.0, 520.0, 8.0],
                    [390.0, 930.0, 5.0],
                    [560.0, 890.0, 5.0],
                    [530.0, 420.0, 5.0],
                    [360.0, 180.0, 5.0],
                ],
                rival_models: &["STREETRACER1 MOD", "STREETRACER5", "STREETRACER4"],
            },
        ];

        Self { circuits }
    }

    /// Retrieve circuit by ID (0..16).
    pub fn get_circuit(&self, id: u8) -> Option<&RaceCircuit> {
        self.circuits.iter().find(|c| c.id == id)
    }

    /// Start a new street race session. Deducts entry fee if player has enough cash.
    pub fn start_race(
        &self,
        circuit_id: u8,
        player_cash: &mut u32,
    ) -> Result<RaceSession, &'static str> {
        let circuit = self.get_circuit(circuit_id).ok_or("Circuit not found")?;
        if *player_cash < circuit.entry_fee {
            return Err("Fonds insuffisants pour régler les frais d'inscription à la course");
        }
        *player_cash -= circuit.entry_fee;
        Ok(RaceSession::new(circuit.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circuit_catalog_initializes_all_16_races() {
        let mgr = RaceManager::new();
        assert_eq!(mgr.circuits.len(), 16);
        for id in 0..16 {
            let c = mgr.get_circuit(id).expect("circuit missing");
            assert_eq!(c.id, id);
            assert!(c.checkpoints.len() >= 4);
            assert!(c.reward_cash >= c.entry_fee);
            assert!(c.target_gold_s < c.target_silver_s);
            assert!(c.target_silver_s < c.target_bronze_s);
        }
    }

    #[test]
    fn race_start_validates_cash_and_entry_fee() {
        let mgr = RaceManager::new();
        let mut poor_cash = 50;
        let res = mgr.start_race(0, &mut poor_cash);
        assert!(res.is_err(), "Should reject poor player");

        let mut rich_cash = 500;
        let res = mgr.start_race(0, &mut rich_cash);
        assert!(res.is_ok(), "Should accept player with enough cash");
        assert_eq!(rich_cash, 400, "100 entry fee deducted");
    }

    #[test]
    fn race_countdown_progresses_to_active_racing() {
        let mgr = RaceManager::new();
        let mut cash = 1000;
        let mut session = mgr.start_race(0, &mut cash).unwrap();

        assert_eq!(session.state, RaceState::Countdown(3.0));

        let mut save = SaveProgress::default();
        // Tick 2 seconds
        session.update(2.0, [360.0, 180.0, 5.0], true, &mut save, &mut cash);
        assert!(matches!(session.state, RaceState::Countdown(t) if t <= 1.0));

        // Tick 1.5 seconds more -> should transition to Racing
        session.update(1.5, [360.0, 180.0, 5.0], true, &mut save, &mut cash);
        assert_eq!(session.state, RaceState::Racing);
    }

    #[test]
    fn race_checkpoint_progression_and_lap_completion() {
        let mgr = RaceManager::new();
        let mut cash = 1000;
        let mut session = mgr.start_race(0, &mut cash).unwrap();
        let mut save = SaveProgress::default();

        // Pass countdown
        session.update(3.5, [360.0, 180.0, 5.0], true, &mut save, &mut cash);
        assert_eq!(session.state, RaceState::Racing);
        assert_eq!(session.lap, 1);
        assert_eq!(session.checkpoint_idx, 0);

        // Drive through each checkpoint of lap 1
        for &cp in session.circuit.checkpoints {
            session.update(0.1, cp, true, &mut save, &mut cash);
        }

        // Lap 1 completed -> lap is now 2
        assert_eq!(session.lap, 2);
        assert_eq!(session.lap_times_ms.len(), 1);
        assert_eq!(session.checkpoint_idx, 0);

        // Drive through each checkpoint of lap 2 (final lap)
        for &cp in session.circuit.checkpoints {
            session.update(0.1, cp, true, &mut save, &mut cash);
        }

        // Race should be finished!
        assert!(matches!(session.state, RaceState::Finished { .. }));
        assert_eq!(session.lap_times_ms.len(), 2);
    }

    #[test]
    fn victory_awards_reward_and_commits_best_time_to_save_progress() {
        let mgr = RaceManager::new();
        let mut cash = 1000;
        let mut session = mgr.start_race(0, &mut cash).unwrap();
        let mut save = SaveProgress::default();

        // Fast-forward countdown
        session.state = RaceState::Racing;
        session.player_rank = 1; // 1st place

        // Complete both laps
        for _ in 0..2 {
            for &cp in session.circuit.checkpoints {
                session.update(0.5, cp, true, &mut save, &mut cash);
            }
        }

        match session.state {
            RaceState::Finished { rank, reward, .. } => {
                assert_eq!(rank, 1);
                assert_eq!(reward, 1000);
            }
            _ => panic!("Expected race to be finished"),
        }

        // Verify save_progress has record saved in race_best_time[0]
        let best_time = Stats::best_race_time(&save, 0x4C);
        assert!(best_time > 0, "Best time must be committed to save_progress");
        assert_eq!(cash, 1900, "1000 starting - 100 entry + 1000 prize = 1900");
    }

    #[test]
    fn abandoned_vehicle_fails_race_after_timeout() {
        let mgr = RaceManager::new();
        let mut cash = 1000;
        let mut session = mgr.start_race(0, &mut cash).unwrap();
        let mut save = SaveProgress::default();

        session.state = RaceState::Racing;

        // Player leaves car for 5 seconds (warning)
        session.update(5.0, [360.0, 180.0, 5.0], false, &mut save, &mut cash);
        assert_eq!(session.state, RaceState::Racing);
        assert!(session.banner_message.as_ref().unwrap().0.contains("RETOURNEZ AU VÉHICULE"));

        // Player stays out for 6 more seconds (total 11s > 10s timeout)
        session.update(6.0, [360.0, 180.0, 5.0], false, &mut save, &mut cash);
        assert!(matches!(session.state, RaceState::Failed { .. }));
        assert!(session.banner_message.as_ref().unwrap().0.contains("FORFAITE"));
    }
}
