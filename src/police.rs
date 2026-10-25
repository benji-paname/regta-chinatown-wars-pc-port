// SPDX-License-Identifier: GPL-3.0-only
//! Chinatown Wars PSP authentic Wanted Level and Police Pursuit System (cWantedRating, cPoliceManager, cRoadBlock, cSpikeStrip).
//! Faithful implementation of Chinatown Wars' unique takedown mechanic:
//! Disabling pursuing police cars (smashing them into buildings, obstacles, or performing PIT maneuvers)
//! crosses out takedown badges to drop the wanted level.

#![allow(dead_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrimeType {
    PedHit,
    PedKill,
    CarTheft,
    CopAttack,
    Explosion,
}

/// A police spike strip (stinger) deployed at road checkpoints to deflate car tires (cSpikeStrip).
#[derive(Clone, Debug)]
pub struct SpikeStrip {
    pub p: [f32; 3],
    pub angle: f32,
    pub length: f32,
    pub deployed: bool,
}

impl SpikeStrip {
    pub fn new(p: [f32; 3], angle: f32, length: f32) -> Self {
        Self {
            p,
            angle,
            length,
            deployed: true,
        }
    }

    /// Tests if a car's wheels or body intersects the spike strip.
    pub fn check_car_collision(&self, car: &mut crate::actors::Car) -> bool {
        if !self.deployed {
            return false;
        }
        let dx = car.p[0] - self.p[0];
        let dy = car.p[1] - self.p[1];
        let dist = dx.hypot(dy);
        if dist > self.length * 0.65 {
            return false;
        }
        // Test alignment along strip
        let local_x = dx * self.angle.cos() + dy * self.angle.sin();
        let local_y = -dx * self.angle.sin() + dy * self.angle.cos();
        if local_x.abs() < self.length * 0.5 && local_y.abs() < 1.4 {
            // Punctures tires!
            car.flat_tires = [true; 4];
            car.speed *= 0.65;
            car.sparks = 1.0;
            return true;
        }
        false
    }
}

/// A police roadblock consisting of two angled cruisers, sawhorse barriers and a spike strip (cRoadBlock).
#[derive(Clone, Debug)]
pub struct RoadBlock {
    pub id: usize,
    pub p: [f32; 3],
    pub angle: f32,
    pub cruisers: [[f32; 3]; 2],
    pub spike_strip: Option<SpikeStrip>,
    pub barrier_left: [f32; 3],
    pub barrier_right: [f32; 3],
    pub light_timer: f32,
}

impl RoadBlock {
    pub fn new(id: usize, p: [f32; 3], angle: f32, has_spikes: bool) -> Self {
        let perp = [angle.cos(), angle.sin()];
        let c1 = [p[0] - perp[0] * 3.5, p[1] - perp[1] * 3.5, p[2]];
        let c2 = [p[0] + perp[0] * 3.5, p[1] + perp[1] * 3.5, p[2]];
        let b1 = [p[0] - perp[0] * 6.0, p[1] - perp[1] * 6.0, p[2]];
        let b2 = [p[0] + perp[0] * 6.0, p[1] + perp[1] * 6.0, p[2]];

        let spike_strip = if has_spikes {
            Some(SpikeStrip::new(p, angle, 6.0))
        } else {
            None
        };

        Self {
            id,
            p,
            angle,
            cruisers: [c1, c2],
            spike_strip,
            barrier_left: b1,
            barrier_right: b2,
            light_timer: 0.0,
        }
    }
}

/// Manager for dynamic tactical police roadblocks spawned at 3+ wanted stars (cRoadBlockManager).
#[derive(Clone, Debug, Default)]
pub struct RoadBlockManager {
    pub blocks: Vec<RoadBlock>,
    pub next_id: usize,
    pub spawn_timer: f32,
}

