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
}
