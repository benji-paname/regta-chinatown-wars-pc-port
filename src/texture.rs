// SPDX-License-Identifier: GPL-3.0-only
//! PSP paletted texture resources, verified on ULES01347.
//! 16-byte header, GE swizzled pixels in 16-byte x 8-row tiles, RGBA palette.
use crate::iso::{bytes, u16le, u32le, Result};
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct Info {
    pub id: usize,
    pub width: usize,
    pub height: usize,
    pub bits: usize,
    pub colors: usize,
}
pub struct Texture {
    pub info: Info,
    pub rgba: Vec<[u8; 4]>,
    pub high_res: Option<std::sync::Arc<crate::texture_pack::Replacement>>,
}
/// PSP animated texture: type 0xff, a count followed by (texture, duration) u16 pairs.
pub fn animation(d: &[u8]) -> Result<Option<Vec<(u16, u16)>>> {
    if u32le(d, 0)? != 255 {
        return Ok(None);
    }
    if bytes(d, 6, 2)? != [1, 0] {
        return Err("Unsupported animated texture flags".into());
    }
    let n = u16le(d, 16)? as usize;
    if n == 0 || n > 1024 {
        return Err("Invalid animated texture count".into());
    }
    bytes(d, 18, n * 4)?;
    let mut frames = Vec::new();
    for k in 0..n {
        let id = u16le(d, 18 + k * 4)?;
        let duration = u16le(d, 20 + k * 4)?;
        if duration == 0 {
            return Err("Zero animated texture duration".into());
        }
        frames.push((id, duration));
    }
    Ok(Some(frames))
}
pub fn frame(frames: &[(u16, u16)], ticks: usize) -> u16 {
    let total: usize = frames.iter().map(|v| v.1 as usize).sum();
    let mut t = ticks % total;
    for &(id, n) in frames {
        if t < n as usize {
            return id;
        }
        t -= n as usize;
    }
    frames[0].0
}
pub fn info(id: usize, d: &[u8]) -> Result<Option<Info>> {
    if d.len() < 16 {
        return Ok(None);
    }
    let typ = u32le(d, 0)?;
    if ![4, 5].contains(&typ) || d[4] > 9 || d[5] > 9 || d[6] != 0 || d[7] > 1 || u32le(d, 8)? != 0
    {
        return Ok(None);
    }
    let bits = if typ == 4 { 4 } else { 8 };
    let (width, height) = (1usize << d[4], 1usize << d[5]);
    let pixel_bytes = (width * bits / 8).max(16) * height.max(8);
    let size = u32le(d, 12)? as usize;
    bytes(d, 16, size)?;
    let palette = size
        .checked_sub(pixel_bytes)
        .ok_or("Texture payload too short")?;
    if palette == 0 || palette % 4 != 0 || palette > 4 * (1 << bits) {
        return Err("Invalid texture palette".into());
    }
    Ok(Some(Info {
        id,
        width,
        height,
        bits,
        colors: palette / 4,
    }))
}
fn swizzle(x_byte: usize, y: usize, stride: usize) -> usize {
    ((y / 8) * (stride / 16) + x_byte / 16) * 128 + (y % 8) * 16 + x_byte % 16
}
pub fn decode(id: usize, d: &[u8]) -> Result<Texture> {
    let info = info(id, d)?.ok_or_else(|| {
        format!(
            "Unsupported PSP texture layout: resource {id}, header {:02x?}",
            &d[..d.len().min(16)]
        )
    })?;
    let stride = (info.width * info.bits / 8).max(16);
    let pal = 16 + stride * info.height.max(8);
    let mut rgba = Vec::with_capacity(info.width * info.height);
    for y in 0..info.height {
        for x in 0..info.width {
            let raw = d[16 + swizzle(x * info.bits / 8, y, stride)];
            let index = if info.bits == 4 {
                (raw >> (4 * (x % 2))) & 15
            } else {
                raw
            } as usize;
            if index >= info.colors {
                return Err(format!("Texture {id}: palette index {index} out of bounds").into());
            }
            rgba.push(bytes(d, pal + index * 4, 4)?.try_into()?);
        }
    }
    let high_res = crate::texture_pack::replacement(id, d, [info.width, info.height])?;
    Ok(Texture {
        info,
        rgba,
        high_res,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn animation_uses_psp_frame_durations() {
        let mut d = vec![0; 26];
        d[0] = 255;
        d[6] = 1;
        d[16] = 2;
        d[18] = 10;
        d[20] = 5;
        d[22] = 11;
        d[24] = 10;
        let a = animation(&d).unwrap().unwrap();
        assert_eq!(frame(&a, 4), 10);
        assert_eq!(frame(&a, 5), 11);
        assert_eq!(frame(&a, 15), 10);
        d[24] = 0;
        assert!(animation(&d).is_err());
    }
    fn fixture() -> Vec<u8> {
        let mut d = vec![0; 16 + 256 + 64];
        d[0] = 4;
        d[4] = 6;
        d[5] = 3;
        d[12..16].copy_from_slice(&320u32.to_le_bytes());
        d[16 + swizzle(17, 6, 32)] = 0x21;
        d[16 + 256 + 4..16 + 256 + 12].copy_from_slice(&[255, 0, 0, 255, 0, 255, 0, 128]);
        d
    }
    #[test]
    fn swizzled_pixels_nibbles_and_alpha() {
        let t = decode(7, &fixture()).unwrap();
        assert_eq!(t.rgba[6 * 64 + 34], [255, 0, 0, 255]);
        assert_eq!(t.rgba[6 * 64 + 35], [0, 255, 0, 128]);
        assert_eq!(t.rgba[0], [0; 4]);
    }
    #[test]
    fn invalid_texture_sizes_fail() {
        let mut d = fixture();
        d[12..16].copy_from_slice(&255u32.to_le_bytes());
        assert!(decode(0, &d).is_err());
        assert!(decode(0, &fixture()[..20]).is_err());
    }
}
