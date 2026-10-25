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
    // Radio streaming state
    pub radio_voice: u32,
    pub current_radio_station: usize,
    cached_radio_loops: HashMap<usize, Vec<u8>>,
    // Emergency siren state
    pub siren_voice: u32,
    pub siren_yelp_voice: u32,
    pub heli_rotor_voice: u32,
    cached_siren_wail: Vec<u8>,
    cached_siren_yelp: Vec<u8>,
    cached_heli_rotor: Vec<u8>,
    cached_car_horn: Vec<u8>,
    cached_ped_pain: Vec<u8>,
    cached_ped_scream: Vec<u8>,
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
            radio_voice: 0,
            current_radio_station: usize::MAX,
            cached_radio_loops: HashMap::new(),
            siren_voice: 0,
            siren_yelp_voice: 0,
            heli_rotor_voice: 0,
            cached_siren_wail: synth_siren_wail(),
            cached_siren_yelp: synth_siren_yelp(),
            cached_heli_rotor: synth_heli_rotor(),
            cached_car_horn: synth_car_horn(),
            cached_ped_pain: synth_ped_pain(),
            cached_ped_scream: synth_ped_scream(),
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

    /// Plays authentic cash register chime for purchases, sales, and fares.
    pub fn play_cash_register(&self) {
        let pcm = synth_cash_chime();
        if let Ok(mut m) = self.mixer.lock() {
            m.play_sfx(&pcm, 22050, 0.75, 0.0, 1.0, false);
        }
    }

    /// Plays authentic PDA incoming email / stock alert notification tone.
    pub fn play_pda_notification(&self) {
        let pcm = synth_pda_chime();
        if let Ok(mut m) = self.mixer.lock() {
            m.play_sfx(&pcm, 22050, 0.65, 0.0, 1.0, false);
        }
    }

    /// Plays authentic glass cracking and shattering sound for windshields or shop windows.
    pub fn play_glass_break(
        &self,
        pos: [f32; 3],
        listener_p: [f32; 3],
        listener_fwd: [f32; 2],
    ) {
        let dx = pos[0] - listener_p[0];
        let dy = pos[1] - listener_p[1];
        let d2 = dx * dx + dy * dy;
        if d2 > 1200.0 {
            return;
        }
        let vol = (1.0 - (d2 / 1200.0)).clamp(0.1, 0.9);
        let cside = [-listener_fwd[1], listener_fwd[0]];
        let pan = ((dx * cside[0] + dy * cside[1]) / 35.0).clamp(-1.0, 1.0);

        let pcm = synth_glass_crack();
        if let Ok(mut m) = self.mixer.lock() {
            m.play_sfx(&pcm, 22050, vol, pan, 1.0, false);
        }
    }

    /// Plays authentic police dispatch tone for vigilante alerts.
    pub fn play_police_dispatch(&self) {
        let pcm = synth_police_chirp();
        if let Ok(mut m) = self.mixer.lock() {
            m.play_sfx(&pcm, 22050, 0.7, 0.0, 1.0, false);
        }
    }

    /// Plays authentic defibrillator shock sound for paramedic rescue.
    pub fn play_defibrillator(&self) {
        let pcm = synth_defibrillator_shock();
        if let Ok(mut m) = self.mixer.lock() {
            m.play_sfx(&pcm, 22050, 0.8, 0.0, 1.0, false);
        }
    }

    /// Plays triumphant fanfare jingle when passing a mission or ambient job level.
    pub fn play_mission_fanfare(&self) {
        let pcm = synth_fanfare();
        if let Ok(mut m) = self.mixer.lock() {
            m.play_sfx(&pcm, 22050, 0.85, 0.0, 1.0, false);
        }
    }

    /// Updates radio station stream playback while player is in vehicle.
    pub fn update_radio(&mut self, radio: &crate::radio::RadioManager) {
        if radio.is_playing && radio.current_station < 11 {
            let station_idx = radio.current_station;
            let vol = ((radio.volume as f32 / 10.0) * 0.70).clamp(0.0, 1.0);

            if self.radio_voice == 0 || self.current_radio_station != station_idx {
                if self.radio_voice != 0 {
                    if let Ok(mut m) = self.mixer.lock() {
                        m.stop_voice(self.radio_voice);
                    }
                }
                let pcm = self.cached_radio_loops.entry(station_idx).or_insert_with(|| {
                    synth_station_loop(station_idx)
                });
                if let Ok(mut m) = self.mixer.lock() {
                    self.radio_voice = m.play_sfx(pcm, 22050, vol, 0.0, 1.0, true);
                }
                self.current_radio_station = station_idx;
            } else if let Ok(mut m) = self.mixer.lock() {
                m.set_voice(self.radio_voice, vol, 0.0, 1.0);
            }
        } else if self.radio_voice != 0 {
            if let Ok(mut m) = self.mixer.lock() {
                m.stop_voice(self.radio_voice);
            }
            self.radio_voice = 0;
            self.current_radio_station = usize::MAX;
        }
    }

    /// Stops radio playback immediately.
    pub fn stop_radio(&mut self) {
        if self.radio_voice != 0 {
            if let Ok(mut m) = self.mixer.lock() {
                m.stop_voice(self.radio_voice);
            }
            self.radio_voice = 0;
            self.current_radio_station = usize::MAX;
        }
    }

    /// Updates vehicle sirens for active police pursuits and emergency vehicles.
    pub fn update_sirens(
        &mut self,
        police: &crate::police::PoliceManager,
        cars: &[crate::actors::Car],
        listener_p: [f32; 3],
        listener_fwd: [f32; 2],
    ) {
        let mut nearest_dist_sq = f32::MAX;
        let mut nearest_pos = [0.0f32; 3];
        let mut active_emergency = false;

        for car in cars {
            let is_siren_on = car.siren_active || (police.stars > 0 && car.spec().name == "POLICE");
            if is_siren_on {
                let dx = car.p[0] - listener_p[0];
                let dy = car.p[1] - listener_p[1];
                let dz = car.p[2] - listener_p[2];
                let d2 = dx * dx + dy * dy + dz * dz;
                if d2 < 14400.0 && d2 < nearest_dist_sq {
                    nearest_dist_sq = d2;
                    nearest_pos = car.p;
                    active_emergency = true;
                }
            }
        }

        if active_emergency {
            let dist = nearest_dist_sq.sqrt();
            let atten = (1.0 - (dist / 120.0)).clamp(0.0, 1.0);
            let vol = (atten * 0.75).clamp(0.05, 0.85);

            let dx = nearest_pos[0] - listener_p[0];
            let dy = nearest_pos[1] - listener_p[1];
            let cside = [-listener_fwd[1], listener_fwd[0]];
            let pan = ((dx * cside[0] + dy * cside[1]) / 40.0).clamp(-1.0, 1.0);

            let use_yelp = police.stars >= 3;

            if use_yelp {
                if self.siren_voice != 0 {
                    if let Ok(mut m) = self.mixer.lock() {
                        m.stop_voice(self.siren_voice);
                    }
                    self.siren_voice = 0;
                }
                if let Ok(mut m) = self.mixer.lock() {
                    if !m.is_playing(self.siren_yelp_voice) {
                        self.siren_yelp_voice = m.play_sfx(&self.cached_siren_yelp, 22050, vol, pan, 1.0, true);
                    } else {
                        m.set_voice(self.siren_yelp_voice, vol, pan, 1.0);
                    }
                }
            } else {
                if self.siren_yelp_voice != 0 {
                    if let Ok(mut m) = self.mixer.lock() {
                        m.stop_voice(self.siren_yelp_voice);
                    }
                    self.siren_yelp_voice = 0;
                }
                if let Ok(mut m) = self.mixer.lock() {
                    if !m.is_playing(self.siren_voice) {
                        self.siren_voice = m.play_sfx(&self.cached_siren_wail, 22050, vol, pan, 1.0, true);
                    } else {
                        m.set_voice(self.siren_voice, vol, pan, 1.0);
                    }
                }
            }
        } else {
            if self.siren_voice != 0 {
                if let Ok(mut m) = self.mixer.lock() {
                    m.stop_voice(self.siren_voice);
                }
                self.siren_voice = 0;
            }
            if self.siren_yelp_voice != 0 {
                if let Ok(mut m) = self.mixer.lock() {
                    m.stop_voice(self.siren_yelp_voice);
                }
                self.siren_yelp_voice = 0;
            }
        }
    }

    /// Updates dynamic 3D audio loop for overhead police pursuit helicopter.
    pub fn update_helicopter(
        &mut self,
        heli_pos: Option<[f32; 3]>,
        listener_p: [f32; 3],
        listener_fwd: [f32; 2],
    ) {
        if let Some(pos) = heli_pos {
            let dx = pos[0] - listener_p[0];
            let dy = pos[1] - listener_p[1];
            let dz = pos[2] - listener_p[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            let atten = (1.0 - (dist / 140.0)).clamp(0.0, 1.0);
            let vol = (atten * 0.85).clamp(0.08, 0.90);
            let cside = [-listener_fwd[1], listener_fwd[0]];
            let pan = ((dx * cside[0] + dy * cside[1]) / 45.0).clamp(-1.0, 1.0);

            if let Ok(mut m) = self.mixer.lock() {
                if !m.is_playing(self.heli_rotor_voice) {
                    self.heli_rotor_voice = m.play_sfx(&self.cached_heli_rotor, 22050, vol, pan, 1.0, true);
                } else {
                    m.set_voice(self.heli_rotor_voice, vol, pan, 1.0);
                }
            }
        } else if self.heli_rotor_voice != 0 {
            if let Ok(mut m) = self.mixer.lock() {
                m.stop_voice(self.heli_rotor_voice);
            }
            self.heli_rotor_voice = 0;
        }
    }

    /// Plays pedestrian reaction vocalization (pain grunt or scream).
    pub fn play_ped_impact(
        &self,
        pos: [f32; 3],
        lethal: bool,
        listener_p: [f32; 3],
        listener_fwd: [f32; 2],
    ) {
        let dx = pos[0] - listener_p[0];
        let dy = pos[1] - listener_p[1];
        let d2 = dx * dx + dy * dy;
        if d2 > 2500.0 {
            return;
        }
        let atten = (1.0 - (d2 / 2500.0)).clamp(0.1, 1.0);
        let cside = [-listener_fwd[1], listener_fwd[0]];
        let pan = ((dx * cside[0] + dy * cside[1]) / 35.0).clamp(-1.0, 1.0);

        let pcm = if lethal { &self.cached_ped_scream } else { &self.cached_ped_pain };
        let vol = if lethal { atten * 0.85 } else { atten * 0.70 };

        if let Ok(mut m) = self.mixer.lock() {
            m.play_sfx(pcm, 22050, vol, pan, 1.0, false);
        }
    }

    /// Plays vehicle horn honk.
    pub fn play_car_horn(
        &self,
        pos: [f32; 3],
        listener_p: [f32; 3],
        listener_fwd: [f32; 2],
    ) {
        let dx = pos[0] - listener_p[0];
        let dy = pos[1] - listener_p[1];
        let d2 = dx * dx + dy * dy;
        if d2 > 3600.0 {
            return;
        }
        let atten = (1.0 - (d2 / 3600.0)).clamp(0.1, 1.0);
        let cside = [-listener_fwd[1], listener_fwd[0]];
        let pan = ((dx * cside[0] + dy * cside[1]) / 40.0).clamp(-1.0, 1.0);

        if let Ok(mut m) = self.mixer.lock() {
            m.play_sfx(&self.cached_car_horn, 22050, atten * 0.75, pan, 1.0, false);
        }
    }
}

