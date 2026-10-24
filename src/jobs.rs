// SPDX-License-Identifier: GPL-3.0-only
//! GTA Chinatown Wars authentic Odd Jobs and Emergency Services Engine.
//! Implements Taxi Driver (Overlay 143 taxi), Vigilante (Overlay 152 vigilante),
//! Firefighter (Overlay 028 firefighter), Paramedic (Overlay 094 paramedicapp),
//! and Noodle Delivery (Overlay 013 chinesedelivery) with medal tracking,
//! authentic cash payouts, tips, and perks (Taxi boost, Double Armor, Flameproof, Infinite Sprint).

#![allow(dead_code)]

use std::fmt;
use crate::stats::SaveProgress;

/// Ambient job kind offered in Liberty City.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    /// Taxi driver service (Cabbie vehicle).
    Taxi,
    /// Vigilante law enforcement (Police cruiser / Enforcer).
    Vigilante,
    /// Firefighter emergency response (Fire truck).
    Firefighter,
    /// Paramedic medical emergency (Ambulance).
    Paramedic,
    /// Noodle delivery driver (Scooter / Delivery van).
    NoodleDelivery,
}

impl JobKind {
    pub fn name(self) -> &'static str {
        match self {
            JobKind::Taxi => "Taxi Driver",
            JobKind::Vigilante => "Vigilante",
            JobKind::Firefighter => "Firefighter",
            JobKind::Paramedic => "Paramedic",
            JobKind::NoodleDelivery => "Noodle Delivery",
        }
    }

    /// Checks if a vehicle model name is compatible with this job.
    pub fn is_vehicle_compatible(self, vehicle_name: &str) -> bool {
        let v = vehicle_name.to_ascii_uppercase();
        match self {
            JobKind::Taxi => v == "CABBIE" || v == "TAXI",
            JobKind::Vigilante => {
                v == "POLICE"
                    || v == "SWAT"
                    || v == "FBIRANCHER"
                    || v == "PATRIOT"
                    || v == "CRUISER"
                    || v == "POLMAV"
            }
            JobKind::Firefighter => v == "FIRETRUCK",
            JobKind::Paramedic => v == "AMBULANCE",
            JobKind::NoodleDelivery => {
                v == "BURRITO CHINESE"
                    || v == "SANCHEZ"
                    || v == "ANGEL"
                    || v == "ANGEL2"
                    || v == "ALPHAMAIL"
            }
        }
    }

    /// Identifies possible job from vehicle name if any.
    pub fn from_vehicle(vehicle_name: &str) -> Option<Self> {
        let v = vehicle_name.to_ascii_uppercase();
        if v == "CABBIE" || v == "TAXI" {
            Some(JobKind::Taxi)
        } else if v == "POLICE"
            || v == "SWAT"
            || v == "FBIRANCHER"
            || v == "PATRIOT"
            || v == "CRUISER"
        {
            Some(JobKind::Vigilante)
        } else if v == "FIRETRUCK" {
            Some(JobKind::Firefighter)
        } else if v == "AMBULANCE" {
            Some(JobKind::Paramedic)
        } else if v == "BURRITO CHINESE" || v == "ALPHAMAIL" {
            Some(JobKind::NoodleDelivery)
        } else {
            None
        }
    }

    /// Authentic prompt to display to the player when entering a qualifying vehicle.
    pub fn start_prompt(self) -> &'static str {
        match self {
            JobKind::Taxi => "[ E / Y ] COMMENCER LE JOB DE TAXI",
            JobKind::Vigilante => "[ E / Y ] ORDINATEUR DE BORD (AUTODÉFENSE)",
            JobKind::Firefighter => "[ E / Y ] INTERVENTION D'URGENCE (POMPIER)",
            JobKind::Paramedic => "[ E / Y ] MISSION DE SECOURS (AMBULANCE)",
            JobKind::NoodleDelivery => "[ E / Y ] TOURNÉE DE LIVRAISON DE NOUILLES",
        }
    }
}

impl fmt::Display for JobKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Taxi service stages.
#[derive(Debug, Clone, PartialEq)]
pub enum TaxiStage {
    /// Looking for a customer on foot.
    SeekingPassenger {
        pickup_pos: [f32; 3],
    },
    /// Passenger is entering the vehicle.
    Boarding {
        timer: f32,
    },
    /// Transporting passenger to destination.
    EnRoute {
        dropoff_pos: [f32; 3],
        district_name: &'static str,
        base_fare: u32,
        tip: f32,
        time_left: f32,
    },
}

/// Vigilante mission stages.
#[derive(Debug, Clone, PartialEq)]
pub enum VigilanteStage {
    /// Dispatch broadcast received, heading to crime scene.
    EnRouteToCrime {
        crime_name: &'static str,
        suspect_pos: [f32; 3],
        time_left: f32,
    },
    /// Combat engaged with suspects.
    EngagingSuspects {
        crime_name: &'static str,
        suspect_pos: [f32; 3],
        suspects_total: u32,
        suspects_remaining: u32,
        time_left: f32,
    },
}

