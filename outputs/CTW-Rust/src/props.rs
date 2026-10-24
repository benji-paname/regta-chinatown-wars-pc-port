// SPDX-License-Identifier: GPL-3.0-only
//! Chinatown Wars PSP street furniture and dynamic props (cDynamicPropManager,
//! PropLibrary, cStreetFurniture, and cPropDynamics).
//! Exact binary structures from PAK resources 1866 (prop definitions) and
//! 1867 (prop kinds and physics parameters).
use crate::{
    actors::{forward, Particle},
    collision::Collision,
    iso::{u16le, u32le, Disc, Result},
    pak::Pak,
};

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum PropShape {
    Box { offset: [f32; 3], half: [f32; 3] },
    Cylinder { offset: [f32; 3], radius: f32, height: f32 },
    Sphere { offset: [f32; 3], radius: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PropKind {
    pub hit_eff: u8,
    pub smash_eff: u8,
    pub health: u8,
    pub flags: u8,
    pub mass: f32,
    pub smash_force: f32,
    pub uproot_force: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropDef {
    pub model: u16,
    pub broken: u16,
    pub kind: i8,
    pub shapes: Vec<PropShape>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropLibrary {
    pub defs: Vec<PropDef>,
    pub kinds: Vec<PropKind>,
}

impl PropLibrary {
    pub fn load(disc: &mut Disc, pak: &Pak) -> Result<Self> {
        let d1867 = pak.read(disc, 1867)?.0;
        let mut kinds = Vec::new();
        let mut at = 0;
        while at + 16 <= d1867.len() {
            let end = u16le(&d1867, at + 14)?;
            if end != 0xdead {
                break;
            }
            let hit_eff = d1867[at];
            let smash_eff = d1867[at + 1];
            let health = d1867[at + 2];
            let flags = d1867[at + 3];
            let mass = u16le(&d1867, at + 8)? as i16 as f32 / 4096.0;
            let smash_force = u16le(&d1867, at + 10)? as i16 as f32 / 4096.0;
            let uproot_force = u16le(&d1867, at + 12)? as i16 as f32 / 4096.0;
            kinds.push(PropKind {
                hit_eff,
                smash_eff,
                health,
                flags,
                mass,
                smash_force,
                uproot_force,
            });
            at += 16;
        }

        let d1866 = pak.read(disc, 1866)?.0;
        let mut defs = Vec::new();
        let mut o = 0;
        let shape_sizes = [0, 0x1c, 0x18, 0x14, 0x10, 0x10, 0x10];
        while o + 8 <= d1866.len() {
            let magic = u32le(&d1866, o)?;
            if magic == 0xdeadbeef {
                break;
            }
            let model = u16le(&d1866, o)?;
            let broken = u16le(&d1866, o + 2)?;
            let dead = u16le(&d1866, o + 4)?;
            if dead != 0xdead {
                break;
            }
            let kind = d1866[o + 6] as i8;
            let n_shapes = d1866[o + 7] as usize;
            o += 8;
            let mut shapes = Vec::with_capacity(n_shapes);
            for _ in 0..n_shapes {
                if o + 4 > d1866.len() {
                    break;
                }
                let stype = u32le(&d1866, o)? as usize;
                if stype < 1 || stype > 6 || o + shape_sizes[stype] > d1866.len() {
                    break;
                }
                let fx = |idx: usize| -> Result<f32> {
                    Ok(u32le(&d1866, o + 4 + idx * 4)? as i32 as f32 / 4096.0)
                };
                match stype {
                    1 => {
                        let offset = [fx(0)?, fx(1)?, fx(2)?];
                        let half = [fx(3)? * 0.5, fx(4)? * 0.5, fx(5)? * 0.5];
                        shapes.push(PropShape::Box { offset, half });
                    }
                    2 => {
                        let offset = [fx(0)?, fx(1)?, fx(2)?];
                        let radius = fx(3)?;
                        let height = fx(4)?;
                        shapes.push(PropShape::Cylinder {
                            offset,
                            radius,
                            height,
                        });
                    }
                    3 => {
                        let offset = [fx(0)?, fx(1)?, fx(2)?];
                        let radius = fx(3)?;
                        shapes.push(PropShape::Sphere { offset, radius });
                    }
                    _ => {}
                }
                o += shape_sizes[stype];
            }
            defs.push(PropDef {
                model,
                broken,
                kind,
                shapes,
            });
        }
        Ok(Self { defs, kinds })
    }

    pub fn kind_for(&self, prop_id: usize) -> Option<&PropKind> {
        let def = self.defs.get(prop_id)?;
        if def.kind >= 0 && (def.kind as usize) < self.kinds.len() {
            Some(&self.kinds[def.kind as usize])
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropState {
    Standing,
    Tilted,
    Loose,
    Broken,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ActiveProp {
    pub cell_x: usize,
    pub cell_y: usize,
    pub prop_idx: usize,
    pub prop_id: u16,
    pub model_id: u16,
    pub p: [f32; 3],
    pub angle: f32,
    pub v: [f32; 3],
    pub tilt: f32,
    pub tilt_axis: [f32; 2],
    pub state: PropState,
    pub collidable: bool,
    pub fountain: bool,
    pub health: u8,
    pub mass: f32,
    pub smash_eff: u8,
    pub smash_force: f32,
    pub uproot_force: f32,
}

pub struct PropsManager {
    pub library: Option<std::sync::Arc<PropLibrary>>,
    pub active: Vec<ActiveProp>,
}

impl PropsManager {
    pub fn new() -> Self {
        Self {
            library: None,
            active: Vec::new(),
        }
    }

    pub fn with_library(library: std::sync::Arc<PropLibrary>) -> Self {
        Self {
            library: Some(library),
            active: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn init(&mut self, disc: &mut Disc, pak: &Pak) -> Result<()> {
        self.library = Some(std::sync::Arc::new(PropLibrary::load(disc, pak)?));
        Ok(())
    }

    /// Synchronizes active dynamic props within the streaming bubble around focus.
    pub fn sync_area(&mut self, collision: &Collision, focus: [f32; 3]) {
        let Some(lib) = &self.library else {
            return;
        };
        let Some((cx, cy)) = Collision::cell_xy(focus[0], focus[1]) else {
            return;
        };
        for x in cx.saturating_sub(1)..=(cx + 1).min(139) {
            for y in cy.saturating_sub(1)..=(cy + 1).min(99) {
                let cell = &collision.cells[x * 100 + y];
                for (idx, p) in cell.props.iter().enumerate() {
                    let prop_id = p.prop as usize;
                    if prop_id >= lib.defs.len() {
                        continue;
                    }
                    let def = &lib.defs[prop_id];
                    let kind = lib.kind_for(prop_id);
                    let (mass, smash_eff, smash_force, uproot_force, health) = if let Some(k) = kind {
                        (k.mass, k.smash_eff, k.smash_force, k.uproot_force, k.health)
                    } else {
                        (1.0, 0, -1.0, -1.0, 15)
                    };
                    // Ignore indestructible/purely decorative scenery
                    if smash_force < 0.0 && uproot_force < 0.0 {
                        continue;
                    }
                    if self
                        .active
                        .iter()
                        .any(|ap| ap.cell_x == x && ap.cell_y == y && ap.prop_idx == idx)
                    {
                        continue;
                    }
                    let angle = p.angle();
                    self.active.push(ActiveProp {
                        cell_x: x,
                        cell_y: y,
                        prop_idx: idx,
                        prop_id: p.prop,
                        model_id: def.model,
                        p: p.p,
                        angle,
                        v: [0.0; 3],
                        tilt: 0.0,
                        tilt_axis: [0.0, 0.0],
                        state: PropState::Standing,
                        collidable: true,
                        fountain: false,
                        health,
                        mass: if mass > 0.0 { mass } else { 1.0 },
                        smash_eff,
                        smash_force,
                        uproot_force,
                    });
                }
            }
        }
        // Cleanup distant props beyond 85m (unless currently spewing a fountain)
        self.active.retain(|ap| {
            ap.fountain || {
                let dx = ap.p[0] - focus[0];
                let dy = ap.p[1] - focus[1];
                dx * dx + dy * dy < 85.0 * 85.0
            }
        });
    }

    /// Resolves vehicle impact with street furniture (hydrants, bins, fences, etc.).
    pub fn check_vehicle_impact(
        &mut self,
        car: &mut crate::actors::Car,
        _dt: f32,
        particles: &mut Vec<Particle>,
    ) {
        let speed = car.speed.abs();
        if speed < 1.5 {
            return;
        }
        let cf = forward(car.angle);
        let cside = [cf[1], -cf[0]];
        let mass = car.spec().mass();
        let force = speed * mass * 0.11;

        for prop in &mut self.active {
            if !prop.collidable && prop.state != PropState::Standing {
                continue;
            }
            let dx = prop.p[0] - car.p[0];
            let dy = prop.p[1] - car.p[1];
            let long_dist = dx * cf[0] + dy * cf[1];
            let lat_dist = (dx * cside[0] + dy * cside[1]).abs();

            if long_dist.abs() < car.h[1] + 0.65 && lat_dist < car.h[0] + 0.55 {
                let impact_dir = [cf[0] * car.speed.signum(), cf[1] * car.speed.signum()];
                let damage = (force / (prop.mass * 0.5 + 0.5)).clamp(1.0, 50.0) as u8;
                prop.health = prop.health.saturating_sub(damage);
                // Fire Hydrant (effect 9)
                if prop.smash_eff == 9 {
                    prop.tilt = 0.42; // ~24 degrees tilt
                    prop.tilt_axis = [-impact_dir[1], impact_dir[0]];
                    prop.state = PropState::Tilted;
                    prop.collidable = false;
                    prop.fountain = true;
                    car.speed *= 0.88;
                }
                // Trash can / garbage bin (effect 35)
                else if prop.smash_eff == 35 {
                    prop.state = PropState::Loose;
                    prop.collidable = false;
                    prop.v = [
                        impact_dir[0] * speed * 0.75,
                        impact_dir[1] * speed * 0.75,
                        2.8,
                    ];
                    car.speed *= 0.94;
                    // Emit garbage debris
                    for _ in 0..6 {
                        if particles.len() < 160 {
                            particles.push(Particle {
                                p: [prop.p[0], prop.p[1], prop.p[2] + 0.4],
                                v: [
                                    impact_dir[0] * 2.5 + (particles.len() % 5) as f32 * 0.4 - 0.8,
                                    impact_dir[1] * 2.5 + (particles.len() % 7) as f32 * 0.4 - 1.2,
                                    1.8 + (particles.len() % 4) as f32 * 0.5,
                                ],
                                life: 0.0,
                                max_life: 0.7,
                                size: 0.22,
                                grow: 0.4,
                                kind: 2, // garbage debris
                            });
                        }
                    }
                }
                // Wooden barrier / fence (effect 16, 26, 27)
                else if [16, 26, 27].contains(&prop.smash_eff) {
                    prop.state = PropState::Broken;
                    prop.collidable = false;
                    car.speed *= 0.92;
                    // Emit wood slat splinters
                    for i in 0..8 {
                        if particles.len() < 160 {
                            let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
                            particles.push(Particle {
                                p: [prop.p[0], prop.p[1], prop.p[2] + 0.5],
                                v: [
                                    impact_dir[0] * 3.0 + cside[0] * sign * 1.5,
                                    impact_dir[1] * 3.0 + cside[1] * sign * 1.5,
                                    2.2 + (i as f32) * 0.3,
                                ],
                                life: 0.0,
                                max_life: 0.65,
                                size: 0.28,
                                grow: 0.35,
                                kind: 3, // wood slat splinters
                            });
                        }
                    }
                }
                // Generic breakable or uprootable prop
                else if force >= prop.smash_force || force >= prop.uproot_force {
                    prop.state = PropState::Broken;
                    prop.collidable = false;
                    car.speed *= 0.90;
                }
            }
        }
    }

    /// Updates dynamic props physics, tumble integration, and water hydrant geysers.
    pub fn update(&mut self, dt: f32, particles: &mut Vec<Particle>) {
        for prop in &mut self.active {
            // Indefinite water geyser fountain for smashed hydrants
            if prop.fountain {
                if particles.len() < 160 {
                    let rx = ((particles.len() * 37) % 100) as f32 / 200.0 - 0.25;
                    let ry = ((particles.len() * 53) % 100) as f32 / 200.0 - 0.25;
                    particles.push(Particle {
                        p: [prop.p[0] + rx, prop.p[1] + ry, prop.p[2] + 0.2],
                        v: [rx * 2.0, ry * 2.0, 7.5],
                        life: 0.0,
                        max_life: 0.85,
                        size: 0.25,
                        grow: 1.8,
                        kind: 1, // water spray geyser
                    });
                }
            }
            // Loose tumbling props (bins, crates)
            if prop.state == PropState::Loose {
                prop.p[0] += prop.v[0] * dt;
                prop.p[1] += prop.v[1] * dt;
                prop.p[2] += prop.v[2] * dt;
                prop.v[2] -= 9.8 * dt;
                prop.v[0] *= (1.0 - 1.2 * dt).max(0.0);
                prop.v[1] *= (1.0 - 1.2 * dt).max(0.0);
                if prop.p[2] <= 0.0 {
                    prop.p[2] = 0.0;
                    prop.v[2] = -prop.v[2] * 0.35;
                    if prop.v[2].abs() < 0.3 {
                        prop.v[2] = 0.0;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::make_vehicle;

    #[test]
    fn authentic_psp_props_hydrant_geyser_and_trash_bins() {
        let mut manager = PropsManager::new();
        // Hydrant (effect 9)
        manager.active.push(ActiveProp {
            cell_x: 10,
            cell_y: 10,
            prop_idx: 0,
            prop_id: 175,
            model_id: 2195,
            p: [100.0, 100.0, 0.0],
            angle: 0.0,
            v: [0.0; 3],
            tilt: 0.0,
            tilt_axis: [0.0, 0.0],
            state: PropState::Standing,
            collidable: true,
            fountain: false,
            health: 20,
            mass: 1.5,
            smash_eff: 9,
            smash_force: 15.0,
            uproot_force: 30.0,
        });

        // Trash bin (effect 35)
        manager.active.push(ActiveProp {
            cell_x: 10,
            cell_y: 10,
            prop_idx: 1,
            prop_id: 173,
            model_id: 2193,
            p: [200.0, 200.0, 0.0],
            angle: 0.0,
            v: [0.0; 3],
            tilt: 0.0,
            tilt_axis: [0.0, 0.0],
            state: PropState::Standing,
            collidable: true,
            fountain: false,
            health: 10,
            mass: 0.8,
            smash_eff: 35,
            smash_force: 8.0,
            uproot_force: 12.0,
        });

        let mut particles = Vec::new();
        let mut car = make_vehicle([100.0, 98.8, 0.0], 0.0, 0, 0, 0, 0, false, false);
        car.speed = 10.0;
        car.h = [0.9, 2.1, 0.7];

        // Impact with hydrant
        manager.check_vehicle_impact(&mut car, 0.016, &mut particles);
        let hydrant = &manager.active[0];
        assert_eq!(hydrant.state, PropState::Tilted, "Hydrant tilts on impact");
        assert!(hydrant.fountain, "Hydrant enables fountain geyser");
        assert!(!hydrant.collidable, "Hydrant snaps solid collision");
        assert!((hydrant.tilt - 0.42).abs() < 1e-4, "Hydrant tilts ~24 degrees");
        assert!(car.speed < 10.0, "Car slows down on impact");

        // Hydrant geyser emission on update
        manager.update(0.016, &mut particles);
        assert!(!particles.is_empty(), "Hydrant produces water spray particles");
        assert_eq!(particles[0].kind, 1, "Water spray particle kind is 1");
        assert_eq!(particles[0].v[2], 7.5, "Water geyser shoots upward at 7.5 u/s");

        // Impact with trash bin
        let mut car2 = make_vehicle([200.0, 198.8, 0.0], 0.0, 0, 0, 0, 0, false, false);
        car2.speed = 12.0;
        car2.h = [0.9, 2.1, 0.7];
        let p_count_before = particles.len();
        manager.check_vehicle_impact(&mut car2, 0.016, &mut particles);
        let bin = &manager.active[1];
        assert_eq!(bin.state, PropState::Loose, "Trash bin becomes loose body");
        assert!(!bin.collidable, "Bin collision disabled while tumbling");
        assert!(bin.v[2] > 2.0, "Bin launched upward");
        assert!(particles.len() > p_count_before, "Garbage debris particles emitted");
        assert!(particles.iter().any(|p| p.kind == 2), "Garbage debris particle kind is 2");

        // Tumble integration
        let bin_p_before = manager.active[1].p;
        manager.update(0.1, &mut particles);
        assert_ne!(manager.active[1].p, bin_p_before, "Loose bin translates in update");
    }
}
