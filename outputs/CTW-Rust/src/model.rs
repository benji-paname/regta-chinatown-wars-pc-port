// SPDX-License-Identifier: GPL-3.0-only
//! Initial PSP model parser. This is NOT the Android MG layout.
//! Hierarchy/strip approach follows ctw-native gfx/model.cpp (C) 2026 Kryo.to;
//! offsets below were measured on PSP resources 30, 7207 and 17463.
//! Supports the observed 12-byte vertices and one strip per material only.
use crate::iso::{bytes, u16le, u32le, Result};
#[derive(Clone, Copy)]
struct Matrix {
    r: [[f32; 3]; 3],
    t: [f32; 3],
}
impl Matrix {
    fn identity() -> Self {
        Self {
            r: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            t: [0.; 3],
        }
    }
    fn point(&self, p: [f32; 3]) -> [f32; 3] {
        std::array::from_fn(|a| {
            self.r[a][0] * p[0] + self.r[a][1] * p[1] + self.r[a][2] * p[2] + self.t[a]
        })
    }
    fn compose(&self, b: Self) -> Self {
        Self {
            r: std::array::from_fn(|a| {
                std::array::from_fn(|c| (0..3).map(|k| self.r[a][k] * b.r[k][c]).sum())
            }),
            t: self.point(b.t),
        }
    }
}
fn f32le(d: &[u8], p: usize) -> Result<f32> {
    let x = f32::from_bits(u32le(d, p)?);
    if !x.is_finite() {
        return Err("Non-finite PSP model coordinate".into());
    }
    Ok(x)
}
pub struct Mesh {
    pub triangles: Vec<[[f32; 3]; 3]>,
    pub surfaces: Vec<Surface>,
    pub vertices: usize,
    pub nodes: usize,
    pub texture_ids: Vec<u16>,
    pub declared_bounds: [[f32; 3]; 2],
}
#[derive(Clone, Copy)]
pub struct Vertex {
    pub p: [f32; 3],
    pub uv: [f32; 2],
}
pub struct Surface {
    pub v: [Vertex; 3],
    pub texture: u16,
    pub flags: u8,
    pub alpha: u8,
}
pub fn parse(d: &[u8]) -> Result<Mesh> {
    let nodes = u16le(d, 0)? as usize;
    let materials = u16le(d, 2)? as usize;
    let strips = u16le(d, 4)? as usize;
    let vertices = u16le(d, 6)? as usize;
    if nodes == 0
        || nodes > 256
        || materials == 0
        || materials != strips
        || materials > 256
        || vertices < 3
    {
        return Err("Unsupported PSP model counts".into());
    }
    let extra = u16le(d, 12)? as usize;
    if extra > 65536 {
        return Err("Unsupported PSP model auxiliary data".into());
    }
    let mat0 = 24 + nodes * 52;
    let strip0 = mat0 + materials * 8;
    let collision =
        u16le(d, 14)? as usize * 6 + u16le(d, 16)? as usize * 6 + u16le(d, 18)? as usize * 8;
    let vertex0 = strip0 + strips * 16 + extra * 24 + collision;
    bytes(d, 0, vertex0 + vertices * 12)?;
    let scale = f32le(d, 72)?;
    if scale <= 0. || scale > 1. {
        return Err("Unsupported PSP vertex scale".into());
    }
    let mut transforms = vec![Matrix::identity()];
    for k in 1..nodes {
        let p = 76 + (k - 1) * 52;
        let parent = u16le(d, p + 36)? as usize;
        if parent > k {
            return Err("PSP model parent order invalid".into());
        }
        let mut m = Matrix::identity();
        for a in 0..3 {
            for c in 0..3 {
                m.r[a][c] = f32le(d, p + (a * 3 + c) * 4)?;
            }
            m.t[a] = f32le(d, p + 40 + a * 4)?;
        }
        transforms.push(if parent == 0 {
            m
        } else {
            transforms[parent - 1].compose(m)
        });
    }
    let mut mesh = Mesh {
        triangles: Vec::new(),
        surfaces: Vec::new(),
        vertices,
        nodes,
        texture_ids: Vec::new(),
        declared_bounds: [
            [
                f32le(d, 48)? - f32le(d, 60)?,
                f32le(d, 52)? - f32le(d, 64)?,
                f32le(d, 56)? - f32le(d, 68)?,
            ],
            [
                f32le(d, 48)? + f32le(d, 60)?,
                f32le(d, 52)? + f32le(d, 64)?,
                f32le(d, 56)? + f32le(d, 68)?,
            ],
        ],
    };
    let mut cursor = 0;
    let uv_scale = [f32le(d, 32)?, f32le(d, 36)?];
    let uv_offset = [f32le(d, 40)?, f32le(d, 44)?];
    for i in 0..materials {
        let p = mat0 + i * 8;
        let node = bytes(d, p + 4, 1)?[0] as usize;
        if node >= nodes || bytes(d, p + 5, 3)? != [1, 0, 0] {
            return Err("Only one-strip PSP materials supported".into());
        }
        mesh.texture_ids.push(u16le(d, p)?);
        let count = u16le(d, strip0 + i * 16)? as usize;
        if cursor + count > vertices {
            return Err("PSP strip exceeds vertices".into());
        }
        let mut strip = Vec::with_capacity(count);
        for v in cursor..cursor + count {
            let p = vertex0 + v * 12;
            let mut xyz = [0.; 3];
            for (a, x) in xyz.iter_mut().enumerate() {
                *x = i16::from_le_bytes(bytes(d, p + 6 + a * 2, 2)?.try_into()?) as f32 * scale;
            }
            strip.push(Vertex {
                p: transforms[node].point(xyz),
                uv: std::array::from_fn(|a| d[p + a] as f32 / 128. * uv_scale[a] - uv_offset[a]),
            });
        }
        for j in 2..count {
            let t = if j % 2 == 0 {
                [strip[j - 2], strip[j - 1], strip[j]]
            } else {
                [strip[j - 1], strip[j - 2], strip[j]]
            };
            if t[0].p != t[1].p && t[1].p != t[2].p && t[0].p != t[2].p {
                mesh.triangles.push(t.map(|v| v.p));
                mesh.surfaces.push(Surface {
                    v: t,
                    texture: u16le(d, p)?,
                    flags: d[p + 2],
                    alpha: d[p + 3],
                });
            }
        }
        cursor += count;
    }
    if cursor != vertices {
        return Err("PSP strips do not partition vertices".into());
    }
    if mesh.triangles.is_empty() {
        return Err("Empty PSP mesh".into());
    }
    Ok(mesh)
}