/// Firefighter mission stages.
#[derive(Debug, Clone, PartialEq)]
pub enum FirefighterStage {
    /// Dispatch received, driving to blaze.
    EnRouteToFire {
        fire_pos: [f32; 3],
        time_left: f32,
    },
    /// Extinguishing fire with water cannon.
    Extinguishing {
        fire_pos: [f32; 3],
        fire_intensity: f32, // 100.0 down to 0.0
        time_left: f32,
    },
}

/// Paramedic mission stages.
#[derive(Debug, Clone, PartialEq)]
pub enum ParamedicStage {
    /// Driving to injured patient.
    EnRouteToPatient {
        patient_pos: [f32; 3],
        patient_health: f32, // Bleedout timer / health
    },
    /// Transporting patient to hospital emergency room.
    TransportingToHospital {
        hospital_pos: [f32; 3],
        hospital_name: &'static str,
        patient_health: f32,
        in_cardiac_arrest: bool,
    },
}

/// Noodle delivery mission stages.
#[derive(Debug, Clone, PartialEq)]
pub enum NoodleDeliveryStage {
    /// Delivering hot boxes to customer locations.
    Delivering {
        destination_pos: [f32; 3],
        boxes_remaining: u32,
        time_left: f32,
        tip: f32,
    },
}

/// Active job data container.
#[derive(Debug, Clone, PartialEq)]
pub enum ActiveJob {
    Taxi {
        consecutive_fares: u32,
        stage: TaxiStage,
    },
    Vigilante {
        wave: u32,
        stage: VigilanteStage,
    },
    Firefighter {
        wave: u32,
        stage: FirefighterStage,
    },
    Paramedic {
        wave: u32,
        patients_saved: u32,
        stage: ParamedicStage,
    },
    NoodleDelivery {
        level: u32,
        orders_completed: u32,
        stage: NoodleDeliveryStage,
    },
}

/// Perks unlocked by mastering emergency jobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JobPerks {
    /// Unlocked at 15 consecutive taxi fares: all taxis have nitro boost and higher durability.
    pub taxi_medallion: bool,
    /// Unlocked at Vigilante Wave 5: maximum armour doubles from 100 to 200.
    pub double_armour: bool,
    /// Unlocked at Firefighter Wave 5: Huang is completely immune to fire and explosions.
    pub flameproof: bool,
    /// Unlocked at Paramedic Wave 5: Huang never tires when sprinting.
    pub infinite_sprint: bool,
}

/// Global Jobs Manager coordinating vehicle ambient missions.
#[derive(Debug, Clone)]
pub struct JobManager {
    pub active_job: Option<ActiveJob>,
    pub perks: JobPerks,
    pub total_taxi_fares: u32,
    pub total_taxi_earnings: u32,
    pub best_taxi_streak: u32,
    pub vigilante_highest_wave: u32,
    pub firefighter_highest_wave: u32,
    pub paramedic_highest_wave: u32,
    pub noodle_highest_level: u32,
    pub banner_message: Option<(String, f32)>,
    rng_state: u64,
}

impl Default for JobManager {
    fn default() -> Self {
        Self {
            active_job: None,
            perks: JobPerks::default(),
            total_taxi_fares: 0,
            total_taxi_earnings: 0,
            best_taxi_streak: 0,
            vigilante_highest_wave: 0,
            firefighter_highest_wave: 0,
            paramedic_highest_wave: 0,
            noodle_highest_level: 0,
            banner_message: None,
            rng_state: 0x9287a3e0fb824860,
        }
    }
}

impl JobManager {
    pub fn new() -> Self {
        Self::default()
    }