impl RoadBlockManager {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            next_id: 1,
            spawn_timer: 0.0,
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
        stars: u8,
        player_p: [f32; 3],
        player_forward: [f32; 2],
        cars: &mut [crate::actors::Car],
    ) {
        // Animate hazard lights
        for block in &mut self.blocks {
            block.light_timer = (block.light_timer + dt * 5.0).rem_euclid(std::f32::consts::TAU);
        }

        // Check tire punctures against all active roadblocks
        for block in &self.blocks {
            if let Some(ref spike) = block.spike_strip {
                for car in cars.iter_mut() {
                    spike.check_car_collision(car);
                }
            }
        }

        // Roadblocks only spawn at wanted level 3 or higher
        if stars < 3 {
            self.blocks.clear();
            self.spawn_timer = 0.0;
            return;
        }

        self.spawn_timer += dt;
        // Clean up distant roadblocks (> 180m from player)
        self.blocks.retain(|b| {
            let dx = b.p[0] - player_p[0];
            let dy = b.p[1] - player_p[1];
            dx * dx + dy * dy < 180.0 * 180.0
        });

        // Spawn a roadblock ~65m ahead every 22 seconds if fewer than 2 active
        if self.spawn_timer >= 22.0 && self.blocks.len() < 2 {
            self.spawn_timer = 0.0;
            let block_p = [
                player_p[0] + player_forward[0] * 65.0,
                player_p[1] + player_forward[1] * 65.0,
                player_p[2],
            ];
            let block_angle = player_forward[1].atan2(player_forward[0]) + std::f32::consts::FRAC_PI_2;
            let has_spikes = stars >= 4;
            let block = RoadBlock::new(self.next_id, block_p, block_angle, has_spikes);
            self.next_id += 1;
            self.blocks.push(block);
        }
    }
}

/// Tactical motorized searchlight mounted on police helicopter (cHeliSearchLight).
#[derive(Clone, Debug)]
pub struct HeliSearchLight {
    pub beam_pos: [f32; 3],
    pub target_pos: [f32; 3],
    pub radius: f32,
    pub player_illuminated: bool,
    pub alert_flash: f32,
    pub slew_speed: f32,
}

impl Default for HeliSearchLight {
    fn default() -> Self {
        Self::new()
    }
}

impl HeliSearchLight {
    pub fn new() -> Self {
        Self {
            beam_pos: [0.0, 0.0, 0.0],
            target_pos: [0.0, 0.0, 0.0],
            radius: 8.5,
            player_illuminated: false,
            alert_flash: 0.0,
            slew_speed: 18.0,
        }
    }

    pub fn update(&mut self, dt: f32, _heli_pos: [f32; 3], player_pos: [f32; 3]) {
        self.target_pos = player_pos;
        let dx = self.target_pos[0] - self.beam_pos[0];
        let dy = self.target_pos[1] - self.beam_pos[1];
        let dist = dx.hypot(dy);
        if dist > 0.05 {
            let step = (self.slew_speed * dt).min(dist);
            self.beam_pos[0] += (dx / dist) * step;
            self.beam_pos[1] += (dy / dist) * step;
            self.beam_pos[2] = player_pos[2];
        }

        // Check if player is caught in spotlight
        let pdx = player_pos[0] - self.beam_pos[0];
        let pdy = player_pos[1] - self.beam_pos[1];
        self.player_illuminated = (pdx * pdx + pdy * pdy) < (self.radius * self.radius);
        if self.player_illuminated {
            self.alert_flash = (self.alert_flash + dt * 6.0).rem_euclid(std::f32::consts::TAU);
        } else {
            self.alert_flash = 0.0;
        }
    }
}

/// LCPD Police Patrol Helicopter tracking and pursuing player at 3+ stars (cPoliceHeli).
#[derive(Clone, Debug)]
pub struct PoliceHeli {
    pub p: [f32; 3],
    pub vel: [f32; 3],
    pub angle: f32,
    pub rotor_angle: f32,
    pub tail_rotor_angle: f32,
    pub health: f32,
    pub max_health: f32,
    pub is_crashing: bool,
    pub spin_rate: f32,
    pub searchlight: HeliSearchLight,
    pub orbit_angle: f32,
    pub orbit_radius: f32,
    pub altitude: f32,
}