/// Orthographic geometry diagnostic: original positions, solid neutral material.
/// UVs, material flags, animation and world placements are deliberately not inferred.
pub fn preview(mesh: &Mesh, w: usize, h: usize, yaw: f32) -> Vec<u32> {
    let mut b = vec![0x171f28; w * h];
    let mut z = vec![f32::NEG_INFINITY; w * h];
    let (s, c) = yaw.sin_cos();
    let tilt = 0.58f32;
    let (st, ct) = tilt.sin_cos();
    let project = |p: [f32; 3]| {
        let x = p[0] * c - p[1] * s;
        let y = p[0] * s + p[1] * c;
        [x, p[2] * ct - y * st, y * ct + p[2] * st]
    };
    let tris: Vec<_> = mesh.triangles.iter().map(|t| t.map(project)).collect();
    let mut lo = [f32::INFINITY; 2];
    let mut hi = [f32::NEG_INFINITY; 2];
    for t in &tris {
        for p in t {
            for a in 0..2 {
                lo[a] = lo[a].min(p[a]);
                hi[a] = hi[a].max(p[a]);
            }
        }
    }
    let scale = (w as f32 * 0.8 / (hi[0] - lo[0]).max(0.01))
        .min(h as f32 * 0.8 / (hi[1] - lo[1]).max(0.01));
    for t in tris {
        let u: [f32; 3] = std::array::from_fn(|a| t[1][a] - t[0][a]);
        let v: [f32; 3] = std::array::from_fn(|a| t[2][a] - t[0][a]);
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let len = (n.iter().map(|x| x * x).sum::<f32>()).sqrt();
        if len < 1e-8 {
            continue;
        }
        let light = 0.3 + 0.7 * ((n[0] * 0.25 + n[1] * 0.5 + n[2] * 0.83) / len).abs();
        let shade = |x: f32| (x * light) as u32;
        let color = shade(178.) << 16 | shade(196.) << 8 | shade(211.);
        let t = t.map(|p| {
            [
                (p[0] - (lo[0] + hi[0]) * 0.5) * scale + w as f32 * 0.5,
                h as f32 * 0.5 - (p[1] - (lo[1] + hi[1]) * 0.5) * scale,
                p[2],
            ]
        });
        let edge = |a: [f32; 3], b: [f32; 3], x: f32, y: f32| {
            (x - a[0]) * (b[1] - a[1]) - (y - a[1]) * (b[0] - a[0])
        };
        let area = edge(t[0], t[1], t[2][0], t[2][1]);
        if area.abs() < 1e-5 {
            continue;
        }
        let minx = t
            .iter()
            .map(|p| p[0])
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.) as usize;
        let maxx = t
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(w as f32 - 1.)
            .max(0.) as usize;
        let miny = t
            .iter()
            .map(|p| p[1])
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.) as usize;
        let maxy = t
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .min(h as f32 - 1.)
            .max(0.) as usize;
        for y in miny..=maxy {
            for x in minx..=maxx {
                let a = edge(t[1], t[2], x as f32 + 0.5, y as f32 + 0.5) / area;
                let bb = edge(t[2], t[0], x as f32 + 0.5, y as f32 + 0.5) / area;
                let cc = 1. - a - bb;
                if a < 0. || bb < 0. || cc < 0. {
                    continue;
                }
                let depth = a * t[0][2] + bb * t[1][2] + cc * t[2][2];
                let p = y * w + x;
                if depth > z[p] {
                    z[p] = depth;
                    b[p] = color;
                }
            }
        }
    }
    b
}
#[cfg(test)]
mod tests {
    use super::*;
    fn triangle() -> Vec<u8> {
        let mut d = vec![0; 136];
        for (p, v) in [(0, 1u16), (2, 1), (4, 1), (6, 3), (84, 3)] {
            d[p..p + 2].copy_from_slice(&v.to_le_bytes());
        }
        d[72..76].copy_from_slice(&1f32.to_le_bytes());
        d[81] = 1;
        for (i, xyz) in [[0i16, 0, 0], [1, 0, 0], [0, 1, 0]].iter().enumerate() {
            for a in 0..3 {
                let p = 106 + i * 12 + a * 2;
                d[p..p + 2].copy_from_slice(&xyz[a].to_le_bytes());
            }
        }
        d
    }
    #[test]
    fn verifies_strips_and_bounds() {
        let d = triangle();
        let m = parse(&d).unwrap();
        assert_eq!(m.triangles.len(), 1);
        assert_eq!(m.triangles[0][1], [1., 0., 0.]);
        assert!(parse(&d[..135]).is_err());
        let mut bad = d;
        bad[84] = 4;
        assert!(parse(&bad).is_err());
    }
    #[test]
    fn odd_collision_vertex_count_does_not_align_render_vertices() {
        let mut d = triangle();
        // One packed collision vertex occupies exactly six bytes, including when
        // this leaves the render stream only 2-byte aligned (PSP tree model 2690).
        d.splice(100..100, [0; 6]);
        d[14..16].copy_from_slice(&1u16.to_le_bytes());
        assert_eq!(
            parse(&d).unwrap().triangles[0],
            [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]
        );
    }
    #[test]
    fn rejects_future_parent_and_unsupported_material() {
        let mut d = triangle();
        d[80] = 1;
        assert!(parse(&d).is_err());
        d[80] = 0;
        d[81] = 2;
        assert!(parse(&d).is_err());
    }
}
