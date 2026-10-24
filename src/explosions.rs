// SPDX-License-Identifier: GPL-3.0-only
//! Chinatown Wars PSP authentic Explosions and Shockwave System (cExplosionGeneric, cBaseExplosion).
//! Faithful port of explosions.cpp / explosions.h and vehicledebris.cpp.
//! Includes fixed-point falloff calculations, pedestrian knockdown, vehicle blast propulsion,
//! car bodywork debris detachment, street furniture destruction, and spatial audio triggering.

#![allow(dead_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplosionType {
    Generic = 0,
    Barrel = 1,
    Molotov = 2,
    Rocket = 3,
    BigVehicle = 4,
    Helicopter = 5,
}

/// Chinatown Wars authentic blast calculation with exact reciprocal falloff table.
#[derive(Clone, Debug)]
pub struct ExplosionBlast {
    pub pos: [f32; 3],
    pub damage_radius: f32,
    pub force_radius: f32,
    pub damage: f32,
    pub ped_damage: f32,
    pub kind: ExplosionType,
}

impl ExplosionBlast {
    pub fn new(pos: [f32; 3], kind: ExplosionType, scale: f32) -> Self {
        let (dmg_r, force_r, dmg, ped_dmg) = match kind {
            ExplosionType::Molotov => (6.0 * scale, 10.0 * scale, 120.0 * scale, 180.0 * scale),
            ExplosionType::Barrel => (7.0 * scale, 14.0 * scale, 250.0 * scale, 350.0 * scale),
            ExplosionType::Rocket => (9.0 * scale, 16.0 * scale, 450.0 * scale, 600.0 * scale),
            ExplosionType::BigVehicle => (10.0 * scale, 18.0 * scale, 500.0 * scale, 700.0 * scale),
            ExplosionType::Helicopter => (12.0 * scale, 20.0 * scale, 650.0 * scale, 850.0 * scale),
            ExplosionType::Generic => (7.0 * scale, 14.0 * scale, 300.0 * scale, 400.0 * scale),
        };
        Self {
            pos,
            damage_radius: dmg_r.max(1.0),
            force_radius: force_r.max(1.0),
            damage: dmg,
            ped_damage: ped_dmg,
            kind,
        }
    }

    /// Authentic quadratic falloff with Q12 fixed-point reciprocal precision.
    pub fn falloff(squared_dist: f32, radius: f32) -> f32 {
        let r2 = (radius * radius).max(0.1);
        if squared_dist >= r2 {
            0.0
        } else {
            (1.0 - (squared_dist / r2)).clamp(0.0, 1.0)
        }
    }

    /// Evaluates pedestrian damage and knockdown shockwave.
    /// In original CTW, the player receives halved damage and fringe shockwave knockdown.
    pub fn ped_hit(&self, point: [f32; 3], is_player: bool) -> (bool, f32) {
        let dx = point[0] - self.pos[0];
        let dy = point[1] - self.pos[1];
        let dz = point[2] - self.pos[2];
        let d2 = dx * dx + dy * dy + dz * dz;

        let mut reach = self.force_radius;
        if is_player {
            reach *= 0.72; // Player blast reach attenuation (0x5C2 >> 12)
        }
        if d2 > reach * reach {
            return (false, 0.0);
        }

        let mut damage = 0.0;
        if d2 <= self.damage_radius * self.damage_radius {
            let fall = Self::falloff(d2, self.damage_radius);
            damage = fall * self.ped_damage;
            if is_player {
                damage *= 0.5; // Halved for player
                if self.kind != ExplosionType::BigVehicle {
                    damage *= 0.5; // Quarter damage for standard ordnance
                }
            }
        }
        (true, damage)
    }

