// SPDX-License-Identifier: GPL-3.0-only
//! PSP ULES01347 radar data. See RADAR-PSP.md for binary addresses and limits.
use crate::iso::{u16le, u32le, Result};

pub const CANVAS: [f32; 2] = [480., 272.];
pub const CENTER: [f32; 2] = [48., 224.];
pub const MAP_RADIUS: f32 = 38.;
pub const OUTER_SCALE: f32 = 18038.5 / 32768.; // PSP GE float24 of 0x468ced91
pub const WORLD_SCALE: f32 = 1638. / 4096.;
pub const TILE_SCREEN_SCALE: f32 = 32767. / 32768.;
pub const WORLD_SPAN: f32 = 76. / (WORLD_SCALE * TILE_SCREEN_SCALE);
pub const MAP_ALPHA: f32 = 191. / 255.;
pub const HEALTH: [u8; 3] = [82, 123, 82];
pub const LOW_HEALTH: [u8; 3] = [123, 54, 54];
pub const ARMOUR: [u8; 3] = [53, 113, 156];
pub const MAX_VITAL: u8 = 127;

pub fn scale() -> [f32; 2] {
    [
        crate::gpu::W as f32 / CANVAS[0],
        crate::gpu::H as f32 / CANVAS[1],
    ]
}
pub fn center(bands: bool) -> [f32; 2] {
    let s = scale();
    // Optional PC information bands reserve the bottom 130 pixels.
    [
        CENTER[0] * s[0],
        CENTER[1] * s[1] - if bands { 130. } else { 0. },
    ]
}
pub fn rect(bands: bool) -> [f32; 4] {
    let s = scale();
    let c = center(bands);
    [
        c[0] - MAP_RADIUS * s[0],
        c[1] - MAP_RADIUS * s[1],
        2. * MAP_RADIUS * s[0],
        2. * MAP_RADIUS * s[1],
    ]
}
pub fn vital_steps(value: u8) -> usize {
    ((value.min(MAX_VITAL) as usize) << 7) / MAX_VITAL as usize
}
pub fn health_color(value: u8, tick: usize) -> Option<[u8; 3]> {
    if vital_steps(value) < 25 {
        if tick & 0x1c == 0x1c {
            None
        } else {
            Some(LOW_HEALTH)
        }
    } else {
        Some(HEALTH)
    }
}
/// cRadar constructor: 256 steps, Q12 sine, signed integer radius 76.
/// Screen Y is inverted by the PSP projection. Keep the integer contour.
pub fn perimeter(k: usize) -> [f32; 2] {
    static POINTS: std::sync::OnceLock<[[f32; 2]; 257]> = std::sync::OnceLock::new();
    POINTS.get_or_init(|| {
        std::array::from_fn(|i| {
            let angle = 65536 - i as i32 * 256;
            let sine = |a: i32| ((a as f32 * f32::from_bits(0x38c90fd8)).sin() * 4096.) as i32;
            [
                ((sine(angle) * 76) >> 12) as f32,
                -((sine(angle + 16384) * 76) >> 12) as f32,
            ]
        })
    })[k % 257]
}
/// Map texture origin is integer pixels; markers retain their subpixel positions.
pub fn snapped_focus(p: [f32; 2]) -> [f32; 2] {
    [
        ((p[0] + 3540.) * WORLD_SCALE).floor() / WORLD_SCALE - 3540.,
        2520. - ((2520. - p[1]) * WORLD_SCALE).floor() / WORLD_SCALE,
    ]
}

#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    pub id: u16,
    pub source: [f32; 4],
    pub offset: [f32; 2],
}
impl Sprite {
    pub fn target(self, anchor: [f32; 2], scale: [f32; 2]) -> [f32; 4] {
        [
            anchor[0] - self.offset[0] * scale[0],
            anchor[1] - self.offset[1] * scale[1],
            self.source[2] * scale[0],
            self.source[3] * scale[1],
        ]
    }
}
pub fn sprites(data: &[u8], size: [usize; 2]) -> Result<Vec<Sprite>> {
    let count = u32le(data, 0)? as usize;
    if count != 48 || data.len() != 4 + count * 16 {
        return Err("Invalid PSP SS_Radar.bin directory".into());
    }
    let mut out = Vec::with_capacity(count);
    for row in data[4..].chunks_exact(16) {
        let id = u16le(row, 0)?;
        let x = u16le(row, 4)? as usize;
        let y = u16le(row, 6)? as usize;
        let w = u16le(row, 12)? as usize + 1;
        let h = u16le(row, 14)? as usize + 1;
        if u16le(row, 2)? != 4
            || x + w > size[0]
            || y + h > size[1]
            || out.iter().any(|s: &Sprite| s.id == id)
        {
            return Err("Invalid PSP radar sprite rectangle/slot/ID".into());
        }
        out.push(Sprite {
            id,
            source: [x as f32, y as f32, w as f32, h as f32],
            offset: [u16le(row, 8)? as i16 as f32, u16le(row, 10)? as i16 as f32],
        });
    }
    for id in [37, 38, 43] {
        if !out.iter().any(|s| s.id == id) {
            return Err("Missing PSP radar sprite".into());
        }
    }
    Ok(out)
}

