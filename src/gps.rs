// SPDX-License-Identifier: GPL-3.0-only
//! PSP ai.bin layout verified against ULES01347 (2008 nodes / 4108 links).
//! Layout reference: ctw-native roads.cpp/.h, (C) 2026 Kryo.to.
//! Routing and interface are native Rust implementations, not the original GPS code.
use crate::iso::{bytes, u16le, u32le, Result};
use std::{cmp::Ordering, collections::BinaryHeap};

pub fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoadMeta {
    pub lanes: u8,
    pub lanes_minus_1: u8,
    pub flag5: bool,
    pub hwy: bool,
    pub traffic_light: bool,
    pub half_width: f32,
    pub lane_offset: f32,
    pub wobble: f32,
    pub width: f32,
}

#[derive(Clone)]
pub struct Node {
    pub p: [f32; 3],
    pub flags: u16,
    pub links: Vec<usize>,
}
impl Node {
    pub fn land(&self) -> bool {
        self.p[2] >= -2. && self.flags & 0x800 == 0
    }
    /// RoadNetwork::meta (roads.cpp): lane count, two-way flag5, highway widths and wobble.
    pub fn meta(&self) -> RoadMeta {
        let two = ((self.flags >> 9) & 1) as u8;
        let lanes = two + 1;
        let flag5 = ((self.flags >> 5) & 1) != 0;
        let hwy = ((self.flags >> 10) & 1) != 0;
        let traffic_light = (self.flags as i16) < 0;
        let (half_width, lane_offset, wobble) = if hwy {
            (0x3c00 as f32 / 4096., 0x0c00 as f32 / 4096., 0x0140 as f32 / 4096.)
        } else {
            (0x2800 as f32 / 4096., 0x0800 as f32 / 4096., 0x0100 as f32 / 4096.)
        };
        RoadMeta {
            lanes,
            lanes_minus_1: two,
            flag5,
            hwy,
            traffic_light,
            half_width,
            lane_offset,
            wobble,
            width: lanes as f32 * half_width,
        }
    }
}
pub struct Roads {
    pub nodes: Vec<Node>,
}
#[derive(Clone, Copy, Debug)]
pub struct Anchor {
    pub a: usize,
    pub b: usize,
    pub t: f32,
    pub p: [f32; 3],
}
#[derive(Clone, Copy, PartialEq)]
struct Queue(f32, usize);
impl Eq for Queue {}
impl Ord for Queue {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .0
            .total_cmp(&self.0)
            .then_with(|| other.1.cmp(&self.1))
    }
}
impl PartialOrd for Queue {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Roads {
    pub fn parse(d: &[u8]) -> Result<Self> {
        let nl = u32le(d, 0)? as usize;
        let nn = u32le(d, 4)? as usize;
        let off = u32le(d, 12)? as usize;
        if nn == 0 || nn > 65536 || nl > 65536 || off < 16 + nl * 2 {
            return Err("Invalid PSP road header".into());
        }
        bytes(d, 16, nl * 2)?;
        bytes(d, off, nn * 10)?;
        let links = (0..nl)
            .map(|i| u16le(d, 16 + i * 2).map(|v| v as usize))
            .collect::<Result<Vec<_>>>()?;
        if links.iter().any(|&n| n >= nn) {
            return Err("Invalid PSP road link".into());
        }
        let mut nodes = Vec::with_capacity(nn);
        for i in 0..nn {
            let at = off + i * 10;
            let first = u16le(d, at)? as usize;
            let flags = u16le(d, at + 2)?;
            let count = (flags >> 2 & 7) as usize;
            if first + count > nl {
                return Err("Invalid PSP node links".into());
            }
            nodes.push(Node {
                p: [
                    u16le(d, at + 4)? as i16 as f32 / 8.,
                    u16le(d, at + 6)? as i16 as f32 / 8.,
                    d[at + 8] as i8 as f32 / 2.,
                ],
                flags,
                links: links[first..first + count].to_vec(),
            });
        }
        Ok(Self { nodes })
    }
    pub fn edges(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.land())
            .flat_map(move |(a, n)| {
                n.links
                    .iter()
                    .copied()
                    .filter(move |&b| self.nodes[b].land())
                    .map(move |b| (a, b))
            })
    }
    pub fn nearest(&self, p: [f32; 3], max: f32) -> Option<Anchor> {
        if !p.iter().all(|v| v.is_finite()) {
            return None;
        }
        let mut best = max;
        let mut result = None;
        for (a, b) in self.edges() {
            let x = self.nodes[a].p;
            let y = self.nodes[b].p;
            let delta = [y[0] - x[0], y[1] - x[1], y[2] - x[2]];
            let norm = delta.iter().map(|v| v * v).sum::<f32>();
            if norm < 0.001 {
                continue;
            }
            let t =
                (((p[0] - x[0]) * delta[0] + (p[1] - x[1]) * delta[1] + (p[2] - x[2]) * delta[2])
                    / norm)
                    .clamp(0., 1.);
            let q = std::array::from_fn(|i| x[i] + delta[i] * t);
            let dist = distance(p, q);
            if dist < best {
                best = dist;
                result = Some(Anchor { a, b, t, p: q });
            }
        }
        result
    }
    /// Virtual endpoints on directed edges; never invent a reverse road link.
    pub fn route(&self, start: Anchor, end: Anchor) -> Option<Vec<[f32; 3]>> {
        if start.a == end.a && start.b == end.b && start.t <= end.t {
            return Some(vec![start.p, end.p]);
        }
        let mut costs = vec![f32::INFINITY; self.nodes.len()];
        let mut prev = vec![usize::MAX; self.nodes.len()];
        let mut heap = BinaryHeap::new();
        costs[start.b] = 0.;
        heap.push(Queue(0., start.b));
        while let Some(Queue(cost, a)) = heap.pop() {
            if cost > costs[a] {
                continue;
            }
            if a == end.a {
                break;
            }
            for &b in &self.nodes[a].links {
                if !self.nodes[b].land() {
                    continue;
                }
                let next = cost + distance(self.nodes[a].p, self.nodes[b].p);
                if next < costs[b] {
                    costs[b] = next;
                    prev[b] = a;
                    heap.push(Queue(next, b));
                }
            }
        }
        if !costs[end.a].is_finite() {
            return None;
        }
        let mut route = vec![end.p];
        let mut at = end.a;
        loop {
            route.push(self.nodes[at].p);
            if at == start.b {
                break;
            }
            at = prev[at];
        }
        route.push(start.p);
        route.reverse();
        route.dedup_by(|a, b| distance(*a, *b) < 0.001);
        Some(route)
    }