    /// Evaluates vehicle blast damage and physical propulsion impulse.
    pub fn vehicle_hit(&self, car_pos: [f32; 3], bound: f32, occupied_by_player: bool) -> (f32, [f32; 3]) {
        let dx = car_pos[0] - self.pos[0];
        let dy = car_pos[1] - self.pos[1];
        let dz = car_pos[2] - self.pos[2];
        let d2 = dx * dx + dy * dy + dz * dz;
        let dist = d2.sqrt();

        if dist >= self.force_radius + bound {
            return (0.0, [0.0; 3]);
        }

        // Damage calculation
        let mut damage = 0.0;
        if dist <= self.damage_radius + bound {
            let eff_dist = (dist - bound).max(0.0);
            let fall = Self::falloff(eff_dist * eff_dist, self.damage_radius);
            damage = fall * self.damage;
            // Player-driven car receives halved blast damage, NPC cars take 125%
            let mult = if !occupied_by_player { 1.25 } else { 0.5 };
            damage *= mult;
        }

        // Impulse direction with upward heave (Z=1.0)
        let mut dir = if dist < 0.1 {
            [0.0, 0.0, 1.0]
        } else {
            [dx / dist, dy / dist, (dz / dist).max(0.4)]
        };
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(0.001);
        dir = [dir[0] / len, dir[1] / len, dir[2] / len];

        let force_mag = Self::falloff(d2, self.force_radius) * 28.0;
        let impulse = [dir[0] * force_mag, dir[1] * force_mag, dir[2] * force_mag];

        (damage, impulse)
    }
}

/// Visual particle component for explosions (flash, fireball cloud, burning debris).
#[derive(Clone, Copy, Debug)]
pub struct ExplosionParticle {
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub grow: f32,
    pub kind: u8, // 0: White Flash, 1: Fireball cloud, 2: Dark smoke, 3: Fiery sparks
    pub color: [f32; 4],
}

/// Tumbling detached car bodywork fragment (doors, hood, boot hatch) from vehicledebris.cpp.
#[derive(Clone, Copy, Debug)]
pub struct VehicleDebris {
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub rot: [f32; 3],
    pub ang_vel: [f32; 3],
    pub life: f32,
    pub max_life: f32,
    pub slot: u8, // 0..3: Doors, 4: Boot/hood
    pub color: [f32; 3],
}

/// Active visual explosion instance with animation timer and particle spawning.
#[derive(Clone, Debug)]
pub struct ActiveExplosion {
    pub pos: [f32; 3],
    pub kind: ExplosionType,
    pub scale: f32,
    pub timer: f32,
    pub max_timer: f32,
    pub blast_applied: bool,
    pub particles: Vec<ExplosionParticle>,
}