impl PoliceHeli {
    pub fn new(spawn_p: [f32; 3]) -> Self {
        Self {
            p: [spawn_p[0], spawn_p[1], spawn_p[2] + 32.0],
            vel: [0.0; 3],
            angle: 0.0,
            rotor_angle: 0.0,
            tail_rotor_angle: 0.0,
            health: 450.0,
            max_health: 450.0,
            is_crashing: false,
            spin_rate: 0.0,
            searchlight: HeliSearchLight::new(),
            orbit_angle: 0.0,
            orbit_radius: 26.0,
            altitude: 32.0,
        }
    }

    pub fn apply_damage(&mut self, damage: f32) -> bool {
        self.health = (self.health - damage).max(0.0);
        if self.health <= 0.0 && !self.is_crashing {
            self.is_crashing = true;
            self.spin_rate = 14.0;
            return true;
        }
        false
    }

    pub fn update(&mut self, dt: f32, player_p: [f32; 3]) -> Option<[f32; 3]> {
        self.rotor_angle = (self.rotor_angle + dt * 48.0).rem_euclid(std::f32::consts::TAU);
        self.tail_rotor_angle = (self.tail_rotor_angle + dt * 64.0).rem_euclid(std::f32::consts::TAU);

        if self.is_crashing {
            self.angle += self.spin_rate * dt;
            self.p[2] -= 9.5 * dt;
            self.p[0] += self.vel[0] * dt;
            self.p[1] += self.vel[1] * dt;

            if self.p[2] <= player_p[2] + 0.5 {
                return Some(self.p);
            }
            return None;
        }

        self.orbit_angle = (self.orbit_angle + dt * 0.45).rem_euclid(std::f32::consts::TAU);
        let target_x = player_p[0] + self.orbit_angle.cos() * self.orbit_radius;
        let target_y = player_p[1] + self.orbit_angle.sin() * self.orbit_radius;
        let target_z = player_p[2] + self.altitude;

        let dx = target_x - self.p[0];
        let dy = target_y - self.p[1];
        let dz = target_z - self.p[2];

        self.vel[0] = dx * 1.8;
        self.vel[1] = dy * 1.8;
        self.vel[2] = dz * 1.5;

        self.p[0] += self.vel[0] * dt;
        self.p[1] += self.vel[1] * dt;
        self.p[2] += self.vel[2] * dt;

        self.angle = (self.orbit_angle + std::f32::consts::FRAC_PI_2).rem_euclid(std::f32::consts::TAU);
        self.searchlight.update(dt, self.p, player_p);

        None
    }
}

/// Global Police Helicopter Dispatch and Tactical Tracking Manager (cHelicopterManager).
#[derive(Clone, Debug, Default)]
pub struct HelicopterManager {
    pub heli: Option<PoliceHeli>,
    pub spawn_timer: f32,
    pub crash_explosion: Option<[f32; 3]>,
}

