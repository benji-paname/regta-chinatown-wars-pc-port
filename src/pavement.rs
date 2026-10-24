// SPDX-License-Identifier: GPL-3.0-only
//! Global PSP pavement graph used by civilians.
//! Node choice adapted from ctw-native port/game/peds.cpp and localarea.cpp
//! (cWanderPath, cSectorNodeData::GetRandomNode/ResolveBridgeNode,
//! cFleeOnFoot, AddPathingNodes), Copyright (C) 2026 Kryo.to, GPL-3.0-only.
//! Data comes only from world.bin section 3 of the user's PSP ISO.
//!
//! Each reciprocal bridge pair (two nodes at the same 2D point on both sides of
//! a 50-unit cell edge) is merged into one node, so the walk continues past the
//! edge exactly like ResolveBridgeNode without keeping per-sector references.
use crate::collision::{Cell, Collision};

/// cWanderPath: cTarget radius 0.5.
pub const ARRIVE: f32 = 0.5;
/// cFleeOnFoot: cTarget::SetNode(node, 2.0).
pub const FLEE_ARRIVE: f32 = 2.0;
/// cWanderPath target offset: up to +-0.75 units per axis.
pub const JITTER: f32 = 0.75;
/// GetRandomNode: a link within about 20 degrees of straight back is only a
/// fallback (dot < -3849 in 4096 units).
const BACK_DOT: f32 = -3849. / 4096.;

#[derive(Debug, Clone)]
pub struct NetNode {
    pub p: [f32; 3],
    pub flags: u16,
    pub links: Vec<u32>,
}
impl NetNode {
    /// AddPathingNodes skips nodes with flags & 3 when creating pedestrians.
    pub fn spawnable(&self) -> bool {
        self.flags & 3 == 0
    }
}

#[derive(Default)]
pub struct Network {
    pub nodes: Vec<NetNode>,
    /// Node ids keyed by world collision cell (x * 100 + y) of their position.
    by_cell: Vec<Vec<u32>>,
    pub merged_bridges: usize,
}

/// Small deterministic generator, so validation runs are reproducible.
#[derive(Clone, Copy)]
pub struct Rng(pub u32);
impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed | 1)
    }
    pub fn next(&mut self) -> u32 {
        // xorshift32
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    /// Uniform in 0..n (n > 0), like Rand16Critical(n).
    pub fn below(&mut self, n: u32) -> u32 {
        ((self.next() >> 8) % n.max(1)) as u32
    }
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next() >> 8) as f32 / (1u32 << 24) as f32
    }
}

fn normalise(v: [f32; 2]) -> [f32; 2] {
    let l = v[0].hypot(v[1]);
    if l > 1e-6 {
        [v[0] / l, v[1] / l]
    } else {
        [0., 0.]
    }
}
fn cell_index(p: [f32; 3]) -> Option<usize> {
    Collision::cell_xy(p[0], p[1]).map(|(x, y)| x * 100 + y)
}
fn find(parent: &mut [u32], mut i: u32) -> u32 {
    while parent[i as usize] != i {
        parent[i as usize] = parent[parent[i as usize] as usize];
        i = parent[i as usize];
    }
    i
}

impl Network {
    /// Builds a network from already global nodes (used by tests and `build`).
    pub fn from_nodes(nodes: Vec<NetNode>) -> Self {
        let mut by_cell = vec![Vec::new(); 14000];
        for (i, n) in nodes.iter().enumerate() {
            if let Some(c) = cell_index(n.p) {
                by_cell[c].push(i as u32);
            }
        }
        Self {
            nodes,
            by_cell,
            merged_bridges: 0,
        }
    }

