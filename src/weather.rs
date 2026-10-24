// SPDX-License-Identifier: GPL-3.0-only
//! Experimental weather manager; transition selection checked against PSP 0x08a8e930.
//! Exact binary layout from data/tables/weather_tables.bin (CTWWTHR1, 8x8 chance matrix, 5 thunder samples).
//! RNG scheduling, lighting, rain and lightning remain prototype implementations.

#![allow(dead_code)]

use crate::iso::{u32le, Result};

pub const NUM_WEATHERS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeatherKind {
    Sunny = 0,
    ExtraSunny = 1,
    Cloudy = 2,
    Overcast = 3,
    Rain = 4,
    Thunderstorm = 5,
    Foggy = 6,
    LightningFlash = 7,
}

impl WeatherKind {
    pub fn from_u8(v: u8) -> Self {
        match v % 8 {
            0 => Self::Sunny,
            1 => Self::ExtraSunny,
            2 => Self::Cloudy,
            3 => Self::Overcast,
            4 => Self::Rain,
            5 => Self::Thunderstorm,
            6 => Self::Foggy,
            _ => Self::LightningFlash,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Sunny => "ENSOLEILLE",
            Self::ExtraSunny => "TRES ENSOLEILLE",
            Self::Cloudy => "NUAGEUX",
            Self::Overcast => "COUVERT",
            Self::Rain => "PLUIE",
            Self::Thunderstorm => "ORAGE / TEMPETE",
            Self::Foggy => "BRUMEUX",
            Self::LightningFlash => "ECLAIR",
        }
    }

    pub fn is_wet(self) -> bool {
        matches!(self, Self::Rain | Self::Thunderstorm)
    }
}

/// Weather transition probability tables and resident sound bank thunder samples.
#[derive(Clone, Debug)]
pub struct WeatherTables {
    pub chance: [[u8; 8]; 8],
    pub thunder_samples: [u32; 5],
}

impl WeatherTables {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 92 {
            return Err("weather_tables.bin truncated".into());
        }
        if &data[0..8] != b"CTWWTHR1" {
            return Err("Invalid weather_tables magic".into());
        }

        let mut chance = [[0u8; 8]; 8];
        for row in 0..8 {
            for col in 0..8 {
                chance[row][col] = data[8 + row * 8 + col];
            }
        }

        let mut thunder_samples = [0u32; 5];
        for i in 0..5 {
            thunder_samples[i] = u32le(data, 72 + i * 4)?;
        }

        Ok(Self {
            chance,
            thunder_samples,
        })
    }

    /// PSP ULES01347 1.01, 0x08a8e930: subtract each weight, accept on BLEZ.
    /// Zero and exact cumulative boundaries belong to the earlier slot.
    /// The caller supplies the draw; this does not reproduce the PSP RNG state.
    pub fn roll_next(&self, current: WeatherKind, roll_val: u8) -> WeatherKind {
        let row = &self.chance[current as usize];
        let mut remaining = i32::from(roll_val);
        for (idx, &prob) in row.iter().enumerate() {
            remaining -= i32::from(prob);
            if remaining <= 0 {
                return WeatherKind::from_u8(idx as u8);
            }
        }
        WeatherKind::from_u8(6)
    }
}

/// Dynamic Weather & Circadian Lightning state controller.
#[derive(Clone, Debug)]
pub struct WeatherManager {
    pub tables: Option<WeatherTables>,
    pub current: WeatherKind,
    pub next: WeatherKind,
    pub transition: f32, // 0.0 .. 1.0 hourly transition progress
    pub lightning_flash: bool,
    pub lightning_intensity: f32,
    pub thunder_pending: Option<(u32, f32)>, // (thunder sample, delay timer)
    pub strike_timer: f32,
    pub rain_intensity: f32,
    pub hour: u8,
    pub minute: u8,
}

impl WeatherManager {
    pub fn new(tables: Option<WeatherTables>) -> Self {
        Self {
            tables,
            current: WeatherKind::Sunny,
            next: WeatherKind::Sunny,
            transition: 0.0,
            lightning_flash: false,
            lightning_intensity: 0.0,
            thunder_pending: None,
            strike_timer: 12.0,
            rain_intensity: 0.0,
            hour: 8,
            minute: 0,
        }
    }

    /// Updates clock time from frame ticks (30 ticks per game second).
    pub fn update_time(&mut self, ticks: usize) {
        let total_minutes = (8 * 60 + ticks / 30) % 1440;
        let new_hour = (total_minutes / 60) as u8;
        let new_minute = (total_minutes % 60) as u8;

        if new_hour != self.hour {
            self.on_hour_change(new_hour);
        }
        self.hour = new_hour;
        self.minute = new_minute;
        self.transition = (new_minute as f32) / 60.0;
    }

    fn on_hour_change(&mut self, new_hour: u8) {
        self.current = self.next;
        if let Some(tables) = &self.tables {
            // Deterministic roll pseudo-seed based on hour
            let roll = ((new_hour as usize * 37 + 13) % 100) as u8;
            self.next = tables.roll_next(self.current, roll);
        }
    }

