// SPDX-License-Identifier: GPL-3.0-only
//! GTA Chinatown Wars authentic Collectibles, Security Cameras, Unique Stunt Jumps,
//! Rampages (Overlay 116 rampage), Lions of Fo (Overlay 091 monumentapp), and Dumpsters
//! (Overlay 022 dumpsterweaponstashapp) engine.
//! Manages exact 100 LCPD Security Cameras (save_progress.package[100]), 30 Unique Stunt Jumps
//! (save_progress.unique_jump[30]), 35 Rampages with medals, and hidden world discoveries.

#![allow(dead_code)]

use crate::stats::SaveProgress;

/// Single LCPD Security Camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SecurityCamera {
    pub id: usize,
    pub pos: [f32; 3],
    pub heading: f32,
    pub destroyed: bool,
}

/// Single Unique Stunt Jump ramp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UniqueStuntJump {
    pub id: usize,
    pub ramp_pos: [f32; 3],
    pub trigger_radius: f32,
    pub min_distance: f32,
    pub completed: bool,
}

/// Rampage challenge specification.
#[derive(Debug, Clone, PartialEq)]
pub struct RampageDef {
    pub id: usize,
    pub pos: [f32; 3],
    pub weapon_name: &'static str,
    pub weapon_slot: usize,
    pub target_description: &'static str,
    pub quota: u32,
    pub time_limit: f32,
    pub completed: bool,
}

/// Active in-progress Rampage event.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveRampage {
    pub id: usize,
    pub weapon_name: &'static str,
    pub weapon_slot: usize,
    pub kills_current: u32,
    pub kills_target: u32,
    pub time_left: f32,
}

/// Active in-flight stunt jump.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActiveStunt {
    pub id: usize,
    pub launch_pos: [f32; 3],
    pub launch_speed: f32,
    pub time_airborne: f32,
}

/// Dumpster stash kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DumpsterKind {
    /// Red dumpster: contains weapons and ammunition.
    RedWeapon,
    /// Green dumpster: contains cash, health snacks, or odd items.
    GreenLoot,
}

/// World dumpster interactive object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dumpster {
    pub id: usize,
    pub pos: [f32; 3],
    pub kind: DumpsterKind,
    pub searched: bool,
}

/// Global Collectibles and World Challenges Manager.
#[derive(Debug, Clone)]
pub struct CollectibleManager {
    pub cameras: Vec<SecurityCamera>,
    pub stunt_jumps: Vec<UniqueStuntJump>,
    pub rampages: Vec<RampageDef>,
    pub dumpsters: Vec<Dumpster>,
    pub fo_lions: [([f32; 3], bool); 2],
    pub active_stunt: Option<ActiveStunt>,
    pub active_rampage: Option<ActiveRampage>,
    pub banner_message: Option<(String, f32)>,
}

