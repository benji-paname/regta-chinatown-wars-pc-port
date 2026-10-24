// SPDX-License-Identifier: GPL-3.0-only
//! Chinatown Wars PSP authentic Radio System (cRadioApp, cWavStream).
//! Exact binary layout from data/tables/radio_tables.bin (CTWRAD2 format, 11 stations, 33 audio streams).
//! Supports vehicle station tuning, station cycling, volume control, in-vehicle HUD banner notifications,
//! and authentic track positioning.

#![allow(dead_code)]

use crate::iso::{u32le, Result};

pub const NUM_STATIONS: usize = 11;
pub const NUM_STREAMS: usize = 33;

#[derive(Clone, Debug, Default)]
pub struct StationDef {
    pub icon: i32,
    pub stream: i32,
    pub label: i32,
    pub name: i32,
}

#[derive(Clone, Debug)]
pub struct RadioTables {
    pub stations: Vec<StationDef>,
    pub streams: Vec<String>,
    pub volume_sprites: [u8; 10],
}

impl RadioTables {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 26 + NUM_STATIONS * 16 + NUM_STREAMS * 40 {
            return Err("radio_tables.bin truncated".into());
        }
        if &data[0..8] != b"CTWRAD2\0" {
            return Err("Invalid radio_tables magic".into());
        }

        let n_stations = u32le(data, 8)? as usize;
        let n_streams = u32le(data, 12)? as usize;
        if n_stations != NUM_STATIONS || n_streams != NUM_STREAMS {
            return Err("Unsupported station or stream count".into());
        }

        let mut volume_sprites = [0u8; 10];
        volume_sprites.copy_from_slice(&data[16..26]);

        let mut offset = 26;
        let mut stations = Vec::with_capacity(n_stations);
        for _ in 0..n_stations {
            let icon = u32le(data, offset)? as i32;
            let stream = u32le(data, offset + 4)? as i32;
            let label = u32le(data, offset + 8)? as i32;
            let name = u32le(data, offset + 12)? as i32;
            stations.push(StationDef {
                icon,
                stream,
                label,
                name,
            });
            offset += 16;
        }

        let mut streams = Vec::with_capacity(n_streams);
        for _ in 0..n_streams {
            let raw = &data[offset..offset + 40];
            let len = raw.iter().position(|&b| b == 0).unwrap_or(40);
            let s = String::from_utf8_lossy(&raw[..len]).to_string();
            streams.push(s);
            offset += 40;
        }

        Ok(Self {
            stations,
            streams,
            volume_sprites,
        })
    }
}

/// Official Chinatown Wars station metadata.
#[derive(Clone, Debug)]
pub struct StationInfo {
    pub id: usize,
    pub name: &'static str,
    pub genre: &'static str,
    pub stream_file: &'static str,
    pub icon_idx: u8,
}

pub const OFFICIAL_STATIONS: [StationInfo; 11] = [
    StationInfo { id: 0, name: "TRUTHWAVE", genre: "Rock Indie / Alternatif", stream_file: "Radio1.mp3", icon_idx: 3 },
    StationInfo { id: 1, name: "RADIO BROKER", genre: "Indie Rock / Post-Punk", stream_file: "Radio2.mp3", icon_idx: 8 },
    StationInfo { id: 2, name: "TORTOISE", genre: "Post-Rock / Instrumental", stream_file: "Radio3.mp3", icon_idx: 9 },
    StationInfo { id: 3, name: "TURNTABLES ON THE HUDSON", genre: "Hip-Hop / Funk / Soul", stream_file: "Radio4.mp3", icon_idx: 12 },
    StationInfo { id: 4, name: "THE PRAIRIE CARTEL", genre: "Acid Rock / Electronica", stream_file: "Radio5.mp3", icon_idx: 16 },
    StationInfo { id: 5, name: "DJ KHALIL", genre: "West Coast Hip-Hop", stream_file: "Khalil.mp3", icon_idx: 10 },
    StationInfo { id: 6, name: "TORTOISE SPECIAL", genre: "Tortoise Instrumental Beats", stream_file: "Tortoise.mp3", icon_idx: 19 },
    StationInfo { id: 7, name: "WONDERWHEEL RECORDINGS", genre: "Deep Afrobeat & Latin", stream_file: "Wonderwheel.mp3", icon_idx: 31 },
    StationInfo { id: 8, name: "ANVIL", genre: "Heavy Metal Canadien", stream_file: "Anvil.mp3", icon_idx: 0 },
    StationInfo { id: 9, name: "SINOWAV FM", genre: "Musique Traditionnelle & Electro Chinoise", stream_file: "Chinese.mp3", icon_idx: 4 },
    StationInfo { id: 10, name: "DFA RECORDS", genre: "Electro Dance / Disco Punk", stream_file: "DFA.mp3", icon_idx: 6 },
];

