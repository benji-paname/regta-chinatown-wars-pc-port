// SPDX-License-Identifier: GPL-3.0-only
// Name hashing and GXT layout adapted from ctw-native (C) 2026 Kryo.to.
// PSP TOC extents independently verified against ULES01347 1.01.
use crate::iso::{bytes, u16le, u32le, Disc, Result};
use serde::Serialize;
use std::collections::BTreeMap;

pub const PATH: &str = "PSP_GAME/USRDIR/WAD/ROM.WAD";

pub fn name_hash(name: &str) -> u32 {
    let mut hash = 0u32;
    for c in name.bytes() {
        hash = hash
            .wrapping_add(c.to_ascii_uppercase() as u32)
            .wrapping_mul(1025);
        hash ^= hash >> 6;
    }
    hash = hash.wrapping_mul(9);
    (hash ^ (hash >> 11)).wrapping_mul(32769)
}

#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    pub hash: u32,
    pub offset: u32,
    pub size: u32,
}

pub fn parse_toc(data: &[u8], wad_size: u64) -> Result<BTreeMap<u32, Entry>> {
    if data.is_empty() || data.len() % 12 != 0 {
        return Err("Invalid ROM.TOC length".into());
    }
    let mut entries = BTreeMap::new();
    let mut last = None;
    for row in data.chunks_exact(12) {
        let e = Entry {
            hash: u32le(row, 0)?,
            offset: u32le(row, 4)?,
            size: u32le(row, 8)?,
        };
        if last.is_some_and(|h| e.hash <= h) {
            return Err("Unsorted or duplicate TOC hash".into());
        }
        if e.offset as u64 + e.size as u64 > wad_size {
            return Err("WAD extent out of bounds".into());
        }
        last = Some(e.hash);
        entries.insert(e.hash, e);
    }
    Ok(entries)
}

pub struct Wad {
    pub entries: BTreeMap<u32, Entry>,
}
impl Wad {
    pub fn open(disc: &mut Disc) -> Result<Self> {
        let size = disc.entries.get(PATH).ok_or("ROM.WAD absent")?.size as u64;
        Ok(Self {
            entries: parse_toc(&disc.read("PSP_GAME/USRDIR/WAD/ROM.TOC")?, size)?,
        })
    }
    pub fn read(&self, disc: &mut Disc, hash: u32) -> Result<Vec<u8>> {
        let e = self.entries.get(&hash).ok_or("WAD resource absent")?;
        disc.range(PATH, e.offset as usize, e.size as usize)
    }
}

/// Keep original 16-bit units: game text contains font indices/control codes,
/// which must not be assumed to be ordinary Unicode in the future text renderer.
pub fn gxt(data: &[u8]) -> Result<Vec<Vec<u16>>> {
    if bytes(data, 0, 6)? != b"DS_GXT" {
        return Err("Not a DS_GXT resource".into());
    }
    let count = u16le(data, 6)? as usize;
    let mut at = 8;
    let mut strings = Vec::with_capacity(count);
    for _ in 0..count {
        let n = u16le(data, at)? as usize;
        at += 2;
        let raw = bytes(data, at, n * 2)?;
        at += n * 2;
        strings.push(
            raw.chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect(),
        );
    }
    if at != data.len() {
        return Err("Trailing GXT data".into());
    }
    Ok(strings)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verified_psp_names() {
        assert_eq!(name_hash("world.bin"), 0xf3487577);
        assert_eq!(name_hash("SS_Hud.bin"), 0x23cfa1de);
        assert_eq!(name_hash("ss_hud.BIN"), name_hash("SS_Hud.bin"));
        assert_eq!(name_hash("utility.bin"), 0x4db179af);
    }
    #[test]
    fn toc_rejects_duplicates_truncation_and_oob() {
        let row = [1u32, 32, 8]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>();
        assert!(parse_toc(&row, 40).is_ok());
        assert!(parse_toc(&row, 39).is_err());
        assert!(parse_toc(&row[..11], 40).is_err());
        assert!(parse_toc(&[row.clone(), row].concat(), 40).is_err());
    }
    #[test]
    fn gxt_preserves_game_codes_and_checks_end() {
        let mut b = b"DS_GXT\x01\x00\x02\x00A\x00\xf1\xfe".to_vec();
        assert_eq!(gxt(&b).unwrap(), vec![vec![65, 0xfef1]]);
        assert!(gxt(&b[..b.len() - 1]).is_err());
        b.push(0);
        assert!(gxt(&b).is_err());
    }
}