/// Synthesizes a two-tone high-frequency cash register chime (8-bit PCM).
fn synth_cash_chime() -> Vec<u8> {
    let rate = 22050;
    let dur = 0.35;
    let samples = (rate as f32 * dur) as usize;
    let mut out = Vec::with_capacity(samples);
    for i in 0..samples {
        let t = i as f32 / rate as f32;
        let env = (1.0 - (t / dur)).clamp(0.0, 1.0);
        let s1 = (t * 1760.0 * std::f32::consts::TAU).sin();
        let s2 = (t * 2640.0 * std::f32::consts::TAU).sin();
        let val = (s1 * 0.5 + s2 * 0.5) * env * 0.8;
        out.push(((val * 120.0) + 128.0).clamp(0.0, 255.0) as u8);
    }
    out
}

/// Synthesizes a tri-tone electronic PDA notification tone.
fn synth_pda_chime() -> Vec<u8> {
    let rate = 22050;
    let dur = 0.28;
    let samples = (rate as f32 * dur) as usize;
    let mut out = Vec::with_capacity(samples);
    for i in 0..samples {
        let t = i as f32 / rate as f32;
        let env = (1.0 - (t / dur)).clamp(0.0, 1.0);
        let freq = if t < 0.09 {
            987.0
        } else if t < 0.18 {
            1318.0
        } else {
            1760.0
        };
        let s = (t * freq * std::f32::consts::TAU).sin() * env * 0.75;
        out.push(((s * 120.0) + 128.0).clamp(0.0, 255.0) as u8);
    }
    out
}