/// Active in-game Radio state controller (managed inside Scene and play.rs).
#[derive(Clone, Debug)]
pub struct RadioManager {
    pub tables: Option<RadioTables>,
    pub current_station: usize, // 0..10: Radio stations, 11: Radio Off
    pub volume: u8, // 0..10 volume steps
    pub hud_banner_timer: f32, // Display timer for station name when changing
    pub is_playing: bool,
    pub car_active: bool,
    pub track_positions: [f32; 11], // Track position memory per station (seconds)
}

impl RadioManager {
    pub fn new(tables: Option<RadioTables>) -> Self {
        Self {
            tables,
            current_station: 0,
            volume: 8,
            hud_banner_timer: 0.0,
            is_playing: false,
            car_active: false,
            track_positions: [0.0; 11],
        }
    }

    /// Returns the currently tuned station metadata, or None if turned OFF.
    pub fn current(&self) -> Option<&'static StationInfo> {
        if self.current_station < OFFICIAL_STATIONS.len() {
            Some(&OFFICIAL_STATIONS[self.current_station])
        } else {
            None
        }
    }

    /// Returns the display string of the active station.
    pub fn display_name(&self) -> &'static str {
        if let Some(st) = self.current() {
            st.name
        } else {
            "RADIO : ETEINTE"
        }
    }

    /// Tunes to the next radio station (cycles 0..11).
    pub fn next_station(&mut self) {
        self.current_station = (self.current_station + 1) % (OFFICIAL_STATIONS.len() + 1);
        self.hud_banner_timer = 2.8;
    }

    /// Tunes to the previous radio station.
    pub fn prev_station(&mut self) {
        if self.current_station == 0 {
            self.current_station = OFFICIAL_STATIONS.len();
        } else {
            self.current_station -= 1;
        }
        self.hud_banner_timer = 2.8;
    }

    /// Tunes directly to a specific station index.
    pub fn set_station(&mut self, idx: usize) {
        if idx <= OFFICIAL_STATIONS.len() {
            self.current_station = idx;
            self.hud_banner_timer = 2.8;
        }
    }

    /// Updates radio playback state when entering or exiting a vehicle.
    pub fn update(&mut self, dt: f32, in_car: bool) {
        self.hud_banner_timer = (self.hud_banner_timer - dt).max(0.0);

        if in_car && !self.car_active {
            // Just entered vehicle: turn radio on and show banner
            self.car_active = true;
            self.is_playing = self.current_station < OFFICIAL_STATIONS.len();
            self.hud_banner_timer = 3.0;
        } else if !in_car && self.car_active {
            // Exited vehicle: turn off radio stream
            self.car_active = false;
            self.is_playing = false;
        }

        // Advance track position in background
        if self.is_playing && self.current_station < 11 {
            self.track_positions[self.current_station] += dt;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_authentic_radio_tables() {
        let data = std::fs::read("data/tables/radio_tables.bin").unwrap();
        let tables = RadioTables::parse(&data).unwrap();

        assert_eq!(tables.stations.len(), 11, "11 authentic stations");
        assert_eq!(tables.streams.len(), 33, "33 stream entries");
        assert_eq!(tables.volume_sprites.len(), 10);

        // Verify station 0 (Truthwave / Radio1.mp3)
        assert_eq!(tables.stations[0].stream, 0);
        assert_eq!(tables.streams[0], "Radio1.mp3");

        // Verify station 9 (SinoWav / Chinese.mp3)
        assert_eq!(tables.stations[9].stream, 18);
        assert_eq!(tables.streams[18], "Chinese.mp3");
    }

    #[test]
    fn radio_manager_tuning_and_car_transition() {
        let mut radio = RadioManager::new(None);
        assert_eq!(radio.current_station, 0);
        assert_eq!(radio.display_name(), "TRUTHWAVE");

        // Cycle through all 11 stations to OFF
        for _ in 0..11 {
            radio.next_station();
        }
        assert_eq!(radio.current_station, 11, "Tuned to Radio OFF");
        assert_eq!(radio.display_name(), "RADIO : ETEINTE");

        // Previous station returns to DFA RECORDS
        radio.prev_station();
        assert_eq!(radio.current_station, 10);
        assert_eq!(radio.display_name(), "DFA RECORDS");

        // Vehicle entry activates playback
        radio.update(0.1, true);
        assert!(radio.is_playing);
        assert!(radio.hud_banner_timer > 2.0);

        // Vehicle exit stops playback
        radio.update(0.1, false);
        assert!(!radio.is_playing);
    }
}
