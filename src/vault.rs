// SPDX-License-Identifier: GPL-3.0-only
//! Contextual traversal reconstructed for Rust; original PSP thresholds unverified.
//! Static boxes, cylinders and low mesh faces use the same full-body path checks.
use crate::collision::Collision;
pub const RADIUS: f32 = 0.875;
pub const HEIGHT: f32 = 2.;
const MAX_RISE: f32 = 3.25;
const MAX_DROP: f32 = 3.;

#[derive(Clone, Debug)]
pub struct Plan {
    pub start: [f32; 3],
    pub front: [f32; 2],
    pub end: [f32; 3],
    pub top: f32,
    pub heading: f32,
    pub duration: f32,
    pub kind: &'static str,
}
impl Plan {
    pub fn position(&self, progress: f32) -> [f32; 3] {
        let t = progress.clamp(0., 1.);
        let smooth = |v: f32| v * v * (3. - 2. * v);
        let peak = self.top + 0.1;
        if t < 0.28 {
            let f = smooth(t / 0.28);
            [
                self.start[0] + (self.front[0] - self.start[0]) * f,
                self.start[1] + (self.front[1] - self.start[1]) * f,
                self.start[2] + (peak - self.start[2]) * f,
            ]
        } else if t < 0.72 {
            let f = smooth((t - 0.28) / 0.44);
            [
                self.front[0] + (self.end[0] - self.front[0]) * f,
                self.front[1] + (self.end[1] - self.front[1]) * f,
                peak,
            ]
        } else {
            [
                self.end[0],
                self.end[1],
                peak + (self.end[2] - peak) * smooth((t - 0.72) / 0.28),
            ]
        }
    }
    pub fn clear_between(&self, collision: &Collision, from: f32, to: f32) -> bool {
        let horizontal = (self.end[0] - self.start[0]).hypot(self.end[1] - self.start[1]);
        let length = horizontal + 2. * (self.top + 0.1) - self.start[2] - self.end[2];
        let n = ((to - from).abs() * (length / 0.04).clamp(160., 400.))
            .ceil()
            .max(1.) as usize;
        (0..=n).all(|i| {
            collision.body_clear(
                self.position(from + (to - from) * i as f32 / n as f32),
                RADIUS,
                HEIGHT,
            )
        })
    }
}
#[derive(Clone, Debug)]
pub struct Vault {
    pub plan: Plan,
    pub elapsed: f32,
    pub entry_level: u8,
}
impl Vault {
    pub fn progress(&self) -> f32 {
        (self.elapsed / self.plan.duration).clamp(0., 1.)
    }
}
struct Candidate {
    distance: f32,
    direction: [f32; 2],
    top: f32,
    score: f32,
}