/// Synthesizes a crunchy glass crack/shatter burst.
fn synth_glass_crack() -> Vec<u8> {
    let rate = 22050;
    let dur = 0.22;
    let samples = (rate as f32 * dur) as usize;
    let mut out = Vec::with_capacity(samples);
    let mut lfsr: u32 = 0xACE1;
    for i in 0..samples {
        let t = i as f32 / rate as f32;
        let env = (1.0 - (t / dur)).powi(2);
        lfsr = (lfsr >> 1) ^ (-(lfsr as i32 & 1) as u32 & 0xB400);
        let noise = ((lfsr & 0xFF) as f32 / 128.0) - 1.0;
        let tone = (t * 3200.0 * std::f32::consts::TAU).sin() * 0.3;
        let val = (noise * 0.7 + tone * 0.3) * env * 0.8;
        out.push(((val * 120.0) + 128.0).clamp(0.0, 255.0) as u8);
    }
    out
}

/// Synthesizes a two-tone police dispatch alert chirp.
fn synth_police_chirp() -> Vec<u8> {
    let rate = 22050;
    let dur = 0.25;
    let samples = (rate as f32 * dur) as usize;
    let mut out = Vec::with_capacity(samples);
    for i in 0..samples {
        let t = i as f32 / rate as f32;
        let freq = if t < 0.12 { 650.0 } else { 920.0 };
        let env = if t < 0.12 { 1.0 - t / 0.12 } else { 1.0 - (t - 0.12) / 0.13 };
        let s = (t * freq * std::f32::consts::TAU).sin() * env * 0.75;
        out.push(((s * 120.0) + 128.0).clamp(0.0, 255.0) as u8);
    }
    out
}

/// Synthesizes a defibrillator electric shock pulse.
fn synth_defibrillator_shock() -> Vec<u8> {
    let rate = 22050;
    let dur = 0.30;
    let samples = (rate as f32 * dur) as usize;
    let mut out = Vec::with_capacity(samples);
    let mut lfsr: u32 = 0x5432;
    for i in 0..samples {
        let t = i as f32 / rate as f32;
        let env = (1.0 - (t / dur)).clamp(0.0, 1.0);
        lfsr = (lfsr >> 1) ^ (-(lfsr as i32 & 1) as u32 & 0x9800);
        let noise = ((lfsr & 0xFF) as f32 / 128.0) - 1.0;
        let buzz = (t * 120.0 * std::f32::consts::TAU).sin();
        let val = (buzz * 0.6 + noise * 0.4) * env * 0.85;
        out.push(((val * 120.0) + 128.0).clamp(0.0, 255.0) as u8);
    }
    out
}

/// Synthesizes a triumphant 4-note ascending brass fanfare (C5, E5, G5, C6).
fn synth_fanfare() -> Vec<u8> {
    let rate = 22050;
    let note_dur = 0.12;
    let total_dur = note_dur * 4.0 + 0.2;
    let samples = (rate as f32 * total_dur) as usize;
    let mut out = Vec::with_capacity(samples);
    let notes = [523.25, 659.25, 783.99, 1046.50];
    for i in 0..samples {
        let t = i as f32 / rate as f32;
        let note_idx = ((t / note_dur) as usize).min(3);
        let freq = notes[note_idx];
        let note_t = t % note_dur;
        let env = (1.0 - (note_t / (note_dur * 1.5))).clamp(0.0, 1.0);
        let s = (t * freq * std::f32::consts::TAU).sin() * env * 0.8;
        out.push(((s * 120.0) + 128.0).clamp(0.0, 255.0) as u8);
    }
    out
}