    pub fn build(cells: &[Cell]) -> Self {
        let mut base = Vec::with_capacity(cells.len() + 1);
        let mut total = 0u32;
        for c in cells {
            base.push(total);
            total += c.paths.nodes.len() as u32;
        }
        base.push(total);
        let mut parent: Vec<u32> = (0..total).collect();
        let mut merged = 0;
        for (id, cell) in cells.iter().enumerate() {
            for b in &cell.paths.bridges {
                let Some(other) = cells.get(b.cell) else {
                    continue;
                };
                let p = cell.paths.nodes[b.node as usize].position;
                // ResolveBridgeNode: the neighbour's bridge back to this cell at the same point.
                let twin = other.paths.bridges.iter().find(|back| {
                    back.cell == id && other.paths.nodes[back.node as usize].position[..2] == p[..2]
                });
                if let Some(twin) = twin {
                    let a = find(&mut parent, base[id] + b.node as u32);
                    let t = find(&mut parent, base[b.cell] + twin.node as u32);
                    if a != t {
                        parent[a.max(t) as usize] = a.min(t);
                        merged += 1;
                    }
                }
            }
        }
        let mut compact = vec![u32::MAX; total as usize];
        let mut nodes: Vec<NetNode> = Vec::new();
        for (id, cell) in cells.iter().enumerate() {
            for (i, n) in cell.paths.nodes.iter().enumerate() {
                let root = find(&mut parent, base[id] + i as u32) as usize;
                if compact[root] == u32::MAX {
                    compact[root] = nodes.len() as u32;
                    nodes.push(NetNode {
                        p: n.position,
                        flags: 0,
                        links: Vec::new(),
                    });
                }
                // Keep the spawn exclusion of either half of a merged bridge.
                nodes[compact[root] as usize].flags |= n.flags & !0x80;
            }
        }
        for (id, cell) in cells.iter().enumerate() {
            for (i, n) in cell.paths.nodes.iter().enumerate() {
                let me = compact[find(&mut parent, base[id] + i as u32) as usize];
                for &j in &n.neighbours {
                    let other = compact[find(&mut parent, base[id] + j as u32) as usize];
                    let links = &mut nodes[me as usize].links;
                    if other != me && !links.contains(&other) {
                        links.push(other);
                    }
                }
            }
        }
        let mut out = Self::from_nodes(nodes);
        out.merged_bridges = merged;
        out
    }

    pub fn position(&self, id: u32) -> [f32; 3] {
        self.nodes[id as usize].p
    }

