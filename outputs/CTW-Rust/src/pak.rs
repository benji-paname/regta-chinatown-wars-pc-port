// SPDX-License-Identifier: GPL-3.0-only
//! PSP PK02 container. Unlike the Android ctw-native format: 2048-byte pages,
//! eight segment thresholds, and a contiguous table at 0x2c before the payload.
use crate::iso::{bytes, u16le, u32le, Disc, Result};
use flate2::read::ZlibDecoder;
use serde::Serialize;
use std::io::Read;

pub const GAME: &str = "PSP_GAME/USRDIR/WAD/GAME.PAK";
pub const OVERLAY: &str = "PSP_GAME/USRDIR/WAD/OVERLAY/CTW_PSP_UMD_MASTER_SCEE/OVERLAYS.PAK";
const LIMIT: usize = 64 * 1024 * 1024;
#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub id: usize,
    pub offset: usize,
    pub size: usize,
}
#[derive(Debug, Serialize)]
pub struct Pak {
    pub path: String,
    pub segments: Vec<u32>,
    pub data_offset: usize,
    pub entries: Vec<Entry>,
}
impl Pak {
    pub fn parse(path: &str, header: &[u8], file_size: usize) -> Result<Self> {
        if bytes(header, 0, 4)? != b"20KP" {
            return Err("Not a PSP PK02 archive".into());
        }
        let count = u32le(header, 36)? as usize;
        let base = u32le(header, 40)? as usize;
        if count == 0
            || count > 1_000_000
            || base < 44 + count * 2
            || base > file_size
            || base % 2048 != 0
            || file_size % 2048 != 0
        {
            return Err("Invalid PSP PAK header".into());
        }
        bytes(header, 44, count * 2)?;
        let segments = (0..8)
            .map(|i| u32le(header, 4 + i * 4))
            .collect::<Result<Vec<_>>>()?;
        let mut previous = 0;
        let mut ended = false;
        for &s in &segments {
            if s == u32::MAX {
                ended = true;
                continue;
            }
            if ended || s == 0 || s as usize >= count || s <= previous {
                return Err("Invalid PAK segment threshold".into());
            }
            previous = s;
        }
        let mut starts = Vec::with_capacity(count + 1);
        for i in 0..count {
            let segment = segments
                .iter()
                .filter(|&&s| s != u32::MAX && i >= s as usize)
                .count();
            let page = u16le(header, 44 + 2 * i)? as usize + segment * 65536;
            let start = base
                .checked_add(page.checked_mul(2048).ok_or("PAK page overflow")?)
                .ok_or("PAK overflow")?;
            if start >= file_size || starts.last().is_some_and(|&last| start <= last) {
                return Err("Invalid PAK resource offset".into());
            }
            starts.push(start);
        }
        if starts[0] != base {
            return Err("First PAK block does not start at payload".into());
        }
        starts.push(file_size);
        let entries = starts
            .windows(2)
            .enumerate()
            .map(|(id, w)| Entry {
                id,
                offset: w[0],
                size: w[1] - w[0],
            })
            .collect();
        Ok(Self {
            path: path.into(),
            segments,
            data_offset: base,
            entries,
        })
    }
    pub fn open(disc: &mut Disc, path: &str) -> Result<Self> {
        let size = disc.entries.get(path).ok_or("PAK absent")?.size as usize;
        let h = disc.range(path, 0, 44)?;
        let count = u32le(&h, 36)? as usize;
        if count > 1_000_000 {
            return Err("PAK count exceeds limit".into());
        }
        let header = disc.range(path, 0, 44 + count * 2)?;
        Self::parse(path, &header, size)
    }
    pub fn read(&self, disc: &mut Disc, id: usize) -> Result<(Vec<u8>, bool)> {
        let e = self.entries.get(id).ok_or("PAK resource absent")?;
        if e.size > LIMIT {
            return Err("PAK resource exceeds limit".into());
        }
        unpack(&disc.range(&self.path, e.offset, e.size)?)
    }
}
pub fn unpack(raw: &[u8]) -> Result<(Vec<u8>, bool)> {
    if !raw.starts_with(b"10SA") {
        return Ok((raw.to_vec(), false));
    }
    let compressed = u32le(raw, 4)? as usize;
    let size = u32le(raw, 8)? as usize;
    if size > LIMIT || compressed == 0 {
        return Err("Invalid AS01 size".into());
    }
    let mut decoder = ZlibDecoder::new(bytes(raw, 12, compressed)?);
    let mut out = Vec::with_capacity(size.min(1024 * 1024));
    (&mut decoder).take(size as u64 + 1).read_to_end(&mut out)?;
    if out.len() != size || decoder.total_in() as usize != compressed {
        return Err("AS01 size/stream mismatch".into());
    }
    Ok((out, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn header(pages: &[u16], segment: u32) -> Vec<u8> {
        let mut h = vec![0; 44 + pages.len() * 2];
        h[..4].copy_from_slice(b"20KP");
        for i in 0..8 {
            h[4 + i * 4..8 + i * 4].copy_from_slice(&u32::MAX.to_le_bytes());
        }
        h[4..8].copy_from_slice(&segment.to_le_bytes());
        h[36..40].copy_from_slice(&(pages.len() as u32).to_le_bytes());
        h[40..44].copy_from_slice(&2048u32.to_le_bytes());
        for (i, p) in pages.iter().enumerate() {
            h[44 + i * 2..46 + i * 2].copy_from_slice(&p.to_le_bytes());
        }
        h
    }
    #[test]
    fn page_wrap_uses_explicit_segment() {
        let h = header(&[0, 65530, 5], 2);
        let p = Pak::parse("test", &h, (65536 + 7) * 2048 + 2048).unwrap();
        assert_eq!(p.entries[2].offset, 2048 + (65536 + 5) * 2048);
        assert_eq!(p.entries[2].size, 4096);
        assert!(Pak::parse("bad", &header(&[0, 65530, 5], u32::MAX), 200_000_000).is_err());
    }
    #[test]
    fn bad_tables_are_rejected() {
        assert!(Pak::parse("x", &header(&[0, 1], u32::MAX), 6144).is_ok());
        assert!(Pak::parse("x", &header(&[0, 1], u32::MAX), 4096).is_err());
        assert!(Pak::parse("x", &header(&[0, 0], u32::MAX), 6144).is_err());
        assert!(Pak::parse("x", &header(&[0, 1], 1), 6144).is_err());
    }
    #[test]
    fn compressed_roundtrip_and_declared_length() {
        use flate2::{write::ZlibEncoder, Compression};
        use std::io::Write;
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(b"PSP test data").unwrap();
        let z = enc.finish().unwrap();
        let mut raw = b"10SA".to_vec();
        raw.extend((z.len() as u32).to_le_bytes());
        raw.extend(13u32.to_le_bytes());
        raw.extend(z);
        assert_eq!(unpack(&raw).unwrap().0, b"PSP test data");
        raw[8] = 12;
        assert!(unpack(&raw).is_err());
        raw[8] = 14;
        assert!(unpack(&raw).is_err());
    }
}