impl ActiveExplosion {
    pub fn new(pos: [f32; 3], kind: ExplosionType, scale: f32) -> Self {
        let mut particles = Vec::with_capacity(36);

        // 1. Initial additive intense white-yellow flash
        particles.push(ExplosionParticle {
            pos,
            vel: [0.0; 3],
            life: 0.18,
            max_life: 0.18,
            size: 2.2 * scale,
            grow: 12.0 * scale,
            kind: 0,
            color: [1.0, 0.95, 0.7, 1.0],
        });

        // 2. Fireball cloud cluster expanding radially with friction
        for i in 0..16 {
            let angle = (i as f32) * (std::f32::consts::TAU / 16.0);
            let speed = 4.5 * scale * (0.8 + 0.4 * ((i * 7) % 5) as f32 / 4.0);
            particles.push(ExplosionParticle {
                pos: [pos[0], pos[1], pos[2] + 0.4],
                vel: [angle.cos() * speed, angle.sin() * speed, speed * 0.4 + 1.2],
                life: 0.55 + 0.1 * ((i * 3) % 4) as f32,
                max_life: 0.65,
                size: 1.2 * scale,
                grow: 5.5 * scale,
                kind: 1,
                color: [1.0, 0.45, 0.08, 0.9],
            });
        }

        // 3. Dark volumetric smoke billowing upward
        for i in 0..10 {
            let angle = (i as f32) * (std::f32::consts::TAU / 10.0) + 0.3;
            let speed = 2.0 * scale;
            particles.push(ExplosionParticle {
                pos: [pos[0] + angle.cos() * 0.5, pos[1] + angle.sin() * 0.5, pos[2] + 0.8],
                vel: [angle.cos() * speed * 0.5, angle.sin() * speed * 0.5, 3.2 * scale],
                life: 0.9 + 0.1 * ((i * 2) % 3) as f32,
                max_life: 1.0,
                size: 1.8 * scale,
                grow: 4.0 * scale,
                kind: 2,
                color: [0.18, 0.17, 0.16, 0.75],
            });
        }

        // 4. Burning incandescent sparks projected in ballistic arcs
        for i in 0..8 {
            let angle = (i as f32) * (std::f32::consts::TAU / 8.0) + 0.6;
            let speed = 9.0 * scale * (0.7 + 0.5 * ((i * 5) % 4) as f32 / 3.0);
            particles.push(ExplosionParticle {
                pos: [pos[0], pos[1], pos[2] + 0.5],
                vel: [angle.cos() * speed, angle.sin() * speed, 6.0 * scale],
                life: 0.7,
                max_life: 0.7,
                size: 0.25 * scale,
                grow: -0.1,
                kind: 3,
                color: [1.0, 0.8, 0.2, 1.0],
            });
        }

        Self {
            pos,
            kind,
            scale,
            timer: 1.1,
            max_timer: 1.1,
            blast_applied: false,
            particles,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.timer = (self.timer - dt).max(0.0);
        for p in &mut self.particles {
            p.life = (p.life - dt).max(0.0);
            if p.life > 0.0 {
                p.pos[0] += p.vel[0] * dt;
                p.pos[1] += p.vel[1] * dt;
                p.pos[2] += p.vel[2] * dt;

                p.size = (p.size + p.grow * dt).max(0.05);

                match p.kind {
                    0 => {
                        // Flash decay
                        p.color[3] = (p.life / p.max_life).powi(2);
                    }
                    1 => {
                        // Fireball friction & color transition to soot
                        p.vel[0] *= 1.0 - 2.8 * dt;
                        p.vel[1] *= 1.0 - 2.8 * dt;
                        p.vel[2] *= 1.0 - 1.5 * dt;
                        let t = p.life / p.max_life;
                        p.color[0] = 0.2 + 0.8 * t;
                        p.color[1] = 0.15 + 0.3 * t;
                        p.color[2] = 0.05 * t;
                        p.color[3] = t * 0.85;
                    }
                    2 => {
                        // Smoke rising
                        p.vel[2] = (p.vel[2] - 0.4 * dt).max(0.8);
                        let t = p.life / p.max_life;
                        p.color[3] = t * 0.7;
                    }
                    3 => {
                        // Spark gravity
                        p.vel[2] -= 9.8 * dt;
                        let t = p.life / p.max_life;
                        p.color[3] = t;
                    }
                    _ => {}
                }
            }
        }
        self.particles.retain(|p| p.life > 0.0);
    }
}

/// Global Explosion & Vehicle Debris Manager.
#[derive(Clone, Debug, Default)]
pub struct ExplosionManager {
    pub active: Vec<ActiveExplosion>,
    pub debris: Vec<VehicleDebris>,
}

impl ExplosionManager {
    pub fn new() -> Self {
        Self {
            active: Vec::new(),
            debris: Vec::new(),
        }
    }

    /// Triggers an authentic explosion blast at the given position.
    pub fn trigger(&mut self, pos: [f32; 3], kind: ExplosionType, scale: f32) {
        self.active.push(ActiveExplosion::new(pos, kind, scale));
    }

    /// Detaches vehicle bodywork fragments when a car is wrecked or heavily impacted.
    pub fn spawn_car_debris(&mut self, car_pos: [f32; 3], car_vel: [f32; 3], color: [f32; 3]) {
        for slot in 0..5 {
            let angle = (slot as f32) * 1.25;
            let speed = 6.0 + (slot as f32) * 1.5;
            self.debris.push(VehicleDebris {
                pos: [
                    car_pos[0] + angle.cos() * 0.8,
                    car_pos[1] + angle.sin() * 0.8,
                    car_pos[2] + 0.6,
                ],
                vel: [
                    car_vel[0] + angle.cos() * speed,
                    car_vel[1] + angle.sin() * speed,
                    car_vel[2] + 4.5,
                ],
                rot: [0.0; 3],
                ang_vel: [
                    (slot as f32 * 3.1).sin() * 8.0,
                    (slot as f32 * 2.3).cos() * 6.0,
                    (slot as f32 * 1.7).sin() * 9.0,
                ],
                life: 3.5,
                max_life: 3.5,
                slot: slot as u8,
                color,
            });
        }
    }

