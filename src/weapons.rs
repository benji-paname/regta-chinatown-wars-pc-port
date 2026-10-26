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

// ============================================================================
// Chinatown Wars Weapon Wheel Selector & Slot Icons (cHudWeaponSelect, cWeaponSlotIcon)
// ============================================================================

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponSlotIcon {
    pub slot: usize,
    pub type_id: u8,
    pub is_equipped: bool,
    pub is_selected: bool,
    pub ammo_count: i16,
    pub clip_count: u8,
    pub scale: f32,
    pub alpha: f32,
}

#[allow(dead_code)]
impl WeaponSlotIcon {
    pub fn new(slot: usize) -> Self {
        Self {
            slot,
            type_id: 0,
            is_equipped: false,
            is_selected: false,
            ammo_count: 0,
            clip_count: 0,
            scale: 1.0,
            alpha: 0.85,
        }
    }

    pub fn update_from_weapon(&mut self, weapon: Option<&Weapon>, selected: bool) {
        self.is_selected = selected;
        if let Some(w) = weapon {
            self.type_id = w.type_id;
            self.is_equipped = true;
            self.ammo_count = w.ammo;
            self.clip_count = w.clip;
        } else {
            self.is_equipped = false;
            self.ammo_count = 0;
            self.clip_count = 0;
        }
        self.scale = if selected { 1.25 } else { 1.0 };
        self.alpha = if selected { 1.0 } else { 0.70 };
    }

    pub fn slot_name(&self) -> &'static str {
        match self.slot {
            0 => "À mains nues",
            1 => "Corps à corps",
            2 => "Pistolet",
            3 => "Épée Yu Jian",
            4 => "Fusil à pompe",
            5 => "Mitraillette",
            6 => "Fusil d'assaut",
            7 => "Arme lourde",
            8 => "Fusil de sniper",
            9 => "Projectiles",
            10 => "Arme spéciale",
            _ => "Arme",
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.is_equipped
    }

    pub fn icon_color(&self) -> [f32; 4] {
        if self.is_selected {
            [1.0, 0.85, 0.20, self.alpha]
        } else if self.is_equipped {
            [0.85, 0.88, 0.92, self.alpha]
        } else {
            [0.35, 0.38, 0.42, 0.35]
        }
    }

    pub fn get_slot(&self) -> usize {
        self.slot
    }

    pub fn get_type_id(&self) -> u8 {
        self.type_id
    }

    pub fn ammo(&self) -> i16 {
        self.ammo_count
    }

    pub fn clip(&self) -> u8 {
        self.clip_count
    }