impl HelicopterManager {
    pub fn new() -> Self {
        Self {
            heli: None,
            spawn_timer: 0.0,
            crash_explosion: None,
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
        stars: u8,
        player_p: [f32; 3],
        takedowns_done: &mut u8,
        stars_val: &mut u8,
        recent_flash: &mut f32,
    ) {
        if stars < 3 {
            if let Some(ref mut h) = self.heli {
                if !h.is_crashing {
                    self.heli = None;
                }
            }
            return;
        }

        if self.heli.is_none() {
            self.spawn_timer += dt;
            if self.spawn_timer >= 6.0 {
                self.spawn_timer = 0.0;
                let spawn_p = [player_p[0] + 60.0, player_p[1] + 60.0, player_p[2]];
                self.heli = Some(PoliceHeli::new(spawn_p));
            }
        }

        if let Some(ref mut h) = self.heli {
            if let Some(impact_pos) = h.update(dt, player_p) {
                self.crash_explosion = Some(impact_pos);
                // Epic shootdown takedown: award takedown and drop star!
                *takedowns_done = takedowns_done.saturating_add(1);
                *recent_flash = 1.8;
                *stars_val = stars_val.saturating_sub(1);
                self.heli = None;
                self.spawn_timer = 0.0;
            }
        }
    }

    pub fn check_projectile_hit(&mut self, hit_pos: [f32; 3], damage: f32) -> bool {
        if let Some(ref mut h) = self.heli {
            if h.is_crashing {
                return false;
            }
            let dx = (hit_pos[0] - h.p[0]).abs();
            let dy = (hit_pos[1] - h.p[1]).abs();
            let dz = (hit_pos[2] - h.p[2]).abs();
            if dx < 4.8 && dy < 4.8 && dz < 3.8 {
                h.apply_damage(damage);
                return true;
            }
        }
        false
    }
}

#[derive(Clone, Debug)]
pub struct PoliceManager {
    pub stars: u8,
    pub takedowns_required: u8,
    pub takedowns_done: u8,
    pub crime_score: f32,
    pub evade_timer: f32,
    pub flash_timer: f32,
    pub cooldown_timer: f32,
    pub recent_takedown_flash: f32,
    pub cop_car_indices: Vec<usize>,
    pub spawn_cooldown: f32,
    pub roadblocks: RoadBlockManager,
    pub heli_manager: HelicopterManager,
}

impl Default for PoliceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PoliceManager {
    pub fn new() -> Self {
        Self {
            stars: 0,
            takedowns_required: 0,
            takedowns_done: 0,
            crime_score: 0.0,
            evade_timer: 0.0,
            flash_timer: 0.0,
            cooldown_timer: 0.0,
            recent_takedown_flash: 0.0,
            cop_car_indices: Vec::new(),
            spawn_cooldown: 0.0,
            roadblocks: RoadBlockManager::new(),
            heli_manager: HelicopterManager::new(),
        }
    }

    pub fn set_stars(&mut self, stars: u8) {
        self.stars = stars.clamp(0, 6);
        self.takedowns_required = match self.stars {
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 4,
            5 => 5,
            6 => 6,
            _ => 0,
        };
        self.takedowns_done = 0;
        self.evade_timer = 0.0;
        self.cooldown_timer = 0.0;
    }

    pub fn start_cooldown(&mut self, duration: f32) {
        if self.stars > 0 {
            self.cooldown_timer = duration;
        }
    }

    pub fn is_cooling_down(&self) -> bool {
        self.cooldown_timer > 0.0
    }

    pub fn takedown_progress(&self) -> (u8, u8, u8) {
        (self.stars, self.takedowns_done, self.takedowns_required)
    }

    pub fn report_crime(&mut self, crime: CrimeType) {
        let add = match crime {
            CrimeType::PedHit => 15.0,
            CrimeType::CarTheft => 25.0,
            CrimeType::PedKill => 50.0,
            CrimeType::CopAttack => 80.0,
            CrimeType::Explosion => 100.0,
        };
        self.crime_score += add;
        // Breaking cooldown
        self.cooldown_timer = 0.0;

        if self.stars == 0 && self.crime_score >= 35.0 {
            self.set_stars(1);
        } else if self.stars == 1 && self.crime_score >= 90.0 {
            self.set_stars(2);
        } else if self.stars == 2 && self.crime_score >= 180.0 {
            self.set_stars(3);
        } else if self.stars == 3 && self.crime_score >= 300.0 {
            self.set_stars(4);
        }
    }

    /// Chinatown Wars unique mechanic: disabling a pursuing cop car crosses out a badge.
    pub fn on_cop_takedown(&mut self) -> bool {
        if self.stars == 0 {
            return false;
        }
        self.takedowns_done += 1;
        self.recent_takedown_flash = 1.6;
        if self.takedowns_done >= self.takedowns_required {
            self.set_stars(self.stars.saturating_sub(1));
            true
        } else {
            false
        }
    }

    /// Triggers a PIT maneuver or high-speed ram takedown on a specific cop car.
    pub fn trigger_cop_ram_takedown(&mut self, cop_index: usize, cars: &mut [crate::actors::Car]) -> bool {
        if cop_index < cars.len() {
            let cop = &mut cars[cop_index];
            cop.disabled = true;
            cop.siren_active = false;
            cop.yaw_rate = 12.0; // spin out
            cop.speed *= 0.35;
            cop.sparks = 1.5;
        }
        self.on_cop_takedown()
    }