#[cfg(test)]
pub fn plan(c: &Collision, p: [f32; 3], heading: f32) -> Option<Plan> {
    plan_moving(c, p, heading, 0.)
}
pub fn plan_moving(c: &Collision, p: [f32; 3], heading: f32, speed: f32) -> Option<Plan> {
    if !p.iter().all(|v| v.is_finite()) || !heading.is_finite() || !speed.is_finite() {
        return None;
    }
    let forward = [-heading.sin(), heading.cos()];
    let reach = RADIUS + 1.1 + speed.clamp(0., 18.) * 0.055;
    let mut candidates = Vec::new();
    let mut add = |xy: [f32; 2], top: f32| {
        let delta = [xy[0] - p[0], xy[1] - p[1]];
        let distance = delta[0].hypot(delta[1]);
        // Anything above Walker's automatic 0.5-unit step needs traversal.
        if distance < 0.01 || distance > reach || top - p[2] <= 0.5 || top - p[2] > MAX_RISE {
            return;
        }
        let direction = [delta[0] / distance, delta[1] / distance];
        let facing = direction[0] * forward[0] + direction[1] * forward[1];
        // Allow oblique approaches and corners, but never pull the player backwards.
        if facing < 0.12 {
            return;
        }
        candidates.push(Candidate {
            distance,
            direction,
            top,
            score: distance + (1. - facing) * 0.4,
        });
    };
    for cell in c.nearby_cells(p[0], p[1]) {
        for b in &cell.boxes {
            if b.c[2] - b.h[2] >= p[2] + HEIGHT {
                continue;
            }
            let q = b.local(p[0], p[1]);
            let local = [q[0].clamp(-b.h[0], b.h[0]), q[1].clamp(-b.h[1], b.h[1])];
            let (s, co) = b.angle.sin_cos();
            add(
                [
                    b.c[0] + local[0] * co - local[1] * s,
                    b.c[1] + local[0] * s + local[1] * co,
                ],
                b.c[2] + b.h[2],
            );
        }
        for b in &cell.cylinders {
            if b.p[2] >= p[2] + HEIGHT {
                continue;
            }
            let delta = [p[0] - b.p[0], p[1] - b.p[1]];
            let length = delta[0].hypot(delta[1]);
            if length <= b.r {
                continue;
            }
            add(
                [
                    b.p[0] + delta[0] * b.r / length,
                    b.p[1] + delta[1] * b.r / length,
                ],
                b.p[2] + b.h,
            );
        }
        for t in &cell.triangles {
            let a = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
            let b = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
            let n = [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ];
            let len = (n.iter().map(|v| v * v).sum::<f32>()).sqrt();
            if len < 1e-6 || n[2].abs() / len > 0.65 {
                continue;
            }
            let top = t.iter().map(|v| v[2]).fold(f32::NEG_INFINITY, f32::max);
            if t.iter().all(|v| v[2] >= p[2] + HEIGHT) {
                continue;
            }
            for i in 0..3 {
                let a = t[i];
                let b = t[(i + 1) % 3];
                let d = [b[0] - a[0], b[1] - a[1]];
                let ll = d[0] * d[0] + d[1] * d[1];
                if ll < 1e-6 {
                    continue;
                }
                let f = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / ll).clamp(0., 1.);
                add([a[0] + d[0] * f, a[1] + d[1] * f], top);
            }
        }
    }
    candidates.sort_by(|a, b| a.score.total_cmp(&b.score));
    candidates.dedup_by(|a, b| {
        (a.distance - b.distance).abs() < 0.03
            && (a.top - b.top).abs() < 0.02
            && (a.direction[0] - b.direction[0]).abs() < 0.02
            && (a.direction[1] - b.direction[1]).abs() < 0.02
    });
    for candidate in candidates.iter().take(12) {
        let d = candidate.direction;
        let top = candidate.top;
        let safe_approach = (candidate.distance - RADIUS - 0.08).max(0.);
        let front = [p[0] + d[0] * safe_approach, p[1] + d[1] * safe_approach];
        // Choose the first supported point: on a broad object, or beyond a thin
        // fence. A single rule handles walls, circular fountains and car roofs.
        for step in 0..24 {
            let length = candidate.distance + RADIUS + 0.18 + step as f32 * 0.25;
            if length > 7. {
                break;
            }
            let xy = [p[0] + d[0] * length, p[1] + d[1] * length];
            let g = c.ground(xy[0], xy[1], top);
            if g.water || g.z > top + 0.02 || p[2] - g.z > MAX_DROP {
                continue;
            }
            let end = [xy[0], xy[1], g.z];
            if !c.body_clear(end, RADIUS, HEIGHT)
                || !(0..8).all(|k| {
                    let a = k as f32 * std::f32::consts::TAU / 8.;
                    let h = c.ground(end[0] + a.cos() * RADIUS, end[1] + a.sin() * RADIUS, g.z);
                    !h.water && (h.z - g.z).abs() <= 0.3
                })
            {
                continue;
            }
            let result = Plan {
                start: p,
                front,
                end,
                top,
                heading: -d[0].atan2(d[1]),
                // Twelve PSP frames; ~0.8s observed in the supplied recording.
                // Do not accelerate this clip merely because the player sprints.
                duration: 0.8,
                kind: if end[2] > p[2] + 0.5 {
                    "climb"
                } else {
                    "vault"
                },
            };
            if result.clear_between(c, 0., 1.) {
                return Some(result);
            }
        }
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::{BoxShape, Cell, Cylinder, Walker};
    fn scene(angle: f32) -> Collision {
        let mut c = Collision {
            cells: (0..14000)
                .map(|_| Cell {
                    ground: vec![0x55; 400],
                    ..Default::default()
                })
                .collect(),
        };
        c.cells[70 * 100 + 50].boxes.push(BoxShape {
            c: [10., 10., 1.25],
            h: [5., 0.4, 1.25],
            angle,
            flags: 0,
        });
        c
    }
    #[test]
    fn small_obstacles_above_step_height_have_no_dead_zone() {
        for height in [0.501, 0.55, 0.6, 1., 2.5, MAX_RISE] {
            let mut c = scene(0.);
            c.cells[7050].boxes[0].c[2] = height / 2.;
            c.cells[7050].boxes[0].h[2] = height / 2.;
            let result = plan(&c, [10., 8.5, 0.], 0.).unwrap();
            assert!(result.clear_between(&c, 0., 1.));
        }
    }
    #[test]
    fn all_wall_rotations_oblique_angles_and_speed_levels_are_supported() {
        for degrees in (0..360).step_by(30) {
            let angle = (degrees as f32).to_radians();
            let c = scene(angle);
            let p = [10. + angle.sin() * 1.5, 10. - angle.cos() * 1.5, 0.];
            for offset in [-80f32, -60., -40., -20., 0., 20., 40., 60., 80.] {
                for speed in [0., 12., 18.] {
                    let a = plan_moving(&c, p, angle + offset.to_radians(), speed).unwrap_or_else(
                        || panic!("rotation {degrees}, approach {offset}, speed {speed}"),
                    );
                    assert!(a.clear_between(&c, 0., 1.));
                }
            }
        }
    }
    #[test]
    fn round_fountain_and_triangle_fence_are_traversable() {
        let mut c = scene(0.);
        c.cells[7050].boxes.clear();
        c.cells[7050].cylinders.push(Cylinder {
            p: [10., 10., 0.],
            r: 2.,
            h: 1.5,
        });
        for degrees in (0..360).step_by(30) {
            let a = (degrees as f32).to_radians();
            let p = [10. + a.sin() * 3.4, 10. - a.cos() * 3.4, 0.];
            let result = plan(&c, p, a).unwrap();
            assert_eq!(result.kind, "climb");
            assert_eq!(result.end[2], 1.5);
        }
        c.cells[7050].cylinders.clear();
        c.cells[7050].triangles.extend([
            [[5., 10., 0.], [15., 10., 0.], [15., 10., 2.5]],
            [[5., 10., 0.], [15., 10., 2.5], [5., 10., 2.5]],
        ]);
        let result = plan(&c, [10., 8.5, 0.], 0.).unwrap();
        assert_eq!(result.kind, "vault");
        assert!(result.end[1] > 10. + RADIUS);
    }
    #[test]
    fn early_press_is_buffered_and_running_resumes_after_landing() {
        let c = scene(0.);
        let mut w = Walker::new(&c, [10., 6.6]).unwrap();
        w.level = 3;
        w.queue_vault();
        w.step(&c, [0., 1.], 1. / 30., true, false);
        assert!(w.vault.is_none());
        w.step(&c, [0., 1.], 1. / 30., true, false);
        assert!(w.vault.is_some());
        while w.vault.is_some() {
            w.step(&c, [0., 1.], 1. / 30., true, false);
        }
        assert_eq!(w.level, 3);
        assert_eq!(w.vaults_completed, 1);
        assert_eq!(w.vaults_rejected, 0);
        assert!(w.velocity[1] > 17.);
        let y = w.p[1];
        w.step(&c, [0., 1.], 1. / 30., true, false);
        assert!(w.p[1] > y + 0.5);
    }
    #[test]
    fn expired_and_cancelled_requests_do_not_trigger_later() {
        let c = scene(0.);
        let mut w = Walker::new(&c, [10., 2.]).unwrap();
        w.queue_vault();
        for _ in 0..10 {
            w.step(&c, [0.; 2], 1. / 30., false, false);
        }
        assert_eq!(w.vaults_rejected, 1);
        w.p = [10., 8.5, 0.];
        w.step(&c, [0.; 2], 1. / 30., false, false);
        assert!(w.vault.is_none());
        w.queue_vault();
        w.cancel_vault_input();
        w.step(&c, [0.; 2], 1. / 30., false, false);
        assert!(w.vault.is_none());
    }
    #[test]
    fn rotated_wall_is_cleared_with_full_body_and_continuous_motion() {
        for angle in [0., 0.35, 0.87, 1.57] {
            let c = scene(angle);
            let p = [10. + angle.sin() * 1.5, 10. - angle.cos() * 1.5, 0.];
            let a = plan(&c, p, angle).unwrap();
            assert_eq!(a.position(0.), p);
            assert_eq!(a.position(1.), a.end);
            for i in 1..=160 {
                let prev = a.position((i - 1) as f32 / 160.);
                let cur = a.position(i as f32 / 160.);
                assert!(c.body_clear(cur, RADIUS, HEIGHT));
                assert!(
                    (0..3)
                        .map(|k| (cur[k] - prev[k]).powi(2))
                        .sum::<f32>()
                        .sqrt()
                        < 0.15
                );
            }
            assert!(a.end[1] > p[1] || a.end[0] < p[0]);
        }
    }
    #[test]
    fn high_distant_and_backwards_approaches_are_rejected() {
        let mut c = scene(0.);
        assert!(plan(&c, [10., 7., 0.], 0.).is_none());
        assert!(plan(&c, [10., 8.5, 0.], 1.8).is_none());
        c.cells[7050].boxes[0].h[2] = 4.;
        assert!(plan(&c, [10., 8.5, 0.], 0.).is_none());
        c.cells[7050].boxes[0].h = [5., 1.5, 1.25];
        assert_eq!(plan(&c, [10., 7.5, 0.], 0.).unwrap().kind, "climb");
    }
    #[test]
    fn oblique_approach_and_reset_leave_no_attached_state() {
        let c = scene(0.35);
        let p = [10. + 0.35f32.sin() * 1.5, 10. - 0.35f32.cos() * 1.5, 0.];
        assert!(plan(&c, p, 0.6).is_some());
        let mut w = Walker::new(&c, [p[0], p[1]]).unwrap();
        w.set_heading(-0.35f32.to_degrees());
        assert!(w.request_vault(&c));
        w.step(&c, [0.; 2], 1. / 30., false, false);
        assert!(!w.request_vault(&c));
        w = Walker::new(&c, [p[0], p[1]]).unwrap();
        assert!(w.vault.is_none() && w.grounded);
        assert_eq!(w.vaults_completed, 0);
        w.grounded = false;
        assert!(!w.request_vault(&c));
    }
    #[test]
    fn blocked_landing_low_ceiling_and_triangle_ceiling_are_rejected() {
        let p = [10., 8.5, 0.];
        let mut c = scene(0.);
        let a = plan(&c, p, 0.).unwrap();
        c.cells[7050].cylinders.push(Cylinder {
            p: a.end,
            r: 0.2,
            h: 5.,
        });
        assert!(plan(&c, p, 0.).is_none());
        c.cells[7050].cylinders.clear();
        c.cells[7050].boxes.push(BoxShape {
            c: [10., 10., 4.3],
            h: [6., 6., 0.1],
            angle: 0.,
            flags: 0,
        });
        assert!(plan(&c, p, 0.).is_none());
        c.cells[7050].boxes.pop();
        c.cells[7050]
            .triangles
            .push([[5., 5., 4.], [16., 5., 4.], [10., 16., 4.]]);
        assert!(plan(&c, p, 0.).is_none());
    }
    #[test]
    fn water_and_excessive_drop_are_rejected() {
        let p = [10., 8.5, 0.];
        let mut c = scene(0.);
        c.cells[7050].ground.fill(0);
        assert!(plan(&c, p, 0.).is_none());
        c.cells[7050].boxes.push(BoxShape {
            c: [10., 12., -5.],
            h: [5., 1., 0.2],
            angle: 0.,
            flags: 0,
        });
        assert!(plan(&c, p, 0.).is_none());
    }
    #[test]
    fn vault_lands_once_and_returns_to_locomotion() {
        let c = scene(0.);
        let mut w = Walker::new(&c, [10., 8.5]).unwrap();
        let end = plan(&c, w.p, w.heading).unwrap().end;
        assert!(w.request_vault(&c));
        assert!(!w.request_vault(&c));
        for _ in 0..35 {
            w.step(&c, [0.; 2], 1. / 30., false, false);
        }
        assert_eq!(w.p, end);
        assert_eq!(w.vaults_completed, 1);
        assert!(w.grounded && w.vault.is_none());
        for _ in 0..15 {
            w.step(&c, [0., 1.], 1. / 30., false, false);
        }
        assert!(w.p[1] > end[1]);
    }
    #[test]
    fn new_obstacle_aborts_before_penetration_without_returning_to_start() {
        let mut c = scene(0.);
        let mut w = Walker::new(&c, [10., 8.5]).unwrap();
        assert!(w.request_vault(&c));
        for _ in 0..4 {
            w.step(&c, [0.; 2], 1. / 30., false, false);
        }
        let last = w.p;
        c.cells[7050].boxes.push(BoxShape {
            c: [10., 10., 4.5],
            h: [6., 6., 0.1],
            angle: 0.,
            flags: 0,
        });
        for _ in 0..10 {
            w.step(&c, [0.; 2], 1. / 30., false, false);
            if w.vault.is_none() {
                break;
            }
        }
        assert!(w.vault.is_none() && !w.grounded);
        assert_eq!(w.vaults_completed, 0);
        assert_eq!(w.vaults_aborted, 1);
        assert!(w.p[2] >= last[2]);
        assert!(c.body_clear(w.p, RADIUS, HEIGHT));
    }
}