pub struct Map {
    pub texture: crate::texture::Texture,
    pub origin: [f32; 2],
}
impl Map {
    /// Copy native texels, without the old 7000x5000 resampling. One sea texel
    /// around the atlas makes clamping outside the directory return sea.
    pub fn load(disc: &mut crate::iso::Disc, pak: &crate::pak::Pak) -> Result<Self> {
        let raw = pak.read(disc, 331)?.0;
        let cols = u16le(&raw, 4)? as usize;
        let rows = u16le(&raw, 6)? as usize;
        if (cols, rows) != (8, 7) {
            return Err("Unsupported PSP radar directory".into());
        }
        let default_id = u16le(&raw, 8 + cols * rows * 2)? as usize;
        let sea = crate::texture::decode(default_id, &pak.read(disc, default_id)?.0)?;
        let (width, height) = (cols * 256 + 2, rows * 256 + 2);
        let mut rgba = vec![sea.rgba[0]; width * height];
        for i in 0..cols * rows {
            let id = u16le(&raw, 8 + i * 2)? as usize;
            let tile = if id == 65535 {
                None
            } else {
                Some(crate::texture::decode(id, &pak.read(disc, id)?.0)?)
            };
            let Some(t) = tile.as_ref() else {
                continue;
            };
            if (t.info.width, t.info.height) != (256, 256) {
                return Err("Invalid radar tile size".into());
            }
            for y in 0..256 {
                let start = (i / cols * 256 + y + 1) * width + i % cols * 256 + 1;
                rgba[start..start + 256].copy_from_slice(&t.rgba[y * 256..(y + 1) * 256]);
            }
        }
        Ok(Self {
            origin: [u16le(&raw, 0)? as f32, u16le(&raw, 2)? as f32],
            texture: crate::texture::Texture {
                high_res: None,
                info: crate::texture::Info {
                    id: 331,
                    width,
                    height,
                    bits: 32,
                    colors: 0,
                },
                rgba,
            },
        })
    }
}
pub fn map_uv(world: [f32; 2], origin: [f32; 2], size: [f32; 2]) -> [f32; 2] {
    [
        ((world[0] + 3540.) * WORLD_SCALE - origin[0] + 1.) / size[0],
        ((2520. - world[1]) * WORLD_SCALE - origin[1] + 1.) / size[1],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_canvas_anchor_and_unequal_pixel_scales_are_preserved() {
        let s = scale();
        let c = center(false);
        let r = rect(false);
        assert_eq!(c[0], 128.);
        assert!((c[1] / s[1] - 224.).abs() < 0.0001);
        assert!((r[0] / s[0] - 10.).abs() < 0.0001);
        assert!((r[1] / s[1] - 186.).abs() < 0.0001);
        assert!(WORLD_SPAN > 190. && WORLD_SPAN < 190.1);
        assert_eq!(center(true)[1], c[1] - 130.);
    }
    #[test]
    fn original_gauge_range_threshold_and_blink() {
        assert_eq!(
            [
                vital_steps(0),
                vital_steps(64),
                vital_steps(127),
                vital_steps(255)
            ],
            [0, 64, 128, 128]
        );
        for tick in 0..64 {
            assert_eq!(
                health_color(24, tick),
                if tick & 28 == 28 {
                    None
                } else {
                    Some(LOW_HEALTH)
                }
            );
            assert_eq!(health_color(25, tick), Some(HEALTH));
        }
        let p = perimeter(64);
        assert!(p[0] <= -75. && p[1].abs() <= 1.);
        assert_eq!(perimeter(0), perimeter(256));
    }
    #[test]
    fn native_tile_projection_matches_directory_origin() {
        let p = [461. / WORLD_SCALE - 3540., 2520. - 160. / WORLD_SCALE];
        let uv = map_uv(p, [461., 160.], [2050., 1794.]);
        assert!((uv[0] * 2050. - 1.).abs() < 0.0002);
        assert!((uv[1] * 1794. - 1.).abs() < 0.0002);
        assert!(sprites(&[0; 4], [256, 128]).is_err());
    }
}