    /// Advances weather dynamics, lightning strikes, and rain droplet density.
    pub fn update(&mut self, dt: f32) {
        // Target rain intensity
        let target_rain = match self.current {
            WeatherKind::Rain => 0.65,
            WeatherKind::Thunderstorm => 1.0,
            _ => 0.0,
        };
        self.rain_intensity += (target_rain - self.rain_intensity) * (dt * 0.5).min(1.0);

        // Thunderstorm lightning logic
        if self.current == WeatherKind::Thunderstorm || self.next == WeatherKind::Thunderstorm {
            self.strike_timer = (self.strike_timer - dt).max(0.0);
            if self.strike_timer <= 0.0 {
                // Trigger lightning strike (flash + delayed thunder)
                self.lightning_flash = true;
                self.lightning_intensity = 1.0;
                let sample = self.tables.as_ref().map(|t| t.thunder_samples[0]).unwrap_or(815);
                self.thunder_pending = Some((sample, 0.28)); // 0.28s sound delay
                self.strike_timer = 9.5; // Next strike in ~9.5s
            }
        } else {
            self.lightning_flash = false;
            self.lightning_intensity = 0.0;
        }

        // Decay visual lightning flash quickly (over 0.15s)
        if self.lightning_intensity > 0.0 {
            self.lightning_intensity = (self.lightning_intensity - dt * 6.5).max(0.0);
            if self.lightning_intensity == 0.0 {
                self.lightning_flash = false;
            }
        }

        // Advance thunder sound timer
        if let Some((_, delay)) = &mut self.thunder_pending {
            *delay -= dt;
        }
    }

    /// Evaluates circardian ambient lighting multiplier and sky tint.
    pub fn ambient_color(&self) -> [f32; 3] {
        let time_f = self.hour as f32 + (self.minute as f32) / 60.0;

        // Base daylight curve: peak at 12:00 (1.0), night at 00:00 (0.28)
        let sun_curve = ((time_f - 4.0) * (std::f32::consts::PI / 12.0)).sin().max(-0.4);
        let daylight = (0.35 + 0.65 * (sun_curve.max(0.0))).clamp(0.25, 1.0);

        // Overcast / Rain dimming
        let weather_dim = match self.current {
            WeatherKind::Sunny | WeatherKind::ExtraSunny => 1.0,
            WeatherKind::Cloudy => 0.88,
            WeatherKind::Overcast | WeatherKind::Foggy => 0.72,
            WeatherKind::Rain => 0.60,
            WeatherKind::Thunderstorm => 0.45,
            WeatherKind::LightningFlash => 1.25,
        };

        let intensity = (daylight * weather_dim + self.lightning_intensity * 0.65).clamp(0.2, 1.35);

        // Warm daylight vs cold nocturnal blue tint
        let is_night = self.hour < 6 || self.hour >= 20;
        if is_night {
            [0.22 * intensity, 0.28 * intensity, 0.45 * intensity]
        } else {
            [1.0 * intensity, 0.95 * intensity, 0.88 * intensity]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_weather_layout_and_optional_reference() {
        let mut data = vec![0; 92];
        data[..8].copy_from_slice(b"CTWWTHR1");
        data[8] = 25;
        data[9] = 75;
        data[72..76].copy_from_slice(&1234u32.to_le_bytes());
        let parsed = WeatherTables::parse(&data).unwrap();
        assert_eq!(parsed.thunder_samples[0], 1234);
        assert_eq!(parsed.roll_next(WeatherKind::Sunny, 24), WeatherKind::Sunny);
        assert_eq!(parsed.roll_next(WeatherKind::Sunny, 25), WeatherKind::Sunny);
        assert_eq!(parsed.roll_next(WeatherKind::Sunny, 26), WeatherKind::ExtraSunny);
        assert!(WeatherTables::parse(&data[..91]).is_err());
        data[0] = 0;
        assert!(WeatherTables::parse(&data).is_err());

        if let Some(data) = crate::test_support::reference_table("weather_tables.bin") {
        let tables = WeatherTables::parse(&data).unwrap();

        assert_eq!(tables.chance.len(), 8);
        assert_eq!(tables.thunder_samples, [815, 816, 817, 818, 819]);

        // State 0 (Sunny) transitions mostly to Sunny (40%) and ExtraSunny (50%)
        let next_0 = tables.roll_next(WeatherKind::Sunny, 35);
        assert_eq!(next_0, WeatherKind::Sunny);
        let next_1 = tables.roll_next(WeatherKind::Sunny, 75);
        assert_eq!(next_1, WeatherKind::ExtraSunny);
        }
    }

    #[test]
    fn weather_manager_cycle_and_lightning_dynamics() {
        let mut mgr = WeatherManager::new(None);
        assert_eq!(mgr.current, WeatherKind::Sunny);

        // Advance 2 hours
        mgr.update_time(30 * 60 * 2); // 2 hours in frames
        assert_eq!(mgr.hour, 10);

        // Force thunderstorm
        mgr.current = WeatherKind::Thunderstorm;
        mgr.strike_timer = 0.05;
        mgr.update(0.1);

        assert!(mgr.lightning_flash, "Lightning triggers flash in thunderstorm");
        assert!(mgr.lightning_intensity > 0.0);
        assert!(mgr.thunder_pending.is_some(), "Delayed thunder sound scheduled");
        assert!(mgr.ambient_color()[0] > 0.0, "Calculates ambient color");
    }
}
