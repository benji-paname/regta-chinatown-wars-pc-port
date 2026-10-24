// SPDX-License-Identifier: GPL-3.0-only
// Port of ctw-native FollowPedCam (C) 2026 Kryo.to, followcam.cpp/.h.
// Original 20.12 spring/angles, 30 Hz updates, 60-degree perspective.
// PSP height25: EBOOT 0x089c71d8; wander yaw100/300: 0x088cd954..978.
// Mobile libGame.so 0x72e228/0x72e744; DS ARM9 0x020c69a2/0x020c6cb0.
// Original on-foot recenter states are distinct from ordinary facing wander.
// Collision queries and other camera branches remain reconstructed.
// Sphere/box contact uses our conservative Rust query rather than the original fixed-point sweep.
use crate::collision::Collision;
fn sin(a: i32) -> i32 {
    ((a as f32 * 9.587378e-05).sin() * 4096.) as i32
}
fn mul(a: i64, b: i64) -> i32 {
    ((a * b) >> 12) as i32
}
fn fx(p: [f32; 3]) -> [i32; 3] {
    p.map(|v| (v * 4096.).round() as i32)
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraProfile {
    #[default]
    Psp,
    Mobile,
    Ds,
}
impl CameraProfile {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "psp" => Some(Self::Psp),
            "mobile" => Some(Self::Mobile),
            "ds" => Some(Self::Ds),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Psp => "psp",
            Self::Mobile => "mobile",
            Self::Ds => "ds",
        }
    }
    pub fn height(self) -> i32 {
        match self {
            Self::Psp | Self::Ds => 25,
            Self::Mobile => 35,
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Psp => Self::Mobile,
            Self::Mobile => Self::Ds,
            Self::Ds => Self::Psp,
        }
    }
}
#[derive(Clone, Copy, Default)]
pub struct FollowInput {
    pub sprint: bool,
    // Signed stick direction relative to camera, as GetWalkAngle returns it.
    pub walk_angle: Option<i16>,
    pub recenter: bool,
}
pub struct Camera {
    pub profile: CameraProfile,
    pos: [i32; 3],
    vel: [i32; 3],
    target: [i32; 3],
    pub yaw: u16,
    pub pitch: u16,
    pitch_vel: i32,
    blocked: u32,
    recenter_mode: u8,
    turn_side: i8,
    sprint_ticks: u8,
    previous_walk_angle: i16,
    free: Option<FreeCamera>,
}
struct FreeCamera {
    eye: [f32; 3],
    yaw: f32,
    pitch: f32,
}
impl Camera {
    pub fn new(p: [f32; 3], heading: u16, profile: CameraProfile) -> Self {
        let t = fx(p);
        let cp = sin(55000i32 as i16 as i32 + 0x4000).abs();
        Self {
            profile,
            pos: [
                t[0] + mul(cp as i64 * -24, sin(heading as i16 as i32) as i64),
                t[1] + mul(cp as i64 * -24, sin(heading as i16 as i32 + 0x4000) as i64),
                t[2] + (profile.height() + 2) * 4096,
            ],
            vel: [0; 3],
            target: t,
            yaw: heading,
            pitch: 55000,
            pitch_vel: 0,
            blocked: 0,
            recenter_mode: 0,
            turn_side: 0,
            sprint_ticks: 0,
            previous_walk_angle: 0,
            free: None,
        }
    }
    pub fn set_profile(&mut self, profile: CameraProfile, p: [f32; 3]) {
        // Retain orientation and the free camera; never alter the player.
        let free = self.free.take();
        *self = Self::new(p, self.yaw, profile);
        self.free = free;
    }
    pub fn is_free(&self) -> bool {
        self.free.is_some()
    }
    pub fn toggle_free(&mut self) {
        self.free = if self.is_free() {
            None
        } else {
            Some(FreeCamera {
                eye: self.eye(),
                yaw: self.yaw as i16 as f32 * std::f32::consts::TAU / 65536.,
                pitch: self.pitch as i16 as f32 * std::f32::consts::TAU / 65536.,
            })
        };
    }
    pub fn fly(&mut self, movement: [f32; 3], look: [f32; 2], dt: f32, fast: bool) {
        let Some(free) = &mut self.free else {
            return;
        };
        free.yaw = (free.yaw + look[0] * 0.004).rem_euclid(std::f32::consts::TAU);
        free.pitch = (free.pitch - look[1] * 0.004).clamp(-1.55334, 1.55334);
        let (s, c) = free.yaw.sin_cos();
        let len = movement.iter().map(|v| v * v).sum::<f32>().sqrt().max(1.);
        let step = dt * if fast { 120. } else { 30. } / len;
        free.eye[0] += (movement[0] * c + movement[1] * s) * step;
        free.eye[1] += (-movement[0] * s + movement[1] * c) * step;
        free.eye[2] += movement[2] * step;
    }
    pub fn eye(&self) -> [f32; 3] {
        self.free
            .as_ref()
            .map_or_else(|| self.pos.map(|v| v as f32 / 4096.), |f| f.eye)
    }
    pub fn radar_angle(&self) -> f32 {
        self.free
            .as_ref()
            .map_or(self.yaw as f32 * std::f32::consts::TAU / 65536., |f| f.yaw)
    }
    pub fn recenter_mode(&self) -> u8 {
        self.recenter_mode
    }
    fn recenter_request(&mut self, input: FollowInput) -> u8 {
        // Request uses the previous counter: controller Process precedes camera Update.
        // PSP 0x089cf36c/0x089cff50/0x089d0398; mobile IsToAutoCentre 0x71adc0.
        let automatic = match self.profile {
            CameraProfile::Psp => self.sprint_ticks >= 10,
            CameraProfile::Mobile => self.sprint_ticks >= 30,
            CameraProfile::Ds => false, // DS yoke+0x60: explicit camera action, no PSP timer.
        };
        match self.profile {
            CameraProfile::Psp => {
                self.sprint_ticks = if input.sprint {
                    self.sprint_ticks.saturating_add(1).min(100)
                } else {
                    0
                };
            }
            CameraProfile::Mobile => {
                // APK 0x71bd48..9c: lateral/backwards stick >5000, same sign, sprint.
                let angle = input.walk_angle.unwrap_or(-1);
                if (angle as i32).abs() > 5000 {
                    let previous = self.previous_walk_angle;
                    self.previous_walk_angle = angle;
                    self.sprint_ticks = if input.sprint && previous as i32 * angle as i32 >= 0 {
                        self.sprint_ticks.saturating_add(1).min(100)
                    } else {
                        0
                    };
                } else {
                    self.previous_walk_angle = angle.wrapping_neg();
                    self.sprint_ticks = 0;
                }
            }
            CameraProfile::Ds => self.sprint_ticks = 0,
        }
        if input.recenter {
            if self.profile == CameraProfile::Ds {
                1
            } else {
                2
            }
        } else if automatic {
            1
        } else {
            0
        }
    }
    fn face_heading(&mut self, heading: u16, moving: bool, can_see: bool) {
        if !moving && self.recenter_mode == 0 {
            return;
        }
        let mut diff = heading.wrapping_sub(self.yaw) as i16 as i32;
        let (mut div, mut max) = if self.recenter_mode != 0 {
            (10, 0xaaa)
        } else if can_see {
            (2, 100)
        } else {
            (3, 300)
        };
        if self.profile == CameraProfile::Mobile {
            // ProcessFacingWander 0x72dc48: /3 for a normal player outside cutscenes.
            max /= 3;
        }
        if self.recenter_mode == 1 && self.profile != CameraProfile::Ds {
            // PSP automatic recenter: commit to one side through a large turn.
            if diff.abs() >= 0x5557 {
                if self.turn_side == 0 {
                    self.turn_side = if diff < 0 { -1 } else { 1 };
                }
                diff = self.turn_side as i32 * 0x5556;
            } else {
                self.turn_side = 0;
            }
            div = div * 3 / 2;
            max /= 2;
        } else if self.recenter_mode != 0 {
            // PSP/mobile explicit recenter and DS's single recenter state.
            if diff.abs() > 0x71c7 && (self.profile != CameraProfile::Ds || moving) {
                max *= 2;
                div /= 4;
            }
        } else if diff.abs() > 0x7c72 {
            div = 1;
            max = 1;
        }
        let step = (if diff.abs() > div {
            diff / div.max(1)
        } else {
            diff
        })
        .clamp(-max, max);
        if self.recenter_mode != 0 && step.abs() < 40 {
            self.recenter_mode = 0;
        }
        self.yaw = self.yaw.wrapping_add(step as u16);
    }
    #[cfg(test)]
    pub fn step(&mut self, p: [f32; 3], heading: u16, velocity: [f32; 3], col: &Collision) {
        self.step_with_input(p, heading, velocity, col, FollowInput::default());
    }
    pub fn step_with_input(
        &mut self,
        p: [f32; 3],
        heading: u16,
        velocity: [f32; 3],
        col: &Collision,
        input: FollowInput,
    ) {
        if self.is_free() {
            return;
        }
        let request = self.recenter_request(input);
        // Original Update clears a latched recenter when the normal player moves,
        // then promotes it with this tick's yoke request. Cutscenes are not ported.
        if self.target != fx(p) {
            self.recenter_mode = 0;
        }
        self.recenter_mode = self.recenter_mode.max(request);
        let recentering = self.recenter_mode != 0;
        let old = self.eye();
        let target = self.target.map(|v| v as f32 / 4096.);
        let yaw = self.yaw as i16 as i32;
        let a = [target[0], target[1], target[2] + 2.];
        let k = sin(0x16d8).abs();
        let b = [
            target[0] + mul(k as i64 * -24, sin(yaw) as i64) as f32 / 4096.,
            target[1] + mul(k as i64 * -24, sin(yaw + 0x4000) as i64) as f32 / 4096.,
            target[2] + (self.profile.height() + 2) as f32,
        ];
        let can_see = col.trace_boxes(a, b, 0., true).is_none();
        let mut pitch_target: u16 = if can_see { 55000 } else { 0xc000 };
        if !can_see {
            let d = [target[0] - old[0], target[1] - old[1]];
            let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
            let yawf = yaw as f32 * std::f32::consts::TAU / 65536.;
            if len > 0.1
                && (d[0] * yawf.sin() + d[1] * yawf.cos()) / len > 0.9
                && col
                    .trace_boxes(a, [target[0], target[1], target[2] + 102.], 0., false)
                    .is_some()
            {
                pitch_target = 55000;
            }
        }
        let diff = pitch_target.wrapping_sub(self.pitch) as i16 as i32 * 4096;
        self.pitch_vel = mul(diff as i64, 399) + mul(self.pitch_vel as i64, 0x7ce);
        self.pitch = self.pitch.wrapping_add((self.pitch_vel >> 12) as u16);
        self.target = fx(p);
        self.face_heading(
            heading,
            velocity.iter().map(|v| v * v).sum::<f32>() >= 40. / 4096.,
            can_see,
        );
        let yaw = self.yaw as i16 as i32;
        let cp = sin(self.pitch as i16 as i32 + 0x4000).abs();
        let offsets = [
            mul(cp as i64 * -24, sin(yaw) as i64),
            mul(cp as i64 * -24, sin(yaw + 0x4000) as i64),
            (self.profile.height() + 2) * 4096,
        ];
        for i in 0..3 {
            self.vel[i] = mul(
                (self.target[i] - self.pos[i] + offsets[i]) as i64,
                // Update selects gain0x666 during recenter, 0x333 in wander;
                // ProcessFacingWander doubles it on Z on all three originals.
                (if recentering { 0x666 } else { 0x333 }) * if i == 2 { 2 } else { 1 },
            ) + mul(self.vel[i] as i64, 0x38d);
        }
        let mut want = std::array::from_fn(|i| (self.pos[i] + self.vel[i]) as f32 / 4096.);
        let mut from = old;
        for _ in 0..4 {
            let Some((t, n)) = col.trace_boxes(from, want, 1.30005, false) else {
                break;
            };
            let contact = std::array::from_fn::<_, 3, _>(|i| from[i] + (want[i] - from[i]) * t);
            let dot = (0..3).map(|i| (want[i] - contact[i]) * n[i]).sum::<f32>();
            want = std::array::from_fn(|i| want[i] - dot * n[i] + n[i] * 0.01);
            from = std::array::from_fn(|i| contact[i] + n[i] * 0.01);
        }
        let a = [p[0], p[1], p[2] + 2.];
        if self.blocked >= 6
            && col
                .trace_boxes(a, [p[0], p[1], want[2] + 30.], 0., false)
                .is_none()
        {
            want[0] = p[0];
            want[1] = p[1];
        }
        if col.trace_boxes(a, want, 0., false).is_some() {
            self.blocked = self.blocked.saturating_add(1);
        } else {
            self.blocked = 0;
        }
        self.pos = fx(want);
    }
    pub fn matrix(&self) -> [f32; 16] {
        let (y, p) = self.free.as_ref().map_or(
            (
                self.yaw as i16 as f32 * std::f32::consts::TAU / 65536.,
                self.pitch as i16 as f32 * std::f32::consts::TAU / 65536.,
            ),
            |f| (f.yaw, f.pitch),
        );
        let (sy, cy) = y.sin_cos();
        let (sp, cp) = p.sin_cos();
        let s = [cy, -sy, 0.];
        let u = [-sy * sp, -cy * sp, cp];
        let f = [sy * cp, cy * cp, sp];
        let e = self.eye();
        let dot = |v: [f32; 3]| (0..3).map(|i| v[i] * e[i]).sum::<f32>();
        [
            s[0],
            u[0],
            -f[0],
            0.,
            s[1],
            u[1],
            -f[1],
            0.,
            s[2],
            u[2],
            -f[2],
            0.,
            -dot(s),
            -dot(u),
            dot(f),
            1.,
        ]
    }
    pub fn movement(&self, x: f32, y: f32) -> [f32; 2] {
        let a = self.yaw as f32 * std::f32::consts::TAU / 65536.;
        let (s, c) = a.sin_cos();
        [x * c + y * s, -x * s + y * c]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn psp_height_turn_rate_and_profile_switch_preserve_state() {
        let col = Collision {
            cells: (0..14000).map(|_| Default::default()).collect(),
        };
        let p = [10., 20., 3.];
        let mut cam = Camera::new(p, 0, CameraProfile::Psp);
        assert_eq!(cam.eye()[2], 30.);
        cam.step(p, 16384, [1., 0., 0.], &col);
        assert_eq!(cam.yaw, 100);
        cam.toggle_free();
        cam.fly([1., 0., 0.], [0., 0.], 1., false);
        let free = cam.eye();
        cam.set_profile(CameraProfile::Mobile, p);
        assert_eq!(cam.eye(), free);
        assert_eq!(cam.yaw, 100);
        cam.toggle_free();
        assert_eq!(cam.eye()[2], 40.);
        cam.set_profile(CameraProfile::Psp, p);
        assert_eq!(cam.eye()[2], 30.);
        assert_eq!(CameraProfile::parse("ds"), Some(CameraProfile::Ds));
        assert_eq!(CameraProfile::Psp.next().next().next(), CameraProfile::Psp);
        assert_eq!(CameraProfile::parse("unknown"), None);
    }
    #[test]
    fn psp_sprint_recenter_starts_after_ten_controller_ticks_and_tracks_heading() {
        let col = Collision {
            cells: (0..14000).map(|_| Default::default()).collect(),
        };
        let mut cam = Camera::new([0.; 3], 0, CameraProfile::Psp);
        let input = FollowInput {
            sprint: true,
            walk_angle: Some(16384),
            recenter: false,
        };
        for tick in 0..10 {
            cam.step_with_input([tick as f32, 0., 0.], 16384, [18., 0., 0.], &col, input);
            assert_eq!(cam.yaw, (tick + 1) * 100);
            assert_eq!(cam.recenter_mode(), 0);
        }
        cam.step_with_input([10., 0., 0.], 16384, [18., 0., 0.], &col, input);
        assert_eq!(cam.yaw, 2025); // (16384 - 1000)/15, cap1365.
        for tick in 11..70 {
            cam.step_with_input([tick as f32, 0., 0.], 16384, [18., 0., 0.], &col, input);
        }
        assert!((cam.yaw as i32 - 16384).abs() < 400);
    }
    #[test]
    fn mobile_original_requires_thirty_ticks_and_consistent_lateral_stick() {
        let mut cam = Camera::new([0.; 3], 0, CameraProfile::Mobile);
        let mut input = FollowInput {
            sprint: true,
            walk_angle: Some(16384),
            recenter: false,
        };
        for _ in 0..30 {
            assert_eq!(cam.recenter_request(input), 0);
        }
        assert_eq!(cam.recenter_request(input), 1);
        input.walk_angle = Some(-16384);
        assert_eq!(cam.recenter_request(input), 1); // Previous counter is sampled first.
        assert_eq!(cam.recenter_request(input), 0);
        input.walk_angle = Some(0);
        for _ in 0..40 {
            assert_eq!(cam.recenter_request(input), 0);
        }
        cam.recenter_mode = 1;
        cam.face_heading(16384, true, true);
        assert_eq!(cam.yaw, 455); // APK /3, then automatic-mode /2.
    }
    #[test]
    fn ds_manual_recenter_uses_original_single_state_and_half_turn_rule() {
        let mut cam = Camera::new([0.; 3], 0, CameraProfile::Ds);
        for _ in 0..100 {
            assert_eq!(
                cam.recenter_request(FollowInput {
                    sprint: true,
                    walk_angle: Some(16384),
                    recenter: false,
                }),
                0
            );
        }
        cam.recenter_mode = cam.recenter_request(FollowInput {
            recenter: true,
            ..Default::default()
        });
        cam.face_heading(32768, true, true);
        assert_eq!(cam.yaw, 0u16.wrapping_sub(5460));
        let mut psp = Camera::new([0.; 3], 0, CameraProfile::Psp);
        psp.recenter_mode = 1;
        psp.face_heading(32768, true, true);
        assert_eq!(psp.yaw, 0u16.wrapping_sub(1365));
        psp.face_heading(31000, true, true); // Cross the angle seam, keep committed turn side.
        assert_eq!(psp.yaw, 0u16.wrapping_sub(2730));
    }
    #[test]
    fn recenter_finishes_at_rest_and_profile_switch_resets_controller_state() {
        let col = Collision {
            cells: (0..14000).map(|_| Default::default()).collect(),
        };
        for profile in [CameraProfile::Psp, CameraProfile::Mobile, CameraProfile::Ds] {
            let mut cam = Camera::new([0.; 3], 0, profile);
            cam.step_with_input(
                [0.; 3],
                16384,
                [0.; 3],
                &col,
                FollowInput {
                    recenter: true,
                    ..Default::default()
                },
            );
            assert!(cam.yaw > 0);
            for _ in 0..120 {
                cam.step([0.; 3], 16384, [0.; 3], &col);
            }
            assert_eq!(cam.recenter_mode(), 0);
            assert!((cam.yaw as i32 - 16384).abs() < 400);
            cam.sprint_ticks = 50;
            cam.toggle_free();
            let eye = cam.eye();
            cam.step_with_input(
                [50.; 3],
                32000,
                [18.; 3],
                &col,
                FollowInput {
                    recenter: true,
                    ..Default::default()
                },
            );
            assert_eq!(cam.eye(), eye);
            cam.set_profile(profile.next(), [0.; 3]);
            assert_eq!(cam.sprint_ticks, 0);
            assert_eq!(cam.recenter_mode(), 0);
            assert_eq!(cam.eye(), eye);
        }
    }
    #[test]
    fn free_camera_moves_independently_and_restores_follow_view() {
        let mut c = Camera::new([10., 20., 3.], 0, CameraProfile::Mobile);
        let eye = c.eye();
        let view = c.matrix();
        c.toggle_free();
        c.fly([0., 1., 0.], [0., 0.], 1., false);
        assert!((c.eye()[1] - eye[1] - 30.).abs() < 0.001);
        c.fly([0., 0., 1.], [80., -10000.], 0.5, true);
        assert!((c.eye()[2] - eye[2] - 60.).abs() < 0.001);
        assert!(c.matrix().iter().all(|v| v.is_finite()));
        c.toggle_free();
        assert_eq!(c.eye(), eye);
        assert_eq!(c.matrix(), view);
    }
    #[test]
    fn fixed_point_reset_matches_reference() {
        let c = Camera::new([10., 20., 3.], 0, CameraProfile::Mobile);
        assert_eq!(c.pos[2], 40 * 4096);
        assert_eq!(c.pos[0], 10 * 4096);
        assert_eq!(c.pitch, 55000);
        assert!(c.eye()[1] < 8.);
        let m = c.matrix();
        assert!(m.iter().all(|v| v.is_finite()));
    }
    #[test]
    fn reference_yaw_limits_and_180_degree_rule() {
        let c = Collision {
            cells: (0..14000).map(|_| Default::default()).collect(),
        };
        let mut cam = Camera::new([0.; 3], 0, CameraProfile::Mobile);
        cam.step([0.; 3], 16384, [1., 0., 0.], &c);
        assert_eq!(cam.yaw, 33);
        let mut cam = Camera::new([0.; 3], 0, CameraProfile::Mobile);
        cam.step([0.; 3], 32768, [1., 0., 0.], &c);
        assert_eq!(cam.yaw, 65535);
    }
    #[test]
    fn obstruction_changes_pitch_and_clear_view_restores_it() {
        let mut c = Collision {
            cells: (0..14000).map(|_| Default::default()).collect(),
        };
        c.cells[70 * 100 + 49]
            .boxes
            .push(crate::collision::BoxShape {
                c: [0., -9., 27.],
                h: [3., 2., 6.],
                angle: 0.,
                flags: 0,
            });
        let mut cam = Camera::new([0.; 3], 0, CameraProfile::Mobile);
        cam.step([0.; 3], 0, [0.; 3], &c);
        assert!(cam.pitch < 55000);
        c.cells[70 * 100 + 49].boxes.clear();
        for _ in 0..120 {
            cam.step([0.; 3], 0, [0.; 3], &c);
        }
        // Integer spring settles within five angle units (0.028 degrees).
        assert!((cam.pitch as i32 - 55000).abs() <= 5);
    }
}