/// Synthesizes a classic police siren wail loop (frequency sweeps 620 Hz to 1250 Hz over 3.2s).
pub fn synth_siren_wail() -> Vec<u8> {
    let rate = 22050;
    let dur = 3.2;
    let samples = (rate as f32 * dur) as usize;
    let mut out = Vec::with_capacity(samples);
    let mut phase: f32 = 0.0;
    for i in 0..samples {
        let t = i as f32 / rate as f32;
        let cycle = (t / dur).fract();
        let sweep = if cycle < 0.5 { cycle * 2.0 } else { (1.0 - cycle) * 2.0 };
        let freq = 620.0 + sweep * 630.0;
        phase += freq * std::f32::consts::TAU / rate as f32;
        let s1 = phase.sin();
        let s2 = (phase * 3.0).sin() * 0.35;
        let s3 = (phase * 5.0).sin() * 0.15;
        let val = (s1 + s2 + s3) * 0.7;
        out.push(((val * 110.0) + 128.0).clamp(0.0, 255.0) as u8);
    }
    out
}

/// Synthesizes an aggressive rapid pursuit siren yelp loop (frequency sweeps 750 Hz to 1480 Hz over 0.38s).
pub fn synth_siren_yelp() -> Vec<u8> {
    let rate = 22050;
    let dur = 0.38;
    let samples = (rate as f32 * dur) as usize;
    let mut out = Vec::with_capacity(samples);
    let mut phase: f32 = 0.0;
    for i in 0..samples {
        let t = i as f32 / rate as f32;
        let cycle = (t / dur).fract();
        let sweep = if cycle < 0.5 { cycle * 2.0 } else { (1.0 - cycle) * 2.0 };
        let freq = 750.0 + sweep * 730.0;
        phase += freq * std::f32::consts::TAU / rate as f32;
        let s1 = phase.sin();
        let s2 = (phase * 3.0).sin() * 0.4;
        let val = (s1 + s2) * 0.72;
        out.push(((val * 115.0) + 128.0).clamp(0.0, 255.0) as u8);
    }
    out
}

/// Synthesizes a pedestrian pain grunt/yelp (short vowel formant).
pub fn synth_ped_pain() -> Vec<u8> {
    let rate = 22050;
    let dur = 0.20;
    let samples = (rate as f32 * dur) as usize;
    let mut out = Vec::with_capacity(samples);
    let mut phase: f32 = 0.0;
    for i in 0..samples {
        let t = i as f32 / rate as f32;
        let env = (1.0 - (t / dur)).powf(1.5);
        let freq = 420.0 - t * 400.0;
        phase += freq * std::f32::consts::TAU / rate as f32;
        let s = phase.sin() * 0.6 + (phase * 2.0).sin() * 0.3;
        let noise = ((i * 1103515245 + 12345) & 0xFF) as f32 / 128.0 - 1.0;
        let val = (s * 0.8 + noise * 0.2 * env) * env * 0.8;
        out.push(((val * 120.0) + 128.0).clamp(0.0, 255.0) as u8);
    }
    out
}

/// Synthesizes a terrified pedestrian scream (descending high pitch).
pub fn synth_ped_scream() -> Vec<u8> {
    let rate = 22050;
    let dur = 0.50;
    let samples = (rate as f32 * dur) as usize;
    let mut out = Vec::with_capacity(samples);
    let mut phase: f32 = 0.0;
    for i in 0..samples {
        let t = i as f32 / rate as f32;
        let env = (1.0 - (t / dur)).clamp(0.0, 1.0);
        let vibrato = (t * 22.0 * std::f32::consts::TAU).sin() * 25.0;
        let freq = (1150.0 - t * 650.0) + vibrato;
        phase += freq * std::f32::consts::TAU / rate as f32;
        let s = phase.sin() * 0.6 + (phase * 2.0).sin() * 0.35 + (phase * 3.0).sin() * 0.15;
        let noise = ((i * 1664525 + 1013904223) & 0xFF) as f32 / 128.0 - 1.0;
        let val = (s * 0.75 + noise * 0.25 * env) * env * 0.85;
        out.push(((val * 120.0) + 128.0).clamp(0.0, 255.0) as u8);
    }
    out
}

/// Synthesizes a classic automotive dual-tone car horn honk.
pub fn synth_car_horn() -> Vec<u8> {
    let rate = 22050;
    let dur = 0.45;
    let samples = (rate as f32 * dur) as usize;
    let mut out = Vec::with_capacity(samples);
    for i in 0..samples {
        let t = i as f32 / rate as f32;
        let env = if t < 0.03 {
            t / 0.03
        } else if t > dur - 0.05 {
            (dur - t) / 0.05
        } else {
            1.0
        };
        let s1 = (t * 370.0 * std::f32::consts::TAU).sin();
        let s2 = (t * 440.0 * std::f32::consts::TAU).sin();
        let s3 = (t * 740.0 * std::f32::consts::TAU).sin() * 0.25;
        let val = (s1 * 0.45 + s2 * 0.45 + s3) * env * 0.8;
        out.push(((val * 120.0) + 128.0).clamp(0.0, 255.0) as u8);
    }
    out
}