    /// AISpline::worldPos (aispline.cpp): point along link (a -> b) with right-hand lane offset.
    pub fn lane_world_pos(&self, a: usize, b: usize, t: f32, lane: u8, wobble: f32) -> [f32; 3] {
        let Some(sn) = self.nodes.get(a) else {
            return [0.; 3];
        };
        let Some(en) = self.nodes.get(b) else {
            return sn.p;
        };
        let mut m = sn.meta();
        // If link (a -> b) also has a reverse link (b -> a), enforce two-way lane offset even if
        // a synthetic test node omitted flag5/two-lane bits.
        if en.links.contains(&a) && m.lanes < 2 {
            m.lanes = 2;
            m.lanes_minus_1 = 1;
            m.flag5 = true;
            m.width = m.lanes as f32 * m.half_width;
        }
        let mut o = [
            sn.p[0] + (en.p[0] - sn.p[0]) * t,
            sn.p[1] + (en.p[1] - sn.p[1]) * t,
            sn.p[2] + (en.p[2] - sn.p[2]) * t,
        ];
        let dx = en.p[0] - sn.p[0];
        let dy = en.p[1] - sn.p[1];
        let len = dx.hypot(dy);
        if len > 1e-4 {
            let d = [dx / len, dy / len];
            let lane_clamped = lane.min(m.lanes.saturating_sub(1));
            let mut lat = m.half_width * (2.0 * lane_clamped as f32 - m.lanes_minus_1 as f32);
            if m.lanes > 1 {
                lat = if lat < 0.0 {
                    lat - m.lane_offset
                } else {
                    lat + m.lane_offset
                };
            }
            lat += wobble;
            // In AISpline::worldPos: o += (-d[1] * lat, d[0] * lat).
            // For lane 0 (lat < 0), this shifts along (+d[1], -d[0]), the right-hand side of travel.
            o[0] += -d[1] * lat;
            o[1] += d[0] * lat;
        }
        o
    }

    /// RoadNetwork::selectLane (roads.cpp): choose initial lane and lateral wobble on link (a -> b).
    pub fn select_lane(&self, a: usize, b: usize, seed: u32) -> (u8, f32) {
        let Some(sn) = self.nodes.get(a) else {
            return (0, 0.);
        };
        let m = sn.meta();
        let bidirectional = self.nodes.get(b).is_some_and(|en| en.links.contains(&a));
        let lane = if m.lanes <= 1 || m.flag5 || bidirectional {
            0
        } else {
            (seed & 1) as u8
        };
        let w_raw = (((seed >> 3) & 0x1f) as i32 - 16) as f32 / 16.0;
        (lane, w_raw * m.wobble)
    }

    /// AISpline::laneConvert (aispline.cpp): preserve or adapt lane across node b onto (b -> c).
    pub fn convert_lane(&self, a: usize, b: usize, c: usize, lane: u8, seed: u32) -> (u8, f32) {
        let Some(sn) = self.nodes.get(a) else {
            return (0, 0.);
        };
        let Some(en) = self.nodes.get(b) else {
            return (0, 0.);
        };
        let m0 = sn.meta();
        let m1 = en.meta();
        let bidir1 = self.nodes.get(c).is_some_and(|cn| cn.links.contains(&b));
        let next_lane = if m1.lanes <= 1 || m1.flag5 || bidir1 {
            0
        } else if !m0.flag5 && m0.lanes > 1 {
            lane.min(m1.lanes - 1)
        } else {
            (seed & 1) as u8
        };
        let w_raw = (((seed >> 4) & 0x1f) as i32 - 16) as f32 / 16.0;
        (next_lane, w_raw * m1.wobble)
    }

