#![allow(dead_code)]
// SPDX-License-Identifier: GPL-3.0-only
//! Chinatown Wars PSP native audio system (cAudioManager, cSoundEvents, cAudioBaseOAL).
//! Reads authentic PSP sound banks (resbnk.bin, carbnkN.bin from ROM.WAD) and
//! binary lookup tables (sound_tables.bin).
//! Native Windows low-latency PCM mixer using winmm.

use crate::iso::{u16le, u32le, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub const OUT_RATE: u32 = 44100;
pub const MAX_VOICES: usize = 32;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EventInfo {
    pub bank: i32,
    pub mode: i32,
    pub sfx: u16,
    pub keep: u8,
    pub priority: u8,
    pub pan: i8,
    pub radius_boost: u8,
    pub alt: u8,
    pub pad: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GearSound {
    pub horn: i16,
    pub horn2: i16,
    pub drive: i16,
    pub idle: i16,
    pub start: i16,
    pub pad: i16,
    pub bank: i32,
    pub volume: u8,
    pub gears: u8,
    pub step: [u8; 6],
    pub thr: [i32; 5],
    pub pitch: i16,
    pub vol: i16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PropSfx {
    pub sample: u16,
    pub volume: u8,
    pub pad: u8,
}

#[derive(Clone, Debug)]
pub struct SoundTables {
    pub events: Vec<EventInfo>,
    pub gears: Vec<GearSound>,
    pub collision_low: [i32; 3],
    pub collision_med: [i32; 3],
    pub collision_high: [i32; 3],
    pub rev_after_shift: [i32; 4],
    pub prop_sfx: Vec<PropSfx>,
}

impl SoundTables {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 3744 {
            return Err("sound_tables.bin truncated".into());
        }
        let magic = &data[0..7];
        if magic != b"CTWSND2" && magic != b"CTWSND4" {
            return Err("Invalid sound tables magic".into());
        }

        let mut pos = 8;
        let mut events = Vec::with_capacity(156);
        for _ in 0..156 {
            let bank = u32le(data, pos)? as i32;
            let mode = u32le(data, pos + 4)? as i32;
            let sfx = u16le(data, pos + 8)?;
            let keep = data[pos + 10];
            let priority = data[pos + 11];
            let pan = data[pos + 12] as i8;
            let radius_boost = data[pos + 13];
            let alt = data[pos + 14];
            let pad = data[pos + 15];
            events.push(EventInfo {
                bank,
                mode,
                sfx,
                keep,
                priority,
                pan,
                radius_boost,
                alt,
                pad,
            });
            pos += 16;
        }

        let mut gears = Vec::with_capacity(20);
        for _ in 0..20 {
            let horn = u16le(data, pos)? as i16;
            let horn2 = u16le(data, pos + 2)? as i16;
            let drive = u16le(data, pos + 4)? as i16;
            let idle = u16le(data, pos + 6)? as i16;
            let start = u16le(data, pos + 8)? as i16;
            let pad = u16le(data, pos + 10)? as i16;
            let bank = u32le(data, pos + 12)? as i32;
            let volume = data[pos + 16];
            let num_gears = data[pos + 17];
            let mut step = [0u8; 6];
            step.copy_from_slice(&data[pos + 18..pos + 24]);
            let mut thr = [0i32; 5];
            for k in 0..5 {
                thr[k] = u32le(data, pos + 24 + k * 4)? as i32;
            }
            let pitch = u16le(data, pos + 44)? as i16;
            let vol = u16le(data, pos + 46)? as i16;
            gears.push(GearSound {
                horn,
                horn2,
                drive,
                idle,
                start,
                pad,
                bank,
                volume,
                gears: num_gears,
                step,
                thr,
                pitch,
                vol,
            });
            pos += 48;
        }

        let mut collision_low = [0i32; 3];
        for k in 0..3 {
            collision_low[k] = u32le(data, pos + k * 4)? as i32;
        }
        pos += 12;

        let mut collision_med = [0i32; 3];
        for k in 0..3 {
            collision_med[k] = u32le(data, pos + k * 4)? as i32;
        }
        pos += 12;

        let mut collision_high = [0i32; 3];
        for k in 0..3 {
            collision_high[k] = u32le(data, pos + k * 4)? as i32;
        }
        pos += 12;

        let mut rev_after_shift = [0i32; 4];
        for k in 0..4 {
            rev_after_shift[k] = u32le(data, pos + k * 4)? as i32;
        }
        pos += 16;

        let mut prop_sfx = Vec::with_capacity(57);
        for _ in 0..57 {
            let sample = u16le(data, pos)?;
            let volume = data[pos + 2];
            let pad = data[pos + 3];
            prop_sfx.push(PropSfx { sample, volume, pad });
            pos += 4;
        }

        Ok(Self {
            events,
            gears,
            collision_low,
            collision_med,
            collision_high,
            rev_after_shift,
            prop_sfx,
        })
    }
}

#[derive(Clone, Debug)]
pub struct BankSample {
    pub offset: u32,
    pub size: u32,
    pub rate: u32,
    pub pcm: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct SoundBank {
    pub samples: Vec<BankSample>,
}

impl SoundBank {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err("Sound bank truncated".into());
        }
        let count = u32le(data, 0)? as usize;
        let mut samples = Vec::with_capacity(count);
        for i in 0..count {
            let base = 4 + i * 12;
            if base + 12 > data.len() {
                break;
            }
            let off = u32le(data, base)? as usize;
            let sz = u32le(data, base + 4)? as usize;
            let rate = u32le(data, base + 8)?;
            if off + 0x20 <= data.len() {
                let sample_len = u32le(data, off + 4)? as usize;
                let sample_rate = u32le(data, off + 0x14)?;
                let end = (off + 0x20 + sample_len).min(data.len());
                let pcm = data[off + 0x20..end].to_vec();
                samples.push(BankSample {
                    offset: off as u32,
                    size: sz as u32,
                    rate: if sample_rate > 0 { sample_rate } else { rate },
                    pcm,
                });
            }
        }
        Ok(Self { samples })
    }

    pub fn sample(&self, idx: usize) -> Option<(&[u8], u32)> {
        let s = self.samples.get(idx)?;
        if s.pcm.is_empty() {
            None
        } else {
            Some((&s.pcm, s.rate))
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Voice {
    pub active: bool,
    pub id: u32,
    pub data: Vec<u8>,
    pub pos: f64,
    pub step: f64,
    pub vol: f32,
    pub pan: f32,
    pub pitch: f32,
    pub rate: u32,
    pub loop_sound: bool,
}

pub struct VoiceMixer {
    pub voices: [Voice; MAX_VOICES],
    pub next_id: u32,
    pub master_volume: f32,
    pub paused: bool,
}

impl VoiceMixer {
    pub fn new() -> Self {
        Self {
            voices: std::array::from_fn(|_| Voice::default()),
            next_id: 1,
            master_volume: 0.85,
            paused: false,
        }
    }

    pub fn play_sfx(
        &mut self,
        pcm: &[u8],
        rate: u32,
        vol: f32,
        pan: f32,
        pitch: f32,
        loop_sound: bool,
    ) -> u32 {
        if pcm.is_empty() {
            return 0;
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);

        // Find inactive voice, or oldest non-looping voice
        let idx = self
            .voices
            .iter()
            .position(|v| !v.active)
            .or_else(|| self.voices.iter().position(|v| !v.loop_sound))
            .unwrap_or(0);

        let v = &mut self.voices[idx];
        v.active = true;
        v.id = id;
        v.data.clear();
        v.data.extend_from_slice(pcm);
        v.pos = 0.0;
        v.rate = if rate > 0 { rate } else { 22050 };
        v.vol = vol.clamp(0.0, 1.0);
        v.pan = pan.clamp(-1.0, 1.0);
        v.pitch = pitch.max(0.05);
        v.step = (v.rate as f64 * v.pitch as f64) / (OUT_RATE as f64);
        v.loop_sound = loop_sound;
        id
    }

    pub fn set_voice(&mut self, id: u32, vol: f32, pan: f32, pitch: f32) {
        if id == 0 {
            return;
        }
        for v in &mut self.voices {
            if v.active && v.id == id {
                v.vol = vol.clamp(0.0, 1.0);
                v.pan = pan.clamp(-1.0, 1.0);
                v.pitch = pitch.max(0.05);
                v.step = (v.rate as f64 * v.pitch as f64) / (OUT_RATE as f64);
                break;
            }
        }
    }

    pub fn stop_voice(&mut self, id: u32) {
        if id == 0 {
            return;
        }
        for v in &mut self.voices {
            if v.active && v.id == id {
                v.active = false;
                break;
            }
        }
    }

    pub fn is_playing(&self, id: u32) -> bool {
        if id == 0 {
            return false;
        }
        self.voices.iter().any(|v| v.active && v.id == id)
    }

    /// Renders 16-bit interleaved stereo PCM into `out`.
    pub fn mix(&mut self, out: &mut [i16]) {
        if self.paused {
            out.fill(0);
            return;
        }
        out.fill(0);
        let frames = out.len() / 2;
        let master = self.master_volume;

        for v in &mut self.voices {
            if !v.active || v.data.is_empty() {
                continue;
            }
            let angle = (v.pan + 1.0) * 0.25 * std::f32::consts::PI;
            let gl = angle.cos() * v.vol * master * 256.0;
            let gr = angle.sin() * v.vol * master * 256.0;

            for i in 0..frames {
                let idx0 = v.pos as usize;
                if idx0 >= v.data.len() {
                    if !v.loop_sound {
                        v.active = false;
                        break;
                    }
                    v.pos %= v.data.len() as f64;
                }
                let i0 = (v.pos as usize).min(v.data.len() - 1);
                let sample_val = (v.data[i0] as f32) - 128.0;

                let l = out[i * 2] as i32 + (sample_val * gl) as i32;
                let r = out[i * 2 + 1] as i32 + (sample_val * gr) as i32;
                out[i * 2] = l.clamp(-32768, 32767) as i16;
                out[i * 2 + 1] = r.clamp(-32768, 32767) as i16;

                v.pos += v.step;
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod winmm {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread::{self, JoinHandle};

    #[repr(C)]
    struct WaveFormatEx {
        w_format_tag: u16,
        n_channels: u16,
        n_samples_per_sec: u32,
        n_avg_bytes_per_sec: u32,
        n_block_align: u16,
        w_bits_per_sample: u16,
        cb_size: u16,
    }

    #[repr(C)]
    struct WaveHdr {
        lp_data: *mut u8,
        dw_buffer_length: u32,
        dw_bytes_recorded: u32,
        dw_user: usize,
        dw_flags: u32,
        dw_loops: u32,
        lp_next: usize,
        reserved: usize,
    }

    #[link(name = "winmm")]
    extern "system" {
        fn waveOutOpen(
            phwo: *mut usize,
            u_device_id: u32,
            pwfx: *const WaveFormatEx,
            dw_callback: usize,
            dw_instance: usize,
            fdw_open: u32,
        ) -> u32;
        fn waveOutPrepareHeader(hwo: usize, pwh: *mut WaveHdr, cbwh: u32) -> u32;
        fn waveOutWrite(hwo: usize, pwh: *mut WaveHdr, cbwh: u32) -> u32;
        fn waveOutUnprepareHeader(hwo: usize, pwh: *mut WaveHdr, cbwh: u32) -> u32;
        fn waveOutClose(hwo: usize) -> u32;
        fn waveOutReset(hwo: usize) -> u32;
    }

    pub struct WinmmAudioPlayer {
        running: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl WinmmAudioPlayer {
        pub fn start(mixer: Arc<Mutex<VoiceMixer>>) -> Option<Self> {
            let wfx = WaveFormatEx {
                w_format_tag: 1, // WAVE_FORMAT_PCM
                n_channels: 2,
                n_samples_per_sec: OUT_RATE,
                n_avg_bytes_per_sec: OUT_RATE * 4,
                n_block_align: 4,
                w_bits_per_sample: 16,
                cb_size: 0,
            };

            let mut hwo: usize = 0;
            let res = unsafe {
                waveOutOpen(
                    &mut hwo,
                    u32::MAX, // WAVE_MAPPER
                    &wfx,
                    0,
                    0,
                    0,
                )
            };
            if res != 0 || hwo == 0 {
                return None;
            }

            let running = Arc::new(AtomicBool::new(true));
            let r_clone = running.clone();

            let thread = thread::spawn(move || {
                const BUFFER_FRAMES: usize = 2048;
                let mut buf_a = vec![0i16; BUFFER_FRAMES * 2];
                let mut buf_b = vec![0i16; BUFFER_FRAMES * 2];

                let mut hdr_a = WaveHdr {
                    lp_data: buf_a.as_mut_ptr() as *mut u8,
                    dw_buffer_length: (BUFFER_FRAMES * 4) as u32,
                    dw_bytes_recorded: 0,
                    dw_user: 0,
                    dw_flags: 0,
                    dw_loops: 0,
                    lp_next: 0,
                    reserved: 0,
                };
                let mut hdr_b = WaveHdr {
                    lp_data: buf_b.as_mut_ptr() as *mut u8,
                    dw_buffer_length: (BUFFER_FRAMES * 4) as u32,
                    dw_bytes_recorded: 0,
                    dw_user: 0,
                    dw_flags: 0,
                    dw_loops: 0,
                    lp_next: 0,
                    reserved: 0,
                };

                unsafe {
                    waveOutPrepareHeader(hwo, &mut hdr_a, std::mem::size_of::<WaveHdr>() as u32);
                    waveOutPrepareHeader(hwo, &mut hdr_b, std::mem::size_of::<WaveHdr>() as u32);
                }

                let mut toggle = false;
                while r_clone.load(Ordering::Relaxed) {
                    let cur_hdr = if toggle { &mut hdr_b } else { &mut hdr_a };
                    let cur_buf = if toggle { &mut buf_b } else { &mut buf_a };

                    // Wait until buffer is done playing
                    while (cur_hdr.dw_flags & 0x01) == 0 && (cur_hdr.dw_flags & 0x02) != 0 {
                        thread::sleep(std::time::Duration::from_millis(5));
                        if !r_clone.load(Ordering::Relaxed) {
                            break;
                        }
                    }

                    if let Ok(mut m) = mixer.lock() {
                        m.mix(cur_buf);
                    }

                    unsafe {
                        waveOutWrite(hwo, cur_hdr, std::mem::size_of::<WaveHdr>() as u32);
                    }
                    toggle = !toggle;
                    thread::sleep(std::time::Duration::from_millis(15));
                }

                unsafe {
                    waveOutReset(hwo);
                    waveOutUnprepareHeader(hwo, &mut hdr_a, std::mem::size_of::<WaveHdr>() as u32);
                    waveOutUnprepareHeader(hwo, &mut hdr_b, std::mem::size_of::<WaveHdr>() as u32);
                    waveOutClose(hwo);
                }
            });

            Some(Self {
                running,
                thread: Some(thread),
            })
        }
    }

    impl Drop for WinmmAudioPlayer {
        fn drop(&mut self) {
            self.running.store(false, Ordering::Relaxed);
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
        }
    }
}

pub struct SoundSystem {
    pub tables: SoundTables,
    pub res_bank: SoundBank,
    pub car_banks: HashMap<String, SoundBank>,
    pub mixer: Arc<Mutex<VoiceMixer>>,
    #[cfg(target_os = "windows")]
    _player: Option<winmm::WinmmAudioPlayer>,
    // Vehicle engine audio state
    engine_voice_idle: u32,
    engine_voice_rev: u32,
    skid_voice: u32,
    hydrant_stream_voice: u32,
}

impl SoundSystem {
    pub fn new(tables: SoundTables, res_bank: SoundBank) -> Self {
        let mixer = Arc::new(Mutex::new(VoiceMixer::new()));
        #[cfg(target_os = "windows")]
        let player = winmm::WinmmAudioPlayer::start(mixer.clone());

        Self {
            tables,
            res_bank,
            car_banks: HashMap::new(),
            mixer,
            #[cfg(target_os = "windows")]
            _player: player,
            engine_voice_idle: 0,
            engine_voice_rev: 0,
            skid_voice: 0,
            hydrant_stream_voice: 0,
        }
    }

    /// Loads engine sound bank for a given vehicle sound profile (e.g. carbnk1.bin).
    pub fn add_car_bank(&mut self, name: &str, bank: SoundBank) {
        self.car_banks.insert(name.to_lowercase(), bank);
    }

    /// Plays a 3D positional one-shot sound effect.
    pub fn play_spatial_sfx(
        &self,
        sample: usize,
        pos: [f32; 3],
        radius: f32,
        vol_scale: f32,
        listener_p: [f32; 3],
        listener_fwd: [f32; 2],
    ) {
        let Some((pcm, rate)) = self.res_bank.sample(sample) else {
            return;
        };
        let dx = pos[0] - listener_p[0];
        let dy = pos[1] - listener_p[1];
        let d2 = dx * dx + dy * dy;
        let r2 = radius * radius;
        if d2 > r2 {
            return;
        }

        // Distance attenuation
        let dist = d2.sqrt();
        let atten = (1.0 - (dist / radius)).clamp(0.0, 1.0);
        let vol = atten * vol_scale;

        // Stereo panning from listener forward vector
        let cside = [-listener_fwd[1], listener_fwd[0]];
        let pan = ((dx * cside[0] + dy * cside[1]) / radius.max(1.0)).clamp(-1.0, 1.0);

        if let Ok(mut m) = self.mixer.lock() {
            m.play_sfx(pcm, rate, vol, pan, 1.0, false);
        }
    }

    /// Plays authentic street furniture smash sound (fire hydrant, trash bin, fence).
    pub fn play_prop_smash(
        &self,
        pos: [f32; 3],
        effect: usize,
        listener_p: [f32; 3],
        listener_fwd: [f32; 2],
    ) {
        if let Some(sfx) = self.tables.prop_sfx.get(effect) {
            let vol = (sfx.volume as f32) / 127.0;
            self.play_spatial_sfx(
                sfx.sample as usize,
                pos,
                24.0,
                vol,
                listener_p,
                listener_fwd,
            );
        }
    }

    /// Plays vehicle impact crunch sound scaled by impact strength.
    pub fn play_car_collision(
        &self,
        pos: [f32; 3],
        strength: f32,
        listener_p: [f32; 3],
        listener_fwd: [f32; 2],
    ) {
        let events = if strength > 14.0 {
            &self.tables.collision_high
        } else if strength > 6.0 {
            &self.tables.collision_med
        } else {
            &self.tables.collision_low
        };
        let ev_id = events[0] as usize;
        if let Some(ev) = self.tables.events.get(ev_id) {
            let vol = (strength / 20.0).clamp(0.2, 1.0);
            self.play_spatial_sfx(
                ev.sfx as usize,
                pos,
                30.0,
                vol,
                listener_p,
                listener_fwd,
            );
        }
    }

    /// Updates vehicle engine sound and tire screech loops.
    pub fn update_vehicle_sound(
        &mut self,
        car: &crate::actors::Car,
        is_player: bool,
    ) {
        if !is_player {
            return;
        }

        let speed = car.speed.abs();
        let skidding = car.rear_skidding || car.front_skidding || (car.wheel_spin && speed > 2.0);

        // Tire screech (event 0x13 in tables -> sample 19)
        if skidding {
            if let Ok(mut m) = self.mixer.lock() {
                if !m.is_playing(self.skid_voice) {
                    if let Some((pcm, rate)) = self.res_bank.sample(19) {
                        self.skid_voice = m.play_sfx(pcm, rate, 0.45, 0.0, 1.0, true);
                    }
                }
            }
        } else if self.skid_voice != 0 {
            if let Ok(mut m) = self.mixer.lock() {
                m.stop_voice(self.skid_voice);
            }
            self.skid_voice = 0;
        }

        // Engine sound loops
        let gear_idx = car.spec().sound_type().clamp(0, 19);
        if let Some(gs) = self.tables.gears.get(gear_idx) {
            let idle_sample = gs.idle.max(0) as usize;
            let drive_sample = gs.drive.max(0) as usize;

            let rpm_factor = (speed / 18.0).clamp(0.0, 1.0);
            let idle_vol = ((1.0 - rpm_factor * 1.5).max(0.0)) * 0.55;
            let drive_vol = (rpm_factor * 1.2).clamp(0.0, 0.85);
            let pitch = 0.85 + rpm_factor * 0.65;

            if let Ok(mut m) = self.mixer.lock() {
                if !m.is_playing(self.engine_voice_idle) {
                    if let Some((pcm, rate)) = self.res_bank.sample(idle_sample) {
                        self.engine_voice_idle = m.play_sfx(pcm, rate, idle_vol, 0.0, 1.0, true);
                    }
                } else {
                    m.set_voice(self.engine_voice_idle, idle_vol, 0.0, 1.0);
                }

                if !m.is_playing(self.engine_voice_rev) && drive_vol > 0.05 {
                    if let Some((pcm, rate)) = self.res_bank.sample(drive_sample) {
                        self.engine_voice_rev = m.play_sfx(pcm, rate, drive_vol, 0.0, pitch, true);
                    }
                } else if self.engine_voice_rev != 0 {
                    m.set_voice(self.engine_voice_rev, drive_vol, 0.0, pitch);
                }
            }
        }
    }

    /// Updates water stream loop sound for active fire hydrant fountains.
    pub fn update_hydrant_fountain(&mut self, active_fountain: bool) {
        if active_fountain {
            if let Ok(mut m) = self.mixer.lock() {
                if !m.is_playing(self.hydrant_stream_voice) {
                    // Event 0x6C -> sample 108 / water gush
                    if let Some((pcm, rate)) = self.res_bank.sample(108) {
                        self.hydrant_stream_voice = m.play_sfx(pcm, rate, 0.55, 0.0, 1.0, true);
                    }
                }
            }
        } else if self.hydrant_stream_voice != 0 {
            if let Ok(mut m) = self.mixer.lock() {
                m.stop_voice(self.hydrant_stream_voice);
            }
            self.hydrant_stream_voice = 0;
        }
    }

    /// Plays authentic explosion blast sound (sample 466 / 0x1D2 in resident sound bank).
    pub fn play_explosion(
        &self,
        pos: [f32; 3],
        listener_p: [f32; 3],
        listener_fwd: [f32; 2],
    ) {
        let dx = pos[0] - listener_p[0];
        let dy = pos[1] - listener_p[1];
        let dz = pos[2] - listener_p[2];
        let d2 = dx * dx + dy * dy + dz * dz;

        if d2 >= 1900.0 {
            return;
        }

        let mut volume = if d2 <= 380.0 {
            1.0
        } else {
            ((1900.0 - d2) / 1520.0).clamp(0.0, 1.0)
        };
        volume *= volume;

        let cside = [-listener_fwd[1], listener_fwd[0]];
        let right = dx * cside[0] + dy * cside[1];
        let forward = dx * listener_fwd[0] + dy * listener_fwd[1];
        let pan = (right / forward.max(36.0)).clamp(-1.0, 1.0);

        if let Some((pcm, rate)) = self.res_bank.sample(466) {
            if let Ok(mut m) = self.mixer.lock() {
                m.play_sfx(pcm, rate, volume, pan, 1.0, false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sound_layout_and_optional_reference() {
        // Synthetic data, generated in memory; no extracted samples or tables.
        let mut data = vec![0; 3868];
        data[..8].copy_from_slice(b"CTWSND2\0");
        data[8..12].copy_from_slice(&(-7i32).to_le_bytes());
        data[20] = (-30i8) as u8;
        data[3516..3518].copy_from_slice(&321u16.to_le_bytes());
        data[3518] = 77;
        let parsed = SoundTables::parse(&data).unwrap();
        assert_eq!(parsed.events.len(), 156);
        assert_eq!(parsed.gears.len(), 20);
        assert_eq!(parsed.events[0].bank, -7);
        assert_eq!(parsed.events[0].pan, -30);
        assert_eq!(parsed.prop_sfx[0].sample, 321);
        assert_eq!(parsed.prop_sfx[0].volume, 77);
        assert!(SoundTables::parse(&data[..100]).is_err());
        data[0] = 0;
        assert!(SoundTables::parse(&data).is_err());
        if let Some(bytes) = crate::test_support::reference_table("sound_tables.bin") {
            let tables = SoundTables::parse(&bytes).expect("Failed to parse sound tables");
            assert_eq!(tables.events.len(), 156);
            assert_eq!(tables.gears.len(), 20);
            assert_eq!(tables.prop_sfx.len(), 57);

            // Hydrant smash effect (9) -> sample 692, vol 120
            assert_eq!(tables.prop_sfx[9].sample, 692);
            assert_eq!(tables.prop_sfx[9].volume, 120);

            // Trash bin smash effect (35) -> sample 683, vol 120
            assert_eq!(tables.prop_sfx[35].sample, 683);
            assert_eq!(tables.prop_sfx[35].volume, 120);

            // Collision events
            assert_eq!(tables.collision_low, [135, 136, 135]);
            assert_eq!(tables.collision_med, [137, 138, 137]);
            assert_eq!(tables.collision_high, [139, 140, 141]);
        }
    }

    #[test]
    fn mixer_resampling_panning_and_clipping() {
        let mut mixer = VoiceMixer::new();
        let pcm_dummy = vec![128u8, 200, 50, 128, 250, 10];
        let id = mixer.play_sfx(&pcm_dummy, 22050, 1.0, 0.0, 1.0, true);
        assert!(mixer.is_playing(id));

        let mut out = [0i16; 512];
        mixer.mix(&mut out);

        assert!(out.iter().any(|&sample| sample != 0), "Mixer generated non-zero audio");
        mixer.stop_voice(id);
        assert!(!mixer.is_playing(id));
    }
}
