// SPDX-License-Identifier: GPL-3.0-only
//! PSP player impulse/ground drag; conservative reconstructed contacts.
use crate::{
    collision::Collision,
    vault::{HEIGHT, RADIUS},
};
pub const DURATION: f32 = 16. / 30.; // Calibrated to supplied PSP roll video (~0.53s).
                                     // ULES01347: HandlePlayerDive0x089cfc0c strength30;
                                     // ApplyPhysicalEffect(0)0x0883ba38 replaces velocity (does not add sprint speed).
                                     // PhysicalReactionPercentage defaults to100 at0x089a71a8.
pub const INITIAL_SPEED: f32 = 30.;
const TICK: f32 = 1. / 30.;
const INTEGRATION: f32 = 17. / 512.;
pub struct Roll {
    pub elapsed: f32,
    pub heading: f32,
    pub entry_level: u8,
    pub blocked: bool,
    velocity_fixed: [i32; 2],
    tick_remaining: f32,
}
impl Roll {
    pub fn new(heading: f32, entry_level: u8) -> Self {
        Self::with_speed(heading, entry_level, 30)
    }
    pub fn with_speed(heading: f32, entry_level: u8, speed: i32) -> Self {
        Self {
            elapsed: 0.,
            heading,
            entry_level,
            blocked: false,
            velocity_fixed: [
                (-heading.sin() * 4096.) as i32 * speed,
                (heading.cos() * 4096.) as i32 * speed,
            ],
            tick_remaining: 0.,
        }
    }
    pub fn progress(&self) -> f32 {
        (self.elapsed / DURATION).clamp(0., 1.)
    }
    pub fn travel(&mut self, c: &Collision, p: &mut [f32; 3], dt: f32) -> bool {
        let mut remaining = dt.max(0.);
        while remaining > 1e-7 {
            if self.tick_remaining <= 1e-7 {
                // cAnimation::Process calls ApplyPhysics after setting the
                // impulse, before the mover integrates the resulting velocity.
                ground_drag(&mut self.velocity_fixed);
                self.tick_remaining = TICK;
            }
            let step = remaining.min(self.tick_remaining);
            let delta = self
                .velocity_fixed
                .map(|v| v as f32 / 4096. * INTEGRATION * step / TICK);
            if !advance(c, p, delta) {
                self.velocity_fixed = [0; 2];
                return false;
            }
            self.tick_remaining -= step;
            remaining -= step;
        }
        true
    }
}
fn ground_drag(v: &mut [i32; 2]) {
    // PSP ApplyPhysics0x0883c2b0..c36c and0x0883c68c..c7d8.
    // Grounded branch: coefficient40, clamp40..4014, stop below0.5u/s.
    let speed2 = v.iter().map(|&x| i64::from(x) * i64::from(x)).sum::<i64>();
    if speed2 < 0x400000 {
        *v = [0; 2];
        return;
    }
    let k = (((((speed2 >> 12) * 136) >> 12) * 40) >> 12).clamp(0x28, 0xfae);
    for component in v {
        *component = ((i64::from(*component) * (4096 - k)) >> 12) as i32;
    }
}
pub fn advance(c: &Collision, p: &mut [f32; 3], delta: [f32; 2]) -> bool {
    let distance = delta[0].hypot(delta[1]);
    let steps = (distance / 0.08).ceil().max(1.) as usize;
    for _ in 0..steps {
        let mut next = *p;
        next[0] += delta[0] / steps as f32;
        next[1] += delta[1] / steps as f32;
        let g = c.ground(next[0], next[1], p[2]);
        if g.water || g.z > p[2] + 0.5 || g.z < p[2] - 0.5 {
            return false;
        }
        next[2] = g.z;
        if !c.body_clear(next, RADIUS, HEIGHT) {
            return false;
        }
        // Do not roll off a ledge or into water at the edge of the body footprint.
        for i in 0..8 {
            let a = i as f32 * std::f32::consts::TAU / 8.;
            let support = c.ground(
                next[0] + a.cos() * RADIUS,
                next[1] + a.sin() * RADIUS,
                next[2],
            );
            if support.water || (support.z - next[2]).abs() > 0.5 {
                return false;
            }
        }
        *p = next;
    }
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::{BoxShape, Cell, Walker};
    fn scene() -> Collision {
        Collision {
            cells: (0..14000)
                .map(|_| Cell {
                    ground: vec![0x55; 400],
                    ..Cell::default()
                })
                .collect(),
        }
    }
    #[test]
    fn psp_impulse_has_a_real_burst_without_movement_input_then_slows() {
        let c = scene();
        for level in [0, 1, 2, 3] {
            let mut w = Walker::new(&c, [10., 10.]).unwrap();
            w.level = level;
            w.queue_action();
            let mut distances = Vec::new();
            for _ in 0..16 {
                let old = w.p;
                w.step(&c, [0.; 2], TICK, false, false);
                distances.push(w.p[1] - old[1]);
            }
            // Starts faster than sprint18, even when the stick is released.
            assert!(distances[0] / TICK > 18.);
            assert!(distances[0] > distances[15] * 2.);
            assert!(distances.windows(2).all(|d| d[1] < d[0]));
            assert!((w.p[1] - 16.636669).abs() < 0.001);
            let end = w.p;
            w.step(&c, [0.; 2], TICK, false, false);
            assert_eq!(w.p, end); // No uncontrolled drift after recovery.
        }
    }
    #[test]
    fn psp_drag_integer_reference_and_stop_threshold() {
        // Values independently evaluated from the PSP shifts/multiplies.
        let expected = [
            87030, 74302, 66374, 60734, 56419, 52961, 50090, 47668, 45584, 43758, 42144, 40703,
            39401, 38227, 37153, 36164,
        ];
        let mut v = [0, 122880];
        for y in expected {
            ground_drag(&mut v);
            assert_eq!(v, [0, y]);
        }
        v = [0, 2047];
        ground_drag(&mut v);
        assert_eq!(v, [0, 0]);
    }
    #[test]
    fn impulse_integration_does_not_depend_on_render_frame_splits() {
        let c = scene();
        let mut a = Roll::new(0., 0);
        let mut b = Roll::new(0., 0);
        let mut p = [10., 10., 0.];
        let mut q = p;
        for _ in 0..16 {
            assert!(a.travel(&c, &mut p, TICK));
            assert!(b.travel(&c, &mut q, TICK / 2.));
            assert!(b.travel(&c, &mut q, TICK / 2.));
        }
        assert!((p[1] - q[1]).abs() < 0.001);
        assert_eq!(a.velocity_fixed, b.velocity_fixed);
    }
    #[test]
    fn roll_finishes_once_in_all_directions_and_resumes_sprint() {
        let c = scene();
        for degrees in (0..360).step_by(45) {
            let mut w = Walker::new(&c, [10., 10.]).unwrap();
            w.set_heading(degrees as f32);
            w.level = 3;
            let heading = w.heading;
            w.queue_action();
            for tick in 0..16 {
                w.step(&c, [0., 1.], 1. / 30., true, false);
                assert!(c.body_clear(w.p, RADIUS, HEIGHT));
                if tick < 15 {
                    assert!(w.roll.is_some());
                    w.queue_action();
                }
            }
            assert!(w.roll.is_none() && w.vault.is_none());
            assert_eq!(w.rolls_completed, 1);
            assert_eq!(w.rolls_blocked, 0);
            assert_eq!(w.level, 3);
            // 16 original fixed-point ground-drag steps, baseline reaction100%.
            assert!((w.p[0] - (10. - heading.sin() * 6.636669)).abs() < 0.01);
            assert!((w.p[1] - (10. + heading.cos() * 6.636669)).abs() < 0.01);
            w.step(&c, [0., 1.], 1. / 30., true, false);
            assert!(w.roll.is_none());
        }
    }
    #[test]
    fn roll_cannot_tunnel_through_a_wall_or_cross_water_and_ledge() {
        let mut c = scene();
        c.cells[7050].boxes.push(BoxShape {
            c: [10., 12., 4.],
            h: [5., 0.05, 4.],
            angle: 0.,
            flags: 0,
        });
        let mut w = Walker::new(&c, [10., 10.]).unwrap();
        w.queue_action();
        for _ in 0..20 {
            w.step(&c, [0.; 2], 1. / 30., false, false);
            assert!(c.body_clear(w.p, RADIUS, HEIGHT));
        }
        assert_eq!(w.rolls_completed, 1);
        assert_eq!(w.rolls_blocked, 1);
        assert!(w.p[1] < 11.1);
        c.cells[7050].boxes.clear();
        c.cells[7050].ground.fill(0);
        let mut p = [10., 10., 0.];
        assert!(!advance(&c, &mut p, [0., 3.]));
        assert_eq!(p, [10., 10., 0.]);
        c.cells[7050].ground.fill(0x55);
        c.cells[7050].boxes.push(BoxShape {
            c: [10., 10., 1.],
            h: [2., 2., 1.],
            angle: 0.,
            flags: 0,
        });
        p = [10., 10., 2.];
        for _ in 0..16 {
            advance(&c, &mut p, [0., 1.]);
        }
        assert!(p[1] < 11.2);
        assert_eq!(p[2], 2.);
    }
    #[test]
    fn contextual_button_prefers_vault_and_ignores_airborne_input() {
        let mut c = scene();
        c.cells[7050].boxes.push(BoxShape {
            c: [10., 10., 1.25],
            h: [5., 0.4, 1.25],
            angle: 0.,
            flags: 0,
        });
        let mut w = Walker::new(&c, [10., 8.5]).unwrap();
        w.queue_action();
        w.step(&c, [0.; 2], 1. / 30., false, false);
        assert!(w.vault.is_some() && w.roll.is_none());
        let mut w = Walker::new(&c, [20., 20.]).unwrap();
        w.grounded = false;
        w.p[2] = 4.;
        w.queue_action();
        w.step(&c, [0.; 2], 1. / 30., false, false);
        assert!(w.roll.is_none() && w.vault.is_none());
    }
}
