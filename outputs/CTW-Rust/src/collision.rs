// SPDX-License-Identifier: GPL-3.0-only
//! Rust adaptation of ctw-native collision.cpp/.h (C) 2026 Kryo.to.
//! PSP cells have an 8-byte directory entry, not Android's packed 4-byte entry.
use crate::iso::{bytes, u16le, u32le, Result};
use flate2::read::ZlibDecoder;
use std::io::Read;
#[derive(Clone)]
pub struct BoxShape {
    pub c: [f32; 3],
    pub h: [f32; 3],
    pub angle: f32,
    pub flags: u16,
}
#[derive(Clone)]
pub struct Cylinder {
    pub p: [f32; 3],
    pub r: f32,
    pub h: f32,
}
/// PSP world.bin section 7 (sPackedCarGenData, 20 bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CarGen {
    pub p: [f32; 3],
    pub heading: i16,
    pub vehicle: u8,
    pub flags: u8,
    pub colour: u8,
}
impl CarGen {
    pub fn angle(&self) -> f32 {
        -(self.heading as f32) * std::f32::consts::TAU / 65536.
    }
}
/// PSP world.bin section 6 (sPackedPropData, 20 bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prop {
    pub prop: u16,
    pub kind: u16,
    pub heading: i16,
    pub state: u16,
    pub p: [f32; 3],
}
impl Prop {
    pub fn angle(&self) -> f32 {
        -(self.heading as f32) * std::f32::consts::TAU / 65536.
    }
}

/// PSP world.bin section 8 (sSectorEmitterData, 16 bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CityEmitter {
    pub kind: u8,
    pub p: [f32; 3],
}