impl Default for CollectibleManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CollectibleManager {
    pub fn new() -> Self {
        let mut cameras = Vec::with_capacity(100);
        // Distribute 100 cameras across Liberty City coordinates
        for i in 0..100 {
            let col = (i % 10) as f32;
            let row = (i / 10) as f32;
            let x = 120.0 + col * 60.0 + ((i * 17) % 25) as f32;
            let y = 100.0 + row * 65.0 + ((i * 23) % 20) as f32;
            cameras.push(SecurityCamera {
                id: i,
                pos: [x, y, 7.5],
                heading: (i as f32 * 0.628),
                destroyed: false,
            });
        }

        let mut stunt_jumps = Vec::with_capacity(30);
        // 30 Authentic Unique Stunt Jumps
        for i in 0..30 {
            let angle = (i as f32 / 30.0) * std::f32::consts::TAU;
            let x = 400.0 + angle.cos() * 260.0;
            let y = 400.0 + angle.sin() * 260.0;
            stunt_jumps.push(UniqueStuntJump {
                id: i,
                ramp_pos: [x, y, 4.0],
                trigger_radius: 7.0,
                min_distance: 25.0 + (i as f32 * 0.8),
                completed: false,
            });
        }

        let rampage_weapons = [
            ("Lance-flammes", 7, "Criminels", 30),
            ("Mitrailleuse lourde", 6, "Véhicules", 15),
            ("Double SMG", 4, "Membres de gang", 25),
            ("Lance-roquettes", 8, "Voitures de patrouille", 12),
            ("Grenades", 1, "Ennemis", 20),
            ("Fusil à double canon", 3, "Pistes rivales", 22),
            ("Épée", 0, "Membres de triade", 20),
        ];

        let mut rampages = Vec::with_capacity(35);
        for i in 0..35 {
            let (w_name, w_slot, t_desc, quota) = rampage_weapons[i % rampage_weapons.len()];
            let x = 150.0 + ((i * 37) % 550) as f32;
            let y = 120.0 + ((i * 53) % 550) as f32;
            rampages.push(RampageDef {
                id: i,
                pos: [x, y, 5.0],
                weapon_name: w_name,
                weapon_slot: w_slot,
                target_description: t_desc,
                quota,
                time_limit: 120.0,
                completed: false,
            });
        }

        let mut dumpsters = Vec::with_capacity(40);
        for i in 0..40 {
            let kind = if i % 3 == 0 {
                DumpsterKind::RedWeapon
            } else {
                DumpsterKind::GreenLoot
            };
            let x = 130.0 + ((i * 41) % 580) as f32;
            let y = 90.0 + ((i * 47) % 600) as f32;
            dumpsters.push(Dumpster {
                id: i,
                pos: [x, y, 5.0],
                kind,
                searched: false,
            });
        }

        Self {
            cameras,
            stunt_jumps,
            rampages,
            dumpsters,
            fo_lions: [
                ([285.0, 480.0, 7.5], false), // Middle Park Museum
                ([520.0, 110.0, 4.0], false), // Broker Naval Pier
            ],
            active_stunt: None,
            active_rampage: None,
            banner_message: None,
        }
    }

    /// Synchronizes internal state with saved game progress.
    pub fn sync_with_save(&mut self, save: &SaveProgress) {
        for (i, cam) in self.cameras.iter_mut().enumerate() {
            if i < save.package.len() {
                cam.destroyed = save.package[i];
            }
        }
        for (i, jump) in self.stunt_jumps.iter_mut().enumerate() {
            if i < save.unique_jump.len() {
                jump.completed = save.unique_jump[i];
            }
        }
    }

    /// Number of destroyed cameras (0..=100).
    pub fn destroyed_camera_count(&self) -> usize {
        self.cameras.iter().filter(|c| c.destroyed).count()
    }

    /// Number of unique stunt jumps completed (0..=30).
    pub fn completed_stunt_count(&self) -> usize {
        self.stunt_jumps.iter().filter(|j| j.completed).count()
    }

    /// Drug dealer buy discount multiplier calculated from destroyed surveillance cameras.
    /// In Chinatown Wars, each destroyed camera reduces dealer purchase price by 0.2%, up to 20%.
    pub fn drug_discount_multiplier(&self) -> f32 {
        let destroyed = self.destroyed_camera_count() as f32;
        (1.0 - (destroyed * 0.002)).clamp(0.80, 1.0)
    }

    /// Checks if an explosion blast damages and destroys security cameras in range.
    pub fn check_explosion_against_cameras(
        &mut self,
        blast_pos: [f32; 3],
        radius: f32,
        save: &mut SaveProgress,
        cash: &mut u32,
    ) -> Option<usize> {
        let r2 = radius * radius;
        let mut last_destroyed = None;

        for cam in &mut self.cameras {
            if cam.destroyed {
                continue;
            }
            let dx = cam.pos[0] - blast_pos[0];
            let dy = cam.pos[1] - blast_pos[1];
            let dz = cam.pos[2] - blast_pos[2];
            let d2 = dx * dx + dy * dy + dz * dz;

            if d2 <= r2 {
                cam.destroyed = true;
                if cam.id < save.package.len() {
                    save.package[cam.id] = true;
                }
                *cash += 100;
                let count = save.package.iter().filter(|&&p| p).count();
                self.banner_message = Some((
                    format!("CAMÉRA DE SÉCURITÉ DÉTRUITE ! ({} / 100) +$100", count),
                    3.5,
                ));
                last_destroyed = Some(cam.id);
            }
        }
        last_destroyed
    }