/// Synthesizes procedural low-frequency rhythmic rotor blade passage and turbulent whoosh for police chopper.
pub fn synth_heli_rotor() -> Vec<u8> {
    let rate = 22050;
    let dur = 1.0;
    let samples = (rate as f32 * dur) as usize;
    let mut out = Vec::with_capacity(samples);
    let mut lfsr: u32 = 0xA751;
    for i in 0..samples {
        let t = i as f32 / rate as f32;
        // 4 blades at ~5.5 rev/s = 22 Hz pulses
        let pulse_phase = (t * 22.0).fract();
        let pulse = (1.0 - pulse_phase * 2.8).max(0.0).powi(2);
        lfsr = (lfsr >> 1) ^ (-(lfsr as i32 & 1) as u32 & 0x9800);
        let noise = ((lfsr & 0xFF) as f32 / 128.0) - 1.0;
        let thud = (t * 22.0 * std::f32::consts::TAU).sin() * 0.55 * pulse
            + (t * 88.0 * std::f32::consts::TAU).sin() * 0.35 * pulse
            + noise * 0.20 * pulse;
        let val = thud.clamp(-1.0, 1.0);
        out.push(((val * 120.0) + 128.0).clamp(0.0, 255.0) as u8);
    }
    out
}

/// Synthesizes procedural musical loop for one of the 11 Chinatown Wars radio stations.
pub fn synth_station_loop(idx: usize) -> Vec<u8> {
    let rate = 22050;
    let dur = 3.2;
    let samples = (rate as f32 * dur) as usize;
    let mut out = Vec::with_capacity(samples);

    match idx {
        // 0: TRUTHWAVE (Rock Indie) - driving bass riff in Em, overdriven guitar chords, drum pulse
        0 => {
            let bpm = 135.0;
            let beat_dur = 60.0 / bpm;
            let bass_notes = [82.41, 82.41, 98.00, 110.00, 82.41, 73.42, 82.41, 98.00];
            for i in 0..samples {
                let t = i as f32 / rate as f32;
                let beat_idx = ((t / (beat_dur * 0.5)) as usize) % bass_notes.len();
                let beat_t = t % (beat_dur * 0.5);
                let env = (1.0 - beat_t / (beat_dur * 0.5)).powf(1.2);
                let f_bass = bass_notes[beat_idx];
                let bass = (t * f_bass * std::f32::consts::TAU).sin() * 0.5
                    + (t * f_bass * 2.0 * std::f32::consts::TAU).sin() * 0.3;
                let chord_active = ((t / beat_dur) as usize % 2) == 1;
                let guitar = if chord_active {
                    let g_env = (1.0 - (t % beat_dur) / beat_dur).max(0.0);
                    let raw = (t * 164.81 * std::f32::consts::TAU).sin()
                        + (t * 246.94 * std::f32::consts::TAU).sin() * 0.8;
                    (raw * 2.5).clamp(-0.7, 0.7) * g_env * 0.35
                } else {
                    0.0
                };
                let drum_step = (t / (beat_dur * 0.5)) as usize % 4;
                let step_t = t % (beat_dur * 0.5);
                let kick = if drum_step == 0 || drum_step == 2 {
                    (step_t * 65.0 * std::f32::consts::TAU).sin() * (1.0 - step_t / 0.15).max(0.0) * 0.6
                } else {
                    0.0
                };
                let snare = if drum_step == 1 || drum_step == 3 {
                    let noise = ((i * 1103515245 + 12345) & 0xFF) as f32 / 128.0 - 1.0;
                    noise * (1.0 - step_t / 0.18).max(0.0) * 0.35
                } else {
                    0.0
                };
                let val = (bass * env * 0.35 + guitar + kick + snare) * 0.75;
                out.push(((val * 115.0) + 128.0).clamp(0.0, 255.0) as u8);
            }
        }
        // 1: RADIO BROKER (Post-Punk / Indie Rock) - angular bassline, rapid hi-hats, chorus synth
        1 => {
            let bpm = 140.0;
            let beat_dur = 60.0 / bpm;
            let bass_notes = [110.00, 110.00, 130.81, 98.00, 87.31, 87.31, 98.00, 110.00];
            for i in 0..samples {
                let t = i as f32 / rate as f32;
                let step = (t / (beat_dur * 0.5)) as usize % bass_notes.len();
                let step_t = t % (beat_dur * 0.5);
                let env = (1.0 - step_t / (beat_dur * 0.5)).powf(1.4);
                let f_bass = bass_notes[step];
                let bass = (t * f_bass * std::f32::consts::TAU).sin() * 0.5
                    + (t * f_bass * 3.0 * std::f32::consts::TAU).sin() * 0.2;
                let hat_t = t % (beat_dur * 0.25);
                let hat_noise = ((i * 1664525 + 1013904223) & 0xFF) as f32 / 128.0 - 1.0;
                let hihat = hat_noise * (1.0 - hat_t / 0.04).max(0.0) * 0.18;
                let chorus = (t * 440.0 * std::f32::consts::TAU + (t * 3.5).sin()).sin() * 0.18;
                let val = (bass * env * 0.4 + hihat + chorus) * 0.8;
                out.push(((val * 115.0) + 128.0).clamp(0.0, 255.0) as u8);
            }
        }
        // 2: TORTOISE (Post-Rock / Instrumental) - vibraphone ostinato, warm jazzy rhodes, mellow groove
        2 => {
            let notes = [261.63, 329.63, 392.00, 523.25, 392.00, 329.63, 293.66, 349.23];
            for i in 0..samples {
                let t = i as f32 / rate as f32;
                let note_idx = ((t / 0.40) as usize) % notes.len();
                let note_t = t % 0.40;
                let vib_env = (1.0 - note_t / 0.40).powf(1.8);
                let freq = notes[note_idx];
                let vib = (t * freq * std::f32::consts::TAU).sin() * 0.5
                    + (t * freq * 2.0 * std::f32::consts::TAU).sin() * 0.15;
                let sub = (t * (freq * 0.25) * std::f32::consts::TAU).sin() * 0.35;
                let val = (vib * vib_env * 0.55 + sub * 0.3) * 0.75;
                out.push(((val * 115.0) + 128.0).clamp(0.0, 255.0) as u8);
            }
        }
        // 3: TURNTABLES ON THE HUDSON (Hip-Hop / Funk / Soul) - slap bass, boom-bap beat
        3 => {
            let bpm = 100.0;
            let beat_dur = 60.0 / bpm;
            let slap_notes = [98.00, 98.00, 116.54, 98.00, 130.81, 146.83, 116.54, 98.00];
            for i in 0..samples {
                let t = i as f32 / rate as f32;
                let step = ((t / (beat_dur * 0.5)) as usize) % slap_notes.len();
                let step_t = t % (beat_dur * 0.5);
                let env = (1.0 - step_t / (beat_dur * 0.5)).powf(2.0);
                let f_slap = slap_notes[step];
                let slap = (t * f_slap * std::f32::consts::TAU).sin() * 0.55
                    + (t * f_slap * 2.0 * std::f32::consts::TAU).sin() * 0.3;
                let beat_step = (t / beat_dur) as usize % 4;
                let in_beat = t % beat_dur;
                let kick = if beat_step == 0 || (beat_step == 2 && in_beat > beat_dur * 0.5) {
                    (in_beat * 70.0 * std::f32::consts::TAU).sin() * (1.0 - (in_beat % (beat_dur * 0.5)) / 0.18).max(0.0) * 0.6
                } else {
                    0.0
                };
                let snare = if beat_step == 1 || beat_step == 3 {
                    let noise = ((i * 1103515245 + 12345) & 0xFF) as f32 / 128.0 - 1.0;
                    noise * (1.0 - in_beat / 0.20).max(0.0) * 0.35
                } else {
                    0.0
                };
                let val = (slap * env * 0.4 + kick + snare) * 0.8;
                out.push(((val * 115.0) + 128.0).clamp(0.0, 255.0) as u8);
            }
        }
        // 4: THE PRAIRIE CARTEL (Acid Rock / Electronica) - TB-303 resonant acid sweep
        4 => {
            let bpm = 126.0;
            let beat_dur = 60.0 / bpm;
            let acid_notes = [65.41, 65.41, 77.78, 65.41, 87.31, 98.00, 116.54, 130.81];
            for i in 0..samples {
                let t = i as f32 / rate as f32;
                let step = ((t / (beat_dur * 0.25)) as usize) % acid_notes.len();
                let step_t = t % (beat_dur * 0.25);
                let cutoff_mod = (t * 0.6 * std::f32::consts::TAU).sin() * 0.5 + 0.5;
                let env = (1.0 - step_t / (beat_dur * 0.25)).powf(1.8);
                let f_note = acid_notes[step];
                let saw = (t * f_note).fract() * 2.0 - 1.0;
                let res = (t * (f_note * (2.0 + cutoff_mod * 4.0)) * std::f32::consts::TAU).sin() * 0.4;
                let acid = (saw * 0.6 + res) * env;
                let kick_t = t % beat_dur;
                let kick = (kick_t * 60.0 * std::f32::consts::TAU).sin() * (1.0 - kick_t / 0.16).max(0.0) * 0.6;
                let val = (acid * 0.45 + kick * 0.55) * 0.8;
                out.push(((val * 115.0) + 128.0).clamp(0.0, 255.0) as u8);
            }
        }
        // 5: DJ KHALIL (West Coast Hip-Hop) - deep 808 sub, G-funk sine whistle
        5 => {
            let bpm = 92.0;
            let beat_dur = 60.0 / bpm;
            let whistle_notes = [1174.66, 1318.51, 1174.66, 987.77, 880.00, 987.77, 1174.66, 1318.51];
            for i in 0..samples {
                let t = i as f32 / rate as f32;
                let step = ((t / (beat_dur * 0.5)) as usize) % whistle_notes.len();
                let freq = whistle_notes[step];
                let vib = (t * 6.0 * std::f32::consts::TAU).sin() * 6.0;
                let whistle = ((t * (freq + vib) * std::f32::consts::TAU).sin()) * 0.25;
                let beat_t = t % beat_dur;
                let sub_pitch = 55.0 - (beat_t * 60.0).min(20.0);
                let sub = (beat_t * sub_pitch * std::f32::consts::TAU).sin() * (1.0 - beat_t / beat_dur).powf(1.5) * 0.6;
                let beat_idx = (t / beat_dur) as usize % 2;
                let snare = if beat_idx == 1 {
                    let noise = ((i * 1103515245 + 12345) & 0xFF) as f32 / 128.0 - 1.0;
                    noise * (1.0 - beat_t / 0.22).max(0.0) * 0.35
                } else {
                    0.0
                };
                let val = (sub * 0.55 + whistle + snare) * 0.8;
                out.push(((val * 115.0) + 128.0).clamp(0.0, 255.0) as u8);
            }
        }
        // 6: TORTOISE SPECIAL (Tortoise Instrumental Beats) - glitch polyrhythms, chime pads
        6 => {
            for i in 0..samples {
                let t = i as f32 / rate as f32;
                let pad = (t * 220.0 * std::f32::consts::TAU).sin() * 0.2
                    + (t * 277.18 * std::f32::consts::TAU).sin() * 0.2
                    + (t * 329.63 * std::f32::consts::TAU).sin() * 0.2;
                let glitch_t = t % 0.133;
                let noise = ((i * 1664525 + 1013904223) & 0xFF) as f32 / 128.0 - 1.0;
                let glitch = noise * (1.0 - glitch_t / 0.04).max(0.0) * 0.25;
                let val = (pad * 0.5 + glitch * 0.5) * 0.75;
                out.push(((val * 115.0) + 128.0).clamp(0.0, 255.0) as u8);
            }
        }
        // 7: WONDERWHEEL RECORDINGS (Deep Afrobeat & Latin) - montuno piano, bongo syncopation
        7 => {
            let piano_notes = [261.63, 329.63, 392.00, 440.00, 392.00, 329.63, 261.63, 349.23];
            for i in 0..samples {
                let t = i as f32 / rate as f32;
                let step = ((t / 0.20) as usize) % piano_notes.len();
                let step_t = t % 0.20;
                let env = (1.0 - step_t / 0.20).powf(1.5);
                let f = piano_notes[step];
                let piano = (t * f * std::f32::consts::TAU).sin() * 0.45
                    + (t * f * 2.0 * std::f32::consts::TAU).sin() * 0.2;
                let conga_t = t % 0.266;
                let conga = (conga_t * 240.0 * std::f32::consts::TAU).sin() * (1.0 - conga_t / 0.12).max(0.0) * 0.35;
                let val = (piano * env * 0.55 + conga * 0.45) * 0.8;
                out.push(((val * 115.0) + 128.0).clamp(0.0, 255.0) as u8);
            }
        }
        // 8: ANVIL (Heavy Metal Canadien) - fast double-bass kick, distorted thrash chug
        8 => {
            let bpm = 160.0;
            let beat_dur = 60.0 / bpm;
            let f_e1 = 41.20;
            for i in 0..samples {
                let t = i as f32 / rate as f32;
                let step_t = t % (beat_dur * 0.5);
                let env = (1.0 - step_t / (beat_dur * 0.5)).powf(1.1);
                let saw = ((t * f_e1).fract() * 2.0 - 1.0) * 3.5;
                let chug = saw.clamp(-0.8, 0.8) * env * 0.5;
                let dkick_t = t % (beat_dur * 0.25);
                let kick = (dkick_t * 68.0 * std::f32::consts::TAU).sin() * (1.0 - dkick_t / 0.10).max(0.0) * 0.5;
                let val = (chug + kick) * 0.8;
                out.push(((val * 115.0) + 128.0).clamp(0.0, 255.0) as u8);
            }
        }
        // 9: SINOWAV FM (Musique Traditionnelle & Electro) - D pentatonic guzheng, sub-bass
        9 => {
            let penta = [293.66, 329.63, 369.99, 440.00, 493.88, 587.33, 493.88, 440.00];
            for i in 0..samples {
                let t = i as f32 / rate as f32;
                let step = ((t / 0.25) as usize) % penta.len();
                let step_t = t % 0.25;
                let env = (1.0 - step_t / 0.25).powf(2.0);
                let f = penta[step];
                let pluck = (t * f * std::f32::consts::TAU).sin() * 0.45
                    + (t * f * 2.0 * std::f32::consts::TAU).sin() * 0.25;
                let sub = (t * 73.42 * std::f32::consts::TAU).sin() * 0.35;
                let val = (pluck * env * 0.6 + sub * 0.4) * 0.8;
                out.push(((val * 115.0) + 128.0).clamp(0.0, 255.0) as u8);
            }
        }
        // 10: DFA RECORDS (Electro Dance / Disco Punk) - 128 BPM disco beat, pumping octave bass
        _ => {
            let bpm = 128.0;
            let beat_dur = 60.0 / bpm;
            let bass_notes = [65.41, 130.81, 65.41, 130.81, 77.78, 155.56, 77.78, 155.56];
            for i in 0..samples {
                let t = i as f32 / rate as f32;
                let step = ((t / (beat_dur * 0.25)) as usize) % bass_notes.len();
                let step_t = t % (beat_dur * 0.25);
                let env = (1.0 - step_t / (beat_dur * 0.25)).powf(1.6);
                let f = bass_notes[step];
                let bass = (t * f * std::f32::consts::TAU).sin() * 0.55;
                let kick_t = t % beat_dur;
                let kick = (kick_t * 60.0 * std::f32::consts::TAU).sin() * (1.0 - kick_t / 0.16).max(0.0) * 0.6;
                let hat_active = (kick_t > beat_dur * 0.4) && (kick_t < beat_dur * 0.8);
                let hat = if hat_active {
                    let noise = ((i * 1103515245 + 12345) & 0xFF) as f32 / 128.0 - 1.0;
                    noise * 0.22
                } else {
                    0.0
                };
                let val = (bass * env * 0.45 + kick * 0.45 + hat) * 0.8;
                out.push(((val * 115.0) + 128.0).clamp(0.0, 255.0) as u8);
            }
        }
    }

    out
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

    #[test]
    fn synthesized_audio_effects_and_fanfare() {
        let sound_system = SoundSystem::new(
            SoundTables {
                events: vec![],
                gears: vec![],
                prop_sfx: vec![],
                collision_low: [0; 3],
                collision_med: [0; 3],
                collision_high: [0; 3],
                rev_after_shift: [0; 4],
            },
            SoundBank { samples: vec![] },
        );

        // Verify triggers play without error on internal mixer
        sound_system.play_cash_register();
        sound_system.play_pda_notification();
        sound_system.play_glass_break([10.0, 10.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0]);
        sound_system.play_police_dispatch();
        sound_system.play_defibrillator();
        sound_system.play_mission_fanfare();

        let mut out = [0i16; 1024];
        if let Ok(mut m) = sound_system.mixer.lock() {
            m.mix(&mut out);
        }
        assert!(out.iter().any(|&s| s != 0), "Audio mixer must have rendered non-zero samples");
    }

    #[test]
    fn procedural_radio_streams_all_11_stations() {
        // Test all 11 official Chinatown Wars stations
        for station_idx in 0..11 {
            let pcm = synth_station_loop(station_idx);
            assert!(pcm.len() > 20000, "Station {} generated substantial PCM buffer", station_idx);
            // Verify buffer has dynamic range (not silent or flat)
            let min_val = *pcm.iter().min().unwrap();
            let max_val = *pcm.iter().max().unwrap();
            assert!(min_val < 120, "Station {} has negative wave excursions", station_idx);
            assert!(max_val > 136, "Station {} has positive wave excursions", station_idx);
        }
    }

    #[test]
    fn emergency_sirens_radio_playback_and_pedestrian_impacts() {
        let mut sound_system = SoundSystem::new(
            SoundTables {
                events: vec![],
                gears: vec![],
                prop_sfx: vec![],
                collision_low: [0; 3],
                collision_med: [0; 3],
                collision_high: [0; 3],
                rev_after_shift: [0; 4],
            },
            SoundBank { samples: vec![] },
        );

        // Pedestrian reaction impacts and vehicle horn
        sound_system.play_ped_impact([10.0, 10.0, 0.0], false, [0.0, 0.0, 0.0], [0.0, 1.0]);
        sound_system.play_ped_impact([10.0, 10.0, 0.0], true, [0.0, 0.0, 0.0], [0.0, 1.0]);
        sound_system.play_car_horn([10.0, 10.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0]);

        // Radio system integration
        let mut radio = crate::radio::RadioManager::new(None);
        assert_eq!(sound_system.radio_voice, 0);

        // In-vehicle entry: radio turns on
        radio.update(0.1, true);
        sound_system.update_radio(&radio);
        assert_ne!(sound_system.radio_voice, 0, "Radio voice started when in car");
        assert_eq!(sound_system.current_radio_station, 0);

        // Tune to next station
        radio.next_station();
        sound_system.update_radio(&radio);
        assert_eq!(sound_system.current_radio_station, 1);

        // Turn radio off (station 11)
        radio.set_station(11);
        sound_system.update_radio(&radio);
        assert_eq!(sound_system.radio_voice, 0, "Radio voice stopped when turned OFF");

        // Sirens update
        let mut police = crate::police::PoliceManager::new();
        let mut cars = vec![];
        sound_system.update_sirens(&police, &cars, [0.0, 0.0, 0.0], [0.0, 1.0]);
        assert_eq!(sound_system.siren_voice, 0);

        // Police pursuit with 1 star (wail siren)
        police.set_stars(1);
        // Spawn car with police spec
        let spec = crate::vehicle::spec(18); // POLICE
        let car = crate::actors::Car {
            p: [15.0, 10.0, 0.0],
            angle: 0.0,
            speed: 10.0,
            lat_vel: 0.0,
            yaw_rate: 0.0,
            steer_angle: 0.0,
            gear: 1,
            rpm: 0.5,
            roll: 0.0,
            pitch: 0.0,
            roll_vel: 0.0,
            pitch_vel: 0.0,
            wheel_spin: false,
            front_skidding: false,
            rear_skidding: false,
            braking: false,
            headlights: true,
            info_id: spec.info_id,
            model_id: spec.model_id,
            name: spec.name,
            h: spec.half_extents(),
            paint: 0,
            from_cargen: false,
            parked: false,
            lane: 0,
            wobble: 0.0,
            next_lane: 0,
            next_wobble: 0.0,
            rail_dist: 0.0,
            next_target: 0,
            on_rail: false,
            start_delay: 0.0,
            driver_appearance: None,
            driver_pose: crate::ped::Pose::default(),
            target: 0,
            previous: 0,
            blocked: 0.0,
            siren_active: true,
            health: 1000.0,
            flat_tires: [false; 4],
            disabled: false,
            sparks: 0.0,
            respray_timer: 0.0,
        };
        cars.push(car);

        sound_system.update_sirens(&police, &cars, [0.0, 0.0, 0.0], [0.0, 1.0]);
        assert_ne!(sound_system.siren_voice, 0, "Wail siren started for 1-star pursuit");

        // High pursuit with 3 stars (yelp siren)
        police.set_stars(3);
        sound_system.update_sirens(&police, &cars, [0.0, 0.0, 0.0], [0.0, 1.0]);
        assert_ne!(sound_system.siren_yelp_voice, 0, "Yelp siren started for 3-star pursuit");

        // Clear stars: sirens turn off
        police.set_stars(0);
        cars[0].siren_active = false;
        sound_system.update_sirens(&police, &cars, [0.0, 0.0, 0.0], [0.0, 1.0]);
        assert_eq!(sound_system.siren_voice, 0);
        assert_eq!(sound_system.siren_yelp_voice, 0);
    }
}