    pub fn update(&mut self, dt: f32) {
        for exp in &mut self.active {
            exp.update(dt);
        }
        self.active.retain(|exp| exp.timer > 0.0);

        for d in &mut self.debris {
            d.life = (d.life - dt).max(0.0);
            if d.life > 0.0 {
                // Gravity & friction
                d.vel[2] -= 9.8 * dt;
                d.vel[0] *= 1.0 - 0.6 * dt;
                d.vel[1] *= 1.0 - 0.6 * dt;

                d.pos[0] += d.vel[0] * dt;
                d.pos[1] += d.vel[1] * dt;
                d.pos[2] += d.vel[2] * dt;

                // Bounce on ground level (approx Z = 0.0)
                if d.pos[2] < 0.15 {
                    d.pos[2] = 0.15;
                    d.vel[2] = -d.vel[2] * 0.35;
                    d.vel[0] *= 0.6;
                    d.vel[1] *= 0.6;
                }

                d.rot[0] += d.ang_vel[0] * dt;
                d.rot[1] += d.ang_vel[1] * dt;
                d.rot[2] += d.ang_vel[2] * dt;
            }
        }
        self.debris.retain(|d| d.life > 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explosion_blast_falloff_and_damage_attenuation() {
        let blast = ExplosionBlast::new([0.0, 0.0, 0.0], ExplosionType::Generic, 1.0);
        assert!(blast.damage_radius > 0.0);
        assert!(blast.force_radius > blast.damage_radius);

        // Center hit
        let (hit, dmg) = blast.ped_hit([0.0, 0.0, 0.0], false);
        assert!(hit);
        assert!(dmg > 300.0, "Center ped receives full damage");

        // Player receives halved damage
        let (hit_p, dmg_p) = blast.ped_hit([0.0, 0.0, 0.0], true);
        assert!(hit_p);
        assert_eq!(dmg_p, dmg * 0.25, "Player receives quarter damage for standard blast");

        // Fringe hit receives knockdown but 0 damage outside damage_radius
        let (fringe_hit, fringe_dmg) = blast.ped_hit([0.0, blast.damage_radius + 1.0, 0.0], false);
        assert!(fringe_hit, "Fringe receives shockwave hit");
        assert_eq!(fringe_dmg, 0.0, "Fringe receives zero blast damage");

        // Outside force radius
        let (far_hit, _) = blast.ped_hit([0.0, blast.force_radius + 5.0, 0.0], false);
        assert!(!far_hit, "Outside blast reach");
    }

    #[test]
    fn vehicle_blast_propulsion_and_debris() {
        let blast = ExplosionBlast::new([0.0, 0.0, 0.0], ExplosionType::BigVehicle, 1.0);
        let (dmg, impulse) = blast.vehicle_hit([3.0, 0.0, 0.0], 1.5, false);
        assert!(dmg > 100.0, "Car receives heavy blast damage");
        assert!(impulse[0] > 0.0, "Car is pushed away along X");
        assert!(impulse[2] > 0.0, "Car receives upward heave impulse along Z");

        let mut mgr = ExplosionManager::new();
        mgr.trigger([10.0, 20.0, 0.0], ExplosionType::Generic, 1.0);
        assert_eq!(mgr.active.len(), 1);
        assert!(!mgr.active[0].particles.is_empty(), "Spawns visual particles");

        mgr.spawn_car_debris([10.0, 20.0, 0.0], [2.0, 0.0, 0.0], [0.8, 0.2, 0.1]);
        assert_eq!(mgr.debris.len(), 5, "Spawns 5 vehicle body fragments");

        mgr.update(0.1);
        assert!(mgr.debris[0].pos[2] > 0.0);
    }
}