    fn cells_around(&self, p: [f32; 3], radius: f32) -> impl Iterator<Item = u32> + '_ {
        let span = (radius / 50.).ceil() as i32;
        let centre = Collision::cell_xy(p[0], p[1]);
        let by_cell = &self.by_cell;
        centre.into_iter().flat_map(move |(cx, cy)| {
            (-span..=span).flat_map(move |dx| {
                (-span..=span).flat_map(move |dy| {
                    let (x, y) = (cx as i32 + dx, cy as i32 + dy);
                    let ok = (0..140).contains(&x) && (0..100).contains(&y);
                    let list: &[u32] = if ok && !by_cell.is_empty() {
                        &by_cell[(x * 100 + y) as usize]
                    } else {
                        &[]
                    };
                    list.iter().copied()
                })
            })
        })
    }

    /// Closest node within `radius` (2D) and a plausible height range.
    pub fn nearest(&self, p: [f32; 3], radius: f32) -> Option<u32> {
        self.cells_around(p, radius)
            .filter_map(|id| {
                let q = self.nodes[id as usize].p;
                let d = (q[0] - p[0]).hypot(q[1] - p[1]);
                (d <= radius && (q[2] - p[2]).abs() < 3.).then_some((id, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|v| v.0)
    }

    /// cSectorNodeData::GetRandomNode: the next node from `cur`, arriving from
    /// `from`. A link pointing almost straight back is kept only as fallback
    /// (the last such). With `best`, the most aligned candidate wins (ties: the
    /// later one); otherwise a random accepted candidate.
    pub fn next(&self, cur: u32, from: [f32; 3], best: Option<[f32; 2]>, rng: &mut Rng) -> Option<u32> {
        let node = self.nodes.get(cur as usize)?;
        let here = node.p;
        let arrive = normalise([here[0] - from[0], here[1] - from[1]]);
        let mut accepted: Vec<(u32, [f32; 2])> = Vec::new();
        let mut fallback = None;
        for &cand in &node.links {
            let to = self.nodes[cand as usize].p;
            let dir = normalise([to[0] - here[0], to[1] - here[1]]);
            if arrive[0] * dir[0] + arrive[1] * dir[1] < BACK_DOT {
                fallback = Some(cand);
            } else {
                accepted.push((cand, dir));
            }
        }
        if accepted.is_empty() {
            return fallback;
        }
        Some(match best {
            Some(best) => {
                let mut pick = accepted[0].0;
                let mut top = f32::NEG_INFINITY;
                for &(id, dir) in &accepted {
                    let d = best[0] * dir[0] + best[1] * dir[1];
                    if top <= d {
                        top = d;
                        pick = id;
                    }
                }
                pick
            }
            None => accepted[rng.below(accepted.len() as u32) as usize].0,
        })
    }

    /// cWanderPath::SetNext: node position with the original +-0.75 offset.
    pub fn wander_target(&self, id: u32, rng: &mut Rng) -> [f32; 3] {
        let p = self.position(id);
        let mut j = || (rng.below(101) as f32 - 50.) * JITTER / 50.;
        [p[0] + j(), p[1] + j(), p[2]]
    }

    /// Points on spawnable links between `min` and `max` units from `centre`,
    /// as (position, from node, towards node). Both walking directions appear.
    pub fn spawn_points(&self, centre: [f32; 3], min: f32, max: f32, rng: &mut Rng) -> Vec<([f32; 3], u32, u32)> {
        let mut out = Vec::new();
        for a in self.cells_around(centre, max) {
            let na = &self.nodes[a as usize];
            if !na.spawnable() {
                continue;
            }
            for &b in &na.links {
                let nb = &self.nodes[b as usize];
                if b < a || !nb.spawnable() {
                    continue;
                }
                let t = 0.2 + rng.unit() * 0.6;
                let p: [f32; 3] = std::array::from_fn(|i| na.p[i] + (nb.p[i] - na.p[i]) * t);
                let d = (p[0] - centre[0]).hypot(p[1] - centre[1]);
                if d < min || d > max || (p[2] - centre[2]).abs() > 12. {
                    continue;
                }
                if rng.below(2) == 0 {
                    out.push((p, a, b));
                } else {
                    out.push((p, b, a));
                }
            }
        }
        // Deterministic shuffle spreads civilians instead of filling one street.
        for i in (1..out.len()).rev() {
            out.swap(i, rng.below(i as u32 + 1) as usize);
        }
        out
    }

    /// Diagnostic summary for --pavement-audit.
    pub fn stats(&self) -> serde_json::Value {
        let n = self.nodes.len();
        let mut component = vec![u32::MAX; n];
        let mut sizes = Vec::new();
        for start in 0..n {
            if component[start] != u32::MAX {
                continue;
            }
            let id = sizes.len() as u32;
            let mut stack = vec![start as u32];
            component[start] = id;
            let mut size = 0usize;
            while let Some(v) = stack.pop() {
                size += 1;
                for &w in &self.nodes[v as usize].links {
                    if component[w as usize] == u32::MAX {
                        component[w as usize] = id;
                        stack.push(w);
                    }
                }
            }
            sizes.push(size);
        }
        let directed: usize = self.nodes.iter().map(|v| v.links.len()).sum();
        let one_way = self
            .nodes
            .iter()
            .enumerate()
            .flat_map(|(a, v)| v.links.iter().map(move |&b| (a as u32, b)))
            .filter(|&(a, b)| !self.nodes[b as usize].links.contains(&a))
            .count();
        serde_json::json!({
            "global_nodes": n,
            "merged_bridge_pairs": self.merged_bridges,
            "directed_links": directed,
            "one_way_links": one_way,
            "spawnable_nodes": self.nodes.iter().filter(|v| v.spawnable()).count(),
            "dead_ends": self.nodes.iter().filter(|v| v.links.len() == 1).count(),
            "isolated_nodes": self.nodes.iter().filter(|v| v.links.is_empty()).count(),
            "components": sizes.len(),
            "largest_component": sizes.iter().copied().max().unwrap_or(0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ped_paths::{Bridge, Node, Paths};

    pub fn node(p: [f32; 3], links: &[u32]) -> NetNode {
        NetNode {
            p,
            flags: 0,
            links: links.to_vec(),
        }
    }

    #[test]
    fn reciprocal_bridges_merge_cells_into_one_walkable_graph() {
        // Cell 0 (x=0,y=0) and cell 100 (x=1,y=0) share the edge x = -3450.
        let mut cells: Vec<Cell> = (0..14000).map(|_| Cell::default()).collect();
        let pnode = |x: f32, y: f32, n: &[u16], flags: u16| Node {
            position: [x, y, 0.],
            flags,
            cover: 0,
            neighbours: n.to_vec(),
        };
        cells[0].paths = Paths {
            nodes: vec![pnode(-3480., -2490., &[1], 0), pnode(-3450., -2490., &[0], 0x80)],
            bridges: vec![Bridge { node: 1, cell: 100 }],
            link_count: 2,
        };
        cells[100].paths = Paths {
            nodes: vec![pnode(-3450., -2490., &[1], 0x81), pnode(-3420., -2490., &[0], 0)],
            bridges: vec![Bridge { node: 0, cell: 0 }],
            link_count: 2,
        };
        let net = Network::build(&cells);
        assert_eq!(net.nodes.len(), 3);
        assert_eq!(net.merged_bridges, 1);
        let start = net.nearest([-3480., -2490., 0.], 2.).unwrap();
        let edge = net.nearest([-3450., -2490., 0.], 2.).unwrap();
        let end = net.nearest([-3420., -2490., 0.], 2.).unwrap();
        // The merged edge node keeps the spawn exclusion flag of the second half.
        assert!(!net.nodes[edge as usize].spawnable());
        let mut rng = Rng::new(7);
        let from = net.position(start);
        assert_eq!(net.next(edge, from, None, &mut rng), Some(end));
        let stats = net.stats();
        assert_eq!(stats["components"], 1);
        assert_eq!(stats["one_way_links"], 0);
    }

    #[test]
    fn next_node_avoids_turning_back_except_at_dead_ends_and_follows_best_direction() {
        // 0 - 1 - 2 with a branch 1 - 3 to the north; 4 is a dead end after 2.
        let net = Network::from_nodes(vec![
            node([0., 0., 0.], &[1]),
            node([10., 0., 0.], &[0, 2, 3]),
            node([20., 0., 0.], &[1, 4]),
            node([10., 10., 0.], &[1]),
            node([30., 0., 0.], &[2]),
        ]);
        let mut rng = Rng::new(3);
        for _ in 0..50 {
            let n = net.next(1, [0., 0., 0.], None, &mut rng).unwrap();
            assert!(n == 2 || n == 3, "never straight back to 0");
        }
        assert_eq!(net.next(4, [20., 0., 0.], None, &mut rng), Some(2), "dead end turns back");
        assert_eq!(net.next(1, [0., 0., 0.], Some([0., 1.]), &mut rng), Some(3));
        assert_eq!(net.next(1, [0., 0., 0.], Some([1., 0.]), &mut rng), Some(2));
        let t = net.wander_target(1, &mut rng);
        assert!((t[0] - 10.).abs() <= JITTER && t[1].abs() <= JITTER);
    }

    #[test]
    fn spawn_points_respect_ring_and_spawn_flags() {
        let mut nodes = vec![
            node([0., 0., 0.], &[1]),
            node([20., 0., 0.], &[0, 2]),
            node([40., 0., 0.], &[1]),
        ];
        nodes[2].flags = 1;
        let net = Network::from_nodes(nodes);
        let mut rng = Rng::new(11);
        let points = net.spawn_points([0., 0., 0.], 2., 60., &mut rng);
        assert_eq!(points.len(), 1, "the link to a flagged node is not used");
        let (p, a, b) = points[0];
        assert!(p[0] >= 4. && p[0] <= 16.);
        assert!((a, b) == (0, 1) || (a, b) == (1, 0));
    }
}