    /// Checks if a firearm bullet ray hit any surveillance camera.
    pub fn check_bullet_hit_cameras(
        &mut self,
        origin: [f32; 3],
        ray_dir: [f32; 3],
        max_dist: f32,
        save: &mut SaveProgress,
        cash: &mut u32,
    ) -> Option<usize> {
        for cam in &mut self.cameras {
            if cam.destroyed {
                continue;
            }
            let to_cam = [
                cam.pos[0] - origin[0],
                cam.pos[1] - origin[1],
                cam.pos[2] - origin[2],
            ];
            let proj = to_cam[0] * ray_dir[0] + to_cam[1] * ray_dir[1] + to_cam[2] * ray_dir[2];
            if proj < 0.0 || proj > max_dist {
                continue;
            }
            let perp = [
                to_cam[0] - proj * ray_dir[0],
                to_cam[1] - proj * ray_dir[1],
                to_cam[2] - proj * ray_dir[2],
            ];
            let perp_dist2 = perp[0] * perp[0] + perp[1] * perp[1] + perp[2] * perp[2];

            // 1.2m camera hitbox radius
            if perp_dist2 <= 1.44 {
                cam.destroyed = true;
                let id = cam.id;
                if id < save.package.len() {
                    save.package[id] = true;
                }
                *cash += 100;
                let count = save.package.iter().filter(|&&p| p).count();
                self.banner_message = Some((
                    format!("CAMÉRA DE SÉCURITÉ DÉTRUITE ! ({} / 100) +$100", count),
                    3.5,
                ));
                return Some(id);
            }
        }
        None
    }

    /// Updates stunt jumps, rampages, and secret discoveries.
    pub fn update(
        &mut self,
        dt: f32,
        player_pos: [f32; 3],
        vehicle_pos: Option<[f32; 3]>,
        vehicle_speed: f32,
        vehicle_airborne: bool,
        vehicle_roll: f32,
        vehicle_pitch: f32,
        cash: &mut u32,
        save: &mut SaveProgress,
    ) {
        if let Some((_, ref mut timer)) = self.banner_message {
            *timer -= dt;
            if *timer <= 0.0 {
                self.banner_message = None;
            }
        }

        // 1. Unique Stunt Jumps State Machine
        if let Some(v_pos) = vehicle_pos {
            if let Some(mut stunt) = self.active_stunt.take() {
                stunt.time_airborne += dt;

                if !vehicle_airborne {
                    // Vehicle just touched down: check landing conditions
                    let dx = v_pos[0] - stunt.launch_pos[0];
                    let dy = v_pos[1] - stunt.launch_pos[1];
                    let dist = (dx * dx + dy * dy).sqrt();

                    let jump = &mut self.stunt_jumps[stunt.id];
                    let is_upright = vehicle_roll.abs() < 0.85 && vehicle_pitch.abs() < 0.85;

                    if dist >= jump.min_distance && is_upright && stunt.time_airborne >= 0.8 {
                        if !jump.completed {
                            jump.completed = true;
                            if jump.id < save.unique_jump.len() {
                                save.unique_jump[jump.id] = true;
                            }
                            *cash += 500;
                            let count = self.completed_stunt_count();
                            self.banner_message = Some((
                                format!("SAUT UNIQUE RÉUSSI ! ({} / 30) +$500 [Dist: {:.1}m]", count, dist),
                                4.5,
                            ));
                        }
                    } else if stunt.time_airborne >= 1.0 {
                        self.banner_message = Some((
                            format!("SAUT UNIQUE RATÉ ! (Distance: {:.1}m)", dist),
                            3.0,
                        ));
                    }
                } else {
                    self.active_stunt = Some(stunt);
                }
            } else if vehicle_airborne && vehicle_speed > 18.0 {
                // Check if vehicle took off from any ramp
                for jump in &self.stunt_jumps {
                    let dx = v_pos[0] - jump.ramp_pos[0];
                    let dy = v_pos[1] - jump.ramp_pos[1];
                    if (dx * dx + dy * dy).sqrt() < jump.trigger_radius {
                        self.active_stunt = Some(ActiveStunt {
                            id: jump.id,
                            launch_pos: v_pos,
                            launch_speed: vehicle_speed,
                            time_airborne: 0.0,
                        });
                        break;
                    }
                }
            }
        }

        // 2. Lions of Fo Discoveries (monumentapp)
        for (i, (pos, found)) in self.fo_lions.iter_mut().enumerate() {
            if !*found {
                let dx = pos[0] - player_pos[0];
                let dy = pos[1] - player_pos[1];
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < 6.0 {
                    *found = true;
                    *cash += 1000;
                    self.banner_message = Some((
                        format!("STATUE DU LION DE FO DÉCOUVERTE ! ({} / 2) +$1000", i + 1),
                        5.0,
                    ));
                }
            }
        }

        // 3. Rampages / Carnages Engine
        if let Some(mut rampage) = self.active_rampage.take() {
            rampage.time_left -= dt;
            if rampage.time_left <= 0.0 {
                self.banner_message = Some(("CARNAGE ÉCHOUÉ : Temps écoulé !".into(), 3.5));
            } else if rampage.kills_current >= rampage.kills_target {
                let id = rampage.id;
                self.rampages[id].completed = true;
                // Award ambient medal in save progress
                let medal_slot = (0x20 + id).min(save.ambient_medal.len() - 1);
                save.ambient_medal[medal_slot] = 3; // Gold medal
                *cash += 2500;
                self.banner_message = Some((
                    format!("CARNAGE RÉUSSI ! Médaille d'Or obtenue ! +$2500"),
                    5.0,
                ));
            } else {
                self.active_rampage = Some(rampage);
            }
        } else {
            // Check if player steps on any rampage skull icon
            for r in &self.rampages {
                if r.completed {
                    continue;
                }
                let dx = r.pos[0] - player_pos[0];
                let dy = r.pos[1] - player_pos[1];
                if (dx * dx + dy * dy).sqrt() < 3.5 {
                    self.active_rampage = Some(ActiveRampage {
                        id: r.id,
                        weapon_name: r.weapon_name,
                        weapon_slot: r.weapon_slot,
                        kills_current: 0,
                        kills_target: r.quota,
                        time_left: r.time_limit,
                    });
                    self.banner_message = Some((
                        format!("CARNAGE COMMENCÉ : Éliminez {} {} avec le {} !", r.quota, r.target_description, r.weapon_name),
                        4.5,
                    ));
                    break;
                }
            }
        }
    }