    /// Updates pursuit behavior, sirens, takedown detection, cooldown timer, and roadblocks.
    pub fn update(
        &mut self,
        dt: f32,
        player_p: [f32; 3],
        player_forward: [f32; 2],
        cars: &mut [crate::actors::Car],
    ) {
        self.flash_timer = (self.flash_timer + dt * 4.0).rem_euclid(std::f32::consts::TAU);
        self.spawn_cooldown = (self.spawn_cooldown - dt).max(0.0);
        self.recent_takedown_flash = (self.recent_takedown_flash - dt).max(0.0);

        // Handle cooldown (e.g. after Pay 'n' Spray or shaking pursuit)
        if self.cooldown_timer > 0.0 {
            self.cooldown_timer = (self.cooldown_timer - dt).max(0.0);
            if self.cooldown_timer <= 0.0 {
                // Successfully evaded! Clear all stars
                self.set_stars(0);
                self.crime_score = 0.0;
            }
        }

        // Update tactical roadblocks
        self.roadblocks.update(dt, self.stars, player_p, player_forward, cars);

        // Update tactical police helicopter and searchlight tracking
        self.heli_manager.update(
            dt,
            self.stars,
            player_p,
            &mut self.takedowns_done,
            &mut self.stars,
            &mut self.recent_takedown_flash,
        );

        if self.stars == 0 {
            for &idx in &self.cop_car_indices {
                if idx < cars.len() {
                    cars[idx].siren_active = false;
                }
            }
            self.cop_car_indices.clear();
            return;
        }

        // Check if any active cop cars crashed/destroyed
        let mut takedowns = 0;
        self.cop_car_indices.retain(|&idx| {
            if idx >= cars.len() {
                return false;
            }
            let car = &cars[idx];
            // If cop car hit obstacle at speed or was flipped/parked or disabled
            if (car.parked && car.blocked > 1.5) || car.disabled {
                takedowns += 1;
                false
            } else {
                true
            }
        });

        for _ in 0..takedowns {
            self.on_cop_takedown();
        }

        // Update pursuing cop car AI
        for &idx in &self.cop_car_indices {
            if idx >= cars.len() {
                continue;
            }
            let cop = &mut cars[idx];
            if cop.disabled {
                continue;
            }
            let dx = player_p[0] - cop.p[0];
            let dy = player_p[1] - cop.p[1];
            let dist = dx.hypot(dy);

            // Pursue player with siren lights
            cop.headlights = true;
            cop.siren_active = true;
            if dist > 3.0 && dist < 120.0 {
                let target_angle = -dx.atan2(dy);
                let diff = (target_angle - cop.angle + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                let steer = (-diff * 1.8).clamp(-1.0, 1.0);
                cop.angle += steer * 2.2 * dt;
                cop.speed = (cop.speed + 8.5 * dt).min(11.5);
                let f = [-cop.angle.sin(), cop.angle.cos()];
                cop.p[0] += f[0] * cop.speed * dt;
                cop.p[1] += f[1] * cop.speed * dt;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authentic_chinatown_wars_takedown_mechanic() {
        let mut police = PoliceManager::new();
        assert_eq!(police.stars, 0);

        // Report crimes to raise wanted level
        police.report_crime(CrimeType::PedHit);
        police.report_crime(CrimeType::PedKill);
        assert_eq!(police.stars, 1, "Reaches 1 star");
        assert_eq!(police.takedowns_required, 1, "1 takedown required for 1 star");

        // Take down 1 cop car
        let cleared = police.on_cop_takedown();
        assert!(cleared, "Clears star on 1 takedown");
        assert_eq!(police.stars, 0, "Returns to 0 stars");

        // 2 stars requires 2 takedowns
        police.set_stars(2);
        assert_eq!(police.takedowns_required, 2);
        assert!(!police.on_cop_takedown(), "First takedown does not clear 2 stars");
        assert_eq!(police.stars, 2);
        assert!(police.on_cop_takedown(), "Second takedown drops to 1 star");
        assert_eq!(police.stars, 1);
    }

    #[test]
    fn roadblock_spike_strip_and_tire_deflation() {
        let mut car = crate::actors::make_car([20.0, 20.0, 0.0], 0.0, 0, 0, 0);
        car.speed = 12.0;
        assert_eq!(car.flat_tires, [false; 4]);

        let spike = SpikeStrip::new([20.0, 20.0, 0.0], 0.0, 6.0);
        let hit = spike.check_car_collision(&mut car);
        assert!(hit, "Car strikes spike strip");
        assert_eq!(car.flat_tires, [true; 4], "All tires punctured");
        assert!(car.speed < 12.0, "Speed reduced by spike puncture");

        // Test roadblock construction
        let roadblock = RoadBlock::new(1, [100.0, 100.0, 0.0], 0.0, true);
        assert!(roadblock.spike_strip.is_some());
        assert_eq!(roadblock.cruisers.len(), 2);
    }

    #[test]
    fn cooldown_evasion_and_ram_takedown() {
        let mut police = PoliceManager::new();
        police.set_stars(2);
        assert_eq!(police.stars, 2);

        // Start cooldown evasion (e.g. from Pay 'n' Spray)
        police.start_cooldown(10.0);
        assert!(police.is_cooling_down());

        let mut cars = vec![crate::actors::make_car([0.0, 0.0, 0.0], 0.0, 0, 0, 0)];
        cars[0].siren_active = true;

        // Perform a ram takedown on cop car
        let star_dropped = police.trigger_cop_ram_takedown(0, &mut cars);
        assert!(!star_dropped, "Needs 2 takedowns for 2 stars");
        assert!(cars[0].disabled, "Cop car disabled after ram");
        assert!(!cars[0].siren_active, "Siren fizzles out");

        // Step cooldown until expiration
        police.update(10.5, [0.0, 0.0, 0.0], [0.0, 1.0], &mut cars);
        assert!(!police.is_cooling_down());
        assert_eq!(police.stars, 0, "Stars cleared when cooldown expires peacefully");
    }

    #[test]
    fn police_helicopter_searchlight_tracking_and_shootdown_takedown() {
        let mut police = PoliceManager::new();
        police.set_stars(4);
        assert_eq!(police.stars, 4);

        let mut cars = Vec::new();
        let player_p = [50.0, 50.0, 0.0];

        // Step past 6s spawn timer to deploy police chopper
        police.update(6.5, player_p, [0.0, 1.0], &mut cars);
        assert!(police.heli_manager.heli.is_some(), "Chopper dispatched at 4 stars");

        let heli = police.heli_manager.heli.as_ref().unwrap();
        assert_eq!(heli.max_health, 450.0);
        assert!(heli.p[2] >= 28.0, "Chopper maintains high altitude above street");

        // Step searchlight slewing towards player
        police.update(2.0, player_p, [0.0, 1.0], &mut cars);
        let heli_p = police.heli_manager.heli.as_ref().unwrap().p;
        assert!(police.heli_manager.heli.as_ref().unwrap().searchlight.player_illuminated, "Searchlight locks onto player position");

        // Player shoots helicopter with sniper rifle (2x 250 dmg)
        let hit1 = police.heli_manager.check_projectile_hit(heli_p, 250.0);
        assert!(hit1, "Sniper round hits helicopter");
        assert_eq!(police.heli_manager.heli.as_ref().unwrap().health, 200.0);

        // Second shot drops health to 0 -> triggers death spiral
        let hit2 = police.heli_manager.check_projectile_hit(heli_p, 250.0);
        assert!(hit2);
        assert!(police.heli_manager.heli.as_ref().unwrap().is_crashing, "Enters death spiral");

        // Step descent until ground impact
        for _ in 0..30 {
            if police.heli_manager.heli.is_none() {
                break;
            }
            police.update(0.2, player_p, [0.0, 1.0], &mut cars);
        }

        assert!(police.heli_manager.heli.is_none(), "Helicopter destroyed on crash");
        assert!(police.heli_manager.crash_explosion.is_some(), "Explosion triggered at crash site");
        assert_eq!(police.stars, 3, "Drops 1 star upon helicopter takedown");
    }
}