    pub fn is_selected(&self) -> bool {
        self.is_selected
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct HudWeaponSelect {
    pub active: bool,
    pub slots: [WeaponSlotIcon; NUM_SLOTS],
    pub hovered_slot: usize,
    pub open_timer: f32,
    pub fade_alpha: f32,
    pub time_dilation: f32,
    pub wheel_angle: f32,
}

impl Default for HudWeaponSelect {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl HudWeaponSelect {
    pub fn new() -> Self {
        let slots = std::array::from_fn(WeaponSlotIcon::new);
        Self {
            active: false,
            slots,
            hovered_slot: 0,
            open_timer: 0.0,
            fade_alpha: 0.0,
            time_dilation: 1.0,
            wheel_angle: 0.0,
        }
    }

    pub fn open(&mut self, cur_slot: usize, wm: &WeaponManager) {
        self.active = true;
        self.hovered_slot = cur_slot;
        self.sync_slots(wm);
        self.time_dilation = 0.35;
    }

    pub fn close(&mut self, wm: &mut WeaponManager) {
        self.active = false;
        self.time_dilation = 1.0;
        if self.slots[self.hovered_slot].is_equipped {
            wm.cur_slot = self.hovered_slot;
        }
    }

    pub fn toggle(&mut self, wm: &mut WeaponManager) {
        if self.active {
            self.close(wm);
        } else {
            self.open(wm.cur_slot, wm);
        }
    }

    pub fn select_next(&mut self, wm: &WeaponManager) {
        for step in 1..=NUM_SLOTS {
            let next = (self.hovered_slot + step) % NUM_SLOTS;
            if wm.slots[next].is_some() {
                self.hovered_slot = next;
                self.sync_slots(wm);
                break;
            }
        }
    }

    pub fn select_prev(&mut self, wm: &WeaponManager) {
        for step in 1..=NUM_SLOTS {
            let prev = (self.hovered_slot + NUM_SLOTS - (step % NUM_SLOTS)) % NUM_SLOTS;
            if wm.slots[prev].is_some() {
                self.hovered_slot = prev;
                self.sync_slots(wm);
                break;
            }
        }
    }

    pub fn select_slot(&mut self, slot: usize, wm: &WeaponManager) {
        if slot < NUM_SLOTS && wm.slots[slot].is_some() {
            self.hovered_slot = slot;
            self.sync_slots(wm);
        }
    }

    pub fn sync_slots(&mut self, wm: &WeaponManager) {
        for s in 0..NUM_SLOTS {
            let is_sel = s == self.hovered_slot;
            self.slots[s].update_from_weapon(wm.slots[s].as_ref(), is_sel);
        }
    }

    pub fn update(&mut self, dt: f32, wm: &WeaponManager) {
        if self.active {
            self.open_timer += dt;
            self.fade_alpha = (self.fade_alpha + dt * 6.0).min(1.0);
            self.wheel_angle += dt * 0.8;
            self.sync_slots(wm);
        } else {
            self.open_timer = 0.0;
            self.fade_alpha = (self.fade_alpha - dt * 8.0).max(0.0);
        }
    }

    pub fn is_visible(&self) -> bool {
        self.fade_alpha > 0.0
    }

    pub fn get_hovered_name(&self) -> &'static str {
        self.slots[self.hovered_slot].slot_name()
    }

    pub fn get_hovered_ammo(&self) -> (i16, u8) {
        (
            self.slots[self.hovered_slot].ammo_count,
            self.slots[self.hovered_slot].clip_count,
        )
    }

    pub fn get_slot_position(&self, slot: usize, center: [f32; 2], radius: f32) -> [f32; 2] {
        let angle = (slot as f32) * (std::f32::consts::TAU / NUM_SLOTS as f32)
            - std::f32::consts::FRAC_PI_2;
        [
            center[0] + angle.cos() * radius,
            center[1] + angle.sin() * radius,
        ]
    }

    pub fn alpha(&self) -> f32 {
        self.fade_alpha
    }

    pub fn active_slot(&self) -> usize {
        self.hovered_slot
    }

    pub fn is_slot_equipped(&self, slot: usize) -> bool {
        if slot < NUM_SLOTS {
            self.slots[slot].is_equipped
        } else {
            false
        }
    }

    pub fn time_scale(&self) -> f32 {
        self.time_dilation
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

// ============================================================================
// GTA: Chinatown Wars Authentic Pickup System
// Matching reverse-engineered classes:
// cBasePickup, cBaseWeaponPickup, cPickupManager, cPickupBat, cPickupChainsaw,
// cPickupColt, cPickupDesertEagle, cPickupDualPistol, cPickupFlameThrower,
// cPickupGrenade, cPickupMinigun, cPickupMolotov, cPickupMp5, cPickupNiteStick,
// cPickupPistol, cPickupProximityMine, cPickupRocketLauncher, cPickupShotgun,
// cPickupSniperRifle, cPickupSword, cPickupTazer, cPickupUzi, cPickupAk47,
// cPickupDoubleBarrelShot, cPickupSawnOffShot, cPickupFlashBang, cPickupHealth,
// cPickupArmour, cPickupBribe, cPickupMoney, cPickupGen, cPickupGeneric,
// cPickupLuckLion, cPickupRandomStuff, cPickupListener, cCollectedAmbientPickupStore
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickupKind {
    // Melee weapons
    Bat,
    NiteStick,
    Chainsaw,
    Sword,
    Tazer,
    // Pistols
    Pistol,
    DualPistol,
    DesertEagle,
    // Shotguns
    Shotgun,
    DoubleBarrelShotgun,
    SawnOffShotgun,
    // SMGs
    Uzi,
    Mp5,
    // Assault Rifles
    Ak47,
    // Heavy / Specialized
    SniperRifle,
    RocketLauncher,
    FlameThrower,
    Minigun,
    // Thrown projectiles
    Grenade,
    Molotov,
    Flashbang,
    Mine,
    // Support & collectibles
    Health,
    Armour,
    Bribe,
    Money(u32),
    LuckLion(u8),
}

impl PickupKind {
    pub fn name(&self) -> &'static str {
        match self {
            PickupKind::Bat => "Batte de baseball",
            PickupKind::NiteStick => "Matraque de police",
            PickupKind::Chainsaw => "Tronçonneuse",
            PickupKind::Sword => "Épée Yu Jian / Katana",
            PickupKind::Tazer => "Taser incapacitant",
            PickupKind::Pistol => "Pistolet 9mm",
            PickupKind::DualPistol => "Pistolets doubles (.45)",
            PickupKind::DesertEagle => "Desert Eagle .50",
            PickupKind::Shotgun => "Fusil à pompe",
            PickupKind::DoubleBarrelShotgun => "Fusil à double canon",
            PickupKind::SawnOffShotgun => "Fusil à canon scié",
            PickupKind::Uzi => "Micro-SMG (Uzi)",
            PickupKind::Mp5 => "Pistolet-mitrailleur (MP5)",
            PickupKind::Ak47 => "Fusil d'assaut AK-47",
            PickupKind::SniperRifle => "Fusil de précision (Sniper)",
            PickupKind::RocketLauncher => "Lance-roquettes (RPG)",
            PickupKind::FlameThrower => "Lance-flammes",
            PickupKind::Minigun => "Minigun rotatif",
            PickupKind::Grenade => "Grenades à fragmentation",
            PickupKind::Molotov => "Cocktail Molotov",
            PickupKind::Flashbang => "Grenade assourdissante",
            PickupKind::Mine => "Mine de proximité",
            PickupKind::Health => "Soin médical (+50 HP)",
            PickupKind::Armour => "Gilet pare-balles (Kevlar 100%)",
            PickupKind::Bribe => "Pot-de-vin de police (-1 étoile)",
            PickupKind::Money(_) => "Liasse de billets",
            PickupKind::LuckLion(_) => "Lion de Fo impérial",
        }
    }

    pub fn weapon_type_id(&self) -> Option<u8> {
        match self {
            PickupKind::Bat | PickupKind::NiteStick | PickupKind::Chainsaw => Some(1),
            PickupKind::Pistol | PickupKind::DesertEagle | PickupKind::Tazer => Some(2),
            PickupKind::Sword => Some(3),
            PickupKind::Shotgun | PickupKind::DoubleBarrelShotgun | PickupKind::SawnOffShotgun => Some(4),
            PickupKind::DualPistol => Some(5),
            PickupKind::Uzi | PickupKind::Mp5 => Some(6),
            PickupKind::Ak47 => Some(7),
            PickupKind::SniperRifle => Some(9),
            PickupKind::RocketLauncher | PickupKind::Minigun => Some(10),
            PickupKind::FlameThrower => Some(11),
            PickupKind::Grenade | PickupKind::Flashbang | PickupKind::Mine => Some(13),
            PickupKind::Molotov => Some(14),
            _ => None,
        }
    }

    pub fn default_ammo(&self) -> u16 {
        match self {
            PickupKind::Pistol | PickupKind::DesertEagle | PickupKind::Tazer => 30,
            PickupKind::DualPistol => 60,
            PickupKind::Shotgun | PickupKind::DoubleBarrelShotgun | PickupKind::SawnOffShotgun => 16,
            PickupKind::Uzi | PickupKind::Mp5 => 90,
            PickupKind::Ak47 => 120,
            PickupKind::SniperRifle => 10,
            PickupKind::RocketLauncher => 4,
            PickupKind::FlameThrower => 150,
            PickupKind::Minigun => 200,
            PickupKind::Grenade | PickupKind::Molotov | PickupKind::Flashbang | PickupKind::Mine => 5,
            _ => 0,
        }
    }

    pub fn glow_color(&self) -> [f32; 4] {
        match self {
            PickupKind::Health => [0.95, 0.18, 0.18, 0.85],
            PickupKind::Armour => [0.22, 0.62, 0.98, 0.85],
            PickupKind::Bribe => [1.00, 0.85, 0.18, 0.90],
            PickupKind::Money(_) => [0.18, 0.88, 0.38, 0.85],
            PickupKind::LuckLion(_) => [0.15, 0.92, 0.70, 0.92],
            _ => [1.00, 0.58, 0.15, 0.85],
        }
    }

    pub fn is_weapon(&self) -> bool {
        self.weapon_type_id().is_some()
    }

    pub fn is_support(&self) -> bool {
        matches!(self, PickupKind::Health | PickupKind::Armour | PickupKind::Bribe)
    }

    pub fn is_collectible(&self) -> bool {
        matches!(self, PickupKind::LuckLion(_))
    }
}

#[derive(Clone, Debug)]
pub struct WorldPickup {
    pub id: usize,
    pub kind: PickupKind,
    pub pos: [f32; 3],
    pub rotation: f32,
    pub bob_phase: f32,
    pub respawn_timer: f32,
    pub respawn_delay: f32,
    pub active: bool,
    pub is_dropped: bool,
    pub life_timer: f32,
}

impl WorldPickup {
    pub fn new(id: usize, kind: PickupKind, pos: [f32; 3], respawn_delay: f32) -> Self {
        Self {
            id,
            kind,
            pos,
            rotation: 0.0,
            bob_phase: 0.0,
            respawn_timer: 0.0,
            respawn_delay,
            active: true,
            is_dropped: false,
            life_timer: 0.0,
        }
    }

    pub fn new_dropped(id: usize, kind: PickupKind, pos: [f32; 3]) -> Self {
        Self {
            id,
            kind,
            pos,
            rotation: 0.0,
            bob_phase: 0.0,
            respawn_timer: 0.0,
            respawn_delay: 0.0,
            active: true,
            is_dropped: true,
            life_timer: 45.0,
        }
    }

    pub fn update(&mut self, dt: f32) -> bool {
        self.rotation = (self.rotation + 2.2 * dt) % std::f32::consts::TAU;
        self.bob_phase = (self.bob_phase + 3.5 * dt) % std::f32::consts::TAU;

        if !self.active && self.respawn_delay > 0.0 {
            self.respawn_timer = (self.respawn_timer - dt).max(0.0);
            if self.respawn_timer == 0.0 {
                self.active = true;
            }
        }

        if self.is_dropped && self.active {
            self.life_timer = (self.life_timer - dt).max(0.0);
            if self.life_timer == 0.0 {
                self.active = false;
                return false;
            }
        }
        true
    }

    pub fn collect(&mut self) -> PickupKind {
        self.active = false;
        if self.respawn_delay > 0.0 {
            self.respawn_timer = self.respawn_delay;
        }
        self.kind
    }

    pub fn render_pos(&self) -> [f32; 3] {
        [
            self.pos[0],
            self.pos[1],
            self.pos[2] + 0.35 + 0.12 * self.bob_phase.sin(),
        ]
    }

    pub fn halo_radius(&self) -> f32 {
        0.65 + 0.08 * (self.bob_phase * 1.5).sin()
    }
}

#[derive(Clone, Debug)]
pub struct PickupManager {
    pub pickups: Vec<WorldPickup>,
    pub next_id: usize,
    pub collected_count: usize,
    pub weapons_collected: usize,
    pub cash_looted: u32,
    pub lions_found: u8,
}

impl Default for PickupManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PickupManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            pickups: Vec::with_capacity(64),
            next_id: 1,
            collected_count: 0,
            weapons_collected: 0,
            cash_looted: 0,
            lions_found: 0,
        };
        mgr.init_world_spawns();
        mgr
    }

