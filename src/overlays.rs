// SPDX-License-Identifier: GPL-3.0-only
//! GTA Chinatown Wars authentic PSP Code Overlays Manager (OVERLAYS.PAK).
//! The game holds 166 code overlays loaded dynamically into two memory slots:
//! - Slot 0 (0x08b87460): 114 mission and mode overlays (e.g. ken_a01, hes_a01, bik_b01).
//! - Slot 1 (0x08bb2480): 52 app and minigame overlays (e.g. safehouseapp, hudapp, emailapp).
//! Each overlay is stored as a zlib-compressed ELF within OVERLAYS.PAK (headers "20KP" and "10SA").

#![allow(dead_code)]

use flate2::read::ZlibDecoder;
use std::collections::HashMap;
use std::io::Read;

/// Overlays execution slot in PSP memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverlaySlot {
    /// Slot 0: Mission scripts and game modes (base 0x08b87460, up to 172 KB).
    Mission,
    /// Slot 1: Apps, HUD, PDA, and minigames (base 0x08bb2480, up to 87 KB).
    App,
}

impl OverlaySlot {
    pub fn base_address(self) -> u32 {
        match self {
            OverlaySlot::Mission => 0x08b87460,
            OverlaySlot::App => 0x08bb2480,
        }
    }

    pub fn from_address(addr: u32) -> Self {
        if addr >= 0x08bb2480 {
            OverlaySlot::App
        } else {
            OverlaySlot::Mission
        }
    }
}

/// Metadata describing one code overlay extracted from OVERLAYS.PAK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayInfo {
    pub index: u16,
    pub name: String,
    pub slot: OverlaySlot,
    pub load_address: u32,
    pub packed_size: u32,
    pub unpacked_size: u32,
    pub offset_in_pak: usize,
}

/// Catalog and Loader for all 166 Chinatown Wars PSP overlays.
pub struct OverlayManager {
    pub overlays: Vec<OverlayInfo>,
    by_name: HashMap<String, usize>,
}

impl Default for OverlayManager {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlayManager {
    pub fn new() -> Self {
        Self {
            overlays: Vec::new(),
            by_name: HashMap::new(),
        }
    }

    /// Parse authentic OVERLAYS.PAK binary data.
    pub fn parse(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 0x2C || &data[0..4] != b"20KP" {
            return Err("Invalid OVERLAYS.PAK magic (expected 20KP)");
        }

        let count = u32::from_le_bytes(data[0x24..0x28].try_into().map_err(|_| "slice error")?) as usize;
        let sec = u32::from_le_bytes(data[0x28..0x2C].try_into().map_err(|_| "slice error")?) as usize;
        if count < 2 || sec == 0 {
            return Err("Invalid overlay count or sector size");
        }

        let table_end = 0x2C + count * 2;
        if data.len() < table_end {
            return Err("Truncated sector table in OVERLAYS.PAK");
        }

        let mut starts = Vec::with_capacity(count);
        for i in 0..count {
            let off = 0x2C + i * 2;
            let s = u16::from_le_bytes(data[off..off + 2].try_into().map_err(|_| "slice error")?);
            starts.push(s);
        }

        let mut overlays = Vec::with_capacity(count.saturating_sub(1));
        let mut by_name = HashMap::with_capacity(count.saturating_sub(1));

        // Entry 0 is the name hash table. Real overlays start at index 1.
        for i in 1..count {
            let pak_off = 0x800 + (starts[i] as usize) * sec;
            if pak_off + 12 > data.len() {
                continue;
            }
            if &data[pak_off..pak_off + 4] != b"10SA" {
                continue;
            }

            let packed = u32::from_le_bytes(data[pak_off + 4..pak_off + 8].try_into().map_err(|_| "slice")?);
            let unpacked = u32::from_le_bytes(data[pak_off + 8..pak_off + 12].try_into().map_err(|_| "slice")?);

            let zlib_start = pak_off + 12;
            let zlib_end = zlib_start + (packed as usize);
            if zlib_end > data.len() {
                continue;
            }

            // Quick peek into ELF header if possible without full decompression
            // Or decode only the first few bytes to extract the section name
            if let Ok(name) = Self::extract_overlay_name(&data[zlib_start..zlib_end], unpacked as usize) {
                let slot = if name.ends_with("app")
                    || name.contains("hud")
                    || name.contains("trading")
                    || name.contains("safehouse")
                    || name.contains("crane")
                {
                    OverlaySlot::App
                } else {
                    OverlaySlot::Mission
                };

                let info = OverlayInfo {
                    index: i as u16,
                    name: name.clone(),
                    slot,
                    load_address: slot.base_address(),
                    packed_size: packed,
                    unpacked_size: unpacked,
                    offset_in_pak: pak_off,
                };

                let idx = overlays.len();
                overlays.push(info);
                by_name.insert(name.to_ascii_lowercase(), idx);
            }
        }

        Ok(Self { overlays, by_name })
    }

