// SPDX-License-Identifier: GPL-3.0-only
//! PSP ULES01347 world directory and 60-unit sectors, measured from this ISO.
//! Grid dimensions/origin informed by ctw-native worldmap.cpp (C) 2026 Kryo.to.
use crate::{
    iso::{bytes, u16le, u32le, Disc, Result},
    model,
    pak::Pak,
    texture,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
pub struct Instance {
    pub slot: u16,
    pub model: u16,
    pub angle: f32,
    pub p: [f32; 3],
}
pub struct World {
    pub instances: Vec<Instance>,
    pub blocks: usize,
    pub meshes: HashMap<u16, model::Mesh>,
    pub textures: HashMap<u16, texture::Texture>,
    pub animations: HashMap<u16, Vec<(u16, u16)>>,
    pub failures: HashMap<u16, String>,
    pub visible: Vec<usize>,
    bounds: HashMap<u16, ([f32; 3], f32)>,
}
fn i16le(d: &[u8], p: usize) -> Result<i16> {
    Ok(u16le(d, p)? as i16)
}
pub fn sector(d: &[u8], start: usize, sx: i32, sy: i32) -> Result<Vec<Instance>> {
    // Sectors without owned objects begin directly with the neighbour directory.
    if u32le(d, start)? == 40 && u32le(d, start + 4)? >= 40 {
        return Ok(Vec::new());
    }
    let n = u16le(d, start)? as usize;
    let emitters = u16le(d, start + 2)? as usize;
    let groups = u16le(d, start + 4)? as usize;
    let reserved = u16le(d, start + 6)?;
    let lights = u32le(d, start + 8)? as usize;
    if groups > 256 || n > 8192 || reserved != 0 || emitters > 8192 || lights > 8192 {
        return Err("Invalid PSP sector counts".into());
    }
    let inst0 = start + 12 + groups * 16;
    let end = inst0 + n * 12 + emitters * 20 + lights * 40;
    bytes(d, start, end + 40 - start)?;
    let total = (0..groups)
        .map(|g| u32le(d, start + 12 + g * 16 + 12).map(|v| v as usize))
        .collect::<Result<Vec<_>>>()?
        .iter()
        .sum::<usize>();
    if total != n || u32le(d, end)? != 40 {
        return Err("PSP sector group/directory mismatch".into());
    }
    let offsets = (0..10)
        .map(|k| u32le(d, end + k * 4).map(|v| v as usize))
        .collect::<Result<Vec<_>>>()?;
    if offsets.windows(2).any(|p| p[0] > p[1]) || end + offsets[9] > d.len() {
        return Err("Invalid PSP neighbour table".into());
    }
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let p = inst0 + k * 12;
        out.push(Instance {
            slot: u16le(d, p)?,
            model: u16le(d, p + 2)?,
            angle: -(u16le(d, p + 4)? as f32) * std::f32::consts::TAU / 65536.,
            p: [
                // Translation is relative to the sector corner, not its centre.
                (sx * 60 - 3510) as f32 + i16le(d, p + 6)? as f32 / 128.,
                (sy * 60 - 2490) as f32 + i16le(d, p + 8)? as f32 / 128.,
                i16le(d, p + 10)? as f32 / 128.,
            ],
        });
    }
    Ok(out)
}
impl World {
    pub fn load(disc: &mut Disc, pak: &Pak) -> Result<Self> {
        let h = pak.read(disc, 1864)?.0;
        bytes(&h, 22, 59 * 42 * 2)?;
        let mut instances = Vec::new();
        let mut seen = HashSet::new();
        let mut blocks = 0;
        for r in 0..42 {
            for c in 0..59 {
                let id = u16le(&h, 22 + (r * 59 + c) * 2)?;
                if id == 65535 {
                    continue;
                }
                if !(17465..=19350).contains(&id) {
                    return Err("Unexpected PSP world resource".into());
                }
                let d = pak.read(disc, id as usize)?.0;
                blocks += 1;
                for sub in 0..4 {
                    let start = 60
                        + if sub == 0 {
                            0
                        } else {
                            u32le(&d, (sub - 1) * 4)? as usize
                        };
                    for i in sector(
                        &d,
                        start,
                        (c * 2 + sub % 2) as i32,
                        (r * 2 + sub / 2) as i32,
                    )? {
                        // The PSP water entries are render proxies; draw one sea surface instead.
                        if [7207, 7208, 4025].contains(&i.model) {
                            continue;
                        }
                        let key = (
                            i.slot,
                            i.model,
                            (i.p[0] * 128.).round() as i32,
                            (i.p[1] * 128.).round() as i32,
                            (i.p[2] * 128.).round() as i32,
                        );
                        if seen.insert(key) {
                            instances.push(i);
                        }
                    }
                }
            }
        }
        let mut bounds = HashMap::new();
        for i in &instances {
            if bounds.contains_key(&i.model) {
                continue;
            }
            let d = pak.read(disc, i.model as usize)?.0;
            let f = |p| u32le(&d, p).map(f32::from_bits);
            let c = [f(48)?, f(52)?, f(56)?];
            let h = [f(60)?, f(64)?, f(68)?];
            if c.iter().chain(h.iter()).any(|v| !v.is_finite()) {
                return Err("Invalid model streaming bounds".into());
            }
            bounds.insert(i.model, (c, (h[0] * h[0] + h[1] * h[1]).sqrt()));
        }
        Ok(Self {
            instances,
            blocks,
            meshes: HashMap::new(),
            textures: HashMap::new(),
            animations: HashMap::new(),
            failures: HashMap::new(),
            visible: Vec::new(),
            bounds,
        })
    }
    pub fn stream(&mut self, disc: &mut Disc, pak: &Pak, p: [f32; 3], range: f32) -> Result<()> {
        self.visible = self
            .instances
            .iter()
            .enumerate()
            .filter(|(_, i)| {
                let (c, r) = self.bounds[&i.model];
                let (s, co) = i.angle.sin_cos();
                let x = i.p[0] + c[0] * co - c[1] * s;
                let y = i.p[1] + c[0] * s + c[1] * co;
                (x - p[0]).abs() < range + r && (y - p[1]).abs() < range + r
            })
            .map(|(k, _)| k)
            .collect();
        let needed: HashSet<_> = self
            .visible
            .iter()
            .map(|&k| self.instances[k].model)
            .collect();
        self.meshes.retain(|k, _| needed.contains(k));
        for id in needed {
            if self.meshes.contains_key(&id) || self.failures.contains_key(&id) {
                continue;
            }
            let raw = pak.read(disc, id as usize)?.0;
            match model::parse(&raw) {
                Ok(m) => {
                    self.meshes.insert(id, m);
                }
                Err(e) => {
                    self.failures.insert(id, e.to_string());
                }
            }
        }
        let mut needed: HashSet<_> = self
            .meshes
            .values()
            .flat_map(|m| m.texture_ids.iter().copied())
            .collect();
        self.animations.retain(|k, _| needed.contains(k));
        for id in needed.clone() {
            if id == 65535 || self.textures.contains_key(&id) {
                continue;
            }
            let raw = pak.read(disc, id as usize)?.0;
            if let Some(animation) = texture::animation(&raw)? {
                for &(frame, _) in &animation {
                    needed.insert(frame);
                    if !self.textures.contains_key(&frame) {
                        self.textures.insert(
                            frame,
                            texture::decode(frame as usize, &pak.read(disc, frame as usize)?.0)?,
                        );
                    }
                }
                self.animations.insert(id, animation);
            } else {
                self.textures
                    .insert(id, texture::decode(id as usize, &raw)?);
            }
        }
        self.textures.retain(|k, _| needed.contains(k));
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absolute_placement_matches_psp_collision_landmark() {
        // PSP block 17743/subsector 1: pillar slot 8602, model 16924.
        // Independently decoded world.bin pillar centre is (-1045, -1520).
        // Visual/physics centres differ by < 1 unit, never half a sector (30).
        let mut b = vec![0; 80];
        b[0] = 1;
        b[4] = 1;
        b[24] = 1;
        b[28..30].copy_from_slice(&8602u16.to_le_bytes());
        b[30..32].copy_from_slice(&16924u16.to_le_bytes());
        b[34..36].copy_from_slice(&566i16.to_le_bytes());
        b[36..38].copy_from_slice(&1253i16.to_le_bytes());
        for k in 0..10 {
            b[40 + k * 4..44 + k * 4].copy_from_slice(&40u32.to_le_bytes());
        }
        let p = sector(&b, 0, 41, 16).unwrap()[0].p;
        assert!((p[0] + 1045.).abs() < 1.);
        assert!((p[1] + 1520.).abs() < 1.);
    }
    #[test]
    fn placements_match_across_neighbouring_sectors() {
        let mut b = vec![0; 80];
        b[0] = 1;
        b[4] = 1;
        b[24] = 1;
        b[30..32].copy_from_slice(&12006u16.to_le_bytes());
        for k in 0..10 {
            b[40 + k * 4..44 + k * 4].copy_from_slice(&40u32.to_le_bytes());
        }
        let a = sector(&b, 0, 50, 40).unwrap()[0];
        b[34..36].copy_from_slice(&(-7680i16).to_le_bytes());
        let c = sector(&b, 0, 51, 40).unwrap()[0];
        assert_eq!(a.p, c.p);
        b[24] = 2;
        assert!(sector(&b, 0, 50, 40).is_err());
    }
}
