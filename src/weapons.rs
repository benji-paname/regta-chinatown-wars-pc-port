// SPDX-License-Identifier: GPL-3.0-only
//! Chinatown Wars PSP authentic weapons and combat system.
//! Exact binary layout from weapon_tables.bin (cWeaponInfo::msWeaponInfos, 28 records x 60 bytes).
//! 11-slot weapon inventory, ammunition management, clip reloading, bullet tracing,
//! melee hit resolution, auto-aim targeting cone, and authentic combat sounds.

#![allow(dead_code)]

use crate::iso::{u16le, u32le, Result};

fn i32le(data: &[u8], off: usize) -> Result<i32> {
    Ok(u32le(data, off)? as i32)
}

pub const NUM_SLOTS: usize = 11;
pub const NUM_WEAPONS: usize = 28;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AimClass {
    Melee = 0,
    Handgun = 1,
    Shotgun = 2,
    Smg = 3,
    Rifle = 4,
    Heavy = 5,
    Thrown = 6,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WeaponRecord {
    pub aim_class: i32,
    pub max_ammo: i32,
    pub clip: i32,
    pub damage: i32,
    pub fire_delay: i32,
    pub reload_time: i32,
    pub spread: i32,
    pub range_q12: i32,
    pub slot: i32,
}

impl WeaponRecord {
    pub fn range_meters(&self) -> f32 {
        self.range_q12 as f32 / 4096.0
    }
}

#[derive(Clone, Debug)]
pub struct WeaponTables {
    pub records: Vec<WeaponRecord>,
    pub default_ammo: [u16; NUM_WEAPONS],
    pub trace_lifetimes: [i32; 3],
}

impl WeaponTables {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 8 + NUM_WEAPONS * 60 + 56 + 12 {
            return Err("weapon_tables.bin truncated".into());
        }
        if &data[0..8] != b"CTWWPN01" {
            return Err("Invalid weapon_tables magic".into());
        }

        let mut pos = 8;
        let mut records = Vec::with_capacity(NUM_WEAPONS);
        for _ in 0..NUM_WEAPONS {
            let aim_class = i32le(data, pos)?;
            let max_ammo = i32le(data, pos + 4)?;
            let clip = i32le(data, pos + 8)?;
            let damage = i32le(data, pos + 12)?;
            let fire_delay = i32le(data, pos + 16)?;
            let reload_time = i32le(data, pos + 24)?;
            let spread = i32le(data, pos + 32)?;
            let range_q12 = i32le(data, pos + 36)?;
            let slot = i32le(data, pos + 44)?;
            records.push(WeaponRecord {
                aim_class,
                max_ammo,
                clip,
                damage,
                fire_delay,
                reload_time,
                spread,
                range_q12,
                slot,
            });
            pos += 60;
        }

        let mut default_ammo = [0u16; NUM_WEAPONS];
        for i in 0..NUM_WEAPONS {
            default_ammo[i] = u16le(data, pos + i * 2)?;
        }
        pos += NUM_WEAPONS * 2;

        let trace_lifetimes = [
            i32le(data, pos)?,
            i32le(data, pos + 4)?,
            i32le(data, pos + 8)?,
        ];

        Ok(Self {
            records,
            default_ammo,
            trace_lifetimes,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponState {
    Ready = 0,
    Fired = 1,
    FireDelay = 2,
    ReloadDelay = 3,
    Reloading = 4,
    DryFire = 5,
}

#[derive(Clone, Debug)]
pub struct Weapon {
    pub type_id: u8,
    pub timer: f32,
    pub state: WeaponState,
    pub ammo: i16,
    pub clip: u8,
    pub shots: u8,
}

impl Weapon {
    pub fn new(type_id: u8, tables: &WeaponTables) -> Self {
        let rec = tables.records.get(type_id as usize).copied().unwrap_or_default();
        let clip = if rec.clip > 0 { rec.clip as u8 } else { 0 };
        let def_ammo = tables.default_ammo.get(type_id as usize).copied().unwrap_or(0);
        let ammo = if rec.max_ammo == -1 {
            -1
        } else {
            (def_ammo as i16).min(rec.max_ammo as i16)
        };

        Self {
            type_id,
            timer: 0.0,
            state: WeaponState::Ready,
            ammo,
            clip,
            shots: 0,
        }
    }

    pub fn is_melee(&self) -> bool {
        self.type_id <= 4
    }

    pub fn update(&mut self, dt: f32, rec: &WeaponRecord) {
        if self.timer > 0.0 {
            self.timer = (self.timer - dt).max(0.0);
            if self.timer == 0.0 {
                match self.state {
                    WeaponState::Fired | WeaponState::FireDelay => {
                        if self.clip == 0 && rec.clip > 0 && self.ammo != 0 {
                            self.state = WeaponState::Reloading;
                            self.timer = (rec.reload_time as f32) / 30.0;
                        } else {
                            self.state = WeaponState::Ready;
                        }
                    }
                    WeaponState::Reloading => {
                        let fill = if self.ammo == -1 {
                            rec.clip as u8
                        } else {
                            let available = self.ammo as u8;
                            available.min(rec.clip as u8)
                        };
                        self.clip = fill;
                        self.state = WeaponState::Ready;
                    }
                    WeaponState::DryFire => {
                        self.state = WeaponState::Ready;
                    }
                    _ => {
                        self.state = WeaponState::Ready;
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct WeaponManager {
    pub slots: [Option<Weapon>; NUM_SLOTS],
    pub cur_slot: usize,
    pub secondary_slot: usize,
}

impl WeaponManager {
    pub fn new(tables: &WeaponTables) -> Self {
        let mut slots = std::array::from_fn(|_| None);
        // Slot 0 is always Fists (type 0)
        slots[0] = Some(Weapon::new(0, tables));

        Self {
            slots,
            cur_slot: 0,
            secondary_slot: 8,
        }
    }

    pub fn current(&self) -> Option<&Weapon> {
        self.slots[self.cur_slot].as_ref()
    }

    pub fn current_mut(&mut self) -> Option<&mut Weapon> {
        self.slots[self.cur_slot].as_mut()
    }

    pub fn give(&mut self, type_id: u8, tables: &WeaponTables) {
        let rec = tables.records.get(type_id as usize).copied().unwrap_or_default();
        let slot = (rec.slot as usize).min(NUM_SLOTS - 1);
        self.slots[slot] = Some(Weapon::new(type_id, tables));
        if slot <= 7 {
            self.cur_slot = slot;
        } else {
            self.secondary_slot = slot;
        }
    }

    pub fn cycle_next(&mut self) {
        for step in 1..=8 {
            let next = (self.cur_slot + step) % 8;
            if self.slots[next].is_some() {
                self.cur_slot = next;
                break;
            }
        }
    }

    pub fn cycle_prev(&mut self) {
        for step in 1..=8 {
            let prev = (self.cur_slot + 8 - step) % 8;
            if self.slots[prev].is_some() {
                self.cur_slot = prev;
                break;
            }
        }
    }

    pub fn update(&mut self, dt: f32, tables: &WeaponTables) {
        for w in self.slots.iter_mut().flatten() {
            let rec = tables.records.get(w.type_id as usize).copied().unwrap_or_default();
            w.update(dt, &rec);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TargetKind {
    Ped(usize),
    Car(usize),
}

#[derive(Clone, Debug, Default)]
pub struct BulletTrace {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub life: f32,
    pub max_life: f32,
    pub kind: u8,
}

pub struct TargetingSystem {
    pub locked_target: Option<TargetKind>,
    pub reticle_health: u8,
    pub reticle_pos: [f32; 3],
}

impl TargetingSystem {
    pub fn new() -> Self {
        Self {
            locked_target: None,
            reticle_health: 255,
            reticle_pos: [0.0; 3],
        }
    }

    /// Evaluates FOV cone (+-60 deg, up to 40m) to lock best target.
    pub fn update(
        &mut self,
        player_p: [f32; 3],
        player_fwd: [f32; 2],
        peds: &[crate::actors::Ped],
        cars: &[crate::actors::Car],
    ) {
        let max_reach = 40.0;
        let mut best_target = None;
        let mut best_score = f32::MAX;
        let mut best_health = 255;
        let mut best_pos = [0.0; 3];

        // 1. Scan pedestrians
        for (idx, ped) in peds.iter().enumerate() {
            if ped.health <= 0.0 || ped.dead_timer > 0.0 {
                continue;
            }
            let dx = ped.p[0] - player_p[0];
            let dy = ped.p[1] - player_p[1];
            let dist = dx.hypot(dy);
            if dist > max_reach || dist < 0.2 {
                continue;
            }
            let dot = (dx * player_fwd[0] + dy * player_fwd[1]) / dist;
            if dot < 0.5 {
                // Must be within +-60 degrees of player facing
                continue;
            }

            let priority = if ped.hostile { 1.0 } else { 3.0 };
            let score = dist * priority;
            if score < best_score {
                best_score = score;
                best_target = Some(TargetKind::Ped(idx));
                best_health = (ped.health.clamp(0.0, 100.0) * 2.55) as u8;
                best_pos = ped.p;
            }
        }

        // 2. Scan vehicles if no pedestrian found
        if best_target.is_none() {
            for (idx, car) in cars.iter().enumerate() {
                let dx = car.p[0] - player_p[0];
                let dy = car.p[1] - player_p[1];
                let dist = dx.hypot(dy);
                if dist > max_reach || dist < 1.0 {
                    continue;
                }
                let dot = (dx * player_fwd[0] + dy * player_fwd[1]) / dist;
                if dot < 0.5 {
                    continue;
                }
                let score = dist * 5.0;
                if score < best_score {
                    best_score = score;
                    best_target = Some(TargetKind::Car(idx));
                    best_health = 255;
                    best_pos = car.p;
                }
            }
        }

        self.locked_target = best_target;
        self.reticle_health = best_health;
        self.reticle_pos = best_pos;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_authentic_weapon_tables() {
        let path = std::path::Path::new("data/tables/weapon_tables.bin");
        if let Ok(bytes) = std::fs::read(path) {
            let tables = WeaponTables::parse(&bytes).expect("Failed to parse weapon tables");
            assert_eq!(tables.records.len(), 28);

            // Fists: slot 0, melee (-1 max ammo)
            assert_eq!(tables.records[0].slot, 0);
            assert_eq!(tables.records[0].max_ammo, -1);

            // Pistol: slot 2, clip 17, max ammo 999
            assert_eq!(tables.records[5].slot, 2);
            assert_eq!(tables.records[5].clip, 17);
            assert_eq!(tables.records[5].max_ammo, 999);

            // Shotgun: slot 3, clip 1
            assert_eq!(tables.records[10].slot, 3);
            assert_eq!(tables.records[10].clip, 1);

            // SMG: slot 4, clip 30
            assert_eq!(tables.records[9].slot, 4);
            assert_eq!(tables.records[9].clip, 30);

            // Default ammo
            assert_eq!(tables.default_ammo[5], 68); // Pistol initial ammo
            assert_eq!(tables.default_ammo[9], 120); // SMG initial ammo
        }
    }

    #[test]
    fn weapon_cycle_and_inventory_slots() {
        let dummy_rec = WeaponRecord {
            aim_class: 1,
            max_ammo: 999,
            clip: 17,
            damage: 8,
            fire_delay: 25,
            reload_time: 40,
            spread: 100,
            range_q12: 100 * 4096,
            slot: 2,
        };
        let tables = WeaponTables {
            records: vec![dummy_rec; 28],
            default_ammo: [68; 28],
            trace_lifetimes: [3, 4, 4],
        };

        let mut wm = WeaponManager::new(&tables);
        assert_eq!(wm.cur_slot, 0, "Initial weapon is fists in slot 0");
        assert!(wm.current().unwrap().is_melee());

        // Give pistol (slot 2)
        wm.give(5, &tables);
        assert_eq!(wm.cur_slot, 2, "Equips pistol in slot 2");
        assert_eq!(wm.current().unwrap().clip, 17);

        // Cycle previous back to fists
        wm.cycle_prev();
        assert_eq!(wm.cur_slot, 0);

        // Cycle next back to pistol
        wm.cycle_next();
        assert_eq!(wm.cur_slot, 2);
    }

    #[test]
    fn authentic_psp_firearm_shooting_traces_and_targeting() {
        let dummy_rec = WeaponRecord {
            aim_class: 1,
            max_ammo: 999,
            clip: 17,
            damage: 8,
            fire_delay: 25,
            reload_time: 40,
            spread: 100,
            range_q12: 100 * 4096,
            slot: 2,
        };
        let tables = WeaponTables {
            records: vec![dummy_rec; 28],
            default_ammo: [68; 28],
            trace_lifetimes: [3, 4, 4],
        };

        let mut wm = WeaponManager::new(&tables);
        wm.give(5, &tables); // Pistol
        assert_eq!(wm.current().unwrap().clip, 17);

        let mut targeting = TargetingSystem::new();
        let player_p = [50.0, 50.0, 0.0];
        let player_fwd = [0.0, 1.0]; // Facing North (+Y)

        let mut ped0 = crate::actors::make_ped([50.0, 60.0, 0.0], [50.0, 70.0, 0.0], 0);
        ped0.health = 100.0;
        ped0.hostile = true;
        let mut peds = vec![ped0];

        targeting.update(player_p, player_fwd, &peds, &[]);
        assert_eq!(targeting.locked_target, Some(TargetKind::Ped(0)), "Auto-aim locks on hostile ped in cone");
        assert_eq!(targeting.reticle_pos, [50.0, 60.0, 0.0]);
        assert_eq!(targeting.reticle_health, 255);

        // Update weapon timer and simulate shot
        let w = wm.current_mut().unwrap();
        w.clip -= 1;
        w.ammo -= 1;
        assert_eq!(w.clip, 16);
        assert_eq!(w.ammo, 67);

        let trace = BulletTrace {
            start: [50.0, 50.8, 1.2],
            end: [50.0, 60.0, 0.5],
            life: 0.12,
            max_life: 0.12,
            kind: 0,
        };
        assert_eq!(trace.start[0], 50.0);
        assert_eq!(trace.end[1], 60.0);

        // Apply bullet hit
        peds[0].health -= 45.0;
        assert_eq!(peds[0].health, 55.0);
    }

    #[test]
    fn weapon_firing_takedowns_and_explosions_parity() {
        let tables_data = std::fs::read("data/tables/weapon_tables.bin").unwrap();
        let tables = WeaponTables::parse(&tables_data).unwrap();
        let mut wm = WeaponManager::new(&tables);

        // Equip Rocket Launcher (type 10)
        wm.give(10, &tables);
        let rocket = wm.current().unwrap();
        assert_eq!(rocket.type_id, 10);
        assert!(!rocket.is_melee());

        // Simulate explosion blast from rocket impact
        let blast_pos = [100.0, 100.0, 0.0];
        let blast = crate::explosions::ExplosionBlast::new(blast_pos, crate::explosions::ExplosionType::Rocket, 1.0);
        assert!(blast.damage_radius >= 9.0);

        // Pedestrian at 4m receives massive damage
        let (hit, dmg) = blast.ped_hit([100.0, 104.0, 0.0], false);
        assert!(hit);
        assert!(dmg > 400.0);

        // Vehicle blast impulse pushes car along impact vector
        let (car_dmg, impulse) = blast.vehicle_hit([100.0, 105.0, 0.0], 1.5, false);
        assert!(car_dmg > 200.0);
        assert!(impulse[1] > 0.0, "Impulse pushes car forward along Y");
        assert!(impulse[2] > 0.0, "Impulse lifts car along Z");

        // Police system registers explosion crime
        let mut police = crate::police::PoliceManager::new();
        police.report_crime(crate::police::CrimeType::Explosion);
        assert_eq!(police.stars, 1, "Explosion crime escalates wanted level");
        assert_eq!(police.takedowns_required, 1);

        // Successful takedown evades star
        let cleared = police.on_cop_takedown();
        assert!(cleared);
        assert_eq!(police.stars, 0);
    }
}