    /// Decompresses ELF and extracts section 1 name (.overlay_<name>).
    pub fn extract_overlay_name(zlib_bytes: &[u8], expected_size: usize) -> Result<String, &'static str> {
        let mut decoder = ZlibDecoder::new(zlib_bytes);
        let mut elf = Vec::with_capacity(expected_size.min(1024 * 1024));
        decoder.read_to_end(&mut elf).map_err(|_| "Zlib decompression failed")?;

        if elf.len() < 52 || &elf[0..4] != b"\x7fELF" {
            return Err("Invalid ELF magic in decompressed overlay");
        }

        let shoff = u32::from_le_bytes(elf[32..36].try_into().map_err(|_| "slice")?) as usize;
        let shstr = u16::from_le_bytes(elf[50..52].try_into().map_err(|_| "slice")?) as usize;

        if shoff + 40 > elf.len() {
            return Err("Invalid section header offset");
        }

        // Section 1 header is at shoff + 40
        let name_off = u32::from_le_bytes(elf[shoff + 40..shoff + 44].try_into().map_err(|_| "slice")?) as usize;

        // String table section header is at shoff + 40 * shstr
        let str_sh = shoff + 40 * shstr;
        if str_sh + 20 > elf.len() {
            return Err("Invalid string table section header");
        }
        let tab = u32::from_le_bytes(elf[str_sh + 16..str_sh + 20].try_into().map_err(|_| "slice")?) as usize;

        let str_start = tab + name_off;
        if str_start >= elf.len() {
            return Err("String table offset out of bounds");
        }

        let null_pos = elf[str_start..].iter().position(|&b| b == 0).unwrap_or(elf.len() - str_start);
        let raw_name = std::str::from_utf8(&elf[str_start..str_start + null_pos]).map_err(|_| "Non-utf8 name")?;

        let clean_name = if let Some(stripped) = raw_name.strip_prefix(".overlay_") {
            stripped
        } else {
            raw_name
        };

        Ok(clean_name.to_string())
    }

    /// Finds overlay info by official name (case-insensitive, e.g. "ken_a01", "safehouseapp").
    pub fn find_by_name(&self, name: &str) -> Option<&OverlayInfo> {
        let key = name.trim().to_ascii_lowercase();
        self.by_name.get(&key).and_then(|&idx| self.overlays.get(idx))
    }