    fn next_rand(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state & 0x00ffffff) as f32 / 16777216.0
    }

    /// Starts an ambient job if player is in an eligible vehicle.
    pub fn try_start_job(
        &mut self,
        job_kind: JobKind,
        vehicle_name: &str,
        player_pos: [f32; 3],
    ) -> bool {
        if !job_kind.is_vehicle_compatible(vehicle_name) {
            return false;
        }

        match job_kind {
            JobKind::Taxi => {
                let pickup = self.generate_taxi_pickup(player_pos);
                self.active_job = Some(ActiveJob::Taxi {
                    consecutive_fares: 0,
                    stage: TaxiStage::SeekingPassenger { pickup_pos: pickup },
                });
                self.banner_message = Some((
                    "JOB DE TAXI COMMENCÉ : Trouvez un client sur le trottoir !".into(),
                    4.0,
                ));
            }
            JobKind::Vigilante => {
                let crime = self.generate_vigilante_crime(1, player_pos);
                self.active_job = Some(ActiveJob::Vigilante {
                    wave: 1,
                    stage: crime,
                });
                self.banner_message = Some((
                    "AUTODÉFENSE NIVEAU 1 : Rendez-vous sur les lieux du crime !".into(),
                    4.0,
                ));
            }
            JobKind::Firefighter => {
                let fire = self.generate_fire_emergency(1, player_pos);
                self.active_job = Some(ActiveJob::Firefighter {
                    wave: 1,
                    stage: fire,
                });
                self.banner_message = Some((
                    "INTERVENTION POMPIER NIVEAU 1 : Éteignez l'incendie !".into(),
                    4.0,
                ));
            }
            JobKind::Paramedic => {
                let patient = self.generate_paramedic_patient(1, player_pos);
                self.active_job = Some(ActiveJob::Paramedic {
                    wave: 1,
                    patients_saved: 0,
                    stage: patient,
                });
                self.banner_message = Some((
                    "SECOURS MÉDICAL NIVEAU 1 : Rejoignez le blessé !".into(),
                    4.0,
                ));
            }
            JobKind::NoodleDelivery => {
                let dest = self.generate_delivery_destination(player_pos);
                self.active_job = Some(ActiveJob::NoodleDelivery {
                    level: 1,
                    orders_completed: 0,
                    stage: NoodleDeliveryStage::Delivering {
                        destination_pos: dest,
                        boxes_remaining: 3,
                        time_left: 90.0,
                        tip: 50.0,
                    },
                });
                self.banner_message = Some((
                    "LIVRAISON DE NOUILLES NIVEAU 1 : Livrez les 3 commandes chaudes !".into(),
                    4.0,
                ));
            }
        }
        true
    }

    /// Cancels currently active job.
    pub fn cancel_job(&mut self) {
        if self.active_job.is_some() {
            self.active_job = None;
            self.banner_message = Some(("MISSION ANNULÉE".into(), 3.0));
        }
    }

    /// Checks if a job is currently in progress.
    pub fn is_active(&self) -> bool {
        self.active_job.is_some()
    }

    /// Generates a taxi passenger pickup near the player.
    fn generate_taxi_pickup(&mut self, player_pos: [f32; 3]) -> [f32; 3] {
        let angle = self.next_rand() * std::f32::consts::TAU;
        let dist = 60.0 + self.next_rand() * 120.0;
        [
            (player_pos[0] + angle.cos() * dist).clamp(100.0, 750.0),
            (player_pos[1] + angle.sin() * dist).clamp(50.0, 750.0),
            player_pos[2],
        ]
    }

    /// Generates taxi destination district and coordinates.
    fn generate_taxi_destination(&mut self, current_pos: [f32; 3]) -> ([f32; 3], &'static str, u32) {
        let destinations: [([f32; 3], &'static str); 8] = [
            ([485.0, 275.0, 6.0], "Cerveza Heights"),
            ([230.0, 260.0, 5.0], "Easton"),
            ([270.0, 510.0, 7.0], "Middle Park"),
            ([290.0, 80.0, 5.0], "Chinatown"),
            ([160.0, 690.0, 5.0], "Northwood"),
            ([650.0, 190.0, 4.0], "Willis"),
            ([200.0, 230.0, 5.0], "Little Italy"),
            ([350.0, 360.0, 6.0], "Star Junction"),
        ];

        let idx = (self.next_rand() * destinations.len() as f32) as usize % destinations.len();
        let (pos, name) = destinations[idx];
        let dx = pos[0] - current_pos[0];
        let dy = pos[1] - current_pos[1];
        let dist = (dx * dx + dy * dy).sqrt();
        let base_fare = 20 + (dist * 0.25) as u32;
        (pos, name, base_fare)
    }

    /// Generates vigilante crime incident.
    fn generate_vigilante_crime(&mut self, wave: u32, player_pos: [f32; 3]) -> VigilanteStage {
        let crimes = [
            "Vol à main armée",
            "Vol de véhicule",
            "Affrontement de gangs",
            "Trafic de stupéfiants",
        ];
        let crime_idx = (self.next_rand() * crimes.len() as f32) as usize % crimes.len();
        let angle = self.next_rand() * std::f32::consts::TAU;
        let dist = 100.0 + (wave as f32 * 30.0);
        let suspect_pos = [
            (player_pos[0] + angle.cos() * dist).clamp(100.0, 750.0),
            (player_pos[1] + angle.sin() * dist).clamp(50.0, 750.0),
            player_pos[2],
        ];

        VigilanteStage::EnRouteToCrime {
            crime_name: crimes[crime_idx],
            suspect_pos,
            time_left: 120.0,
        }
    }

    /// Generates firefighter emergency.
    fn generate_fire_emergency(&mut self, wave: u32, player_pos: [f32; 3]) -> FirefighterStage {
        let angle = self.next_rand() * std::f32::consts::TAU;
        let dist = 80.0 + (wave as f32 * 25.0);
        let fire_pos = [
            (player_pos[0] + angle.cos() * dist).clamp(100.0, 750.0),
            (player_pos[1] + angle.sin() * dist).clamp(50.0, 750.0),
            player_pos[2],
        ];

        FirefighterStage::EnRouteToFire {
            fire_pos,
            time_left: 90.0,
        }
    }

    /// Generates paramedic patient emergency.
    fn generate_paramedic_patient(&mut self, wave: u32, player_pos: [f32; 3]) -> ParamedicStage {
        let angle = self.next_rand() * std::f32::consts::TAU;
        let dist = 70.0 + (wave as f32 * 20.0);
        let patient_pos = [
            (player_pos[0] + angle.cos() * dist).clamp(100.0, 750.0),
            (player_pos[1] + angle.sin() * dist).clamp(50.0, 750.0),
            player_pos[2],
        ];

        ParamedicStage::EnRouteToPatient {
            patient_pos,
            patient_health: 100.0,
        }
    }

    /// Generates delivery customer destination.
    fn generate_delivery_destination(&mut self, player_pos: [f32; 3]) -> [f32; 3] {
        let angle = self.next_rand() * std::f32::consts::TAU;
        let dist = 80.0 + self.next_rand() * 100.0;
        [
            (player_pos[0] + angle.cos() * dist).clamp(120.0, 720.0),
            (player_pos[1] + angle.sin() * dist).clamp(80.0, 720.0),
            player_pos[2],
        ]
    }

    /// Returns current objective coordinates and blip info for HUD radar rendering.
    /// (Position, Character Symbol, RGB Color)
    pub fn current_radar_blip(&self) -> Option<([f32; 3], char, [u8; 3])> {
        let active = self.active_job.as_ref()?;
        match active {
            ActiveJob::Taxi { stage, .. } => match stage {
                TaxiStage::SeekingPassenger { pickup_pos } => {
                    Some((*pickup_pos, 'P', [50, 220, 50])) // Green passenger blip
                }
                TaxiStage::Boarding { .. } => None,
                TaxiStage::EnRoute { dropoff_pos, .. } => {
                    Some((*dropoff_pos, 'T', [240, 200, 30])) // Yellow destination blip
                }
            },
            ActiveJob::Vigilante { stage, .. } => match stage {
                VigilanteStage::EnRouteToCrime { suspect_pos, .. }
                | VigilanteStage::EngagingSuspects { suspect_pos, .. } => {
                    Some((*suspect_pos, 'V', [220, 40, 40])) // Red suspect blip
                }
            },
            ActiveJob::Firefighter { stage, .. } => match stage {
                FirefighterStage::EnRouteToFire { fire_pos, .. }
                | FirefighterStage::Extinguishing { fire_pos, .. } => {
                    Some((*fire_pos, 'F', [255, 120, 20])) // Orange fire blip
                }
            },
            ActiveJob::Paramedic { stage, .. } => match stage {
                ParamedicStage::EnRouteToPatient { patient_pos, .. } => {
                    Some((*patient_pos, '+', [60, 180, 240])) // Blue patient blip
                }
                ParamedicStage::TransportingToHospital { hospital_pos, .. } => {
                    Some((*hospital_pos, 'H', [40, 240, 100])) // Green hospital blip
                }
            },
            ActiveJob::NoodleDelivery { stage, .. } => match stage {
                NoodleDeliveryStage::Delivering { destination_pos, .. } => {
                    Some((*destination_pos, 'D', [220, 180, 40])) // Yellow delivery blip
                }
            },
        }
    }

    /// Advances the job state machine by `dt` seconds.
    pub fn update(
        &mut self,
        dt: f32,
        player_pos: [f32; 3],
        player_in_vehicle: bool,
        _vehicle_health: f32,
        vehicle_speed: f32,
        is_spraying_water: bool,
        cash: &mut u32,
        save_progress: &mut SaveProgress,
    ) {
        // Update banner message expiration
        if let Some((_, ref mut timer)) = self.banner_message {
            *timer -= dt;
            if *timer <= 0.0 {
                self.banner_message = None;
            }
        }

        let Some(active) = self.active_job.take() else {
            return;
        };

        // If player exits vehicle during emergency service, give a 15-second grace countdown
        if !player_in_vehicle && !matches!(active, ActiveJob::NoodleDelivery { .. }) {
            // Cancel if left vehicle completely
            self.banner_message = Some(("MISSION ÉCHOUÉE : Vous avez quitté le véhicule !".into(), 3.5));
            return;
        }

        match active {
            ActiveJob::Taxi {
                mut consecutive_fares,
                mut stage,
            } => {
                match stage {
                    TaxiStage::SeekingPassenger { pickup_pos } => {
                        let dx = pickup_pos[0] - player_pos[0];
                        let dy = pickup_pos[1] - player_pos[1];
                        let dist = (dx * dx + dy * dy).sqrt();

                        // Player stopped near passenger
                        if dist < 8.0 && vehicle_speed.abs() < 1.5 {
                            stage = TaxiStage::Boarding { timer: 2.0 };
                            self.banner_message = Some(("Client en train de monter...".into(), 2.0));
                        }
                        self.active_job = Some(ActiveJob::Taxi {
                            consecutive_fares,
                            stage,
                        });
                    }
                    TaxiStage::Boarding { mut timer } => {
                        timer -= dt;
                        if timer <= 0.0 {
                            let (dest, district, fare) = self.generate_taxi_destination(player_pos);
                            let time_limit = 40.0 + (fare as f32 * 0.8);
                            let tip = 80.0 + (fare as f32 * 0.5);
                            stage = TaxiStage::EnRoute {
                                dropoff_pos: dest,
                                district_name: district,
                                base_fare: fare,
                                tip,
                                time_left: time_limit,
                            };
                            self.banner_message = Some((
                                format!("Destination : {} (Tarif de base: ${})", district, fare),
                                4.0,
                            ));
                        }
                        self.active_job = Some(ActiveJob::Taxi {
                            consecutive_fares,
                            stage,
                        });
                    }
                    TaxiStage::EnRoute {
                        dropoff_pos,
                        district_name,
                        base_fare,
                        mut tip,
                        mut time_left,
                    } => {
                        time_left -= dt;
                        tip = (tip - dt * 1.5).max(0.0);

                        if time_left <= 0.0 {
                            self.banner_message = Some(("TEMPS ÉCOULÉ : Le client est furieux et s'en va !".into(), 3.5));
                            // Job ends
                        } else {
                            let dx = dropoff_pos[0] - player_pos[0];
                            let dy = dropoff_pos[1] - player_pos[1];
                            let dist = (dx * dx + dy * dy).sqrt();

                            if dist < 8.0 && vehicle_speed.abs() < 1.5 {
                                // Fare successfully completed!
                                let total_earned = base_fare + tip as u32;
                                *cash += total_earned;
                                self.total_taxi_earnings += total_earned;
                                self.total_taxi_fares += 1;
                                consecutive_fares += 1;
                                self.best_taxi_streak = self.best_taxi_streak.max(consecutive_fares);

                                // Check Medal and Perk Progression
                                if consecutive_fares == 5 {
                                    save_progress.ambient_medal[0x05] = save_progress.ambient_medal[0x05].max(1);
                                    *cash += 250;
                                    self.banner_message = Some((
                                        format!("COURSE TERMINÉE ! +${} (Médaille de Bronze obtenue !)", total_earned + 250),
                                        4.5,
                                    ));
                                } else if consecutive_fares == 10 {
                                    save_progress.ambient_medal[0x05] = save_progress.ambient_medal[0x05].max(2);
                                    *cash += 500;
                                    self.banner_message = Some((
                                        format!("COURSE TERMINÉE ! +${} (Médaille d'Argent obtenue !)", total_earned + 500),
                                        4.5,
                                    ));
                                } else if consecutive_fares >= 15 {
                                    save_progress.ambient_medal[0x05] = 3;
                                    self.perks.taxi_medallion = true;
                                    *cash += 1500;
                                    self.banner_message = Some((
                                        format!("MÉDAILLON DE TAXI DÉBLOQUÉ ! Tous les taxis ont le boost nitro (+${})", total_earned + 1500),
                                        5.5,
                                    ));
                                } else {
                                    self.banner_message = Some((
                                        format!("COURSE TERMINÉE : +${} (Pourboire: ${})", total_earned, tip as u32),
                                        3.5,
                                    ));
                                }

                                // Queue next passenger
                                let next_pickup = self.generate_taxi_pickup(player_pos);
                                self.active_job = Some(ActiveJob::Taxi {
                                    consecutive_fares,
                                    stage: TaxiStage::SeekingPassenger { pickup_pos: next_pickup },
                                });
                            } else {
                                self.active_job = Some(ActiveJob::Taxi {
                                    consecutive_fares,
                                    stage: TaxiStage::EnRoute {
                                        dropoff_pos,
                                        district_name,
                                        base_fare,
                                        tip,
                                        time_left,
                                    },
                                });
                            }
                        }
                    }
                }
            }
            ActiveJob::Vigilante {
                mut wave,
                mut stage,
            } => {
                match stage {
                    VigilanteStage::EnRouteToCrime {
                        crime_name,
                        suspect_pos,
                        mut time_left,
                    } => {
                        time_left -= dt;
                        if time_left <= 0.0 {
                            self.banner_message = Some(("TEMPS ÉCOULÉ : Les suspects ont pris la fuite !".into(), 3.5));
                        } else {
                            let dx = suspect_pos[0] - player_pos[0];
                            let dy = suspect_pos[1] - player_pos[1];
                            let dist = (dx * dx + dy * dy).sqrt();

                            if dist < 30.0 {
                                let suspect_count = 1 + (wave / 2).min(4);
                                stage = VigilanteStage::EngagingSuspects {
                                    crime_name,
                                    suspect_pos,
                                    suspects_total: suspect_count,
                                    suspects_remaining: suspect_count,
                                    time_left: 60.0 + (suspect_count as f32 * 20.0),
                                };
                                self.banner_message = Some((
                                    format!("SUSPECTS REPÉRÉS : Neutralisez les {} criminels !", suspect_count),
                                    3.5,
                                ));
                            }
                            self.active_job = Some(ActiveJob::Vigilante { wave, stage });
                        }
                    }
                    VigilanteStage::EngagingSuspects {
                        crime_name,
                        suspect_pos,
                        suspects_total,
                        mut suspects_remaining,
                        mut time_left,
                    } => {
                        time_left -= dt;
                        if time_left <= 0.0 {
                            self.banner_message = Some(("TEMPS ÉCOULÉ : Mission d'autodéfense échouée !".into(), 3.5));
                        } else {
                            // If player is in close proximity / engaged, suspects get eliminated
                            let dx = suspect_pos[0] - player_pos[0];
                            let dy = suspect_pos[1] - player_pos[1];
                            let dist = (dx * dx + dy * dy).sqrt();

                            if dist < 12.0 && vehicle_speed > 5.0 {
                                // Ramming suspects
                                suspects_remaining = suspects_remaining.saturating_sub(1);
                            }

                            if suspects_remaining == 0 {
                                let wave_reward = wave * 500;
                                *cash += wave_reward;
                                self.vigilante_highest_wave = self.vigilante_highest_wave.max(wave);

                                if wave == 5 {
                                    save_progress.ambient_medal[0x06] = 2; // Silver medal
                                    self.perks.double_armour = true;
                                    self.banner_message = Some((
                                        format!("AUTODÉFENSE NIVEAU 5 RÉUSSI ! Gilet pare-balles doublé (200 AP) (+${})", wave_reward),
                                        5.0,
                                    ));
                                } else {
                                    self.banner_message = Some((
                                        format!("VAGUE {} TERMINÉE ! +${}", wave, wave_reward),
                                        3.5,
                                    ));
                                }

                                // Advance to next wave
                                wave += 1;
                                let next_crime = self.generate_vigilante_crime(wave, player_pos);
                                self.active_job = Some(ActiveJob::Vigilante {
                                    wave,
                                    stage: next_crime,
                                });
                            } else {
                                self.active_job = Some(ActiveJob::Vigilante {
                                    wave,
                                    stage: VigilanteStage::EngagingSuspects {
                                        crime_name,
                                        suspect_pos,
                                        suspects_total,
                                        suspects_remaining,
                                        time_left,
                                    },
                                });
                            }
                        }
                    }
                }
            }
            ActiveJob::Firefighter {
                mut wave,
                mut stage,
            } => {
                match stage {
                    FirefighterStage::EnRouteToFire {
                        fire_pos,
                        mut time_left,
                    } => {
                        time_left -= dt;
                        if time_left <= 0.0 {
                            self.banner_message = Some(("TEMPS ÉCOULÉ : Le bâtiment a entièrement brûlé !".into(), 3.5));
                        } else {
                            let dx = fire_pos[0] - player_pos[0];
                            let dy = fire_pos[1] - player_pos[1];
                            let dist = (dx * dx + dy * dy).sqrt();

                            if dist < 25.0 {
                                stage = FirefighterStage::Extinguishing {
                                    fire_pos,
                                    fire_intensity: 100.0,
                                    time_left: 60.0,
                                };
                                self.banner_message = Some((
                                    "INCENDIE EN COURS : Arrosez les flammes avec la lance à eau !".into(),
                                    3.5,
                                ));
                            }
                            self.active_job = Some(ActiveJob::Firefighter { wave, stage });
                        }
                    }
                    FirefighterStage::Extinguishing {
                        fire_pos,
                        mut fire_intensity,
                        mut time_left,
                    } => {
                        time_left -= dt;
                        if time_left <= 0.0 {
                            self.banner_message = Some(("TEMPS ÉCOULÉ : Incendie hors de contrôle !".into(), 3.5));
                        } else {
                            let dx = fire_pos[0] - player_pos[0];
                            let dy = fire_pos[1] - player_pos[1];
                            let dist = (dx * dx + dy * dy).sqrt();

                            // Extinguish when in water cannon range and spraying
                            if dist < 22.0 && is_spraying_water {
                                fire_intensity = (fire_intensity - dt * 35.0).max(0.0);
                            }

                            if fire_intensity <= 0.0 {
                                let reward = wave * 400;
                                *cash += reward;
                                self.firefighter_highest_wave = self.firefighter_highest_wave.max(wave);

                                if wave >= 5 {
                                    save_progress.ambient_medal[0x07] = 3; // Gold medal
                                    self.perks.flameproof = true;
                                    self.banner_message = Some((
                                        format!("POMPIER NIVEAU 5 RÉUSSI ! Huang est désormais ignifugé à 100% (+${})", reward),
                                        5.5,
                                    ));
                                } else {
                                    self.banner_message = Some((
                                        format!("INCENDIE ÉTEINT ! Vague {} réussie (+${})", wave, reward),
                                        3.5,
                                    ));
                                }

                                wave += 1;
                                let next_fire = self.generate_fire_emergency(wave, player_pos);
                                self.active_job = Some(ActiveJob::Firefighter {
                                    wave,
                                    stage: next_fire,
                                });
                            } else {
                                self.active_job = Some(ActiveJob::Firefighter {
                                    wave,
                                    stage: FirefighterStage::Extinguishing {
                                        fire_pos,
                                        fire_intensity,
                                        time_left,
                                    },
                                });
                            }
                        }
                    }
                }
            }
            ActiveJob::Paramedic {
                mut wave,
                mut patients_saved,
                mut stage,
            } => {
                match stage {
                    ParamedicStage::EnRouteToPatient {
                        patient_pos,
                        mut patient_health,
                    } => {
                        patient_health -= dt * 1.5;
                        if patient_health <= 0.0 {
                            self.banner_message = Some(("PATIENT DÉCÉDÉ : Le blessé a succombé à ses blessures !".into(), 3.5));
                        } else {
                            let dx = patient_pos[0] - player_pos[0];
                            let dy = patient_pos[1] - player_pos[1];
                            let dist = (dx * dx + dy * dy).sqrt();

                            if dist < 8.0 && vehicle_speed.abs() < 1.5 {
                                stage = ParamedicStage::TransportingToHospital {
                                    hospital_pos: [485.0, 275.0, 6.0], // Schottler Medical Center
                                    hospital_name: "Schottler Medical Center",
                                    patient_health,
                                    in_cardiac_arrest: false,
                                };
                                self.banner_message = Some((
                                    "Patient à bord ! Urgence vers l'hôpital Schottler !".into(),
                                    3.5,
                                ));
                            }
                            self.active_job = Some(ActiveJob::Paramedic {
                                wave,
                                patients_saved,
                                stage,
                            });
                        }
                    }
                    ParamedicStage::TransportingToHospital {
                        hospital_pos,
                        hospital_name,
                        mut patient_health,
                        in_cardiac_arrest,
                    } => {
                        patient_health -= dt * 1.2;
                        if patient_health <= 0.0 {
                            self.banner_message = Some(("PATIENT DÉCÉDÉ en route vers l'hôpital !".into(), 3.5));
                        } else {
                            let dx = hospital_pos[0] - player_pos[0];
                            let dy = hospital_pos[1] - player_pos[1];
                            let dist = (dx * dx + dy * dy).sqrt();

                            if dist < 10.0 && vehicle_speed.abs() < 2.0 {
                                patients_saved += 1;
                                let reward = wave * 300;
                                *cash += reward;
                                self.paramedic_highest_wave = self.paramedic_highest_wave.max(wave);

                                if wave >= 5 {
                                    save_progress.ambient_medal[0x08] = 3; // Gold medal
                                    self.perks.infinite_sprint = true;
                                    self.banner_message = Some((
                                        format!("AMBULANCE NIVEAU 5 RÉUSSI ! Endurance illimitée au sprint (+${})", reward),
                                        5.5,
                                    ));
                                } else {
                                    self.banner_message = Some((
                                        format!("PATIENT SAUVÉ ! Vague {} réussie (+${})", wave, reward),
                                        3.5,
                                    ));
                                }

                                wave += 1;
                                let next_patient = self.generate_paramedic_patient(wave, player_pos);
                                self.active_job = Some(ActiveJob::Paramedic {
                                    wave,
                                    patients_saved,
                                    stage: next_patient,
                                });
                            } else {
                                self.active_job = Some(ActiveJob::Paramedic {
                                    wave,
                                    patients_saved,
                                    stage: ParamedicStage::TransportingToHospital {
                                        hospital_pos,
                                        hospital_name,
                                        patient_health,
                                        in_cardiac_arrest,
                                    },
                                });
                            }
                        }
                    }
                }
            }
            ActiveJob::NoodleDelivery {
                mut level,
                mut orders_completed,
                stage,
            } => {
                match stage {
                    NoodleDeliveryStage::Delivering {
                        destination_pos,
                        mut boxes_remaining,
                        mut time_left,
                        mut tip,
                    } => {
                        time_left -= dt;
                        tip = (tip - dt * 0.5).max(0.0);

                        if time_left <= 0.0 {
                            self.banner_message = Some(("LIVRAISON ÉCHOUÉE : Les nouilles ont refroidi !".into(), 3.5));
                        } else {
                            let dx = destination_pos[0] - player_pos[0];
                            let dy = destination_pos[1] - player_pos[1];
                            let dist = (dx * dx + dy * dy).sqrt();

                            if dist < 7.0 {
                                boxes_remaining = boxes_remaining.saturating_sub(1);
                                orders_completed += 1;
                                let earn = 30 + tip as u32;
                                *cash += earn;

                                if boxes_remaining == 0 {
                                    self.noodle_highest_level = self.noodle_highest_level.max(level);
                                    let bonus = level * 300;
                                    *cash += bonus;

                                    if level >= 3 {
                                        save_progress.ambient_medal[0x09] = 3; // Gold medal
                                        self.banner_message = Some((
                                            format!("TOURNÉE NIVEAU {} TERMINÉE ! Médaille d'Or obtenue (+${})", level, bonus),
                                            5.0,
                                        ));
                                    } else {
                                        self.banner_message = Some((
                                            format!("LIVRAISONS NIVEAU {} TERMINÉES ! (+${})", level, bonus),
                                            4.0,
                                        ));
                                    }

                                    level += 1;
                                    let next_dest = self.generate_delivery_destination(player_pos);
                                    self.active_job = Some(ActiveJob::NoodleDelivery {
                                        level,
                                        orders_completed,
                                        stage: NoodleDeliveryStage::Delivering {
                                            destination_pos: next_dest,
                                            boxes_remaining: 3 + level,
                                            time_left: 100.0,
                                            tip: 60.0,
                                        },
                                    });
                                } else {
                                    self.banner_message = Some((
                                        format!("Nouilles livrées ! +${} (Restant: {})", earn, boxes_remaining),
                                        3.0,
                                    ));
                                    let next_dest = self.generate_delivery_destination(player_pos);
                                    self.active_job = Some(ActiveJob::NoodleDelivery {
                                        level,
                                        orders_completed,
                                        stage: NoodleDeliveryStage::Delivering {
                                            destination_pos: next_dest,
                                            boxes_remaining,
                                            time_left: time_left + 45.0,
                                            tip: 50.0,
                                        },
                                    });
                                }
                            } else {
                                self.active_job = Some(ActiveJob::NoodleDelivery {
                                    level,
                                    orders_completed,
                                    stage: NoodleDeliveryStage::Delivering {
                                        destination_pos,
                                        boxes_remaining,
                                        time_left,
                                        tip,
                                    },
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vehicle_compatibility_and_detection() {
        assert!(JobKind::Taxi.is_vehicle_compatible("Cabbie"));
        assert!(JobKind::Taxi.is_vehicle_compatible("TAXI"));
        assert!(!JobKind::Taxi.is_vehicle_compatible("Police"));

        assert_eq!(JobKind::from_vehicle("POLICE"), Some(JobKind::Vigilante));
        assert_eq!(JobKind::from_vehicle("FIRETRUCK"), Some(JobKind::Firefighter));
        assert_eq!(JobKind::from_vehicle("AMBULANCE"), Some(JobKind::Paramedic));
        assert_eq!(JobKind::from_vehicle("BURRITO CHINESE"), Some(JobKind::NoodleDelivery));
        assert_eq!(JobKind::from_vehicle("INFERNUS"), None);
    }

    #[test]
    fn test_taxi_job_lifecycle_and_progression() {
        let mut mgr = JobManager::new();
        let mut cash = 100;
        let mut save = SaveProgress::default();

        assert!(mgr.try_start_job(JobKind::Taxi, "CABBIE", [200.0, 200.0, 5.0]));
        assert!(mgr.is_active());

        // Find pickup pos
        let blip = mgr.current_radar_blip().expect("Taxi should have blip");
        assert_eq!(blip.1, 'P');

        // Approach passenger and stop
        mgr.update(0.1, blip.0, true, 1000.0, 0.0, false, &mut cash, &mut save);
        // Advance boarding timer
        mgr.update(2.5, blip.0, true, 1000.0, 0.0, false, &mut cash, &mut save);

        // Now en route
        let dest_blip = mgr.current_radar_blip().expect("En route should have destination blip");
        assert_eq!(dest_blip.1, 'T');

        // Deliver passenger
        mgr.update(0.1, dest_blip.0, true, 1000.0, 0.0, false, &mut cash, &mut save);
        assert!(cash > 100, "Cash should have increased after delivering passenger");
        assert_eq!(mgr.total_taxi_fares, 1);
        assert_eq!(mgr.best_taxi_streak, 1);
    }

    #[test]
    fn test_vigilante_progression_and_double_armour_perk() {
        let mut mgr = JobManager::new();
        let mut cash = 0;
        let mut save = SaveProgress::default();

        assert!(mgr.try_start_job(JobKind::Vigilante, "POLICE", [300.0, 300.0, 5.0]));
        assert!(mgr.is_active());

        // Simulate reaching and clearing 5 waves
        for _wave in 1..=5 {
            let crime_blip = mgr.current_radar_blip().expect("Vigilante blip");
            // Reach crime scene
            mgr.update(0.1, crime_blip.0, true, 1000.0, 0.0, false, &mut cash, &mut save);
            // Ram/eliminate suspects
            for _ in 0..5 {
                mgr.update(0.1, crime_blip.0, true, 1000.0, 15.0, false, &mut cash, &mut save);
            }
        }

        assert!(mgr.perks.double_armour, "Wave 5 vigilante must unlock double armour");
        assert_eq!(save.ambient_medal[0x06], 2, "Wave 5 vigilante awards Silver medal");
        assert!(cash >= 7500, "Cash reward should accumulate across waves");
    }

    #[test]
    fn test_firefighter_progression_and_flameproof_perk() {
        let mut mgr = JobManager::new();
        let mut cash = 0;
        let mut save = SaveProgress::default();

        assert!(mgr.try_start_job(JobKind::Firefighter, "FIRETRUCK", [150.0, 150.0, 5.0]));
        assert!(mgr.is_active());

        for _wave in 1..=5 {
            let fire_blip = mgr.current_radar_blip().expect("Fire blip");
            // Approach fire
            mgr.update(0.1, fire_blip.0, true, 1000.0, 0.0, false, &mut cash, &mut save);
            // Spray water until fire is out
            for _ in 0..40 {
                mgr.update(0.1, fire_blip.0, true, 1000.0, 0.0, true, &mut cash, &mut save);
            }
        }

        assert!(mgr.perks.flameproof, "Wave 5 firefighter must unlock flameproof perk");
        assert_eq!(save.ambient_medal[0x07], 3, "Wave 5 firefighter awards Gold medal");
    }
}
