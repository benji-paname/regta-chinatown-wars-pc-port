// SPDX-License-Identifier: GPL-3.0-only
//! Chinatown Wars PSP authentic Wanted Level and Police Pursuit System (cWantedRating, cPoliceManager).
//! Faithful implementation of Chinatown Wars' unique takedown mechanic:
//! Disabling pursuing police cars (smashing them into buildings, obstacles, or water)
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

#[derive(Clone, Debug)]
pub struct PoliceManager {
    pub stars: u8,
    pub takedowns_required: u8,
    pub takedowns_done: u8,
    pub crime_score: f32,
    pub evade_timer: f32,
    pub flash_timer: f32,
    pub cop_car_indices: Vec<usize>,
    pub spawn_cooldown: f32,
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
            cop_car_indices: Vec::new(),
            spawn_cooldown: 0.0,
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
        if self.takedowns_done >= self.takedowns_required {
            self.set_stars(self.stars.saturating_sub(1));
            true
        } else {
            false
        }
    }

    /// Updates pursuit behavior, sirens, and takedown detection.
    pub fn update(
        &mut self,
        dt: f32,
        player_p: [f32; 3],
        cars: &mut [crate::actors::Car],
    ) {
        self.flash_timer = (self.flash_timer + dt * 4.0).rem_euclid(std::f32::consts::TAU);
        self.spawn_cooldown = (self.spawn_cooldown - dt).max(0.0);

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
            // If cop car hit obstacle at speed or was flipped/parked
            if car.parked && car.blocked > 1.5 {
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
}