    /// AISpline::fixUpTuplet + curvePoint (aispline.cpp): evaluates position and heading along
    /// link (a -> b) with a quadratic Bezier corner transition into outgoing link (b -> c).
    pub fn rail_pose(
        &self,
        a: usize,
        b: usize,
        c: usize,
        dist_along: f32,
        lane_ab: u8,
        wobble_ab: f32,
        lane_bc: u8,
        wobble_bc: f32,
    ) -> ([f32; 3], f32, f32, bool) {
        let p0 = self.lane_world_pos(a, b, 0.0, lane_ab, wobble_ab);
        let p1 = self.lane_world_pos(a, b, 1.0, lane_ab, wobble_ab);
        let seg_dx = p1[0] - p0[0];
        let seg_dy = p1[1] - p0[1];
        let seg_len = seg_dx.hypot(seg_dy).max(0.1);
        let dir0 = [seg_dx / seg_len, seg_dy / seg_len];

        let q0 = self.lane_world_pos(b, c, 0.0, lane_bc, wobble_bc);
        let q1 = self.lane_world_pos(b, c, 1.0, lane_bc, wobble_bc);
        let next_dx = q1[0] - q0[0];
        let next_dy = q1[1] - q0[1];
        let next_len = next_dx.hypot(next_dy).max(0.1);
        let dir1 = [next_dx / next_len, next_dy / next_len];

        let dot = dir0[0] * dir1[0] + dir0[1] * dir1[1];
        let turning = b != c && dot < 0.96 && seg_len > 10.0 && next_len > 10.0;
        let setback = if turning {
            (5.5 * (1.0 - dot).clamp(0.35, 1.35)).min(seg_len * 0.35).min(next_len * 0.35)
        } else {
            0.0
        };
        let straight_len = (seg_len - setback).max(0.1);
        let curve_len = if turning { setback * 1.65 } else { 0.0 };
        let total_len = straight_len + curve_len;
        let s = dist_along.clamp(0.0, total_len);

        if !turning || s <= straight_len {
            let t = (s / seg_len).clamp(0.0, 1.0);
            let pos = [
                p0[0] + (p1[0] - p0[0]) * t,
                p0[1] + (p1[1] - p0[1]) * t,
                p0[2] + (p1[2] - p0[2]) * t,
            ];
            let angle = -dir0[0].atan2(dir0[1]);
            (pos, angle, total_len, false)
        } else {
            let u = ((s - straight_len) / curve_len.max(0.1)).clamp(0.0, 1.0);
            let t_exit = (straight_len / seg_len).clamp(0.0, 1.0);
            let t_entry = (setback / next_len).clamp(0.0, 1.0);
            let c_start = [
                p0[0] + (p1[0] - p0[0]) * t_exit,
                p0[1] + (p1[1] - p0[1]) * t_exit,
                p0[2] + (p1[2] - p0[2]) * t_exit,
            ];
            let c_end = [
                q0[0] + (q1[0] - q0[0]) * t_entry,
                q0[1] + (q1[1] - q0[1]) * t_entry,
                q0[2] + (q1[2] - q0[2]) * t_entry,
            ];
            let det = dir0[0] * dir1[1] - dir0[1] * dir1[0];
            let ctrl = if det.abs() > 0.12 {
                let rx = q0[0] - p0[0];
                let ry = q0[1] - p0[1];
                let ta = (rx * dir1[1] - ry * dir1[0]) / det;
                let ta = ta.clamp(straight_len * 0.85, seg_len + 8.0);
                [
                    p0[0] + dir0[0] * ta,
                    p0[1] + dir0[1] * ta,
                    (c_start[2] + c_end[2]) * 0.5,
                ]
            } else {
                [
                    (p1[0] + q0[0]) * 0.5,
                    (p1[1] + q0[1]) * 0.5,
                    (c_start[2] + c_end[2]) * 0.5,
                ]
            };
            let om = 1.0 - u;
            let pos = [
                om * om * c_start[0] + 2.0 * om * u * ctrl[0] + u * u * c_end[0],
                om * om * c_start[1] + 2.0 * om * u * ctrl[1] + u * u * c_end[1],
                om * om * c_start[2] + 2.0 * om * u * ctrl[2] + u * u * c_end[2],
            ];
            let tx = 2.0 * om * (ctrl[0] - c_start[0]) + 2.0 * u * (c_end[0] - ctrl[0]);
            let ty = 2.0 * om * (ctrl[1] - c_start[1]) + 2.0 * u * (c_end[1] - ctrl[1]);
            let angle = if tx.hypot(ty) > 1e-4 {
                -tx.atan2(ty)
            } else {
                -dir1[0].atan2(dir1[1])
            };
            (pos, angle, total_len, true)
        }
    }