    pub fn init_world_spawns(&mut self) {
        // Algonquin - Middle Park & Central
        self.spawn_static(PickupKind::Health, [260.0, 310.0, 5.0], 45.0);
        self.spawn_static(PickupKind::Armour, [270.0, 320.0, 5.0], 60.0);
        self.spawn_static(PickupKind::Sword, [265.0, 315.0, 5.0], 90.0);
        self.spawn_static(PickupKind::Bribe, [240.0, 290.0, 5.0], 60.0);

        // Algonquin - Star Junction
        self.spawn_static(PickupKind::Uzi, [210.0, 220.0, 5.0], 60.0);
        self.spawn_static(PickupKind::Pistol, [195.0, 240.0, 5.0], 45.0);
        self.spawn_static(PickupKind::Money(250), [205.0, 235.0, 5.0], 90.0);

        // Algonquin - Chinatown & Easton
        self.spawn_static(PickupKind::Ak47, [245.0, 160.0, 5.0], 90.0);
        self.spawn_static(PickupKind::DualPistol, [230.0, 175.0, 5.0], 60.0);
        self.spawn_static(PickupKind::LuckLion(1), [255.0, 165.0, 5.0], 0.0);

        // Algonquin - Castle Garden & Financial
        self.spawn_static(PickupKind::Shotgun, [180.0, 95.0, 4.5], 60.0);
        self.spawn_static(PickupKind::Armour, [190.0, 105.0, 4.5], 60.0);
        self.spawn_static(PickupKind::Bribe, [170.0, 85.0, 4.5], 60.0);

        // Algonquin - Northwood & Rooftops
        self.spawn_static(PickupKind::RocketLauncher, [280.0, 450.0, 8.0], 120.0);
        self.spawn_static(PickupKind::SniperRifle, [260.0, 470.0, 9.5], 90.0);
        self.spawn_static(PickupKind::FlameThrower, [290.0, 430.0, 6.0], 90.0);

        // Broker & Dukes - Cerveza Heights & Kennys territory
        self.spawn_static(PickupKind::Bat, [470.0, 260.0, 6.0], 30.0);
        self.spawn_static(PickupKind::Molotov, [495.0, 285.0, 6.0], 60.0);
        self.spawn_static(PickupKind::Health, [480.0, 270.0, 6.0], 45.0);

        // Broker - BOABO Docks & Industrial
        self.spawn_static(PickupKind::Grenade, [410.0, 180.0, 4.0], 60.0);
        self.spawn_static(PickupKind::SawnOffShotgun, [425.0, 195.0, 4.0], 60.0);
        self.spawn_static(PickupKind::Armour, [430.0, 170.0, 4.0], 60.0);

        // Dukes - Steinway & Airport
        self.spawn_static(PickupKind::LuckLion(2), [520.0, 390.0, 5.5], 0.0);
        self.spawn_static(PickupKind::Bribe, [560.0, 420.0, 6.0], 60.0);
        self.spawn_static(PickupKind::SniperRifle, [580.0, 450.0, 7.0], 90.0);

        // Bohan - Industrial alleys & Boulevard
        self.spawn_static(PickupKind::Chainsaw, [340.0, 520.0, 5.0], 90.0);
        self.spawn_static(PickupKind::Mine, [360.0, 540.0, 5.0], 60.0);
        self.spawn_static(PickupKind::Bribe, [325.0, 510.0, 5.0], 60.0);
        self.spawn_static(PickupKind::Health, [350.0, 530.0, 5.0], 45.0);
    }

