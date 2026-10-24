// SPDX-License-Identifier: GPL-3.0-only
use crate::iso::{bytes, u16le, u32le, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;
pub fn sfo(d: &[u8]) -> Result<BTreeMap<String, Value>> {
    if bytes(d, 0, 4)? != b"\0PSF" || u32le(d, 4)? != 0x101 {
        return Err("Invalid PARAM.SFO signature/version".into());
    }
    let keys = u32le(d, 8)? as usize;
    let data = u32le(d, 12)? as usize;
    let count = u32le(d, 16)? as usize;
    if count > 4096 || keys < 20 + count * 16 || keys > data || data > d.len() {
        return Err("Invalid SFO tables".into());
    }
    let mut out = BTreeMap::new();
    for i in 0..count {
        let p = 20 + i * 16;
        let ko = keys + u16le(d, p)? as usize;
        if ko >= data {
            return Err("SFO key outside key table".into());
        }
        let keybytes = &d[ko..data];
        let end = keybytes
            .iter()
            .position(|&c| c == 0)
            .ok_or("Unterminated SFO key")?;
        let key = std::str::from_utf8(&keybytes[..end])?.to_string();
        let format = u16le(d, p + 2)?;
        let size = u32le(d, p + 4)? as usize;
        let max = u32le(d, p + 8)? as usize;
        if size > max {
            return Err("SFO length exceeds capacity".into());
        }
        let start = data
            .checked_add(u32le(d, p + 12)? as usize)
            .ok_or("SFO offset overflow")?;
        bytes(d, start, max)?;
        let raw = bytes(d, start, size)?;
        let value = match format {
            0x404 if size == 4 => json!(u32le(raw, 0)?),
            0x204 => {
                let text = raw.strip_suffix(&[0]).ok_or("Unterminated SFO value")?;
                json!(std::str::from_utf8(text)?)
            }
            _ => return Err(format!("Unsupported SFO format {format:#x}").into()),
        };
        if out.insert(key, value).is_some() {
            return Err("Duplicate SFO key".into());
        }
    }
    Ok(out)
}
pub fn executable(d: &[u8]) -> Result<Value> {
    if d.iter().all(|&x| x == 0) {
        return Ok(json!({"format":"zero-filled","executable":false}));
    }
    if d.starts_with(b"~PSP") {
        bytes(d, 0, 0x150)?;
        let name = bytes(d, 10, 28)?;
        let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
        Ok(
            json!({"format":"PSP encrypted PRX","module":String::from_utf8_lossy(&name[..end]),
          "declared_elf_size":u32le(d,0x28)?,"declared_psp_size":u32le(d,0x2c)?,
          "tag":format!("{:08x}",u32le(d,0xd0)?),"ready_for_static_recompilation":false}),
        )
    } else if d.starts_with(b"\x7fELF") {
        Ok(
            json!({"format":"ELF","ready_for_static_recompilation":false,"note":"ELF analysis not yet implemented"}),
        )
    } else {
        Ok(json!({"format":"unknown","ready_for_static_recompilation":false}))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prx_and_empty_boot_are_not_native_code() {
        assert_eq!(executable(&[0; 16]).unwrap()["executable"], false);
        assert!(executable(b"~PSP").is_err());
        let mut d = vec![0; 0x150];
        d[..4].copy_from_slice(b"~PSP");
        d[10..13].copy_from_slice(b"CTW");
        assert_eq!(executable(&d).unwrap()["module"], "CTW");
        assert_eq!(
            executable(&d).unwrap()["ready_for_static_recompilation"],
            false
        );
    }
    #[test]
    fn sfo_checks_key_and_data_tables() {
        let mut d = vec![0; 44];
        d[..4].copy_from_slice(b"\0PSF");
        d[4..8].copy_from_slice(&0x101u32.to_le_bytes());
        for (p, v) in [(8, 36u32), (12, 40), (16, 1), (24, 4), (28, 4)] {
            d[p..p + 4].copy_from_slice(&v.to_le_bytes());
        }
        d[22..24].copy_from_slice(&0x404u16.to_le_bytes());
        d[36] = b'A';
        d[40] = 7;
        assert_eq!(sfo(&d).unwrap()["A"], 7);
        d[20] = 4;
        assert!(sfo(&d).is_err());
    }
}