#[derive(Default)]
pub struct Cell {
    pub paths: crate::ped_paths::Paths,
    pub car_gens: Vec<CarGen>,
    pub props: Vec<Prop>,
    pub emitters: Vec<CityEmitter>,
    pub boxes: Vec<BoxShape>,
    pub cylinders: Vec<Cylinder>,
    pub triangles: Vec<[[f32; 3]; 3]>,
    pub ground: Vec<u8>,
}
pub struct Collision {
    pub cells: Vec<Cell>,
}
#[derive(Clone, Copy, Debug)]
pub struct Ground {
    pub z: f32,
    pub water: bool,
}
fn fx(d: &[u8], p: usize) -> Result<f32> {
    Ok(u32le(d, p)? as i32 as f32 / 4096.)
}
fn xyz(d: &[u8], p: usize) -> Result<[f32; 3]> {
    Ok([fx(d, p)?, fx(d, p + 4)?, fx(d, p + 8)?])
}
impl Cell {
    fn parse(d: &[u8]) -> Result<Self> {
        let mut sections = Vec::new();
        let mut p = 0;
        for _ in 0..12 {
            let n = u32le(d, p)? as usize;
            p += 4;
            sections.push(bytes(d, p, n)?);
            p += n;
        }
        if p != d.len() {
            return Err("Unexpected PSP collision trailer".into());
        }
        let mut out = Self::default();
        for (idx, stride) in [(0, 28), (1, 24)] {
            let s = sections[idx];
            if s.is_empty() {
                continue;
            }
            let count = u32le(s, 0)? as usize;
            if count > 65536 || s.len() != 4 + count * stride {
                return Err("Invalid collision shape count".into());
            }
            for k in 0..count {
                let p = 4 + k * stride;
                if idx == 0 {
                    out.boxes.push(BoxShape {
                        c: xyz(s, p)?,
                        h: xyz(s, p + 12)?,
                        angle: u16le(s, p + 24)? as f32 * std::f32::consts::TAU / 65536.,
                        flags: u16le(s, p + 26)?,
                    });
                } else {
                    out.cylinders.push(Cylinder {
                        p: xyz(s, p)?,
                        r: fx(s, p + 12)?,
                        h: fx(s, p + 16)?,
                    });
                }
            }
        }
        let s = sections[2];
        if !s.is_empty() {
            let count = u32le(s, 0)? as usize;
            let mut p = 4;
            if count > 65536 {
                return Err("Invalid collision mesh count".into());
            }
            for _ in 0..count {
                let nv = u16le(s, p + 16)? as usize;
                let nt = u16le(s, p + 18)? as usize;
                p += 20;
                let v = (0..nv)
                    .map(|i| xyz(s, p + i * 12))
                    .collect::<Result<Vec<_>>>()?;
                p += nv * 12;
                bytes(s, p, nt * 40)?;
                for k in 0..nt {
                    let ids = &s[p + k * 40 + 12..p + k * 40 + 15];
                    if ids.iter().any(|&i| i as usize >= nv) {
                        return Err("Invalid collision triangle index".into());
                    }
                    out.triangles.push([
                        v[ids[0] as usize],
                        v[ids[1] as usize],
                        v[ids[2] as usize],
                    ]);
                }
                p += nt * 40;
            }
            if p != s.len() {
                return Err("Collision mesh length mismatch".into());
            }
        }
        if !sections[10].is_empty() && sections[10].len() != 400 {
            return Err("Invalid PSP ground map".into());
        }
        out.ground = sections[10].to_vec();
        out.paths = crate::ped_paths::Paths::parse(sections[3])?;
        let s = sections[6];
        if !s.is_empty() {
            let count = u32le(s, 0)? as usize;
            if count > 65536 || s.len() != 4 + count * 20 {
                return Err("Invalid PSP prop section".into());
            }
            for k in 0..count {
                let p = 4 + k * 20;
                out.props.push(Prop {
                    prop: u16le(s, p)?,
                    kind: u16le(s, p + 2)?,
                    heading: u16le(s, p + 4)? as i16,
                    state: u16le(s, p + 6)?,
                    p: xyz(s, p + 8)?,
                });
            }
        }
        let s = sections[7];
        if !s.is_empty() {
            let count = u32le(s, 0)? as usize;
            if count > 65536 || s.len() != 4 + count * 20 {
                return Err("Invalid PSP car generator section".into());
            }
            for k in 0..count {
                let p = 4 + k * 20;
                out.car_gens.push(CarGen {
                    p: xyz(s, p)?,
                    heading: u16le(s, p + 12)? as i16,
                    vehicle: s[p + 14],
                    flags: s[p + 15],
                    colour: s[p + 16],
                });
            }
        }
        let s = sections[8];
        if !s.is_empty() {
            let count = u32le(s, 0)? as usize;
            if count > 65536 || s.len() != 4 + count * 16 {
                return Err("Invalid PSP city emitter section".into());
            }
            for k in 0..count {
                let p = 4 + k * 16;
                out.emitters.push(CityEmitter {
                    kind: s[p],
                    p: xyz(s, p + 4)?,
                });
            }
        }
        Ok(out)
    }
}
impl Collision {
    pub fn nearby_cells(&self, x: f32, y: f32) -> impl Iterator<Item = &Cell> {
        Self::cell_xy(x, y).into_iter().flat_map(move |(cx, cy)| {
            (cx.saturating_sub(1)..=(cx + 1).min(139)).flat_map(move |x| {
                (cy.saturating_sub(1)..=(cy + 1).min(99)).map(move |y| &self.cells[x * 100 + y])
            })
        })
    }
    /// Strict standing-cylinder clearance, including overhead triangles. Unlike
    /// push_out this neither moves the player nor allows a half-unit step-up.
    pub fn body_clear(&self, p: [f32; 3], radius: f32, height: f32) -> bool {
        if !p.iter().all(|v| v.is_finite())
            || Self::cell_xy(p[0] - radius, p[1] - radius).is_none()
            || Self::cell_xy(p[0] + radius, p[1] + radius).is_none()
        {
            return false;
        }
        for cell in self.nearby_cells(p[0], p[1]) {
            for b in &cell.boxes {
                if p[2] + 0.015 >= b.c[2] + b.h[2] || p[2] + height - 0.015 <= b.c[2] - b.h[2] {
                    continue;
                }
                let q = b.local(p[0], p[1]);
                let dx = q[0] - q[0].clamp(-b.h[0], b.h[0]);
                let dy = q[1] - q[1].clamp(-b.h[1], b.h[1]);
                if dx * dx + dy * dy < (radius - 0.002).powi(2) {
                    return false;
                }
            }
            for b in &cell.cylinders {
                if p[2] + 0.015 >= b.p[2] + b.h || p[2] + height - 0.015 <= b.p[2] {
                    continue;
                }
                if (p[0] - b.p[0]).powi(2) + (p[1] - b.p[1]).powi(2)
                    < (radius + b.r - 0.002).powi(2)
                {
                    return false;
                }
            }
            for t in &cell.triangles {
                if triangle_overlaps_body(t, p, radius, height) {
                    return false;
                }
            }
        }
        true
    }
    /// Segment versus oriented boxes expanded by radius. Conservative at rounded corners.
    pub fn trace_boxes(
        &self,
        a: [f32; 3],
        b: [f32; 3],
        radius: f32,
        skip_flag2: bool,
    ) -> Option<(f32, [f32; 3])> {
        let minx = ((a[0].min(b[0]) - radius + 3500.) / 50.)
            .floor()
            .clamp(0., 139.) as usize;
        let maxx = ((a[0].max(b[0]) + radius + 3500.) / 50.)
            .floor()
            .clamp(0., 139.) as usize;
        let miny = ((a[1].min(b[1]) - radius + 2500.) / 50.)
            .floor()
            .clamp(0., 99.) as usize;
        let maxy = ((a[1].max(b[1]) + radius + 2500.) / 50.)
            .floor()
            .clamp(0., 99.) as usize;
        let mut best = None;
        let mut best_t = 1.1;
        for cx in minx..=maxx {
            for cy in miny..=maxy {
                for box_ in &self.cells[cx * 100 + cy].boxes {
                    if skip_flag2 && box_.flags & 2 != 0 {
                        continue;
                    }
                    let al = box_.local(a[0], a[1]);
                    let bl = box_.local(b[0], b[1]);
                    let p = [al[0], al[1], a[2] - box_.c[2]];
                    let q = [bl[0], bl[1], b[2] - box_.c[2]];
                    let mut near = 0f32;
                    let mut far = 1f32;
                    let mut normal = [0.; 3];
                    let mut valid = true;
                    for k in 0..3 {
                        let half = box_.h[k] + radius;
                        let d = q[k] - p[k];
                        if d.abs() < 1e-6 {
                            if p[k].abs() > half {
                                valid = false;
                                break;
                            }
                            continue;
                        }
                        let t1 = (-half - p[k]) / d;
                        let t2 = (half - p[k]) / d;
                        let lo = t1.min(t2);
                        let hi = t1.max(t2);
                        if lo > near {
                            near = lo;
                            normal = [0.; 3];
                            normal[k] = if d > 0. { -1. } else { 1. };
                        }
                        far = far.min(hi);
                        if near > far {
                            valid = false;
                            break;
                        }
                    }
                    if valid && near < best_t && normal != [0.; 3] {
                        let (s, c) = box_.angle.sin_cos();
                        best_t = near;
                        best = Some((
                            near,
                            [
                                c * normal[0] - s * normal[1],
                                s * normal[0] + c * normal[1],
                                normal[2],
                            ],
                        ));
                    }
                }
            }
        }
        best
    }
    pub fn parse(w: &[u8]) -> Result<Self> {
        bytes(w, 0, 14000 * 8)?;
        let mut cells = Vec::with_capacity(14000);
        for key in 0..14000 {
            let offset = u32le(w, key * 8)?;
            let size = u32le(w, key * 8 + 4)?;
            let n = (size & 65535) as usize;
            let expected = n + (size >> 16) as usize;
            let raw = bytes(w, (offset & 0x7fffffff) as usize, n)?;
            let mut d = Vec::new();
            if offset >> 31 != 0 {
                ZlibDecoder::new(raw)
                    .take(expected as u64 + 1)
                    .read_to_end(&mut d)?;
            } else {
                d.extend_from_slice(raw);
            }
            if d.len() != expected {
                return Err("PSP collision decompression length mismatch".into());
            }
            cells.push(Cell::parse(&d)?);
        }
        Ok(Self { cells })
    }
    pub fn cell_xy(x: f32, y: f32) -> Option<(usize, usize)> {
        if !x.is_finite()
            || !y.is_finite()
            || !(-3500. ..3500.).contains(&x)
            || !(-2500. ..2500.).contains(&y)
        {
            return None;
        }
        Some((
            ((x + 3500.) / 50.).floor() as usize,
            ((y + 2500.) / 50.).floor() as usize,
        ))
    }
    pub fn ground(&self, x: f32, y: f32, z: f32) -> Ground {
        let Some((cx, cy)) = Self::cell_xy(x, y) else {
            return Ground {
                z: -7.5,
                water: true,
            };
        };
        let cell = &self.cells[cx * 100 + cy];
        let mut g = Ground {
            z: -7.5,
            water: true,
        };
        if cell.ground.len() == 400 {
            let col = (((x + 3500.) - cx as f32 * 50.) / 1.25)
                .floor()
                .clamp(0., 39.) as usize;
            let row = (((y + 2500.) - cy as f32 * 50.) / 1.25)
                .floor()
                .clamp(0., 39.) as usize;
            let idx = row * 40 + col;
            match (cell.ground[idx / 4] >> ((idx % 4) * 2)) & 3 {
                1 if z >= -0.5 => {
                    g = Ground {
                        z: 0.,
                        water: false,
                    }
                }
                2 => g.z = -2.5,
                _ => {}
            }
        }
        for neighbor in self.nearby_cells(x, y) {
            for b in &neighbor.cylinders {
                let top = b.p[2] + b.h;
                if (x - b.p[0]).powi(2) + (y - b.p[1]).powi(2) < b.r * b.r
                    && top <= z + 0.5
                    && top > g.z
                {
                    g = Ground {
                        z: top,
                        water: false,
                    };
                }
            }
            for b in &neighbor.boxes {
                let [lx, ly] = b.local(x, y);
                let top = b.c[2] + b.h[2];
                if lx.abs() < b.h[0] && ly.abs() < b.h[1] && top <= z + 0.5 && top > g.z {
                    g = Ground {
                        z: top,
                        water: false,
                    };
                }
            }
            for t in &neighbor.triangles {
                if let Some(h) = triangle_height(t, x, y) {
                    if h > g.z && h <= z + 0.5 {
                        g = Ground { z: h, water: false };
                    }
                }
            }
        }
        g
    }
    pub fn push_out(&self, p: &mut [f32; 3], radius: f32, height: f32) -> bool {
        let Some((cx, cy)) = Self::cell_xy(p[0], p[1]) else {
            return false;
        };
        let mut hit = false;
        for _ in 0..3 {
            for x in cx.saturating_sub(1)..=(cx + 1).min(139) {
                for y in cy.saturating_sub(1)..=(cy + 1).min(99) {
                    let cell = &self.cells[x * 100 + y];
                    for b in &cell.boxes {
                        if p[2] + 0.5 >= b.c[2] + b.h[2] || p[2] + height <= b.c[2] - b.h[2] {
                            continue;
                        }
                        let [lx, ly] = b.local(p[0], p[1]);
                        let qx = lx.clamp(-b.h[0], b.h[0]);
                        let qy = ly.clamp(-b.h[1], b.h[1]);
                        let dx = lx - qx;
                        let dy = ly - qy;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist >= radius {
                            continue;
                        }
                        let (ux, uy) = if dist > 0.00001 {
                            (dx * (radius - dist) / dist, dy * (radius - dist) / dist)
                        } else {
                            let ex = b.h[0] + radius - lx.abs();
                            let ey = b.h[1] + radius - ly.abs();
                            if ex < ey {
                                (ex * if lx >= 0. { 1. } else { -1. }, 0.)
                            } else {
                                (0., ey * if ly >= 0. { 1. } else { -1. })
                            }
                        };
                        let (s, c) = b.angle.sin_cos();
                        p[0] += ux * c - uy * s;
                        p[1] += ux * s + uy * c;
                        hit = true;
                    }
                    for b in &cell.cylinders {
                        if p[2] + 0.5 >= b.p[2] + b.h || p[2] + height <= b.p[2] {
                            continue;
                        }
                        let dx = p[0] - b.p[0];
                        let dy = p[1] - b.p[1];
                        let d = (dx * dx + dy * dy).sqrt();
                        let r = radius + b.r;
                        if d < r {
                            if d > 1e-5 {
                                p[0] += dx * (r - d) / d;
                                p[1] += dy * (r - d) / d;
                            } else {
                                p[0] += r;
                            }
                            hit = true;
                        }
                    }
                    for t in &cell.triangles {
                        let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
                        let l = (n.iter().map(|v| v * v).sum::<f32>()).sqrt();
                        if l < 1e-5 || n[2].abs() / l > 0.65 {
                            continue;
                        }
                        // Conservative 2-D swept-body barrier for steep original mesh faces.
                        let lo = t.iter().map(|v| v[2]).fold(f32::INFINITY, f32::min);
                        let hi = t.iter().map(|v| v[2]).fold(f32::NEG_INFINITY, f32::max);
                        if p[2] + 0.5 >= hi || p[2] + height <= lo {
                            continue;
                        }
                        for k in 0..3 {
                            let a = t[k];
                            let b = t[(k + 1) % 3];
                            let dx = b[0] - a[0];
                            let dy = b[1] - a[1];
                            let ll = dx * dx + dy * dy;
                            if ll < 1e-4 {
                                continue;
                            }
                            let f = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / ll).clamp(0., 1.);
                            let ux = p[0] - a[0] - f * dx;
                            let uy = p[1] - a[1] - f * dy;
                            let d = (ux * ux + uy * uy).sqrt();
                            if d > 1e-5 && d < radius {
                                p[0] += ux * (radius - d) / d;
                                p[1] += uy * (radius - d) / d;
                                hit = true;
                            }
                        }
                    }
                }
            }
        }
        hit
    }
}
impl BoxShape {
    pub(crate) fn local(&self, x: f32, y: f32) -> [f32; 2] {
        let (s, c) = self.angle.sin_cos();
        let dx = x - self.c[0];
        let dy = y - self.c[1];
        [c * dx + s * dy, -s * dx + c * dy]
    }
}
fn triangle_overlaps_body(t: &[[f32; 3]; 3], p: [f32; 3], radius: f32, height: f32) -> bool {
    if (0..2)
        .any(|k| t.iter().all(|v| v[k] < p[k] - radius) || t.iter().all(|v| v[k] > p[k] + radius))
        || t.iter().all(|v| v[2] < p[2] + 0.015)
        || t.iter().all(|v| v[2] > p[2] + height - 0.015)
    {
        return false;
    }
    // Clip the triangle to the body's vertical slab, then test its projected
    // convex polygon against a circle. Covers steep faces and flat ceilings.
    let mut poly = t.to_vec();
    for (z, sign) in [(p[2] + 0.015, 1.), (p[2] + height - 0.015, -1.)] {
        if poly.is_empty() {
            return false;
        }
        let mut clipped = Vec::new();
        for i in 0..poly.len() {
            let a = poly[i];
            let b = poly[(i + 1) % poly.len()];
            let da = (a[2] - z) * sign;
            let db = (b[2] - z) * sign;
            if da >= 0. {
                clipped.push(a);
            }
            if (da >= 0.) != (db >= 0.) {
                let f = da / (da - db);
                clipped.push(std::array::from_fn(|k| a[k] + (b[k] - a[k]) * f));
            }
        }
        poly = clipped;
    }
    if poly.is_empty() {
        return false;
    }
    let mut positive = false;
    let mut negative = false;
    let mut area = 0.;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let side = dx * (p[1] - a[1]) - dy * (p[0] - a[0]);
        positive |= side > 1e-5;
        negative |= side < -1e-5;
        // Translation avoids loss of precision at the map's large coordinates.
        area += (a[0] - p[0]) * (b[1] - p[1]) - (a[1] - p[1]) * (b[0] - p[0]);
        let ll = dx * dx + dy * dy;
        let f = if ll > 1e-10 {
            (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / ll).clamp(0., 1.)
        } else {
            0.
        };
        if (a[0] + f * dx - p[0]).powi(2) + (a[1] + f * dy - p[1]).powi(2)
            < (radius - 0.002).powi(2)
        {
            return true;
        }
    }
    area.abs() > 1e-5 && !(positive && negative)
}
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn triangle_height(t: &[[f32; 3]; 3], x: f32, y: f32) -> Option<f32> {
    let a = sub(t[1], t[0]);
    let b = sub(t[2], t[0]);
    let det = a[0] * b[1] - a[1] * b[0];
    if det.abs() < 1e-6 {
        return None;
    }
    let dx = x - t[0][0];
    let dy = y - t[0][1];
    let u = (dx * b[1] - dy * b[0]) / det;
    let v = (a[0] * dy - a[1] * dx) / det;
    if u >= -1e-5 && v >= -1e-5 && u + v <= 1.00001 {
        Some(t[0][2] + u * a[2] + v * b[2])
    } else {
        None
    }
}
// On-foot movement adapted from ctw-native Player (C) 2026 Kryo.to.
// Original speed levels, turn limit, acceleration counters and gravity;
// static collision resolution remains our Rust substep/slide approximation.
pub struct Walker {
    pub p: [f32; 3],
    pub heading: f32,
    pub distance: f32,
    pub contacts: usize,
    pub level: u8,
    pub velocity: [f32; 3],
    pub grounded: bool,
    pub vault: Option<crate::vault::Vault>,
    pub vault_ready: bool,
    pub vaults_completed: usize,
    pub vaults_aborted: usize,
    pub vaults_rejected: usize,
    pub roll: Option<crate::roll::Roll>,
    pub rolls_completed: usize,
    pub rolls_blocked: usize,
    action_pending: bool,
    vault_buffer: f32,
    counter: u8,
    slowing: bool,
    exhaustion: u8,
    sprint_was_down: bool,
    angle: u16,
}
impl Walker {
    pub fn new(c: &Collision, xy: [f32; 2]) -> Result<Self> {
        let g = c.ground(xy[0], xy[1], 0.);
        if g.water {
            return Err("Le point de depart est dans l'eau".into());
        }
        let mut p = [xy[0], xy[1], g.z];
        c.push_out(&mut p, 0.875, 2.0);
        let g = c.ground(p[0], p[1], p[2]);
        p[2] = g.z;
        Ok(Self {
            p,
            heading: 0.,
            distance: 0.,
            contacts: 0,
            level: 0,
            velocity: [0.; 3],
            grounded: true,
            vault: None,
            vault_ready: false,
            vaults_completed: 0,
            vaults_aborted: 0,
            vaults_rejected: 0,
            roll: None,
            rolls_completed: 0,
            rolls_blocked: 0,
            action_pending: false,
            vault_buffer: 0.,
            counter: 0,
            slowing: false,
            exhaustion: 0,
            sprint_was_down: false,
            angle: 0,
        })
    }
    pub fn speed_fixed(&self) -> i32 {
        self.level as i32 * 819 * 30
    }
    pub fn set_heading(&mut self, clockwise_degrees: f32) {
        self.angle = (clockwise_degrees * 65536. / 360.) as i32 as u16;
        self.heading = -(self.angle as i16 as f32) * std::f32::consts::TAU / 65536.;
    }
    pub fn request_vault(&mut self, c: &Collision) -> bool {
        if self.vault.is_some() || self.roll.is_some() || !self.grounded {
            return false;
        }
        let Some(plan) =
            crate::vault::plan_moving(c, self.p, self.heading, self.speed_fixed() as f32 / 4096.)
        else {
            return false;
        };
        self.vault = Some(crate::vault::Vault {
            plan,
            elapsed: 0.,
            entry_level: self.level,
        });
        self.velocity = [0.; 3];
        self.level = 0;
        self.counter = 0;
        self.slowing = false;
        self.grounded = false;
        self.vault_ready = false;
        true
    }
    pub fn queue_vault(&mut self) {
        if self.vault.is_none() && self.roll.is_none() && self.grounded {
            self.vault_buffer = 0.18;
        }
    }
    pub fn cancel_vault_input(&mut self) {
        self.vault_buffer = 0.;
        self.action_pending = false;
    }
    pub fn queue_action(&mut self) {
        // One press, one action; never queue a second action while traversing.
        if self.vault.is_none() && self.roll.is_none() && self.grounded {
            self.action_pending = true;
        }
    }
    fn update_speed(&mut self, moving: bool, desired: u16, sprint: bool, walk: bool) {
        let released = self.sprint_was_down && !sprint;
        self.sprint_was_down = sprint;
        let diff = self.angle.wrapping_sub(desired) as i16 as i32;
        let limit = if !sprint && self.exhaustion < 11 {
            19000
        } else {
            32000
        };
        let slow = (!moving && !sprint && self.exhaustion < 11) || (self.slowing && self.level > 1);
        if slow {
            if self.level == 0 {
                self.slowing = false;
            } else if !self.slowing {
                self.slowing = true;
                self.counter = 1;
            } else if self.counter > 2 {
                self.counter = 0;
                self.level -= 1;
            } else {
                self.counter += 1;
            }
        } else {
            if moving && diff.abs() >= limit {
                self.level = 0;
                self.counter = 0;
            } else if self.slowing {
                self.slowing = false;
                self.counter = 0;
            }
            if self.level == 0 {
                if self.counter > 1 {
                    self.level = 1;
                    self.counter = 0;
                } else {
                    self.counter += 1;
                }
            } else if self.counter < 8 {
                self.counter += 1;
            } else if self.level < 3 {
                self.level += 1;
                self.counter = 0;
            } else {
                self.counter = 0;
            }
            self.level = self.level.min(if walk { 1 } else { 3 });
        }
        if self.exhaustion == 0 {
            if released {
                self.exhaustion = 10;
            }
        } else {
            let e = self.exhaustion - 1;
            self.exhaustion = if e == 10 { 0 } else { e };
            if released {
                self.exhaustion = 20;
            }
        }
        if self.level == 3 {
            if !sprint && self.exhaustion < 10 {
                self.level = 2;
            } else if self.exhaustion <= 9 {
                self.exhaustion = 10;
            }
        }
    }
    pub fn step(&mut self, c: &Collision, d: [f32; 2], dt: f32, sprint: bool, walk: bool) {
        if std::mem::take(&mut self.action_pending) && self.grounded {
            if !self.request_vault(c) {
                self.roll = Some(crate::roll::Roll::new(self.heading, self.level));
                self.level = 0;
                self.counter = 0;
                self.slowing = false;
                self.velocity = [0.; 3];
                self.vault_ready = false;
            }
        }
        if let Some(mut roll) = self.roll.take() {
            let old = self.p;
            let delta = dt
                .clamp(0., 0.1)
                .min((crate::roll::DURATION - roll.elapsed).max(0.));
            if !roll.blocked && !roll.travel(c, &mut self.p, delta) {
                roll.blocked = true;
                self.rolls_blocked += 1;
            }
            roll.elapsed += delta;
            self.heading = roll.heading;
            self.velocity = std::array::from_fn(|i| (self.p[i] - old[i]) / dt.max(1e-6));
            self.distance += (self.p[0] - old[0]).hypot(self.p[1] - old[1]);
            if roll.elapsed + 1e-6 >= crate::roll::DURATION {
                self.rolls_completed += 1;
                self.velocity = [0.; 3];
                if d[0].hypot(d[1]) > 1e-4 {
                    self.level = roll.entry_level.min(if walk {
                        1
                    } else if sprint {
                        3
                    } else {
                        2
                    });
                }
            } else {
                self.roll = Some(roll);
            }
            return;
        }
        if self.vault_buffer > 0. {
            if self.request_vault(c) {
                self.vault_buffer = 0.;
            } else {
                self.vault_buffer = (self.vault_buffer - dt).max(0.);
                if self.vault_buffer == 0. {
                    self.vaults_rejected += 1;
                }
            }
        }
        if let Some(mut vault) = self.vault.take() {
            let previous = vault.progress();
            vault.elapsed += dt.clamp(0., 0.1);
            let progress = vault.progress();
            if !vault.plan.clear_between(c, previous, progress) {
                // Stay at the last safe point; ordinary gravity resumes next tick.
                self.velocity = [0.; 3];
                self.vaults_aborted += 1;
                return;
            }
            let old = self.p;
            self.p = vault.plan.position(progress);
            let target = (-vault.plan.heading * 65536. / std::f32::consts::TAU) as i32 as u16;
            let delta = target.wrapping_sub(self.angle) as i16 as i32;
            self.angle = self.angle.wrapping_add(delta.clamp(-4000, 4000) as u16);
            self.heading = -(self.angle as i16 as f32) * std::f32::consts::TAU / 65536.;
            self.distance += ((self.p[0] - old[0]).powi(2) + (self.p[1] - old[1]).powi(2)).sqrt();
            self.velocity = std::array::from_fn(|i| (self.p[i] - old[i]) / dt.max(1e-6));
            if progress >= 1. {
                self.grounded = true;
                self.velocity = [0.; 3];
                self.vaults_completed += 1;
                if d[0] * d[0] + d[1] * d[1] > 1e-8 {
                    self.level = vault.entry_level.min(if walk {
                        1
                    } else if sprint {
                        3
                    } else {
                        2
                    });
                    let speed = self.speed_fixed() as f32 / 4096.;
                    self.velocity = [-self.heading.sin() * speed, self.heading.cos() * speed, 0.];
                }
            } else {
                self.vault = Some(vault);
            }
            return;
        }
        let moving = d[0] * d[0] + d[1] * d[1] > 1e-8;
        let desired = if moving {
            (d[0].atan2(d[1]) * 65536. / std::f32::consts::TAU) as i32 as u16
        } else {
            self.angle
        };
        self.update_speed(moving, desired, sprint, walk);
        if self.grounded {
            let a = self.angle as f32 * std::f32::consts::TAU / 65536.;
            let speed = self.speed_fixed() as f32 / 4096.;
            self.velocity[0] = (a.sin() * 4096.) as i32 as f32 / 4096. * speed;
            self.velocity[1] = (a.cos() * 4096.) as i32 as f32 / 4096. * speed;
        }
        if moving {
            let diff = desired.wrapping_sub(self.angle) as i16 as i32;
            self.angle = self.angle.wrapping_add(diff.clamp(-4000, 4000) as u16);
            self.heading = -(self.angle as i16 as f32) * std::f32::consts::TAU / 65536.;
        }
        // Original integration factor is 17/512 per game tick.
        let elapsed = dt.clamp(0., 0.1) * 30. * 17. / 512.;
        let horizontal = (self.velocity[0].powi(2) + self.velocity[1].powi(2)).sqrt() * elapsed;
        let n = (horizontal / 0.1).ceil().max(1.) as usize;
        for _ in 0..n {
            let old = self.p;
            let mut p = old;
            p[0] += self.velocity[0] * elapsed / n as f32;
            p[1] += self.velocity[1] * elapsed / n as f32;
            let hit = c.push_out(&mut p, 0.875, 2.);
            let g = c.ground(p[0], p[1], old[2]);
            if !g.water && g.z <= old[2] + 0.5 {
                if self.grounded && g.z >= old[2] - 1.5 {
                    p[2] = g.z;
                    self.velocity[2] = 0.;
                } else {
                    self.grounded = false;
                }
                self.p = p;
                self.distance += ((p[0] - old[0]).powi(2) + (p[1] - old[1]).powi(2)).sqrt();
            }
            if hit {
                self.contacts += 1;
            }
        }
        if !self.grounded {
            let g = c.ground(self.p[0], self.p[1], self.p[2]);
            self.p[2] += self.velocity[2] * elapsed;
            self.velocity[2] -= 0x1ed0 as f32 / 4096. * dt * 30.;
            if !g.water && self.p[2] <= g.z {
                self.p[2] = g.z;
                self.velocity[2] = 0.;
                self.grounded = true;
            }
        }
        self.vault_ready = self.grounded
            && crate::vault::plan_moving(
                c,
                self.p,
                self.heading,
                self.speed_fixed() as f32 / 4096.,
            )
            .is_some();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn flat_world() -> Collision {
        Collision {
            cells: (0..14000)
                .map(|_| Cell {
                    ground: vec![0x55; 400],
                    ..Default::default()
                })
                .collect(),
        }
    }
    #[test]
    fn acceleration_braking_and_walk_limit_follow_reference_levels() {
        let c = flat_world();
        let mut w = Walker::new(&c, [0., 0.]).unwrap();
        w.step(&c, [0., 1.], 1. / 30., false, false);
        assert_eq!(w.level, 0);
        for _ in 0..30 {
            w.step(&c, [0., 1.], 1. / 30., false, false);
        }
        assert_eq!(w.level, 2);
        assert!((w.velocity[1] - 11.99707).abs() < 0.001);
        let y = w.p[1];
        for _ in 0..12 {
            w.step(&c, [0.; 2], 1. / 30., false, false);
        }
        assert_eq!(w.level, 0);
        assert!(w.p[1] > y && w.p[1] - y < 4.);
        for _ in 0..40 {
            w.step(&c, [0., 1.], 1. / 30., false, true);
        }
        assert_eq!(w.level, 1);
        assert!(w.velocity[1] < 6.);
    }
    #[test]
    fn sprint_release_and_sharp_turn_do_not_snap_direction() {
        let c = flat_world();
        let mut w = Walker::new(&c, [0., 0.]).unwrap();
        for _ in 0..35 {
            w.step(&c, [0., 1.], 1. / 30., true, false);
        }
        assert_eq!(w.level, 3);
        assert!((w.velocity[1] - 17.9956).abs() < 0.001);
        w.step(&c, [0., 1.], 1. / 30., false, false);
        assert_eq!(w.level, 3);
        for _ in 0..12 {
            w.step(&c, [0., 1.], 1. / 30., false, false);
        }
        assert_eq!(w.level, 2);
        w.step(&c, [0., -1.], 1. / 30., false, false);
        assert_eq!(w.level, 0);
        assert_eq!(w.angle as i16, -4000);
    }
    #[test]
    fn unsupported_character_falls_and_lands_without_teleporting() {
        let c = flat_world();
        let mut w = Walker::new(&c, [0., 0.]).unwrap();
        w.p[2] = 5.;
        w.grounded = false;
        w.step(&c, [0.; 2], 1. / 30., false, false);
        assert!(w.p[2] > 4.9);
        for _ in 0..90 {
            w.step(&c, [0.; 2], 1. / 30., false, false);
        }
        assert!(w.grounded);
        assert_eq!(w.p[2], 0.);
        assert_eq!(w.velocity[2], 0.);
    }
    #[test]
    fn ground_triangle_and_world_limits() {
        assert_eq!(Collision::cell_xy(-3500., -2500.), Some((0, 0)));
        assert!(Collision::cell_xy(3500., 0.).is_none());
        let t = [[0., 0., 0.], [2., 0., 2.], [0., 2., 0.]];
        assert_eq!(triangle_height(&t, 0.5, 0.5), Some(0.5));
        assert!(triangle_height(&t, 2., 2.).is_none());
    }
    #[test]
    fn rotated_wall_blocks_walking_and_water_is_rejected() {
        let mut c = Collision {
            cells: (0..14000)
                .map(|_| Cell {
                    ground: vec![0x55; 400],
                    ..Cell::default()
                })
                .collect(),
        };
        c.cells[70 * 100 + 50].boxes.push(BoxShape {
            c: [3., 3., 2.],
            h: [0.5, 50., 2.],
            angle: 0.3,
            flags: 0,
        });
        let mut w = Walker::new(&c, [0., 3.]).unwrap();
        for _ in 0..200 {
            w.step(&c, [1., 0.], 1. / 30., true, false);
        }
        assert!(w.contacts > 0);
        assert!(c.cells[70 * 100 + 50].boxes[0].local(w.p[0], w.p[1])[0] <= -0.94);
        c.cells[70 * 100 + 50].ground = vec![0; 400];
        assert!(Walker::new(&c, [10., 10.]).is_err());
    }
    #[test]
    fn truncated_sections_fail() {
        assert!(Cell::parse(&[255; 48]).is_err());
    }
    #[test]
    fn support_crossing_a_cell_boundary_is_not_lost() {
        let mut c = flat_world();
        c.cells[7050].boxes.push(BoxShape {
            c: [50., 10., 1.],
            h: [2., 5., 1.],
            angle: 0.,
            flags: 0,
        });
        // Shape belongs to the previous cell, but spans the x=50 boundary.
        assert_eq!(c.ground(50.5, 10., 2.).z, 2.);
        assert!(c.body_clear([50.5, 10., 2.], 0.875, 2.));
        assert!(!c.body_clear([50.5, 10., 0.], 0.875, 2.));
    }
}
