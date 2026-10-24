// SPDX-License-Identifier: GPL-3.0-only
//! PSP sprite format measured on ULES01347: 552 animations, 8-byte layers.
//! Animation stepping and layered drawing adapted from ctw-native PedSprites /
//! Player::animate (C) 2026 Kryo.to. PSP body stride is 276, not Android's 275.
use crate::{
    iso::{bytes, u16le, Disc, Result},
    pak::Pak,
    texture::{self, Texture},
};
use std::collections::{HashMap, HashSet};

pub const BODY_STRIDE: usize = 276;
pub const FALL_BACK_OFFSET: usize = 20;
pub const FALL_FRONT_OFFSET: usize = 22;
pub const DEAD_BACK_OFFSET: usize = 24;
pub const DEAD_FRONT_OFFSET: usize = 26;
pub const ROLL_OFFSET: usize = 58;
pub const ROLL_DIVE_OFFSET: usize = 60;
pub const BIKE_RIDE_OFFSET: usize = 92;     // 0x5C / 0x5D
pub const BIKE_REVERSE_OFFSET: usize = 164; // 0xA4 / 0xA5
pub const BIKE_LEAN_OFFSET: usize = 166;    // 0xA6 / 0xA7
pub const ATTACK_OFFSET: usize = 132;
// Matched to Benjamin's PSP recording at16.4-17.2s,20.6-21.4s,42.3-43.1s.
// EBOOT0x0883d970 selects0x8c/0x8d, body stride276 =>416/417.
// Action eligibility/trajectory are still reconstructed.
pub const VAULT_PAIR: [usize; 2] = [416, 417];
// PSP HandlePlayerDive starts state11 (0x089cfc00), then state10:
// SetupForState selects 0x3c/0x3d and 0x3a/0x3b, plus body stride276.
// 408/409 is a kick, NOT the roll. See ROULADE-MANETTE.md.
#[cfg(test)]
pub const ROLL_DIVE_PAIR: [usize; 2] = [336, 337];
pub const ROLL_PAIR: [usize; 2] = [334, 335];
// Original PSP kick sequence: PAK202, sixteen frames at rate7680.
// Action timing and hit detection in the presentation slice are reconstructed.
pub const ATTACK_PAIR: [usize; 2] = [408, 409];
// PAK 209 RGB555 civilian palettes (cPed::Dress: male 0..4 and pedinfo fixed
// 11/13/14/19 on body set 1; female 30..34 on body set 0) and jogger subtype.
// Entry 0 stays male [0, 1] so deterministic combat scenarios are unchanged.
const NPC_APPEARANCES: [(bool, [u8; 2], bool); 12] = [
    (true, [0, 1], false),
    (false, [30, 31], false),
    (true, [2, 3], true),
    (true, [11, 3], false),
    (false, [32, 33], false),
    (true, [19, 5], false),
    (true, [1, 2], false),
    (false, [34, 30], true),
    (true, [13, 13], false),
    (true, [3, 0], false),
    (false, [31, 34], false),
    (true, [14, 14], false),
];
pub fn npc_is_male(appearance: usize) -> bool {
    NPC_APPEARANCES[appearance % NPC_APPEARANCES.len()].0
}
pub fn npc_is_jogger(appearance: usize) -> bool {
    NPC_APPEARANCES[appearance % NPC_APPEARANCES.len()].2
}

