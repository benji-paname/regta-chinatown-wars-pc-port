// SPDX-License-Identifier: GPL-3.0-only
//! XInput state polling; PSP face-button positions, without keyboard emulation.
use std::time::{Duration, Instant};
pub const DPAD_UP: u16 = 0x0001;
pub const DPAD_DOWN: u16 = 0x0002;
pub const DPAD_LEFT: u16 = 0x0004;
pub const DPAD_RIGHT: u16 = 0x0008;
pub const BACK: u16 = 0x0020;
pub const B: u16 = 0x2000;
pub const Y: u16 = 0x8000;
pub const RB: u16 = 0x0200;
pub const A: u16 = 0x1000;
pub const X: u16 = 0x4000;
pub const LB: u16 = 0x0100;
pub const START: u16 = 0x0010;
pub const RSTICK: u16 = 0x0080;
pub const LSTICK: u16 = 0x0040;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RawPad {
    pub buttons: u16,
    pub lt: u8,
    pub rt: u8,
    pub lx: i16,
    pub ly: i16,
    pub rx: i16,
    pub ry: i16,
}
#[repr(C)]
#[derive(Default)]
struct State {
    packet: u32,
    pad: RawPad,
}
#[derive(Clone, Copy, Default)]
pub struct Input {
    pub connected: bool,
    pub movement: [f32; 2],
    pub look: [f32; 2],
    pub lift: f32,
    pub held: u16,
    pub pressed: u16,
}
impl Input {
    pub fn chord(self, mask: u16) -> bool {
        self.held & mask == mask && self.pressed & mask != 0
    }
    pub fn held(self, button: u16) -> bool {
        self.held & button != 0
    }
    pub fn pressed(self, button: u16) -> bool {
        self.pressed & button != 0
    }
    pub fn walk(self) -> bool {
        let magnitude = self.movement[0].hypot(self.movement[1]);
        magnitude > 0. && magnitude < 0.55 && !self.held(A)
    }
}
fn stick(x: i16, y: i16) -> [f32; 2] {
    stick_deadzone(x, y, 7849.)
}
fn stick_deadzone(x: i16, y: i16, deadzone: f32) -> [f32; 2] {
    let (x, y) = (x as f32, y as f32);
    let length = x.hypot(y);
    // Microsoft LEFT_THUMB_DEADZONE, radial and rescaled to avoid drift/jumps.
    if length <= deadzone {
        return [0.; 2];
    }
    let scale = ((length - deadzone) / (32767. - deadzone)).min(1.) / length;
    [x * scale, y * scale]
}
fn trigger(v: u8) -> f32 {
    (v.saturating_sub(30) as f32) / 225.
}
#[derive(Default)]
struct Edges {
    previous: u16,
    suppressed: u16,
    focused: bool,
    slot: Option<u32>,
}
impl Edges {
    fn sample(&mut self, raw: Option<(u32, RawPad)>, active: bool) -> Input {
        let Some((slot, pad)) = raw else {
            *self = Self::default();
            return Input::default();
        };
        if !active || !self.focused || self.slot != Some(slot) {
            self.previous = pad.buttons;
            self.suppressed = pad.buttons;
        }
        self.focused = active;
        self.slot = Some(slot);
        self.suppressed &= pad.buttons;
        let buttons = pad.buttons & !self.suppressed;
        let pressed = buttons & !self.previous;
        self.previous = buttons;
        if !active {
            return Input {
                connected: true,
                ..Input::default()
            };
        }
        Input {
            connected: true,
            movement: stick(pad.lx, pad.ly),
            look: stick_deadzone(pad.rx, pad.ry, 8689.),
            lift: trigger(pad.rt) - trigger(pad.lt),
            held: buttons,
            pressed,
        }
    }
}
type GetState = unsafe extern "system" fn(u32, *mut State) -> u32;
pub struct Gamepad {
    library: *mut std::ffi::c_void,
    get_state: Option<GetState>,
    slot: Option<u32>,
    next_scan: Instant,
    edges: Edges,
}
impl Gamepad {
    pub fn new() -> Self {
        let mut pad = Self {
            library: std::ptr::null_mut(),
            get_state: None,
            slot: None,
            next_scan: Instant::now(),
            edges: Edges::default(),
        };
        // Only system DLLs; no redistributable or additional dependency needed.
        for name in ["xinput1_4.dll", "xinput9_1_0.dll"] {
            let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            unsafe {
                let library = LoadLibraryExW(wide.as_ptr(), std::ptr::null_mut(), 0x800);
                if library.is_null() {
                    continue;
                }
                let function = GetProcAddress(library, b"XInputGetState\0".as_ptr());
                if function.is_null() {
                    FreeLibrary(library);
                    continue;
                }
                pad.library = library;
                pad.get_state = Some(std::mem::transmute::<*mut std::ffi::c_void, GetState>(
                    function,
                ));
                break;
            }
        }
        pad
    }
    fn state(&self, slot: u32) -> Option<RawPad> {
        let mut state = State::default();
        (unsafe { (self.get_state?)(slot, &mut state) } == 0).then_some(state.pad)
    }
    pub fn poll(&mut self, active: bool) -> Input {
        let mut sample = self
            .slot
            .and_then(|slot| self.state(slot).map(|p| (slot, p)));
        if sample.is_none() {
            self.slot = None;
            if Instant::now() >= self.next_scan {
                self.next_scan = Instant::now() + Duration::from_secs(1);
                sample = (0..4).find_map(|slot| self.state(slot).map(|p| (slot, p)));
                self.slot = sample.map(|(slot, _)| slot);
            }
        }
        self.edges.sample(sample, active)
    }
}
impl Drop for Gamepad {
    fn drop(&mut self) {
        if !self.library.is_null() {
            unsafe {
                FreeLibrary(self.library);
            }
        }
    }
}
#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryExW(
        name: *const u16,
        file: *mut std::ffi::c_void,
        flags: u32,
    ) -> *mut std::ffi::c_void;
    fn GetProcAddress(module: *mut std::ffi::c_void, name: *const u8) -> *mut std::ffi::c_void;
    fn FreeLibrary(module: *mut std::ffi::c_void) -> i32;
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn radial_deadzone_and_extremes_are_bounded() {
        assert_eq!(stick(1000, -2000), [0.; 2]);
        assert_eq!(stick(7849, 0), [0.; 2]);
        assert!(stick(7850, 0)[0] < 0.001);
        for (x, y) in [(32767, 0), (-32768, 0), (32767, 32767), (-32768, -32768)] {
            let v = stick(x, y);
            assert!((v[0].hypot(v[1]) - 1.).abs() < 1e-5);
        }
    }
    #[test]
    fn held_buttons_do_not_retrigger_on_focus_or_connection() {
        let mut e = Edges::default();
        let sample = |b| {
            Some((
                0,
                RawPad {
                    buttons: b,
                    ..RawPad::default()
                },
            ))
        };
        assert_eq!(e.sample(sample(X | A), true).held, 0);
        e.sample(sample(0), true);
        assert!(e.sample(sample(X | A), true).pressed(X));
        assert!(!e.sample(sample(X | A), true).pressed(X));
        assert_eq!(e.sample(sample(X | A), false).held, 0);
        assert_eq!(e.sample(sample(X | A), true).held, 0);
        assert!(!e.sample(None, true).connected);
        assert_eq!(e.sample(sample(X), true).pressed, 0);
        e.sample(sample(0), true);
        assert!(e.sample(sample(X), true).pressed(X));
    }
    #[test]
    fn camera_axes_triggers_and_chords_ignore_drift_and_focus_loss() {
        let mut e = Edges::default();
        let mut p = RawPad::default();
        e.sample(Some((0, p)), true);
        p.rx = 4000;
        p.rt = 30;
        let neutral = e.sample(Some((0, p)), true);
        assert_eq!(neutral.look, [0.; 2]);
        assert_eq!(neutral.lift, 0.);
        p.rx = 32767;
        p.ry = 0;
        p.rt = 255;
        let right = e.sample(Some((0, p)), true);
        assert_eq!(right.look, [1., 0.]);
        assert_eq!(right.lift, 1.);
        p.rt = 0;
        p.lt = 255;
        p.buttons = LB;
        e.sample(Some((0, p)), true);
        p.buttons |= Y;
        let reset = e.sample(Some((0, p)), true);
        assert!(reset.chord(LB | Y));
        assert_eq!(reset.lift, -1.);
        assert!(!e.sample(Some((0, p)), true).chord(LB | Y));
        let unfocused = e.sample(Some((0, p)), false);
        assert_eq!(unfocused.look, [0.; 2]);
        assert_eq!(unfocused.lift, 0.);
        assert!(!unfocused.chord(LB | Y));
        assert!(!e.sample(Some((0, p)), true).chord(LB | Y));
    }
    #[test]
    fn native_layout_and_optional_device_are_valid() {
        assert_eq!(std::mem::size_of::<RawPad>(), 12);
        assert_eq!(std::mem::size_of::<State>(), 16);
        let mut pad = Gamepad::new();
        assert!(pad.get_state.is_some());
        // Read-only smoke test: no connected controller required, no vibration.
        assert_eq!(pad.poll(false).held, 0);
    }
}
