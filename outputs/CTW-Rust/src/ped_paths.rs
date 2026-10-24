// SPDX-License-Identifier: GPL-3.0-only
//! PSP pavement section, adapted from ctw-native collision.cpp/.h
//! Copyright (C) 2026 Kryo.to. PSP envelope and scales checked against the ISO.
use crate::iso::{bytes, u16le, u32le, Result};

#[derive(Debug)]
pub struct Node {
    pub position: [f32; 3],
    pub flags: u16,
    pub cover: u8,
    pub neighbours: Vec<u16>,
}

#[derive(Debug)]
pub struct Bridge {
    pub node: u16,
    /// World directory index (x * 100 + y), not the packed sector value.
    pub cell: usize,
}

#[derive(Default, Debug)]
pub struct Paths {
    pub nodes: Vec<Node>,
    pub bridges: Vec<Bridge>,
    pub link_count: usize,
}

impl Paths {
    pub fn parse(s: &[u8]) -> Result<Self> {
        if s.is_empty() {
            return Ok(Self::default());
        }
        bytes(s, 0, 44)?;
        let offset = u32le(s, 32)? as usize;
        let links = u16le(s, 36)? as usize;
        let count = u16le(s, 38)? as usize;
        let bridges = s[42] as usize;
        if bridges > 8 || offset < 44 + links * 2 {
            return Err("Invalid PSP pavement header".into());
        }
        bytes(s, 44, links * 2)?;
        bytes(s, offset, count * 10)?;
        let mut out = Self {
            link_count: links,
            ..Self::default()
        };
        for i in 0..count {
            let p = offset + i * 10;
            let first = u16le(s, p)? as usize;
            let flags = u16le(s, p + 2)?;
            let n = ((flags >> 2) & 7) as usize;
            if first + n > links {
                return Err("PSP pavement link range out of bounds".into());
            }
            let neighbours = (first..first + n)
                .map(|j| u16le(s, 44 + j * 2))
                .collect::<Result<Vec<_>>>()?;
            if neighbours.iter().any(|&j| j as usize >= count) {
                return Err("PSP pavement neighbour out of bounds".into());
            }
            out.nodes.push(Node {
                position: [
                    u16le(s, p + 4)? as i16 as f32 / 8.,
                    u16le(s, p + 6)? as i16 as f32 / 8.,
                    s[p + 8] as i8 as f32 / 2.,
                ],
                flags,
                cover: s[p + 9],
                neighbours,
            });
        }
        for i in 0..bridges {
            let node = u16le(s, i * 4)?;
            let sector = (u16le(s, i * 4 + 2)? & 0x3fff) as usize;
            if node as usize >= count || sector >= 14000 {
                return Err("PSP pavement bridge out of bounds".into());
            }
            out.bridges.push(Bridge {
                node,
                cell: (sector % 140) * 100 + sector / 140,
            });
        }
        Ok(out)
    }
}

/// Checks reciprocal bridges without inventing links across missing sectors.
pub fn audit(cells: &[crate::collision::Cell]) -> Result<serde_json::Value> {
    let mut matched = 0;
    for (id, cell) in cells.iter().enumerate() {
        for b in &cell.paths.bridges {
            let other = cells.get(b.cell).ok_or("Missing pavement sector")?;
            let p = cell.paths.nodes[b.node as usize].position;
            if !other.paths.bridges.iter().any(|back| {
                let q = other.paths.nodes[back.node as usize].position;
                back.cell == id && p[..2] == q[..2]
            }) {
                return Err(format!("Unmatched PSP pavement bridge in cell {id}").into());
            }
            matched += 1;
        }
    }
    Ok(serde_json::json!({
        "cells_with_nodes": cells.iter().filter(|c| !c.paths.nodes.is_empty()).count(),
        "nodes": cells.iter().map(|c| c.paths.nodes.len()).sum::<usize>(),
        "links": cells.iter().map(|c| c.paths.link_count).sum::<usize>(),
        "matching_neighbour_bridges": matched
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Vec<u8> {
        let mut s = vec![0; 56];
        s[32..36].copy_from_slice(&46u32.to_le_bytes());
        s[36] = 1;
        s[38] = 1;
        s[48] = 4;
        s[50..52].copy_from_slice(&(-80i16).to_le_bytes());
        s[52..54].copy_from_slice(&160i16.to_le_bytes());
        s[54] = (-4i8) as u8;
        s
    }
    #[test]
    fn signed_coordinates_and_sector_order() {
        let mut s = sample();
        s[42] = 1;
        s[2..4].copy_from_slice(&(3u16 * 140 + 2).to_le_bytes());
        let p = Paths::parse(&s).unwrap();
        assert_eq!(p.nodes[0].position, [-10., 20., -2.]);
        assert_eq!(p.nodes[0].neighbours, [0]);
        assert_eq!(p.bridges[0].cell, 203);
    }
    #[test]
    fn rejects_truncation_and_bad_indices() {
        let s = sample();
        for n in 1..s.len() {
            assert!(Paths::parse(&s[..n]).is_err());
        }
        let mut bad = s.clone();
        bad[44] = 1;
        assert!(Paths::parse(&bad).is_err());
        let mut bad = s.clone();
        bad[46] = 1;
        assert!(Paths::parse(&bad).is_err());
        let mut bad = s;
        bad[42] = 9;
        assert!(Paths::parse(&bad).is_err());
    }
    #[test]
    fn missing_reciprocal_bridge_is_rejected() {
        let mut cells = vec![crate::collision::Cell::default()];
        let mut s = sample();
        s[42] = 1;
        s[2] = 1;
        cells[0].paths = Paths::parse(&s).unwrap();
        assert!(audit(&cells).is_err());
    }
}