    /// Total number of indexed overlays.
    pub fn count(&self) -> usize {
        self.overlays.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    /// Creates a minimal synthetic ELF with section 1 named ".overlay_testapp"
    fn create_synthetic_overlay_elf(name: &str) -> Vec<u8> {
        let mut elf = vec![0u8; 128];
        elf[0..4].copy_from_slice(b"\x7fELF"); // ELF magic
        elf[32..36].copy_from_slice(&64u32.to_le_bytes()); // shoff = 64
        elf[50..52].copy_from_slice(&2u16.to_le_bytes()); // shstr = 2 (section 2 is strtab)

        // String table data
        let mut strtab = vec![0u8]; // index 0 is empty string
        let name_idx = strtab.len() as u32;
        let full_name = format!(".overlay_{name}\0");
        strtab.extend_from_slice(full_name.as_bytes());

        let strtab_offset = 200u32;
        let strtab_len = strtab.len() as u32;

        // Section 1 header (at 64 + 40 = 104)
        let s1_off = 64 + 40;
        elf.resize(s1_off + 40 + 40, 0); // space for sec 0, 1, 2
        elf[s1_off..s1_off + 4].copy_from_slice(&name_idx.to_le_bytes()); // name_off

        // Section 2 header (strtab at 64 + 40*2 = 144)
        let s2_off = 64 + 40 * 2;
        elf[s2_off + 16..s2_off + 20].copy_from_slice(&strtab_offset.to_le_bytes()); // sh_offset = 200
        elf[s2_off + 20..s2_off + 24].copy_from_slice(&strtab_len.to_le_bytes()); // sh_size

        // Write strtab at offset 200
        elf.resize(strtab_offset as usize + strtab.len(), 0);
        elf[strtab_offset as usize..strtab_offset as usize + strtab.len()].copy_from_slice(&strtab);

        elf
    }

    #[test]
    fn synthetic_overlay_pak_roundtrip() {
        let elf1 = create_synthetic_overlay_elf("ken_a01");
        let elf2 = create_synthetic_overlay_elf("safehouseapp");

        let mut enc1 = ZlibEncoder::new(Vec::new(), Compression::default());
        enc1.write_all(&elf1).unwrap();
        let zlib1 = enc1.finish().unwrap();

        let mut enc2 = ZlibEncoder::new(Vec::new(), Compression::default());
        enc2.write_all(&elf2).unwrap();
        let zlib2 = enc2.finish().unwrap();

        // Build synthetic 20KP container with 3 entries (0: dummy hash, 1: ken_a01, 2: safehouseapp)
        let count = 3u32;
        let sec = 2048u32;
        let mut pak = vec![0u8; 0x800 + 4 * 2048];
        pak[0..4].copy_from_slice(b"20KP");
        pak[0x24..0x28].copy_from_slice(&count.to_le_bytes());
        pak[0x28..0x2C].copy_from_slice(&sec.to_le_bytes());

        // starts: entry 0 at sector 0, entry 1 at sector 1, entry 2 at sector 2
        pak[0x2C..0x2E].copy_from_slice(&0u16.to_le_bytes());
        pak[0x2E..0x30].copy_from_slice(&1u16.to_le_bytes());
        pak[0x30..0x32].copy_from_slice(&2u16.to_le_bytes());

        // Write entry 1 ("10SA")
        let off1 = 0x800 + 1 * 2048;
        pak[off1..off1 + 4].copy_from_slice(b"10SA");
        pak[off1 + 4..off1 + 8].copy_from_slice(&(zlib1.len() as u32).to_le_bytes());
        pak[off1 + 8..off1 + 12].copy_from_slice(&(elf1.len() as u32).to_le_bytes());
        pak[off1 + 12..off1 + 12 + zlib1.len()].copy_from_slice(&zlib1);

        // Write entry 2 ("10SA")
        let off2 = 0x800 + 2 * 2048;
        pak[off2..off2 + 4].copy_from_slice(b"10SA");
        pak[off2 + 4..off2 + 8].copy_from_slice(&(zlib2.len() as u32).to_le_bytes());
        pak[off2 + 8..off2 + 12].copy_from_slice(&(elf2.len() as u32).to_le_bytes());
        pak[off2 + 12..off2 + 12 + zlib2.len()].copy_from_slice(&zlib2);

        let mgr = OverlayManager::parse(&pak).expect("parse synthetic pak");
        assert_eq!(mgr.count(), 2);

        let ken = mgr.find_by_name("ken_a01").expect("find ken_a01");
        assert_eq!(ken.name, "ken_a01");
        assert_eq!(ken.slot, OverlaySlot::Mission);
        assert_eq!(ken.load_address, 0x08b87460);

        let safe = mgr.find_by_name("safehouseapp").expect("find safehouseapp");
        assert_eq!(safe.name, "safehouseapp");
        assert_eq!(safe.slot, OverlaySlot::App);
        assert_eq!(safe.load_address, 0x08bb2480);
    }
}