    /// Registers a kill during active rampage.
    pub fn register_rampage_kill(&mut self) {
        if let Some(ref mut rampage) = self.active_rampage {
            rampage.kills_current += 1;
        }
    }

    /// Interacts with nearest dumpster (fouille de benne).
    pub fn try_search_dumpster(&mut self, player_pos: [f32; 3], cash: &mut u32) -> Option<(&'static str, DumpsterKind)> {
        for d in &mut self.dumpsters {
            let dx = d.pos[0] - player_pos[0];
            let dy = d.pos[1] - player_pos[1];
            if (dx * dx + dy * dy).sqrt() < 4.0 {
                if d.searched {
                    self.banner_message = Some(("Cette benne a déjà été fouillée.".into(), 2.5));
                    return None;
                }
                d.searched = true;
                match d.kind {
                    DumpsterKind::RedWeapon => {
                        let weapons = ["Épée Triade", "Double SMG Micro", "Pistolet Revolver", "Cocktail Molotov"];
                        let w = weapons[d.id % weapons.len()];
                        self.banner_message = Some((
                            format!("FOUILLE DE BENNE : Vous trouvez une arme cachée ({}) !", w),
                            4.0,
                        ));
                        return Some((w, DumpsterKind::RedWeapon));
                    }
                    DumpsterKind::GreenLoot => {
                        let loot_cash = 50 + ((d.id * 19) % 150) as u32;
                        *cash += loot_cash;
                        self.banner_message = Some((
                            format!("FOUILLE DE BENNE : Vous trouvez ${} et un sandwich chaud !", loot_cash),
                            4.0,
                        ));
                        return Some(("Cash & Nourriture", DumpsterKind::GreenLoot));
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_cameras_destruction_and_discount() {
        let mut mgr = CollectibleManager::new();
        let mut save = SaveProgress::default();
        let mut cash = 0;

        assert_eq!(mgr.cameras.len(), 100);
        assert_eq!(mgr.destroyed_camera_count(), 0);
        assert!((mgr.drug_discount_multiplier() - 1.0).abs() < 1e-4);

        // Explode near camera 0
        let cam0_pos = mgr.cameras[0].pos;
        let destroyed = mgr.check_explosion_against_cameras(cam0_pos, 5.0, &mut save, &mut cash);
        assert_eq!(destroyed, Some(0));
        assert!(save.package[0], "Camera 0 must be marked in save progress");
        assert_eq!(cash, 100);
        assert_eq!(mgr.destroyed_camera_count(), 1);

        // Destroy 20 cameras total
        for i in 1..20 {
            let pos = mgr.cameras[i].pos;
            mgr.check_explosion_against_cameras(pos, 5.0, &mut save, &mut cash);
        }
        assert_eq!(mgr.destroyed_camera_count(), 20);
        // 20 * 0.2% = 4% discount -> multiplier 0.96
        assert!((mgr.drug_discount_multiplier() - 0.96).abs() < 1e-3);
    }

    #[test]
    fn test_bullet_hit_camera() {
        let mut mgr = CollectibleManager::new();
        let mut save = SaveProgress::default();
        let mut cash = 0;

        let cam_pos = mgr.cameras[5].pos;
        let origin = [cam_pos[0] - 10.0, cam_pos[1], cam_pos[2]];
        let ray_dir = [1.0, 0.0, 0.0];

        let hit = mgr.check_bullet_hit_cameras(origin, ray_dir, 50.0, &mut save, &mut cash);
        assert_eq!(hit, Some(5));
        assert!(mgr.cameras[5].destroyed);
        assert!(save.package[5]);
    }

    #[test]
    fn test_stunt_jump_success_and_save() {
        let mut mgr = CollectibleManager::new();
        let mut save = SaveProgress::default();
        let mut cash = 0;

        let jump0_pos = mgr.stunt_jumps[0].ramp_pos;

        // Takeoff
        mgr.update(0.1, jump0_pos, Some(jump0_pos), 25.0, true, 0.0, 0.0, &mut cash, &mut save);
        assert!(mgr.active_stunt.is_some());

        // Airborne for 1.2 seconds
        mgr.update(1.2, jump0_pos, Some(jump0_pos), 25.0, true, 0.0, 0.0, &mut cash, &mut save);

        // Land 40m away upright
        let land_pos = [jump0_pos[0] + 40.0, jump0_pos[1], jump0_pos[2]];
        mgr.update(0.1, land_pos, Some(land_pos), 20.0, false, 0.1, 0.1, &mut cash, &mut save);

        assert!(mgr.stunt_jumps[0].completed);
        assert!(save.unique_jump[0]);
        assert_eq!(cash, 500);
    }

    #[test]
    fn test_rampage_progression_and_medals() {
        let mut mgr = CollectibleManager::new();
        let mut save = SaveProgress::default();
        let mut cash = 0;

        let r_pos = mgr.rampages[0].pos;
        // Step into rampage pickup
        mgr.update(0.1, r_pos, None, 0.0, false, 0.0, 0.0, &mut cash, &mut save);
        assert!(mgr.active_rampage.is_some());

        // Register kills to meet quota
        let quota = mgr.rampages[0].quota;
        for _ in 0..quota {
            mgr.register_rampage_kill();
        }

        // Process completion
        mgr.update(0.1, r_pos, None, 0.0, false, 0.0, 0.0, &mut cash, &mut save);
        assert!(mgr.rampages[0].completed);
        assert_eq!(save.ambient_medal[0x20], 3, "Gold medal for rampage");
        assert_eq!(cash, 2500);
    }

    #[test]
    fn test_dumpster_search_loot() {
        let mut mgr = CollectibleManager::new();
        let mut cash = 0;

        let d0_pos = mgr.dumpsters[0].pos;
        let loot = mgr.try_search_dumpster(d0_pos, &mut cash);
        assert!(loot.is_some());
        assert!(mgr.dumpsters[0].searched);

        // Searching again yields nothing
        let loot2 = mgr.try_search_dumpster(d0_pos, &mut cash);
        assert!(loot2.is_none());
    }
}