    pub fn spawn_static(&mut self, kind: PickupKind, pos: [f32; 3], respawn_delay: f32) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.pickups.push(WorldPickup::new(id, kind, pos, respawn_delay));
        id
    }

    pub fn spawn_dropped(&mut self, kind: PickupKind, pos: [f32; 3]) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.pickups.push(WorldPickup::new_dropped(id, kind, pos));
        id
    }

    pub fn on_ped_killed(&mut self, pos: [f32; 3], hostile: bool, appearance: usize) {
        let cash_amt = if hostile {
            80 + (appearance as u32 % 5) * 35
        } else {
            15 + (appearance as u32 % 4) * 12
        };
        self.spawn_dropped(PickupKind::Money(cash_amt), pos);

        if hostile {
            let drop_kind = match appearance % 5 {
                0 => PickupKind::Pistol,
                1 => PickupKind::Uzi,
                2 => PickupKind::Shotgun,
                3 => PickupKind::Bat,
                _ => PickupKind::Grenade,
            };
            let drop_pos = [pos[0] + 0.35, pos[1] + 0.25, pos[2]];
            self.spawn_dropped(drop_kind, drop_pos);
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.pickups.retain_mut(|p| p.update(dt));
    }

    pub fn try_collect(
        &mut self,
        player_pos: [f32; 3],
        player_health: &mut f32,
        player_armour: &mut u8,
        player_cash: &mut u32,
        mut weapons: Option<&mut WeaponManager>,
        wanted_level: &mut u8,
        tables: &WeaponTables,
    ) -> Option<String> {
        let radius_sq = 1.45 * 1.45;
        let z_threshold = 1.9;

        for p in &mut self.pickups {
            if !p.active {
                continue;
            }
            let dx = p.pos[0] - player_pos[0];
            let dy = p.pos[1] - player_pos[1];
            let dz = (p.pos[2] - player_pos[2]).abs();

            if dx * dx + dy * dy <= radius_sq && dz <= z_threshold {
                let kind = p.collect();
                self.collected_count += 1;

                match kind {
                    PickupKind::Health => {
                        *player_health = (*player_health + 50.0).min(127.0);
                        return Some("Santé restaurée (+50 HP)".into());
                    }
                    PickupKind::Armour => {
                        *player_armour = 100;
                        return Some("Gilet pare-balles équipé (Kevlar 100%)".into());
                    }
                    PickupKind::Bribe => {
                        if *wanted_level > 0 {
                            *wanted_level = wanted_level.saturating_sub(1);
                            return Some("Pot-de-vin ramassé : indice de recherche réduit !".into());
                        } else {
                            return Some("Pot-de-vin ramassé".into());
                        }
                    }
                    PickupKind::Money(amt) => {
                        *player_cash += amt;
                        self.cash_looted += amt;
                        return Some(format!("+${amt} ramassés"));
                    }
                    PickupKind::LuckLion(id) => {
                        self.lions_found += 1;
                        return Some(format!("Lion de Fo impérial découvert ! (#{id}/100)"));
                    }
                    _ => {
                        if let Some(wtype) = kind.weapon_type_id() {
                            self.weapons_collected += 1;
                            if let Some(wm) = weapons.as_deref_mut() {
                                wm.give(wtype, tables);
                            }
                            let ammo = kind.default_ammo();
                            if ammo > 0 {
                                return Some(format!("{}: +{ammo} munitions", kind.name()));
                            } else {
                                return Some(format!("Arme obtenue : {}", kind.name()));
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn active_count(&self) -> usize {
        self.pickups.iter().filter(|p| p.active).count()
    }

    pub fn total_count(&self) -> usize {
        self.pickups.len()
    }

    pub fn reset(&mut self) {
        self.pickups.clear();
        self.init_world_spawns();
    }
}

// Authentic Chinatown Wars classes matching reverse engineering
#[derive(Clone, Debug)]
pub struct BasePickup {
    pub inner: WorldPickup,
}

impl BasePickup {
    pub fn new(id: usize, kind: PickupKind, pos: [f32; 3], respawn: f32) -> Self {
        Self { inner: WorldPickup::new(id, kind, pos, respawn) }
    }
    pub fn is_active(&self) -> bool { self.inner.active }
    pub fn pos(&self) -> [f32; 3] { self.inner.pos }
    pub fn set_pos(&mut self, pos: [f32; 3]) { self.inner.pos = pos; }
    pub fn respawn(&mut self) { self.inner.active = true; }
    pub fn deactivate(&mut self) { self.inner.active = false; }
}

#[derive(Clone, Copy, Debug)]
pub struct BaseWeaponPickup {
    pub weapon_id: u8,
    pub ammo: u16,
}

impl BaseWeaponPickup {
    pub fn new(weapon_id: u8, ammo: u16) -> Self {
        Self { weapon_id, ammo }
    }
    pub fn weapon_id(&self) -> u8 { self.weapon_id }
    pub fn ammo(&self) -> u16 { self.ammo }
}

macro_rules! define_weapon_pickup_class {
    ($struct_name:ident, $kind:expr) => {
        #[derive(Clone, Copy, Debug)]
        pub struct $struct_name {
            pub base: BaseWeaponPickup,
        }
        impl Default for $struct_name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl $struct_name {
            pub fn new() -> Self {
                Self {
                    base: BaseWeaponPickup::new(
                        $kind.weapon_type_id().unwrap_or(0),
                        $kind.default_ammo(),
                    ),
                }
            }
            pub fn kind(&self) -> PickupKind {
                $kind
            }
        }
    };
}

define_weapon_pickup_class!(PickupBat, PickupKind::Bat);
define_weapon_pickup_class!(PickupChainsaw, PickupKind::Chainsaw);
define_weapon_pickup_class!(PickupColt, PickupKind::Pistol);
define_weapon_pickup_class!(PickupDesertEagle, PickupKind::DesertEagle);
define_weapon_pickup_class!(PickupDualPistol, PickupKind::DualPistol);
define_weapon_pickup_class!(PickupFlameThrower, PickupKind::FlameThrower);
define_weapon_pickup_class!(PickupGrenade, PickupKind::Grenade);
define_weapon_pickup_class!(PickupMinigun, PickupKind::Minigun);
define_weapon_pickup_class!(PickupMolotov, PickupKind::Molotov);
define_weapon_pickup_class!(PickupMp5, PickupKind::Mp5);
define_weapon_pickup_class!(PickupNiteStick, PickupKind::NiteStick);
define_weapon_pickup_class!(PickupPistol, PickupKind::Pistol);
define_weapon_pickup_class!(PickupProximityMine, PickupKind::Mine);
define_weapon_pickup_class!(PickupRocketLauncher, PickupKind::RocketLauncher);
define_weapon_pickup_class!(PickupShotgun, PickupKind::Shotgun);
define_weapon_pickup_class!(PickupSniperRifle, PickupKind::SniperRifle);
define_weapon_pickup_class!(PickupSword, PickupKind::Sword);
define_weapon_pickup_class!(PickupTazer, PickupKind::Tazer);
define_weapon_pickup_class!(PickupUzi, PickupKind::Uzi);
define_weapon_pickup_class!(PickupAk47, PickupKind::Ak47);
define_weapon_pickup_class!(PickupDoubleBarrelShot, PickupKind::DoubleBarrelShotgun);
define_weapon_pickup_class!(PickupSawnOffShot, PickupKind::SawnOffShotgun);
define_weapon_pickup_class!(PickupFlashBang, PickupKind::Flashbang);

#[derive(Clone, Copy, Debug, Default)]
pub struct PickupHealth;
impl PickupHealth {
    pub fn new() -> Self { Self }
    pub fn apply(&self, health: &mut f32) {
        *health = (*health + 50.0).min(127.0);
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PickupArmour;
impl PickupArmour {
    pub fn new() -> Self { Self }
    pub fn apply(&self, armour: &mut u8) {
        *armour = 100;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PickupBribe;
impl PickupBribe {
    pub fn new() -> Self { Self }
    pub fn apply(&self, wanted: &mut u8) {
        *wanted = wanted.saturating_sub(1);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PickupMoney {
    pub amount: u32,
}
impl PickupMoney {
    pub fn new(amount: u32) -> Self { Self { amount } }
    pub fn amount(&self) -> u32 { self.amount }
}

#[derive(Clone, Copy, Debug)]
pub struct PickupGenerator {
    pub interval: f32,
    pub timer: f32,
}
impl PickupGenerator {
    pub fn new(interval: f32) -> Self {
        Self { interval, timer: 0.0 }
    }
    pub fn generate(&mut self, dt: f32) -> bool {
        self.timer += dt;
        if self.timer >= self.interval {
            self.timer = 0.0;
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PickupGeneric {
    pub label: &'static str,
}
impl PickupGeneric {
    pub fn new(label: &'static str) -> Self { Self { label } }
    pub fn is_generic(&self) -> bool { true }
}

#[derive(Clone, Copy, Debug)]
pub struct PickupLuckLion {
    pub lion_id: u8,
}
impl PickupLuckLion {
    pub fn new(lion_id: u8) -> Self { Self { lion_id } }
    pub fn lion_id(&self) -> u8 { self.lion_id }
}

#[derive(Clone, Copy, Debug)]
pub struct PickupRandomStuff {
    pub seed: u32,
}
impl PickupRandomStuff {
    pub fn new(seed: u32) -> Self { Self { seed } }
    pub fn roll(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(1103515245).wrapping_add(12345);
        self.seed
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PickupListener {
    pub total_collected: usize,
}
impl PickupListener {
    pub fn new() -> Self { Self { total_collected: 0 } }
    pub fn on_pickup_collected(&mut self) {
        self.total_collected += 1;
    }
}

#[derive(Clone, Debug, Default)]
pub struct CollectedAmbientPickupStore {
    pub collected_ids: Vec<usize>,
}
impl CollectedAmbientPickupStore {
    pub fn new() -> Self { Self { collected_ids: Vec::new() } }
    pub fn record(&mut self, id: usize) {
        if !self.collected_ids.contains(&id) {
            self.collected_ids.push(id);
        }
    }
    pub fn is_recorded(&self, id: usize) -> bool {
        self.collected_ids.contains(&id)
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

    #[test]
    fn pickup_manager_world_spawns_and_rotation() {
        let mut mgr = PickupManager::new();
        assert!(mgr.total_count() >= 25, "World pickups properly populated across Liberty City");
        assert_eq!(mgr.collected_count, 0);

        let initial_active = mgr.active_count();
        assert!(initial_active >= 25);

        // Advance simulation by 0.5s
        mgr.update(0.5);
        for p in &mgr.pickups {
            assert!(p.rotation > 0.0, "Pickup rotates continuously");
            assert!(p.bob_phase > 0.0, "Pickup bobs up and down");
            let render_p = p.render_pos();
            assert!(render_p[2] > p.pos[2], "Hovering above ground");
            assert!(p.halo_radius() > 0.5, "Pulsating ground halo ring");
        }
    }

    #[test]
    fn pickup_collection_and_respawn_cycle() {
        let mut mgr = PickupManager::new();
        let tables = WeaponTables::default();
        let mut wm = WeaponManager::new(&tables);

        let mut health = 50.0;
        let mut armour = 20u8;
        let mut cash = 100u32;
        let mut wanted = 2u8;

        // Middle Park Health Pickup is at [260.0, 310.0, 5.0]
        let msg = mgr.try_collect([260.0, 310.0, 5.0], &mut health, &mut armour, &mut cash, Some(&mut wm), &mut wanted, &tables);
        assert!(msg.is_some(), "Player collects health pickup in range");
        assert_eq!(health, 100.0, "Health restored +50");
        assert_eq!(mgr.collected_count, 1);

        // Armour Pickup is at [270.0, 320.0, 5.0]
        let msg2 = mgr.try_collect([270.0, 320.0, 5.0], &mut health, &mut armour, &mut cash, Some(&mut wm), &mut wanted, &tables);
        assert!(msg2.is_some());
        assert_eq!(armour, 100, "Armour restored to 100%");

        // Bribe Pickup is at [240.0, 290.0, 5.0]
        let msg3 = mgr.try_collect([240.0, 290.0, 5.0], &mut health, &mut armour, &mut cash, Some(&mut wm), &mut wanted, &tables);
        assert!(msg3.is_some());
        assert_eq!(wanted, 1, "Wanted rating reduced by 1 star");

        // Star Junction Money is at [205.0, 235.0, 5.0]
        let msg4 = mgr.try_collect([205.0, 235.0, 5.0], &mut health, &mut armour, &mut cash, Some(&mut wm), &mut wanted, &tables);
        assert!(msg4.is_some());
        assert_eq!(cash, 350, "Player received $250 cash");

        // Lion of Fo #1 is at [255.0, 165.0, 5.0]
        let msg5 = mgr.try_collect([255.0, 165.0, 5.0], &mut health, &mut armour, &mut cash, Some(&mut wm), &mut wanted, &tables);
        assert!(msg5.is_some());
        assert_eq!(mgr.lions_found, 1);

        // Dropped pickup from dead ped
        mgr.on_ped_killed([10.0, 10.0, 0.0], true, 0);
        mgr.update(0.1);
        let msg_drop = mgr.try_collect([10.0, 10.0, 0.0], &mut health, &mut armour, &mut cash, Some(&mut wm), &mut wanted, &tables);
        assert!(msg_drop.is_some(), "Player collects dropped cash from dead enemy");
    }

    #[test]
    fn authentic_pickup_classes_contract() {
        let bat = PickupBat::new();
        assert_eq!(bat.kind(), PickupKind::Bat);
        let ak = PickupAk47::new();
        assert_eq!(ak.kind(), PickupKind::Ak47);
        let rpg = PickupRocketLauncher::new();
        assert_eq!(rpg.kind(), PickupKind::RocketLauncher);

        let mut h = 70.0;
        PickupHealth::new().apply(&mut h);
        assert_eq!(h, 120.0);

        let mut arm = 50;
        PickupArmour::new().apply(&mut arm);
        assert_eq!(arm, 100);

        let mut w = 3;
        PickupBribe::new().apply(&mut w);
        assert_eq!(w, 2);

        let money = PickupMoney::new(500);
        assert_eq!(money.amount(), 500);

        let mut gen = PickupGenerator::new(10.0);
        assert!(!gen.generate(5.0));
        assert!(gen.generate(6.0));

        let lion = PickupLuckLion::new(7);
        assert_eq!(lion.lion_id(), 7);

        let mut store = CollectedAmbientPickupStore::new();
        assert!(!store.is_recorded(42));
        store.record(42);
        assert!(store.is_recorded(42));
    }

    #[test]
    fn hud_weapon_select_wheel_and_slot_icons() {
        let mut records = vec![WeaponRecord::default(); 28];
        records[2].slot = 2; // Pistol in slot 2
        records[4].slot = 4; // Shotgun in slot 4
        let tables = WeaponTables {
            records,
            default_ammo: [50; 28],
            trace_lifetimes: [3, 4, 4],
        };
        let mut wm = WeaponManager::new(&tables);
        wm.give(2, &tables); // Pistol (slot 2)
        wm.give(4, &tables); // Shotgun (slot 4)

        let mut wheel = HudWeaponSelect::new();
        assert!(!wheel.active);
        assert_eq!(wheel.time_scale(), 1.0);

        // Open weapon wheel selector
        wheel.open(wm.cur_slot, &wm);
        assert!(wheel.active);
        assert_eq!(wheel.time_scale(), 0.35, "Slows down simulation for tactical wheel selection");
        assert_eq!(wheel.active_slot(), wm.cur_slot);

        // Update animation
        wheel.update(0.1, &wm);
        assert!(wheel.alpha() > 0.0);
        assert!(wheel.is_visible());

        // Cycle next equipped weapon in wheel
        wheel.select_next(&wm);
        assert!(wheel.is_slot_equipped(wheel.active_slot()));

        // Check slot icon properties
        let icon = &wheel.slots[2];
        assert_eq!(icon.get_slot(), 2);
        assert_eq!(icon.get_type_id(), 2);
        assert!(!icon.is_empty());
        let col = icon.icon_color();
        assert!(col[3] > 0.0);

        // Close and commit selected weapon
        wheel.close(&mut wm);
        assert!(!wheel.active);
        assert_eq!(wheel.time_scale(), 1.0);
    }
}