    /// Entry offset along link (b -> c) after completing a corner turn from (a -> b).
    pub fn turn_entry_offset(&self, a: usize, b: usize, c: usize, lane_ab: u8, lane_bc: u8) -> f32 {
        let p0 = self.lane_world_pos(a, b, 0.0, lane_ab, 0.0);
        let p1 = self.lane_world_pos(a, b, 1.0, lane_ab, 0.0);
        let q0 = self.lane_world_pos(b, c, 0.0, lane_bc, 0.0);
        let q1 = self.lane_world_pos(b, c, 1.0, lane_bc, 0.0);
        let l0 = (p1[0] - p0[0]).hypot(p1[1] - p0[1]).max(0.1);
        let l1 = (q1[0] - q0[0]).hypot(q1[1] - q0[1]).max(0.1);
        let d0 = [(p1[0] - p0[0]) / l0, (p1[1] - p0[1]) / l0];
        let d1 = [(q1[0] - q0[0]) / l1, (q1[1] - q0[1]) / l1];
        let dot = d0[0] * d1[0] + d0[1] * d1[1];
        if b != c && dot < 0.96 && l0 > 10.0 && l1 > 10.0 {
            (5.5 * (1.0 - dot).clamp(0.35, 1.35)).min(l0 * 0.35).min(l1 * 0.35)
        } else {
            0.0
        }
    }
}

pub struct Gps {
    pub roads: Roads,
    pub target: Option<Anchor>,
    pub route: Vec<[f32; 3]>,
    pub arrived: bool,
    pub recalculations: usize,
    pub message: &'static str,
    last_position: [f32; 3],
    elapsed: f32,
}
impl Gps {
    pub fn new(roads: Roads) -> Self {
        Self {
            roads,
            target: None,
            route: Vec::new(),
            arrived: false,
            recalculations: 0,
            message: "CHOISIS UNE DESTINATION",
            last_position: [f32::INFINITY; 3],
            elapsed: 1.,
        }
    }
    pub fn clear(&mut self) {
        self.target = None;
        self.route.clear();
        self.arrived = false;
        self.message = "DESTINATION EFFACEE";
    }
    pub fn select(&mut self, xy: [f32; 2], player: [f32; 3]) -> bool {
        let Some(target) = self.roads.nearest([xy[0], xy[1], 0.], 100.) else {
            self.message = "AUCUNE ROUTE A CET ENDROIT";
            return false;
        };
        self.target = Some(target);
        self.arrived = false;
        self.recalculate(player);
        true
    }
    fn recalculate(&mut self, p: [f32; 3]) {
        self.route.clear();
        self.last_position = p;
        self.elapsed = 0.;
        self.recalculations += 1;
        if let Some(target) = self.target {
            if distance(p, target.p) < 6. {
                self.arrived = true;
                self.message = "DESTINATION ATTEINTE";
                return;
            }
            if let Some(start) = self.roads.nearest(p, 150.) {
                self.route = self.roads.route(start, target).unwrap_or_default();
            }
            self.message = if self.route.is_empty() {
                "ITINERAIRE INDISPONIBLE"
            } else {
                "ITINERAIRE ACTIF"
            };
        }
    }
    pub fn update(&mut self, p: [f32; 3], dt: f32) {
        if self.target.is_none() || self.arrived {
            return;
        }
        self.elapsed += dt;
        if self.target.is_some_and(|t| distance(p, t.p) < 6.)
            || (self.elapsed >= 0.5 && distance(p, self.last_position) > 3.)
        {
            self.recalculate(p);
        }
    }
    pub fn remaining(&self, p: [f32; 3]) -> f32 {
        let total = self
            .route
            .windows(2)
            .map(|w| distance(w[0], w[1]))
            .sum::<f32>();
        let mut best = f32::INFINITY;
        let mut result = 0.;
        let mut consumed = 0.;
        // Project onto the immediate approach only: do not skip ahead at a later crossing.
        for w in self.route.windows(2).take(2) {
            let length = distance(w[0], w[1]);
            if length < 0.001 {
                continue;
            }
            let d: [f32; 3] = std::array::from_fn(|k| w[1][k] - w[0][k]);
            let t = ((0..3).map(|k| (p[k] - w[0][k]) * d[k]).sum::<f32>() / (length * length))
                .clamp(0., 1.);
            let q = std::array::from_fn(|k| w[0][k] + d[k] * t);
            let approach = distance(p, q);
            if approach < best {
                best = approach;
                result = approach + (total - consumed - length * t).max(0.);
            }
            consumed += length;
        }
        if best.is_finite() {
            result
        } else {
            self.route.first().map_or(0., |&q| distance(p, q))
        }
    }
    pub fn guidance(&self, p: [f32; 3]) -> String {
        if self.route.is_empty() || self.message == "AUCUNE ROUTE A CET ENDROIT" {
            return self.message.into();
        }
        let mut to = distance(p, self.route[0]);
        if to > 18. {
            return format!("REJOINDRE LA ROUTE - {:.0} M", to);
        }
        for w in self.route.windows(3) {
            let u = [w[1][0] - w[0][0], w[1][1] - w[0][1]];
            let v = [w[2][0] - w[1][0], w[2][1] - w[1][1]];
            to += distance(w[0], w[1]);
            if u[0].hypot(u[1]) < 0.1 || v[0].hypot(v[1]) < 0.1 {
                continue;
            }
            let angle = (u[0] * v[1] - u[1] * v[0]).atan2(u[0] * v[0] + u[1] * v[1]);
            if angle.abs() > 0.55 {
                return format!(
                    "{} DANS {:.0} M",
                    if angle.abs() > 2.5 {
                        "DEMI-TOUR"
                    } else if angle > 0. {
                        "A GAUCHE"
                    } else {
                        "A DROITE"
                    },
                    to
                );
            }
        }
        format!("CONTINUER - {:.0} M", self.remaining(p))
    }
    pub fn report(&self, p: [f32; 3]) -> serde_json::Value {
        serde_json::json!({"source":"PSP ROM.WAD/ai.bin", "original_gps_algorithm":false,"nodes":self.roads.nodes.len(),"links":self.roads.nodes.iter().map(|n|n.links.len()).sum::<usize>(),"land_links":self.roads.edges().count(),"target":self.target.map(|t|t.p),"route":self.route,"remaining":self.remaining(p),"arrived":self.arrived,"recalculations":self.recalculations,"message":self.message,"guidance":self.guidance(p)})
    }
}

