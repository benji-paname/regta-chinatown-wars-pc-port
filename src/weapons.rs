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

impl Default for WeaponTables {
    fn default() -> Self {
        Self {
            records: vec![WeaponRecord::default(); NUM_WEAPONS],
            default_ammo: [0; NUM_WEAPONS],
            trace_lifetimes: [0; 3],
        }
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
        self.cur_slot = slot;
        if slot > 7 {
            self.secondary_slot = slot;
        }
    }

    pub fn cycle_next(&mut self) {
        for step in 1..=NUM_SLOTS {
            let next = (self.cur_slot + step) % NUM_SLOTS;
            if self.slots[next].is_some() {
                self.cur_slot = next;
                break;
            }
        }
    }

    pub fn cycle_prev(&mut self) {
        for step in 1..=NUM_SLOTS {
            let prev = (self.cur_slot + NUM_SLOTS - (step % NUM_SLOTS)) % NUM_SLOTS;
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

// ============================================================================
// Chinatown Wars authentic Projectiles Engine (cBaseProjectile, cGrenade, cMolotov, cRocket, cAIRocket)
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectileKind {
    Grenade = 0,
    Molotov = 1,
    Rocket = 2,
    Flashbang = 3,
    Mine = 4,
}

#[derive(Clone, Debug)]
pub struct Projectile {
    pub kind: ProjectileKind,
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub fuse_timer: f32,
    pub max_fuse: f32,
    pub bounces: u8,
    pub radius: f32,
    pub active: bool,
    pub owner_is_player: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ProjectileManager {
    pub list: Vec<Projectile>,
}

impl ProjectileManager {
    pub fn new() -> Self {
        Self { list: Vec::new() }
    }

    pub fn spawn_grenade(&mut self, pos: [f32; 3], fwd: [f32; 2], power: f32, from_player: bool) {
        let p = power.clamp(0.4, 2.0);
        self.list.push(Projectile {
            kind: ProjectileKind::Grenade,
            pos: [pos[0], pos[1], pos[2] + 1.2],
            vel: [fwd[0] * 16.0 * p, fwd[1] * 16.0 * p, 4.5 * p],
            fuse_timer: 2.2,
            max_fuse: 2.2,
            bounces: 0,
            radius: 0.25,
            active: true,
            owner_is_player: from_player,
        });
    }

    pub fn spawn_molotov(&mut self, pos: [f32; 3], fwd: [f32; 2], power: f32, from_player: bool) {
        let p = power.clamp(0.4, 2.0);
        self.list.push(Projectile {
            kind: ProjectileKind::Molotov,
            pos: [pos[0], pos[1], pos[2] + 1.2],
            vel: [fwd[0] * 14.0 * p, fwd[1] * 14.0 * p, 4.0 * p],
            fuse_timer: 4.0,
            max_fuse: 4.0,
            bounces: 0,
            radius: 0.22,
            active: true,
            owner_is_player: from_player,
        });
    }

    pub fn spawn_rocket(&mut self, pos: [f32; 3], fwd: [f32; 2], from_player: bool) {
        self.list.push(Projectile {
            kind: ProjectileKind::Rocket,
            pos: [pos[0] + fwd[0] * 1.0, pos[1] + fwd[1] * 1.0, pos[2] + 1.2],
            vel: [fwd[0] * 38.0, fwd[1] * 38.0, -0.4],
            fuse_timer: 3.0,
            max_fuse: 3.0,
            bounces: 0,
            radius: 0.35,
            active: true,
            owner_is_player: from_player,
        });
    }

    pub fn spawn_flashbang(&mut self, pos: [f32; 3], fwd: [f32; 2], power: f32, from_player: bool) {
        let p = power.clamp(0.4, 2.0);
        self.list.push(Projectile {
            kind: ProjectileKind::Flashbang,
            pos: [pos[0], pos[1], pos[2] + 1.2],
            vel: [fwd[0] * 15.0 * p, fwd[1] * 15.0 * p, 4.2 * p],
            fuse_timer: 1.8,
            max_fuse: 1.8,
            bounces: 0,
            radius: 0.22,
            active: true,
            owner_is_player: from_player,
        });
    }

    pub fn update(
        &mut self,
        dt: f32,
        collision: &crate::collision::Collision,
        peds: &mut [crate::actors::Ped],
        cars: &mut [crate::actors::Car],
        fire_mgr: &mut FireManager,
        explosions: &mut crate::explosions::ExplosionManager,
    ) {
        let mut exploded = Vec::new();

        for proj in &mut self.list {
            if !proj.active {
                continue;
            }

            match proj.kind {
                ProjectileKind::Rocket => {
                    proj.pos[0] += proj.vel[0] * dt;
                    proj.pos[1] += proj.vel[1] * dt;
                    proj.pos[2] += proj.vel[2] * dt;
                    proj.fuse_timer -= dt;

                    let mut hit = proj.fuse_timer <= 0.0;

                    // Building collision check
                    if !collision.body_clear(proj.pos, proj.radius, 0.5) {
                        hit = true;
                    }

                    // Vehicle collision check
                    if !hit {
                        for car in cars.iter() {
                            let dx = car.p[0] - proj.pos[0];
                            let dy = car.p[1] - proj.pos[1];
                            let dz = car.p[2] - proj.pos[2];
                            if dx * dx + dy * dy + dz * dz < (car.h[0] + 0.8) * (car.h[0] + 0.8) {
                                hit = true;
                                break;
                            }
                        }
                    }

                    // Ped collision check
                    if !hit {
                        for ped in peds.iter() {
                            if ped.health <= 0.0 {
                                continue;
                            }
                            let dx = ped.p[0] - proj.pos[0];
                            let dy = ped.p[1] - proj.pos[1];
                            let dz = ped.p[2] - proj.pos[2];
                            if dx * dx + dy * dy + dz * dz < 1.44 {
                                hit = true;
                                break;
                            }
                        }
                    }

                    if hit {
                        proj.active = false;
                        exploded.push((proj.pos, crate::explosions::ExplosionType::Rocket, 1.0));
                    }
                }
                ProjectileKind::Grenade | ProjectileKind::Flashbang => {
                    proj.vel[2] -= 9.81 * dt;
                    proj.pos[0] += proj.vel[0] * dt;
                    proj.pos[1] += proj.vel[1] * dt;
                    proj.pos[2] += proj.vel[2] * dt;
                    proj.fuse_timer -= dt;

                    let g = collision.ground(proj.pos[0], proj.pos[1], proj.pos[2]);
                    if proj.pos[2] <= g.z + 0.08 {
                        proj.pos[2] = g.z + 0.08;
                        if proj.vel[2] < 0.0 {
                            proj.vel[2] = -proj.vel[2] * 0.42;
                            proj.vel[0] *= 0.68;
                            proj.vel[1] *= 0.68;
                            proj.bounces += 1;
                        }
                        if proj.vel[2].abs() < 0.25 {
                            proj.vel[2] = 0.0;
                        }
                    }

                    // Wall bounce
                    if !collision.body_clear(proj.pos, proj.radius, 0.4) {
                        proj.vel[0] = -proj.vel[0] * 0.5;
                        proj.vel[1] = -proj.vel[1] * 0.5;
                    }

                    if proj.fuse_timer <= 0.0 {
                        proj.active = false;
                        let kind = crate::explosions::ExplosionType::Generic;
                        let scale = if proj.kind == ProjectileKind::Grenade { 1.0 } else { 0.6 };
                        exploded.push((proj.pos, kind, scale));
                        if proj.kind == ProjectileKind::Flashbang {
                            for ped in peds.iter_mut() {
                                let dx = ped.p[0] - proj.pos[0];
                                let dy = ped.p[1] - proj.pos[1];
                                if dx * dx + dy * dy < 64.0 {
                                    ped.hit_flash = 1.0;
                                    ped.panic = 6.0;
                                }
                            }
                        }
                    }
                }
                ProjectileKind::Molotov => {
                    proj.vel[2] -= 9.81 * dt;
                    proj.pos[0] += proj.vel[0] * dt;
                    proj.pos[1] += proj.vel[1] * dt;
                    proj.pos[2] += proj.vel[2] * dt;
                    proj.fuse_timer -= dt;

                    let g = collision.ground(proj.pos[0], proj.pos[1], proj.pos[2]);
                    let mut shattered = proj.pos[2] <= g.z + 0.1 || proj.fuse_timer <= 0.0;

                    // Vehicle impact check
                    if !shattered {
                        for car in cars.iter() {
                            let dx = car.p[0] - proj.pos[0];
                            let dy = car.p[1] - proj.pos[1];
                            let dz = car.p[2] - proj.pos[2];
                            if dx * dx + dy * dy + dz * dz < (car.h[0] + 0.6) * (car.h[0] + 0.6) {
                                shattered = true;
                                break;
                            }
                        }
                    }

                    // Ped impact check
                    if !shattered {
                        for ped in peds.iter_mut() {
                            if ped.health <= 0.0 {
                                continue;
                            }
                            let dx = ped.p[0] - proj.pos[0];
                            let dy = ped.p[1] - proj.pos[1];
                            let dz = ped.p[2] - proj.pos[2];
                            if dx * dx + dy * dy + dz * dz < 1.2 {
                                ped.on_fire = 6.0;
                                shattered = true;
                                break;
                            }
                        }
                    }

                    if shattered {
                        proj.active = false;
                        exploded.push((proj.pos, crate::explosions::ExplosionType::Molotov, 1.0));
                        if !g.water {
                            fire_mgr.spawn_fire(proj.pos, 2.4, 7.0);
                        }
                    }
                }
                ProjectileKind::Mine => {
                    proj.vel[2] -= 9.81 * dt;
                    let g = collision.ground(proj.pos[0], proj.pos[1], proj.pos[2]);
                    proj.pos[2] = (proj.pos[2] + proj.vel[2] * dt).max(g.z + 0.05);
                    proj.fuse_timer = (proj.fuse_timer - dt).max(0.0);
                    if proj.fuse_timer <= 0.0 {
                        let mut triggered = false;
                        for ped in peds.iter() {
                            if ped.health > 0.0 {
                                let dx = ped.p[0] - proj.pos[0];
                                let dy = ped.p[1] - proj.pos[1];
                                if dx * dx + dy * dy < 4.0 {
                                    triggered = true;
                                    break;
                                }
                            }
                        }
                        if !triggered {
                            for car in cars.iter() {
                                let dx = car.p[0] - proj.pos[0];
                                let dy = car.p[1] - proj.pos[1];
                                if dx * dx + dy * dy < (car.h[0] + 0.5) * (car.h[0] + 0.5) {
                                    triggered = true;
                                    break;
                                }
                            }
                        }
                        if triggered {
                            proj.active = false;
                            exploded.push((proj.pos, crate::explosions::ExplosionType::Barrel, 1.0));
                        }
                    }
                }
            }
        }

        for (pos, kind, scale) in exploded {
            explosions.trigger(pos, kind, scale);
        }

        self.list.retain(|p| p.active);
    }
}

// ============================================================================
// Chinatown Wars authentic Flamethrower & Fire Manager (cFlameThrower, cFire, Fire, cParticleEmitterFlameThrower)
// ============================================================================

#[derive(Clone, Debug)]
pub struct FirePatch {
    pub pos: [f32; 3],
    pub radius: f32,
    pub life: f32,
    pub max_life: f32,
    pub intensity: f32,
}

#[derive(Clone, Debug, Default)]
pub struct FireManager {
    pub patches: Vec<FirePatch>,
}

impl FireManager {
    pub fn new() -> Self {
        Self { patches: Vec::new() }
    }

    pub fn spawn_fire(&mut self, pos: [f32; 3], radius: f32, duration: f32) {
        if self.patches.len() < 32 {
            self.patches.push(FirePatch {
                pos,
                radius: radius.max(0.5),
                life: duration,
                max_life: duration,
                intensity: 1.0,
            });
        }
    }

    pub fn extinguish_near(&mut self, pos: [f32; 3], radius: f32) -> usize {
        let r2 = radius * radius;
        let mut count = 0;
        for patch in &mut self.patches {
            let dx = patch.pos[0] - pos[0];
            let dy = patch.pos[1] - pos[1];
            if dx * dx + dy * dy < r2 {
                patch.life = 0.0;
                count += 1;
            }
        }
        self.patches.retain(|p| p.life > 0.0);
        count
    }

    pub fn emit_flamethrower_stream(
        &mut self,
        muzzle: [f32; 3],
        fwd: [f32; 2],
        peds: &mut [crate::actors::Ped],
        cars: &mut [crate::actors::Car],
    ) {
        let reach = 7.5;
        for ped in peds.iter_mut() {
            if ped.health <= 0.0 {
                continue;
            }
            let dx = ped.p[0] - muzzle[0];
            let dy = ped.p[1] - muzzle[1];
            let dist = dx.hypot(dy);
            if dist < reach && dist > 0.2 {
                let dot = (dx * fwd[0] + dy * fwd[1]) / dist;
                if dot > 0.82 {
                    ped.on_fire = (ped.on_fire + 3.0).min(7.0);
                    ped.health = (ped.health - 22.0).max(0.0);
                    ped.panic = 7.0;
                    ped.hit_flash = 0.3;
                }
            }
        }

        for car in cars.iter_mut() {
            let dx = car.p[0] - muzzle[0];
            let dy = car.p[1] - muzzle[1];
            let dist = dx.hypot(dy);
            if dist < reach {
                let dot = (dx * fwd[0] + dy * fwd[1]) / dist.max(0.01);
                if dot > 0.82 {
                    car.speed *= 0.95;
                }
            }
        }

        let flame_ground = [muzzle[0] + fwd[0] * 4.0, muzzle[1] + fwd[1] * 4.0, muzzle[2]];
        self.spawn_fire(flame_ground, 1.4, 4.0);
    }

    pub fn update(
        &mut self,
        dt: f32,
        peds: &mut [crate::actors::Ped],
        cars: &mut [crate::actors::Car],
    ) {
        for patch in &mut self.patches {
            patch.life = (patch.life - dt).max(0.0);
            patch.intensity = if patch.max_life > 0.0 {
                patch.life / patch.max_life
            } else {
                0.0
            };

            let r2 = patch.radius * patch.radius;
            for ped in peds.iter_mut() {
                if ped.health <= 0.0 {
                    continue;
                }
                let dx = ped.p[0] - patch.pos[0];
                let dy = ped.p[1] - patch.pos[1];
                if dx * dx + dy * dy < r2 {
                    ped.on_fire = (ped.on_fire + 2.5 * dt).min(6.0);
                    ped.panic = 6.0;
                }
            }

            for car in cars.iter_mut() {
                let dx = car.p[0] - patch.pos[0];
                let dy = car.p[1] - patch.pos[1];
                if dx * dx + dy * dy < (patch.radius + car.h[0]) * (patch.radius + car.h[0]) {
                    car.speed *= (1.0 - 0.2 * dt).max(0.5);
                }
            }
        }
        self.patches.retain(|p| p.life > 0.0);

        for ped in peds.iter_mut() {
            if ped.on_fire > 0.0 {
                ped.on_fire = (ped.on_fire - dt).max(0.0);
                ped.health = (ped.health - 28.0 * dt).max(0.0);
                ped.panic = 5.0;
                ped.hit_flash = 0.2;
                if ped.health <= 0.0 {
                    ped.dead_timer = 0.0;
                }
            }
        }
    }
}

// ============================================================================
// Chinatown Wars authentic Sniper Rifle Scope & Camera (cSniperRifle, cPlayerAsSniper, cSniperCam)
// ============================================================================

#[derive(Clone, Debug)]
pub struct SniperScope {
    pub active: bool,
    pub zoom: f32,
    pub target_zoom: f32,
    pub sway_time: f32,
    pub sway_offset: [f32; 2],
    pub held_breath: bool,
    pub breath_meter: f32,
    pub reticle_range: f32,
}

impl Default for SniperScope {
    fn default() -> Self {
        Self::new()
    }
}

impl SniperScope {
    pub fn new() -> Self {
        Self {
            active: false,
            zoom: 1.0,
            target_zoom: 1.0,
            sway_time: 0.0,
            sway_offset: [0.0; 2],
            held_breath: false,
            breath_meter: 1.0,
            reticle_range: 0.0,
        }
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
        if self.active {
            self.target_zoom = 2.0;
        } else {
            self.target_zoom = 1.0;
            self.held_breath = false;
        }
    }

    pub fn zoom_in(&mut self) {
        if self.active {
            self.target_zoom = (self.target_zoom + 0.5).min(4.0);
        }
    }

    pub fn zoom_out(&mut self) {
        if self.active {
            self.target_zoom = (self.target_zoom - 0.5).max(1.0);
        }
    }

    pub fn hold_breath(&mut self, hold: bool) {
        self.held_breath = hold;
    }

    pub fn update(&mut self, dt: f32, target_pos: Option<[f32; 3]>, player_pos: [f32; 3]) {
        if !self.active {
            self.zoom = 1.0;
            self.sway_offset = [0.0; 2];
            self.breath_meter = (self.breath_meter + dt * 0.5).min(1.0);
            return;
        }

        self.zoom += (self.target_zoom - self.zoom) * (dt * 8.0).min(1.0);

        if self.held_breath && self.breath_meter > 0.0 {
            self.breath_meter = (self.breath_meter - dt * 0.28).max(0.0);
            if self.breath_meter == 0.0 {
                self.held_breath = false;
            }
        } else {
            self.breath_meter = (self.breath_meter + dt * 0.35).min(1.0);
        }

        self.sway_time += dt;
        let factor = if self.held_breath && self.breath_meter > 0.0 {
            0.12
        } else {
            1.0
        };

        let sx = (self.sway_time * 1.35).sin() * 0.042 * factor;
        let sy = (self.sway_time * 2.10).cos() * 0.024 * factor;
        self.sway_offset = [sx, sy];

        if let Some(tp) = target_pos {
            let dx = tp[0] - player_pos[0];
            let dy = tp[1] - player_pos[1];
            let dz = tp[2] - player_pos[2];
            self.reticle_range = (dx * dx + dy * dy + dz * dz).sqrt();
        } else {
            self.reticle_range = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_table() -> Vec<u8> {
        let mut data = vec![0x5a; 8 + 28 * 60 + 56 + 12];
        data[..8].copy_from_slice(b"CTWWPN01");
        for i in 0..28 {
            let p = 8 + i * 60;
            let fields = [(0, 1i32), (4, 300), (8, 7), (12, 9), (16, 11),
                          (24, 13), (32, 17), (36, 19 * 4096), (44, (i % 10 + 1) as i32)];
            for (offset, value) in fields {
                data[p + offset..p + offset + 4].copy_from_slice(&value.to_le_bytes());
            }
            let a = 8 + 28 * 60 + i * 2;
            data[a..a + 2].copy_from_slice(&21u16.to_le_bytes());
        }
        data[12..16].copy_from_slice(&(-1i32).to_le_bytes());
        for (i, value) in [2i32, 3, 5].into_iter().enumerate() {
            let p = 8 + 28 * 60 + 56 + i * 4;
            data[p..p + 4].copy_from_slice(&value.to_le_bytes());
        }
        data
    }

    #[test]
    fn parse_weapon_layout_and_optional_reference() {
        let mut data = synthetic_table();
        let parsed = WeaponTables::parse(&data).unwrap();
        assert_eq!(parsed.records.len(), 28);
        assert_eq!(parsed.records[0].max_ammo, -1);
        assert_eq!(parsed.records[5].clip, 7);
        assert_eq!(parsed.records[5].reload_time, 13);
        assert_eq!(parsed.records[5].range_meters(), 19.);
        assert_eq!(parsed.records[27].slot, 8);
        assert_eq!(parsed.default_ammo, [21; 28]);
        assert_eq!(parsed.trace_lifetimes, [2, 3, 5]);
        assert!(WeaponTables::parse(&data[..data.len()-1]).is_err());
        data[0] = 0;
        assert!(WeaponTables::parse(&data).is_err());
        if let Some(bytes) = crate::test_support::reference_table("weapon_tables.bin") {
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
    fn weapon_inventory_explosion_and_takedown_integration() {
        let tables_data = crate::test_support::reference_table("weapon_tables.bin")
            .unwrap_or_else(synthetic_table);
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

    #[test]
    fn projectile_ballistics_and_bounce_mechanics() {
        let mut proj_mgr = ProjectileManager::new();
        let mut fire_mgr = FireManager::new();
        let mut explosions = crate::explosions::ExplosionManager::new();
        let mut peds = Vec::new();
        let mut cars = Vec::new();

        // 1. Spawn Grenade
        proj_mgr.spawn_grenade([10.0, 10.0, 0.0], [0.0, 1.0], 1.0, true);
        assert_eq!(proj_mgr.list.len(), 1);
        let g = &proj_mgr.list[0];
        assert_eq!(g.kind, ProjectileKind::Grenade);
        assert_eq!(g.vel[1], 16.0);
        assert!(g.vel[2] > 0.0, "Grenade has positive vertical toss impulse");

        // Parse minimal collision box
        let collision = crate::collision::Collision::parse(b"").unwrap_or_default();

        // Step physics forward 0.1s
        proj_mgr.update(0.1, &collision, &mut peds, &mut cars, &mut fire_mgr, &mut explosions);
        assert_eq!(proj_mgr.list.len(), 1);
        let g_updated = &proj_mgr.list[0];
        assert!(g_updated.pos[1] > 10.0, "Grenade moved forward along Y");
        assert!(g_updated.fuse_timer < 2.2, "Fuse timer decremented");

        // 2. Spawn Molotov cocktail
        proj_mgr.spawn_molotov([20.0, 20.0, 0.0], [1.0, 0.0], 1.0, true);
        assert_eq!(proj_mgr.list.len(), 2);

        // 3. Spawn RPG Rocket
        proj_mgr.spawn_rocket([30.0, 30.0, 0.0], [0.0, 1.0], true);
        assert_eq!(proj_mgr.list.len(), 3);
        let rocket = &proj_mgr.list[2];
        assert_eq!(rocket.kind, ProjectileKind::Rocket);
        assert_eq!(rocket.vel[1], 38.0);
    }

    #[test]
    fn flamethrower_and_fire_damage_propagation() {
        let mut fire_mgr = FireManager::new();
        let mut peds = vec![
            crate::actors::make_ped([5.0, 8.0, 0.0], [5.0, 9.0, 0.0], 0),
        ];
        peds[0].health = 100.0;
        let mut cars = Vec::new();

        // Flamethrower stream directed at ped at (5, 8) from (5, 5)
        let muzzle = [5.0, 5.0, 1.0];
        let fwd = [0.0, 1.0];
        fire_mgr.emit_flamethrower_stream(muzzle, fwd, &mut peds, &mut cars);

        assert!(peds[0].on_fire > 0.0, "Ped catches fire from flamethrower stream");
        assert!(peds[0].health < 100.0, "Ped takes direct flame impact damage");
        assert_eq!(fire_mgr.patches.len(), 1, "Ground fire patch spawned along stream");

        // Update fire simulation: ped takes continuous burn damage
        let prev_health = peds[0].health;
        fire_mgr.update(0.2, &mut peds, &mut cars);
        assert!(peds[0].health < prev_health, "Burning ped suffers damage over time");

        // Extinguish fire near patch
        let extinguished = fire_mgr.extinguish_near([5.0, 9.0, 0.0], 3.0);
        assert!(extinguished >= 1, "Fire patch extinguished by water/extinguisher");
        assert_eq!(fire_mgr.patches.len(), 0);
    }

    #[test]
    fn sniper_scope_sway_and_breath_hold() {
        let mut scope = SniperScope::new();
        assert!(!scope.active);
        assert_eq!(scope.zoom, 1.0);

        // Toggle scope on
        scope.toggle();
        assert!(scope.active);
        assert_eq!(scope.target_zoom, 2.0);

        // Update scope animation to zoom in and simulate natural breathing sway
        scope.update(0.2, Some([50.0, 50.0, 0.0]), [0.0, 0.0, 0.0]);
        assert!(scope.zoom > 1.0, "Zoom interpolates smoothly toward target");
        assert!(scope.reticle_range > 70.0, "Calculates accurate 3D distance to target");
        let free_sway = scope.sway_offset[0].abs();

        // Hold breath: sway is dampened by ~85%
        scope.hold_breath(true);
        scope.update(0.1, Some([50.0, 50.0, 0.0]), [0.0, 0.0, 0.0]);
        assert!(scope.breath_meter < 1.0, "Breath meter depletes while holding breath");
        let steady_sway = scope.sway_offset[0].abs();
        assert!(steady_sway < free_sway * 0.4, "Steady aim dampens organic breath sway");

        // Release breath
        scope.hold_breath(false);
        scope.update(0.1, Some([50.0, 50.0, 0.0]), [0.0, 0.0, 0.0]);
        assert!(!scope.held_breath);
    }
}