#[derive(Clone, Debug)]
pub struct Layer {
    pub texture: u16,
    pub offset: [f32; 2],
    pub uv: [u16; 2],
    pub size: [u8; 2],
}
pub struct Animation {
    pub rate: i16,
    pub angle: u16,
    pub flags: u8,
    pub frames: Vec<Vec<Layer>>,
}
pub struct PedAssets {
    pub animations: Vec<Animation>,
    pub textures: HashMap<(u16, u8), Texture>,
}
pub struct Quad {
    pub texture: (u16, u8),
    pub p: [[f32; 3]; 4],
    pub uv: [[f32; 2]; 4],
}
pub fn parse(d: &[u8]) -> Result<Vec<Animation>> {
    let count = u16le(d, 0)? as usize;
    if count != 552 {
        return Err("Unsupported PSP pedestrian animation directory".into());
    }
    bytes(d, 2, count * 2)?;
    let mut animations = Vec::new();
    for id in 0..count {
        let off = u16le(d, 2 + id * 2)? as usize * 4;
        if off < 2 + count * 2 {
            return Err("Animation overlaps directory".into());
        }
        let n = u16le(d, off)? as usize;
        let rate = u16le(d, off + 2)? as i16;
        let angle = u16le(d, off + 4)?;
        let flags = bytes(d, off + 6, 2)?[0];
        let layers = d[off + 7] as usize;
        if n > 1024 || layers != 2 || rate < 0 || flags >> 4 > 3 {
            return Err("Invalid PSP animation counts or flags".into());
        }
        let end = off + 8 + n * layers * 8;
        bytes(d, off, end - off)?;
        if id + 1 < count && end != u16le(d, 4 + id * 2)? as usize * 4 {
            return Err("PSP animation does not end at next entry".into());
        }
        let mut frames = Vec::new();
        for f in 0..n {
            let mut frame = Vec::new();
            for c in 0..layers {
                let p = off + 8 + (f * layers + c) * 8;
                let texture = u16le(d, p)?;
                let x = d[p + 2];
                let y = d[p + 3];
                let uv = [
                    d[p + 4] as u16 + ((x as u16 >> 7) << 8),
                    d[p + 5] as u16 + ((y as u16 >> 7) << 8),
                ];
                let size = [d[p + 6], d[p + 7]];
                if !(203..=208).contains(&texture)
                    || uv[0] + size[0] as u16 > 512
                    || uv[1] + size[1] as u16 > 512
                {
                    return Err("PSP sprite rectangle outside atlas".into());
                }
                frame.push(Layer {
                    texture,
                    offset: [((x & 127) as f32 - 16.) / 8., ((y & 127) as f32 - 16.) / 8.],
                    uv,
                    size,
                });
            }
            frames.push(frame);
        }
        animations.push(Animation {
            rate,
            angle,
            flags,
            frames,
        });
    }
    Ok(animations)
}
impl PedAssets {
    pub fn load(disc: &mut Disc, pak: &Pak) -> Result<Self> {
        // These PSP directory entries were checked independently against all frames.
        let animations = parse(&pak.read(disc, 202)?.0)?;
        let palette = pak.read(disc, 209)?.0;
        let ids: HashSet<_> = animations
            .iter()
            .flat_map(|a| a.frames.iter().flatten().map(|l| l.texture))
            .collect();
        let mut textures = HashMap::new();
        for id in ids {
            let original = pak.read(disc, id as usize)?.0;
            let info = texture::info(id as usize, &original)?.ok_or("Missing pedestrian atlas")?;
            if info.width != 512 || info.height != 512 || info.bits != 4 || info.colors != 16 {
                return Err("Unsupported pedestrian atlas".into());
            }
            // Atlas 203 holds the female civilian sprites (palettes 30..34);
            // 205/206 hold the male sprites (palettes 0..4 and pedinfo fixed).
            let palettes: &[u8] = if id == 203 {
                &[4, 5, 30, 31, 32, 33, 34]
            } else if [205, 206].contains(&id) {
                &[0, 1, 2, 3, 4, 5, 11, 13, 14, 19]
            } else {
                &[4, 5]
            };
            for &pal in palettes {
                let mut raw = original.clone();
                let offset = 16 + 512 * 512 / 2;
                for k in 0..16 {
                    let rgb = u16le(&palette, pal as usize * 32 + k * 2)?;
                    for c in 0..3 {
                        raw[offset + k * 4 + c] = (((rgb >> (c * 5)) & 31) * 255 / 31) as u8;
                    }
                    // RGB555 uses magenta for colour-key index 0. Keep its alpha
                    // from the PSP atlas, but never filter that magenta into edges.
                    if raw[offset + k * 4 + 3] == 0 {
                        raw[offset + k * 4..offset + k * 4 + 3].fill(0);
                    }
                }
                textures.insert((id, pal), texture::decode(id as usize, &raw)?);
            }
        }
        Ok(Self {
            animations,
            textures,
        })
    }
    pub fn quads(&self, pose: &Pose, pos: [f32; 3], heading: f32) -> Vec<Quad> {
        let mut out = Vec::new();
        for (id, fixed_frame, legs) in [
            (pose.legs, pose.frame_legs, true),
            (pose.upper, pose.frame_upper, false),
        ] {
            let a = &self.animations[id];
            if a.frames.is_empty() {
                continue;
            }
            let f = (fixed_frame as usize >> 8).min(a.frames.len() - 1);
            let h = heading - a.angle as f32 * std::f32::consts::TAU / 65536.;
            let forward = [-h.sin(), h.cos()];
            let side = [-forward[1], forward[0]];
            for (c, l) in a.frames[f].iter().enumerate().rev() {
                if l.size.contains(&0) {
                    continue;
                }
                let z = pos[2]
                    + pose_layer_height(
                        pose.upper,
                        fixed_frame,
                        a.frames.len(),
                        if legs { c + 2 } else { c },
                    );
                let p0 = [
                    pos[0] + l.offset[0] * side[0] + l.offset[1] * forward[0],
                    pos[1] + l.offset[0] * side[1] + l.offset[1] * forward[1],
                    z - c as f32 * 204. / 4096.,
                ];
                let w = l.size[0] as f32 / 8.;
                let h = l.size[1] as f32 / 8.;
                let p1 = [p0[0] + forward[0] * h, p0[1] + forward[1] * h, p0[2]];
                let p2 = [p1[0] + side[0] * w, p1[1] + side[1] * w, p0[2]];
                let p3 = [p0[0] + side[0] * w, p0[1] + side[1] * w, p0[2]];
                let u = l.uv[0] as f32 / 512.;
                let v = l.uv[1] as f32 / 512.;
                let u1 = (l.uv[0] + l.size[0] as u16) as f32 / 512.;
                let v1 = (l.uv[1] + l.size[1] as u16) as f32 / 512.;
                out.push(Quad {
                    texture: (l.texture, if legs && c > 0 { 5 } else { 4 }),
                    p: [p0, p1, p2, p3],
                    uv: [[u, v], [u, v1], [u1, v1], [u1, v]],
                });
            }
        }
        out
    }
    pub fn npc_quads(
        &self,
        pose: &Pose,
        pos: [f32; 3],
        heading: f32,
        appearance: usize,
    ) -> Vec<Quad> {
        let palettes = NPC_APPEARANCES[appearance % NPC_APPEARANCES.len()].1;
        let mut out = self.quads(pose, pos, heading);
        for quad in &mut out {
            let palette = palettes[usize::from(quad.texture.1 == 5)];
            let key = (quad.texture.0, palette);
            // Unused NPC special actions can safely retain their source palette.
            if self.textures.contains_key(&key) {
                quad.texture = key;
            }
        }
        out
    }
    /// cBike::GetPedRenderPos / Player::renderRiding (player.cpp lines 666-683):
    /// Renders seated motorcycle rider with separate upper and legs 3D positions,
    /// layer heights [0.5, 0.2998] for legs and [1.0, 0.75] for upper.
    pub fn riding_quads(
        &self,
        pose: &Pose,
        upper_pos: [f32; 3],
        legs_pos: [f32; 3],
        heading: f32,
    ) -> Vec<Quad> {
        let mut out = Vec::new();
        for (id, fixed_frame, legs, pos, z_layers) in [
            (pose.legs, pose.frame_legs, true, legs_pos, [0.5, 0.2998]),
            (pose.upper, pose.frame_upper, false, upper_pos, [1.0, 0.75]),
        ] {
            let a = &self.animations[id];
            if a.frames.is_empty() {
                continue;
            }
            let f = (fixed_frame as usize >> 8).min(a.frames.len() - 1);
            let h = heading - a.angle as f32 * std::f32::consts::TAU / 65536.;
            let forward = [-h.sin(), h.cos()];
            let side = [-forward[1], forward[0]];
            for (c, l) in a.frames[f].iter().enumerate().rev() {
                if l.size.contains(&0) {
                    continue;
                }
                let z = pos[2] + z_layers[c.min(1)];
                let p0 = [
                    pos[0] + l.offset[0] * side[0] + l.offset[1] * forward[0],
                    pos[1] + l.offset[0] * side[1] + l.offset[1] * forward[1],
                    z - c as f32 * 204. / 4096.,
                ];
                let w = l.size[0] as f32 / 8.;
                let h = l.size[1] as f32 / 8.;
                let p1 = [p0[0] + forward[0] * h, p0[1] + forward[1] * h, p0[2]];
                let p2 = [p1[0] + side[0] * w, p1[1] + side[1] * w, p0[2]];
                let p3 = [p0[0] + side[0] * w, p0[1] + side[1] * w, p0[2]];
                let u = l.uv[0] as f32 / 512.;
                let v = l.uv[1] as f32 / 512.;
                let u1 = (l.uv[0] + l.size[0] as u16) as f32 / 512.;
                let v1 = (l.uv[1] + l.size[1] as u16) as f32 / 512.;
                out.push(Quad {
                    texture: (l.texture, if legs && c > 0 { 5 } else { 4 }),
                    p: [p0, p1, p2, p3],
                    uv: [[u, v], [u, v1], [u1, v1], [u1, v]],
                });
            }
        }
        out
    }
    pub fn npc_riding_quads(
        &self,
        pose: &Pose,
        upper_pos: [f32; 3],
        legs_pos: [f32; 3],
        heading: f32,
        appearance: usize,
    ) -> Vec<Quad> {
        let palettes = NPC_APPEARANCES[appearance % NPC_APPEARANCES.len()].1;
        let mut out = self.riding_quads(pose, upper_pos, legs_pos, heading);
        for quad in &mut out {
            let palette = palettes[usize::from(quad.texture.1 == 5)];
            let key = (quad.texture.0, palette);
            if self.textures.contains_key(&key) {
                quad.texture = key;
            }
        }
        out
    }
}
fn pose_layer_height(upper: usize, frame: i32, frames: usize, layer: usize) -> f32 {
    let rel = upper % BODY_STRIDE;
    if rel == ROLL_OFFSET {
        return layer_height(true, frame, frames, layer);
    }
    // PSP table 0x089b285c pose 3 (fallen/dead on ground): base 0x733 with a
    // small per-layer step so upper/lower sprites never z-fight in OpenGL.
    let fallen = [0x733, 0x713, 0x6f3, 0x6d3][layer];
    if rel == DEAD_BACK_OFFSET || rel == DEAD_FRONT_OFFSET {
        return fallen as f32 / 4096.;
    }
    if (rel == FALL_BACK_OFFSET || rel == FALL_FRONT_OFFSET) && frames > 0 {
        let standing = [8192, 6144, 4096, 2048][layer];
        let fraction = ((frame.max(0) / frames as i32) * 16).clamp(0, 4096);
        let height = (standing * (4096 - fraction) + fallen * fraction) >> 12;
        return height as f32 / 4096.;
    }
    layer_height(false, frame, frames, layer)
}
fn layer_height(recovering: bool, frame: i32, frames: usize, layer: usize) -> f32 {
    // PSP cPedSprite::Z 0x089b1558 and table initializer0x089b285c.
    // State10 interpolates pose1 -> pose0, not literal heights1 -> 0.
    let standing = [8192, 6144, 4096, 2048];
    let crouched = [4096, 3072, 2048, 1228];
    if !recovering || frames == 0 {
        return standing[layer] as f32 / 4096.;
    }
    let fraction = ((frame.max(0) / frames as i32) * 16).clamp(0, 4096);
    let height = (crouched[layer] * (4096 - fraction) + standing[layer] * fraction) >> 12;
    height as f32 / 4096.
}
#[derive(Clone, Debug)]
pub struct Pose {
    pub upper: usize,
    pub legs: usize,
    pub frame_upper: i32,
    pub frame_legs: i32,
}
impl Default for Pose {
    fn default() -> Self {
        Self {
            upper: 276,
            legs: 277,
            frame_upper: 0,
            frame_legs: 0,
        }
    }
}
impl Pose {
    pub fn for_appearance(appearance: usize) -> Self {
        let base = if npc_is_male(appearance) { BODY_STRIDE } else { 0 };
        Self {
            upper: base,
            legs: base + 1,
            frame_upper: 0,
            frame_legs: 0,
        }
    }
    pub fn base(&self) -> usize {
        (self.upper / BODY_STRIDE) * BODY_STRIDE
    }
    pub fn attack(&mut self, assets: &PedAssets, progress: f32) {
        let b = self.base();
        let pair = if !assets.animations[b + ATTACK_OFFSET].frames.is_empty() {
            [b + ATTACK_OFFSET, b + ATTACK_OFFSET + 1]
        } else {
            ATTACK_PAIR
        };
        self.action(assets, progress, pair);
    }
    pub fn vault(&mut self, assets: &PedAssets, progress: f32) {
        self.action(assets, progress, VAULT_PAIR);
    }
    pub fn roll(&mut self, assets: &PedAssets, progress: f32) {
        // Video-calibrated phase split; original task scheduling/physics still
        // need parity work. Preserve every original pose in both sequences.
        let b = self.base();
        let progress = progress.clamp(0., 1.);
        if progress < 0.25 {
            self.action(
                assets,
                progress / 0.25,
                [b + ROLL_DIVE_OFFSET, b + ROLL_DIVE_OFFSET + 1],
            );
        } else {
            self.action(
                assets,
                (progress - 0.25) / 0.75,
                [b + ROLL_OFFSET, b + ROLL_OFFSET + 1],
            );
        }
    }
    /// PSP knockdown/death sequence: 5-frame fall (0x14/0x15 or 0x16/0x17) into
    /// the fallen ground pose (0x18/0x19 back or 0x1a/0x1b front).
    pub fn fall(&mut self, assets: &PedAssets, forward: bool, progress: f32) {
        let b = self.base();
        if progress < 1. {
            let off = if forward { FALL_FRONT_OFFSET } else { FALL_BACK_OFFSET };
            self.action(assets, progress, [b + off, b + off + 1]);
        } else {
            let off = if forward { DEAD_FRONT_OFFSET } else { DEAD_BACK_OFFSET };
            self.action(assets, 1., [b + off, b + off + 1]);
        }
    }
    /// PSP motorcycle riding animations (cBike / Player::renderRiding):
    /// 0x5C/0x5D riding forward, 0xA6/0xA7 stopped with foot down, 0xA4/0xA5 reversing.
    pub fn ride(&mut self, assets: &PedAssets, speed: f32, reversing: bool, leaning_stopped: bool) {
        let b = self.base();
        if leaning_stopped {
            let mut pair = [b + BIKE_LEAN_OFFSET, b + BIKE_LEAN_OFFSET + 1];
            if assets.animations.get(pair[0]).map_or(true, |a| a.frames.is_empty()) {
                pair = [BODY_STRIDE + BIKE_LEAN_OFFSET, BODY_STRIDE + BIKE_LEAN_OFFSET + 1];
            }
            [self.upper, self.legs] = pair;
            self.frame_upper = 0;
            self.frame_legs = 0;
        } else if reversing {
            let mut pair = [b + BIKE_REVERSE_OFFSET, b + BIKE_REVERSE_OFFSET + 1];
            if assets.animations.get(pair[0]).map_or(true, |a| a.frames.is_empty()) {
                pair = [BODY_STRIDE + BIKE_REVERSE_OFFSET, BODY_STRIDE + BIKE_REVERSE_OFFSET + 1];
            }
            [self.upper, self.legs] = pair;
            let step = ((speed.abs() * 4096.) as i64 * 0x88 >> 16) as i32;
            self.frame_upper = advance(&assets.animations[self.upper], self.frame_upper, step);
            self.frame_legs = advance(&assets.animations[self.legs], self.frame_legs, step);
        } else {
            let pair = [b + BIKE_RIDE_OFFSET, b + BIKE_RIDE_OFFSET + 1];
            [self.upper, self.legs] = pair;
            self.frame_upper = 0;
            self.frame_legs = 0;
        }
    }
    fn action(&mut self, assets: &PedAssets, progress: f32, pair: [usize; 2]) {
        [self.upper, self.legs] = pair;
        let frame = |id: usize| {
            let n = assets.animations[id].frames.len() as i32;
            ((progress.clamp(0., 1.) * (n * 256) as f32) as i32).min((n - 1).max(0) * 256)
        };
        self.frame_upper = frame(self.upper);
        self.frame_legs = frame(self.legs);
    }
    pub fn step(&mut self, assets: &PedAssets, level: u8, speed_fixed: i32) {
        let base = self.base();
        let mut upper = base
            + match level {
                1 => 2,
                2 => 4,
                3 => 160,
                _ => 0,
            };
        if assets.animations[upper].frames.is_empty() {
            upper = base + 4;
        }
        let legs = upper + 1;
        let rel = self.upper % BODY_STRIDE;
        if [
            VAULT_PAIR[0] % BODY_STRIDE,
            ROLL_OFFSET,
            ROLL_DIVE_OFFSET,
            ATTACK_OFFSET,
            FALL_BACK_OFFSET,
            FALL_FRONT_OFFSET,
            DEAD_BACK_OFFSET,
            DEAD_FRONT_OFFSET,
            BIKE_RIDE_OFFSET,
            BIKE_REVERSE_OFFSET,
            BIKE_LEAN_OFFSET,
        ]
        .contains(&rel)
        {
            self.upper = upper;
            self.legs = legs;
            self.frame_upper = 0;
            self.frame_legs = 0;
        }
        if self.upper != upper {
            let remap = |frame: i32, old: usize, new: usize| {
                let n = assets.animations[old].frames.len();
                if n == 0 {
                    0
                } else {
                    (frame as i64 * assets.animations[new].frames.len() as i64 / n as i64) as i32
                }
            };
            self.frame_upper = remap(self.frame_upper, self.upper, upper);
            self.frame_legs = remap(self.frame_legs, self.legs, legs);
            self.upper = upper;
            self.legs = legs;
        }
        if level == 0 {
            self.frame_upper = 0;
            self.frame_legs = 0;
            return;
        }
        let step = ((speed_fixed as i64 * 0x88) >> 16) as i32;
        self.frame_upper = advance(&assets.animations[self.upper], self.frame_upper, step);
        self.frame_legs = advance(&assets.animations[self.legs], self.frame_legs, step);
    }
}
fn advance(a: &Animation, frame: i32, step: i32) -> i32 {
    let end = a.frames.len() as i32 * 256;
    if end == 0 {
        return 0;
    }
    let f = (frame + ((a.rate as i32 * step) >> 8)).max(0);
    if a.flags & 1 != 0 {
        f.min(end - 256)
    } else {
        f % end
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roll_recovery_height_starts_crouched_and_rises_without_clipping_ground() {
        assert_eq!(layer_height(true, 0, 9, 0), 1.);
        assert_eq!(layer_height(true, 9 * 256, 9, 0), 2.);
        for layer in 0..4 {
            let mut previous = 0.;
            for frame in 0..=9 * 256 {
                let height = layer_height(true, frame, 9, layer);
                assert!(height >= previous && height > 204. / 4096.);
                assert!(height <= layer_height(false, 0, 9, layer));
                previous = height;
            }
        }
    }
    #[test]
    fn looping_and_one_shot_keep_original_fixed_point_timing() {
        let mut a = Animation {
            rate: 512,
            angle: 0,
            flags: 0,
            frames: vec![vec![]; 11],
        };
        assert_eq!(advance(&a, 0, 51), 102);
        assert_eq!(advance(&a, 2800, 51), 86);
        a.flags = 1;
        assert_eq!(advance(&a, 2800, 51), 2560);
    }
    #[test]
    fn packed_psp_uv_and_offsets_are_not_android_layers() {
        let mut d = vec![0; 1108 + 552 * 24];
        d[0..2].copy_from_slice(&552u16.to_le_bytes());
        for id in 0..552 {
            let p = 1108 + id * 24;
            d[2 + id * 2..4 + id * 2].copy_from_slice(&((p / 4) as u16).to_le_bytes());
            d[p] = 1;
            d[p + 3] = 1;
            d[p + 7] = 2;
            for k in 0..2 {
                d[p + 8 + k * 8..p + 16 + k * 8].copy_from_slice(&[205, 0, 13, 141, 64, 193, 7, 9]);
            }
        }
        let a = parse(&d).unwrap();
        let l = &a[276].frames[0][0];
        assert_eq!(l.uv, [64, 449]);
        assert_eq!(l.offset, [-0.375, -0.375]);
        d[1115] = 6;
        assert!(parse(&d).is_err());
    }
    #[test]
    fn male_and_female_poses_preserve_body_stride_for_walk_roll_and_fall() {
        let a = PedAssets {
            animations: (0..552)
                .map(|_| Animation {
                    rate: 512,
                    angle: 0,
                    flags: 0,
                    frames: vec![vec![]; 6],
                })
                .collect(),
            textures: HashMap::new(),
        };
        let mut male = Pose::for_appearance(0);
        let mut female = Pose::for_appearance(1);
        assert!(npc_is_male(0) && !npc_is_male(1));
        assert!(!npc_is_jogger(0) && npc_is_jogger(2) && npc_is_jogger(7));
        assert_eq!((male.upper, male.legs), (276, 277));
        assert_eq!((female.upper, female.legs), (0, 1));
        male.step(&a, 2, 4096);
        female.step(&a, 2, 4096);
        assert_eq!((male.upper, male.legs), (280, 281));
        assert_eq!((female.upper, female.legs), (4, 5));
        male.roll(&a, 0.1);
        female.roll(&a, 0.1);
        assert_eq!([male.upper, male.legs], ROLL_DIVE_PAIR);
        assert_eq!((female.upper, female.legs), (60, 61));
        male.roll(&a, 0.6);
        female.roll(&a, 0.6);
        assert_eq!([male.upper, male.legs], ROLL_PAIR);
        assert_eq!((female.upper, female.legs), (58, 59));
        male.fall(&a, false, 0.2);
        female.fall(&a, true, 0.2);
        assert_eq!((male.upper, male.legs), (296, 297));
        assert_eq!((female.upper, female.legs), (22, 23));
        male.fall(&a, false, 1.0);
        female.fall(&a, true, 1.0);
        assert_eq!((male.upper, male.legs), (300, 301));
        assert_eq!((female.upper, female.legs), (26, 27));
        assert!((pose_layer_height(male.upper, 0, 1, 0) - 0x733 as f32 / 4096.).abs() < 1e-6);
        male.attack(&a, 0.5);
        female.attack(&a, 0.5);
        assert_eq!([male.upper, male.legs], ATTACK_PAIR);

        male.ride(&a, 10.0, false, false);
        female.ride(&a, 10.0, false, false);
        assert_eq!((male.upper, male.legs), (276 + BIKE_RIDE_OFFSET, 276 + BIKE_RIDE_OFFSET + 1));
        assert_eq!((female.upper, female.legs), (BIKE_RIDE_OFFSET, BIKE_RIDE_OFFSET + 1));

        male.ride(&a, 0.0, false, true);
        assert_eq!((male.upper, male.legs), (276 + BIKE_LEAN_OFFSET, 276 + BIKE_LEAN_OFFSET + 1));
    }
}