/// Common rotated projection: terrain, roads, route and markers use the same transform.
#[derive(Clone, Copy)]
pub struct MapView {
    pub rect: [f32; 4],
    pub center: [f32; 2],
    pub span: f32,
    pub rotation: f32,
    pub circular: bool,
}
impl MapView {
    pub fn screen(&self, p: [f32; 3]) -> [f32; 2] {
        let [x, y, w, h] = self.rect;
        let scale = w / self.span;
        let (s, c) = self.rotation.sin_cos();
        let dx = p[0] - self.center[0];
        let dy = p[1] - self.center[1];
        [
            x + w / 2. + (dx * c - dy * s) * scale,
            y + h / 2. - (dx * s + dy * c) * if self.circular { h / self.span } else { scale },
        ]
    }
    pub fn world(&self, p: [f32; 2]) -> [f32; 2] {
        let [x, y, w, h] = self.rect;
        let scale = self.span / w;
        let (s, c) = self.rotation.sin_cos();
        let dx = (p[0] - x - w / 2.) * scale;
        let dy = -(p[1] - y - h / 2.) * if self.circular { self.span / h } else { scale };
        [
            self.center[0] + dx * c + dy * s,
            self.center[1] - dx * s + dy * c,
        ]
    }
    pub fn edge_marker(&self, p: [f32; 2], inset: f32) -> [f32; 2] {
        let [x, y, w, h] = self.rect;
        let cx = x + w / 2.;
        let cy = y + h / 2.;
        let dx = p[0] - cx;
        let dy = p[1] - cy;
        if self.circular {
            let norm = (dx / (w / 2. - inset)).hypot(dy / (h / 2. - inset));
            let ratio = (1. / norm.max(0.001)).min(1.);
            [cx + dx * ratio, cy + dy * ratio]
        } else {
            let ratio = ((w / 2. - inset) / dx.abs().max(0.001))
                .min((h / 2. - inset) / dy.abs().max(0.001))
                .min(1.);
            [cx + dx * ratio, cy + dy * ratio]
        }
    }
    pub fn contains(&self, p: [f32; 2]) -> bool {
        let [x, y, w, h] = self.rect;
        if self.circular {
            ((p[0] - x - w / 2.) / (w / 2. - 2.)).hypot((p[1] - y - h / 2.) / (h / 2. - 2.)) <= 1.
        } else {
            p[0] >= x && p[0] <= x + w && p[1] >= y && p[1] <= y + h
        }
    }
    pub fn zoom(&mut self, factor: f32, at: [f32; 2]) {
        let old = self.world(at);
        self.span = (self.span * factor).clamp(180., 14000.);
        let new = self.world(at);
        self.center[0] += old[0] - new[0];
        self.center[1] += old[1] - new[1];
    }
    pub fn clip(&self, mut a: [f32; 2], mut b: [f32; 2]) -> Option<([f32; 2], [f32; 2])> {
        let [x, y, w, h] = self.rect;
        let delta = [b[0] - a[0], b[1] - a[1]];
        if self.circular {
            let q = [
                (a[0] - x - w / 2.) / (w / 2. - 3.),
                (a[1] - y - h / 2.) / (h / 2. - 3.),
            ];
            let d = [delta[0] / (w / 2. - 3.), delta[1] / (h / 2. - 3.)];
            let aa = d[0] * d[0] + d[1] * d[1];
            let cc = q[0] * q[0] + q[1] * q[1] - 1.;
            if aa < 1e-8 {
                return if cc <= 0. { Some((a, b)) } else { None };
            }
            let bb = 2. * (q[0] * d[0] + q[1] * d[1]);
            let disc = bb * bb - 4. * aa * cc;
            if disc < 0. {
                return None;
            }
            let lo = ((-bb - disc.sqrt()) / (2. * aa)).max(0.);
            let hi = ((-bb + disc.sqrt()) / (2. * aa)).min(1.);
            if lo > hi {
                return None;
            }
            return Some((
                std::array::from_fn(|i| a[i] + lo * delta[i]),
                std::array::from_fn(|i| a[i] + hi * delta[i]),
            ));
        }
        let mut lo = 0f32;
        let mut hi = 1f32;
        for (p, q) in [
            (-delta[0], a[0] - x),
            (delta[0], x + w - a[0]),
            (-delta[1], a[1] - y),
            (delta[1], y + h - a[1]),
        ] {
            if p.abs() < 1e-8 {
                if q < 0. {
                    return None;
                }
            } else {
                let t = q / p;
                if p < 0. {
                    lo = lo.max(t);
                } else {
                    hi = hi.min(t);
                }
            }
        }
        if lo > hi {
            return None;
        }
        for i in 0..2 {
            b[i] = a[i] + hi * delta[i];
            a[i] += lo * delta[i];
        }
        Some((a, b))
    }
}
pub struct MapUi {
    pub open: bool,
    pub view: MapView,
    pub cursor: [f32; 2],
    pub rotate_minimap: bool,
    pub info_bands: bool,
}
impl MapUi {
    pub fn new(p: [f32; 3]) -> Self {
        let w = crate::gpu::W as f32;
        let h = crate::gpu::H as f32;
        Self {
            open: false,
            view: MapView {
                rect: [24., 100., w - 48., h - 240.],
                center: [p[0], p[1]],
                span: 1400.,
                rotation: 0.,
                circular: false,
            },
            cursor: [w / 2., 100. + (h - 240.) / 2.],
            rotate_minimap: true,
            info_bands: false,
        }
    }
    pub fn toggle(&mut self, p: [f32; 3]) {
        self.open = !self.open;
        if self.open {
            self.recenter(p);
        }
    }
    pub fn recenter(&mut self, p: [f32; 3]) {
        self.view.center = [p[0], p[1]];
        let [x, y, w, h] = self.view.rect;
        self.cursor = [x + w / 2., y + h / 2.];
    }
    pub fn fit_city(&mut self) {
        self.view.center = [0., 0.];
        self.view.span = (7000f32).max(5000. * self.view.rect[2] / self.view.rect[3]) * 1.04;
        let [x, y, w, h] = self.view.rect;
        self.cursor = [x + w / 2., y + h / 2.];
    }
    pub fn fit_route(&mut self, gps: &Gps, player: [f32; 3]) {
        let mut lo = [player[0], player[1]];
        let mut hi = lo;
        for p in &gps.route {
            for k in 0..2 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        self.view.center = [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.];
        self.view.span = ((hi[0] - lo[0] + 100.)
            .max((hi[1] - lo[1] + 100.) * self.view.rect[2] / self.view.rect[3])
            * 1.15)
            .clamp(600., 14000.);
    }
    pub fn move_cursor(&mut self, dir: [f32; 2], dt: f32) {
        let [x, y, w, h] = self.view.rect;
        for (i, d) in [dir[0], -dir[1]].into_iter().enumerate() {
            self.cursor[i] += d * 360. * dt;
            let (lo, hi) = if i == 0 {
                (x + 12., x + w - 12.)
            } else {
                (y + 12., y + h - 12.)
            };
            let excess = self.cursor[i] - self.cursor[i].clamp(lo, hi);
            self.cursor[i] -= excess;
            self.view.center[i] += excess * self.view.span / w * if i == 0 { 1. } else { -1. };
        }
    }
}

/// The mobile directory has a different header; do not use its offsets here.
pub fn original_terrain(
    disc: &mut crate::iso::Disc,
    pak: &crate::pak::Pak,
) -> crate::iso::Result<crate::texture::Texture> {
    use crate::iso::u16le;
    let raw = pak.read(disc, 331)?.0;
    let (ox, oy) = (u16le(&raw, 0)? as f32, u16le(&raw, 2)? as f32);
    let (cols, rows) = (u16le(&raw, 4)? as usize, u16le(&raw, 6)? as usize);
    if cols != 8 || rows != 7 {
        return Err("Unsupported PSP radar directory".into());
    }
    let ids = (0..cols * rows)
        .map(|i| u16le(&raw, 8 + i * 2))
        .collect::<crate::iso::Result<Vec<_>>>()?;
    let mut tiles = std::collections::HashMap::new();
    for &id in &ids {
        if id != 65535 && !tiles.contains_key(&id) {
            tiles.insert(
                id,
                crate::texture::decode(id as usize, &pak.read(disc, id as usize)?.0)?,
            );
        }
    }
    let (width, height) = (2048, 1464);
    let mut rgba = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let wx = (x as f32 + 0.5) * 7000. / width as f32 - 3500.;
            let wy = 2500. - (y as f32 + 0.5) * 5000. / height as f32;
            let mx = (wx + 3540.) * (1638. / 4096.) - ox;
            let my = (2520. - wy) * (1638. / 4096.) - oy;
            let (col, row) = ((mx / 256.).floor() as i32, (my / 256.).floor() as i32);
            let pixel = if col >= 0 && row >= 0 && col < cols as i32 && row < rows as i32 {
                tiles
                    .get(&ids[row as usize * cols + col as usize])
                    .map(|t| {
                        let tx = (mx.rem_euclid(256.) as usize).min(t.info.width - 1);
                        let ty = (my.rem_euclid(256.) as usize).min(t.info.height - 1);
                        t.rgba[ty * t.info.width + tx]
                    })
            } else {
                None
            };
            rgba.push(pixel.unwrap_or([94, 93, 117, 255]));
        }
    }
    Ok(crate::texture::Texture {
        high_res: None,
        info: crate::texture::Info {
            id: 331,
            width,
            height,
            bits: 32,
            colors: 0,
        },
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn city_view_fits_full_world_and_zoom_preserves_cursor_position() {
        let mut ui = MapUi::new([-1000., -1603., 0.]);
        ui.fit_city();
        for p in [[-3500., -2500., 0.], [3500., 2500., 0.]] {
            assert!(ui.view.contains(ui.view.screen(p)));
        }
        let cursor = [250., 350.];
        let before = ui.view.world(cursor);
        ui.view.zoom(0.5, cursor);
        let after = ui.view.world(cursor);
        assert!((before[0] - after[0]).abs() < 0.001);
        assert!((before[1] - after[1]).abs() < 0.001);
        ui.recenter([-1000., -1603., 0.]);
        assert_eq!(ui.view.world(ui.cursor), [-1000., -1603.]);
    }
    fn graph() -> Roads {
        Roads {
            nodes: vec![
                Node {
                    p: [0., 0., 0.],
                    flags: 0,
                    links: vec![1],
                },
                Node {
                    p: [20., 0., 0.],
                    flags: 0,
                    links: vec![2],
                },
                Node {
                    p: [20., 20., 0.],
                    flags: 0,
                    links: vec![3],
                },
                Node {
                    p: [0., 20., 0.],
                    flags: 0,
                    links: vec![0],
                },
                Node {
                    p: [40., 0., -7.],
                    flags: 0,
                    links: vec![5],
                },
                Node {
                    p: [60., 0., -7.],
                    flags: 0,
                    links: vec![4],
                },
            ],
        }
    }
    #[test]
    fn directed_route_projects_endpoints_and_goes_around_one_way_loop() {
        let r = graph();
        let a = r.nearest([15., 0., 0.], 50.).unwrap();
        let b = r.nearest([5., 0., 0.], 50.).unwrap();
        let path = r.route(a, b).unwrap();
        assert_eq!(path.len(), 6);
        assert_eq!(path[0], a.p);
        assert_eq!(*path.last().unwrap(), b.p);
        assert!((path.windows(2).map(|w| distance(w[0], w[1])).sum::<f32>() - 70.).abs() < 0.001);
        assert_eq!(r.route(b, a).unwrap(), vec![b.p, a.p]);
    }
    #[test]
    fn no_boats_restricted_links_or_nonfinite_destinations() {
        let mut r = graph();
        assert!(r.nearest([50., 0., 0.], 10.).is_none());
        assert!(r.nearest([f32::NAN, 0., 0.], 100.).is_none());
        r.nodes[2].flags = 0x800;
        let a = r.nearest([10., 0., 0.], 2.).unwrap();
        let b = r.nearest([0., 10., 0.], 2.).unwrap();
        assert!(r.route(a, b).is_none());
    }
    #[test]
    fn navigation_recalculates_arrives_and_clears() {
        let mut g = Gps::new(graph());
        assert!(g.select([10., 20.], [1., 0., 0.]));
        let first = g.remaining([1., 0., 0.]);
        g.update([20., 10., 0.], 0.6);
        assert_eq!(g.recalculations, 2);
        assert!(g.remaining([20., 10., 0.]) < first);
        g.update([10., 20., 0.], 0.1);
        assert!(g.arrived);
        assert!(g.route.is_empty());
        g.clear();
        assert!(g.target.is_none());
        assert!(!g.arrived);
    }
    #[test]
    fn projection_zoom_and_segment_clipping_share_coordinates() {
        let mut v = MapView {
            rect: [20., 30., 500., 200.],
            center: [-1000., -1500.],
            span: 800.,
            rotation: 0.,
            circular: false,
        };
        let p = [-900., -1450., 0.];
        let s = v.screen(p);
        let xy = v.world(s);
        assert_eq!(xy, [p[0], p[1]]);
        v.zoom(0.5, s);
        assert_eq!(v.world(s), xy);
        assert!(v.clip([0., 0.], [10., 10.]).is_none());
        let (a, b) = v.clip([0., 100.], [600., 100.]).unwrap();
        assert!((a[0] - 20.).abs() < 0.0001 && a[1] == 100.);
        assert!((b[0] - 520.).abs() < 0.0001 && b[1] == 100.);
    }
    #[test]
    fn distance_decreases_between_recalculations_and_shortest_branch_wins() {
        let mut r = Roads {
            nodes: vec![
                Node {
                    p: [0., 0., 0.],
                    flags: 0,
                    links: vec![1],
                },
                Node {
                    p: [10., 0., 0.],
                    flags: 0,
                    links: vec![2, 3],
                },
                Node {
                    p: [1000., 0., 0.],
                    flags: 0,
                    links: vec![4],
                },
                Node {
                    p: [10., 10., 0.],
                    flags: 0,
                    links: vec![4],
                },
                Node {
                    p: [10., 20., 0.],
                    flags: 0,
                    links: vec![5],
                },
                Node {
                    p: [10., 30., 0.],
                    flags: 0,
                    links: vec![],
                },
            ],
        };
        let a = r.nearest([1., 0., 0.], 1.).unwrap();
        let b = r.nearest([10., 25., 0.], 1.).unwrap();
        let path = r.route(a, b).unwrap();
        assert!(!path.contains(&[1000., 0., 0.]));
        let mut g = Gps::new(r);
        g.route = path;
        assert!(g.remaining([5., 0., 0.]) < g.remaining([1., 0., 0.]));
        r = g.roads;
        r.nodes[1].links.clear();
        assert!(r.route(a, b).is_none());
    }
    #[test]
    fn rotating_radar_projection_is_invertible_and_camera_forward_points_up() {
        let mut v = MapView {
            rect: [10., 20., 200., 200.],
            center: [-1000., -1500.],
            span: 240.,
            rotation: 0.,
            circular: true,
        };
        for k in 0..32 {
            v.rotation = k as f32 * std::f32::consts::TAU / 32.;
            let p = [-970., -1530., 0.];
            let xy = v.world(v.screen(p));
            assert!((xy[0] - p[0]).abs() < 0.001 && (xy[1] - p[1]).abs() < 0.001);
            let (sn, cs) = v.rotation.sin_cos();
            let forward = v.screen([-1000. + sn * 60., -1500. + cs * 60., 0.]);
            assert!((forward[0] - 110.).abs() < 0.001 && (forward[1] - 70.).abs() < 0.001);
        }
        v.rotation = 0.;
        assert_eq!(v.screen([-1000., -1440., 0.]), [110., 70.]);
    }
    #[test]
    fn round_mask_clips_crossings_and_edge_markers_preserve_bearing() {
        let v = MapView {
            rect: [0., 0., 200., 200.],
            center: [0.; 2],
            span: 200.,
            rotation: 0.,
            circular: true,
        };
        assert!(v.clip([-100., -10.], [300., -10.]).is_none());
        let (a, b) = v.clip([-100., 100.], [300., 100.]).unwrap();
        assert!((a[0] - 3.).abs() < 0.001 && (b[0] - 197.).abs() < 0.001);
        assert!(v.clip([0., 0.], [0., 0.]).is_none());
        assert!(v.clip([100., 100.], [100., 100.]).is_some());
        for k in 0..32 {
            let a = k as f32 * std::f32::consts::TAU / 32.;
            let p = [100. + a.cos() * 1000., 100. + a.sin() * 1000.];
            let q = v.edge_marker(p, 10.);
            let d = [q[0] - 100., q[1] - 100.];
            assert!((d[0].hypot(d[1]) - 90.).abs() < 0.001);
            assert!((d[0] * a.sin() - d[1] * a.cos()).abs() < 0.001);
            assert!(v.contains(q));
        }
    }
    #[test]
    fn malformed_road_tables_fail_closed() {
        assert!(Roads::parse(&[]).is_err());
        let mut b = vec![0u8; 28];
        b[0] = 1;
        b[4] = 1;
        b[12] = 18;
        b[20] = 4;
        assert!(Roads::parse(&b).is_ok());
        b[16] = 2;
        assert!(Roads::parse(&b).is_err());
        b[16] = 0;
        b[18] = 1;
        assert!(Roads::parse(&b).is_err());
    }
    #[test]
    fn road_meta_and_aispline_lane_offsets_keep_traffic_in_right_hand_lanes() {
        let r = Roads {
            nodes: vec![
                Node {
                    p: [0., 0., 0.],
                    flags: (1 << 9) | (1 << 5), // 2 lanes, flag5 two-way
                    links: vec![1],
                },
                Node {
                    p: [0., 40., 0.],
                    flags: (1 << 9) | (1 << 5),
                    links: vec![0, 2],
                },
                Node {
                    p: [40., 40., 0.],
                    flags: (1 << 9) | (1 << 5),
                    links: vec![1],
                },
            ],
        };
        let (lane_nb, _) = r.select_lane(0, 1, 0);
        let (lane_sb, _) = r.select_lane(1, 0, 0);
        assert_eq!((lane_nb, lane_sb), (0, 0));
        let p_nb = r.lane_world_pos(0, 1, 0.5, lane_nb, 0.);
        let p_sb = r.lane_world_pos(1, 0, 0.5, lane_sb, 0.);
        // Northbound (+Y) sits at +3.0m East; Southbound (-Y) sits at -3.0m West.
        assert!((p_nb[0] - 3.0).abs() < 1e-3 && (p_nb[1] - 20.0).abs() < 1e-3);
        assert!((p_sb[0] + 3.0).abs() < 1e-3 && (p_sb[1] - 20.0).abs() < 1e-3);

        let (next_lane, _) = r.convert_lane(0, 1, 2, lane_nb, 1);
        let (pos, angle, total, in_turn) = r.rail_pose(0, 1, 2, 36.0, lane_nb, 0., next_lane, 0.);
        assert!(in_turn && total > 35.0);
        assert!(pos[0] > 3.0 && pos[1] > 30.0 && angle < -0.1);
        assert!(r.turn_entry_offset(0, 1, 2, lane_nb, next_lane) > 2.0);
    }
}
