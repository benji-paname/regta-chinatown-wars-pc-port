// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Kryo.to
// Port de cStatsManager (gStatsManager) de Grand Theft Auto: Chinatown Wars PSP.
// Conforme a la disposition binaire exacte de sauvegarde (0xFC octets).

#![allow(dead_code)]

/// GXT text IDs des 11 stations de radio officielles (cStatsManager::kStationText à 0x486D50)
pub const STATION_TEXT_IDS: [i32; 11] = [225, 226, 227, 228, 231, 232, 233, 1327, 1328, 1329, 229];

/// Structure binaire exacte de la section des statistiques dans la sauvegarde PSP (0xFC = 252 octets).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatsData {
    pub criminal_rating: i32,        // +0x00
    pub total_playing_time: u32,     // +0x04 en minutes de jeu
    pub unknown08: u32,            // +0x08
    pub unknown0c: u32,            // +0x0C (necessaire pour le rang criminel supreme)
    pub media_value: u32,           // +0x10 somme des indices de recherche les plus eleves chaque heure
    pub distance: [u32; 5],         // +0x14 a pied, nage, voiture, moto, bateau (en 1/16 d'unite)
    pub spent_on_weapons: u32,      // +0x28
    pub spent_on_pay_n_spray: u32,  // +0x2C
    pub spent_on_property: u32,     // +0x30
    pub lost_on_street_races: u32,  // +0x34
    pub spent_on_bribing_cops: u32, // +0x38
    pub spent_on_health_care: u32,  // +0x3C
    pub made_from_activities: u32,  // +0x40
    pub made_from_taxi_tips: u32,   // +0x44
    pub found_on_the_street: u32,   // +0x48
    pub value_of_goods_sold: u32,   // +0x4C
    pub value_of_goods_bought: u32, // +0x50
    pub bullets_fired: u32,         // +0x54
    pub bullets_that_hit: u32,      // +0x58
    pub longest_time_in_vehicle: u32, // +0x5C en secondes
    pub made_from_trading: i32,     // +0x60
    pub spent_on_scratch_cards: i32,// +0x64
    pub made_on_scratch_cards: i32, // +0x68
    pub lions_collected: u16,       // +0x6C
    pub cockroaches_squashed: u16,  // +0x6E
    pub people_killed: u16,         // +0x70
    pub people_burnt_out: u16,      // +0x72
    pub near_misses: u16,           // +0x74
    pub vehicles_stolen: u16,       // +0x76
    pub wanted_stars_attained: u16, // +0x78
    pub wanted_stars_evaded: u16,   // +0x7A
    pub criminals_killed: u16,      // +0x7C
    pub longest_police_chase: u16,  // +0x7E
    pub longest_chase_with_6_stars: u16, // +0x80
    pub units_of_goods_sold: u16,   // +0x82
    pub units_of_goods_bought: u16, // +0x84
    pub cars_and_bikes_destroyed: u16, // +0x86
    pub boats_destroyed: u16,       // +0x88
    pub melee_kills: u16,           // +0x8A
    pub gang_kills: u16,            // +0x8C
    pub weapon_kills: [u16; 24],    // +0x8E (par eWeaponType)
    pub busted: u16,                // +0xBE (12 bits bas de compteur)
    // +0xC0: mot 64 bits (low: bits 0..31, high: bits 32..63)
    // wasted (0-11), unique commodities (12-19), missions failed (20-31),
    // fishes fed (32-43), missions passed (44-51), days passed (52-63)
    pub word_c0: [u32; 2],
    // +0xC8: times cheated (0-11), shipments robbed (12-19), times saved (20-31),
    // longest wheelie (32-43), warehouse raids (44-51), longest stoppie (52-63)
    pub word_c8: [u32; 2],
    // +0xD0: longest 2-wheel dist (0-11), most flips (12-19), farthest jump (20-31),
    // highest jump (32-43), most spin (44-51), explosions (52-63)
    pub word_d0: [u32; 2],
    pub word_d8: [u8; 3],           // +0xD8 (24 bits): weapons picked up (0-11), mission retries (12-23)
    pub pad_db: u8,                 // +0xDB
    pub trades_done: u32,           // +0xDC (20 bits bas)
    pub drugs_confiscated: u32,     // +0xE0
    pub radio_time: [u16; 12],      // +0xE4 (12 stations: 0 = off, 1..11 = stations)
}

impl Default for StatsData {
    fn default() -> Self {
        Self {
            criminal_rating: 0,
            total_playing_time: 0,
            unknown08: 0,
            unknown0c: 0,
            media_value: 0,
            distance: [0; 5],
            spent_on_weapons: 0,
            spent_on_pay_n_spray: 0,
            spent_on_property: 0,
            lost_on_street_races: 0,
            spent_on_bribing_cops: 0,
            spent_on_health_care: 0,
            made_from_activities: 0,
            made_from_taxi_tips: 0,
            found_on_the_street: 0,
            value_of_goods_sold: 0,
            value_of_goods_bought: 0,
            bullets_fired: 0,
            bullets_that_hit: 0,
            longest_time_in_vehicle: 0,
            made_from_trading: 0,
            spent_on_scratch_cards: 0,
            made_on_scratch_cards: 0,
            lions_collected: 0,
            cockroaches_squashed: 0,
            people_killed: 0,
            people_burnt_out: 0,
            near_misses: 0,
            vehicles_stolen: 0,
            wanted_stars_attained: 0,
            wanted_stars_evaded: 0,
            criminals_killed: 0,
            longest_police_chase: 0,
            longest_chase_with_6_stars: 0,
            units_of_goods_sold: 0,
            units_of_goods_bought: 0,
            cars_and_bikes_destroyed: 0,
            boats_destroyed: 0,
            melee_kills: 0,
            gang_kills: 0,
            weapon_kills: [0; 24],
            busted: 0,
            word_c0: [0; 2],
            word_c8: [0; 2],
            word_d0: [0; 2],
            word_d8: [0; 3],
            pad_db: 0,
            trades_done: 0,
            drugs_confiscated: 0,
            radio_time: [0; 12],
        }
    }
}

// Verification de la taille exacte en octets: exactement 0xFC (252 octets)
const _: () = assert!(std::mem::size_of::<StatsData>() == 0xFC);

impl StatsData {
    #[inline]
    pub fn get_word_c0(&self) -> u64 {
        (self.word_c0[0] as u64) | ((self.word_c0[1] as u64) << 32)
    }

    #[inline]
    pub fn set_word_c0(&mut self, val: u64) {
        self.word_c0[0] = val as u32;
        self.word_c0[1] = (val >> 32) as u32;
    }

    #[inline]
    pub fn get_word_c8(&self) -> u64 {
        (self.word_c8[0] as u64) | ((self.word_c8[1] as u64) << 32)
    }

    #[inline]
    pub fn set_word_c8(&mut self, val: u64) {
        self.word_c8[0] = val as u32;
        self.word_c8[1] = (val >> 32) as u32;
    }

    #[inline]
    pub fn get_word_d0(&self) -> u64 {
        (self.word_d0[0] as u64) | ((self.word_d0[1] as u64) << 32)
    }

    #[inline]
    pub fn set_word_d0(&mut self, val: u64) {
        self.word_d0[0] = val as u32;
        self.word_d0[1] = (val >> 32) as u32;
    }
}

/// Progression globale issue de la sauvegarde (SaveProgress)
#[derive(Debug, Clone)]
pub struct SaveProgress {
    pub ambient_medal: [u8; 0x55],    // 0: none, 1: bronze, 2: silver, 3: gold
    pub unique_jump: [bool; 30],      // USJCompleted
    pub package: [bool; 100],         // Cameras de securite detruites
    pub safehouse: [bool; 21],        // Planques achetees / possedees
    pub dealer: [bool; 0x51],         // 81 dealers trouves
    pub item: [bool; 6],              // 6 drogues decouvertes / possedees
    pub encounter: [u8; 8],           // 8 rencontres aleatoires (0x80 = non trouve)
    pub race_best_time: [u32; 16],    // 16 courses de rue (id 0x4C..)
    pub tattoo_best_score: u32,       // Meilleur score tatouage
}

impl Default for SaveProgress {
    fn default() -> Self {
        Self {
            ambient_medal: [0; 0x55],
            unique_jump: [false; 30],
            package: [false; 100],
            safehouse: [false; 21],
            dealer: [false; 0x51],
            item: [false; 6],
            encounter: [0; 8],
            race_best_time: [0; 16],
            tattoo_best_score: 0,
        }
    }
}

/// Utilitaires de manipulation de champs de bits
#[inline]
fn field(word: u64, shift: u32, width: u32) -> u64 {
    (word >> shift) & ((1u64 << width) - 1)
}

#[inline]
fn with_field(word: u64, shift: u32, width: u32, value: u64) -> u64 {
    let mask = ((1u64 << width) - 1) << shift;
    (word & !mask) | ((value << shift) & mask)
}

#[inline]
fn inc_field(word: &mut u64, shift: u32, width: u32) {
    *word = with_field(*word, shift, width, field(*word, shift, width) + 1);
}

/// Gestionnaire des statistiques et de la progression de jeu authentique PSP.
pub struct Stats {
    current_playing_time: u32,
    pub data: StatsData,
    hour_wanted: u32,
    pub enabled: bool,
    pub vehicle_frames: u32,
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}

impl Stats {
    pub fn new() -> Self {
        Self {
            current_playing_time: 0,
            data: StatsData::default(),
            hour_wanted: 0,
            enabled: true,
            vehicle_frames: 0,
        }
    }

    pub fn reset(&mut self) {
        self.current_playing_time = 0;
        self.data = StatsData::default();
        self.hour_wanted = 0;
        self.enabled = true;
        self.vehicle_frames = 0;
    }

    pub fn update_time(&mut self, minutes: i32) {
        if minutes > 0 {
            self.data.total_playing_time = self.data.total_playing_time.wrapping_add(minutes as u32);
            self.current_playing_time = self.current_playing_time.wrapping_add(minutes as u32);
        }
    }

    pub fn current_playing_time(&self) -> u32 {
        self.current_playing_time
    }

    pub fn add_total_playing_time(&mut self, minutes: u32) {
        self.data.total_playing_time = self.data.total_playing_time.wrapping_add(minutes);
    }

    pub fn add_current_playing_time(&mut self, minutes: u32) {
        self.current_playing_time = self.current_playing_time.wrapping_add(minutes);
    }

    pub fn add_distance_travelled_on_foot(&mut self, d: u32) {
        self.data.distance[0] = self.data.distance[0].wrapping_add(d);
    }

    pub fn add_distance_swam(&mut self, d: u32) {
        self.data.distance[1] = self.data.distance[1].wrapping_add(d);
    }

    pub fn add_distance_travelled_by_car(&mut self, d: u32) {
        self.data.distance[2] = self.data.distance[2].wrapping_add(d);
    }

    pub fn add_distance_travelled_by_bike(&mut self, d: u32) {
        self.data.distance[3] = self.data.distance[3].wrapping_add(d);
    }

    pub fn add_distance_travelled_by_boat(&mut self, d: u32) {
        self.data.distance[4] = self.data.distance[4].wrapping_add(d);
    }

    pub fn add_spent_on_weapons(&mut self, v: u32) { self.data.spent_on_weapons = self.data.spent_on_weapons.wrapping_add(v); }
    pub fn add_spent_on_pay_n_spray(&mut self, v: u32) { self.data.spent_on_pay_n_spray = self.data.spent_on_pay_n_spray.wrapping_add(v); }
    pub fn add_spent_on_property(&mut self, v: u32) { self.data.spent_on_property = self.data.spent_on_property.wrapping_add(v); }
    pub fn add_lost_on_street_races(&mut self, v: u32) { self.data.lost_on_street_races = self.data.lost_on_street_races.wrapping_add(v); }
    pub fn add_spent_on_bribing_cops(&mut self, v: u32) { self.data.spent_on_bribing_cops = self.data.spent_on_bribing_cops.wrapping_add(v); }
    pub fn add_spent_on_health_care(&mut self, v: u32) { self.data.spent_on_health_care = self.data.spent_on_health_care.wrapping_add(v); }
    pub fn add_made_from_activities(&mut self, v: u32) { self.data.made_from_activities = self.data.made_from_activities.wrapping_add(v); }
    pub fn add_made_from_taxi_tips(&mut self, v: u32) { self.data.made_from_taxi_tips = self.data.made_from_taxi_tips.wrapping_add(v); }
    pub fn add_found_on_the_street(&mut self, v: u32) { self.data.found_on_the_street = self.data.found_on_the_street.wrapping_add(v); }
    pub fn add_value_of_goods_sold(&mut self, v: u32) { self.data.value_of_goods_sold = self.data.value_of_goods_sold.wrapping_add(v); }
    pub fn add_value_of_goods_bought(&mut self, v: u32) { self.data.value_of_goods_bought = self.data.value_of_goods_bought.wrapping_add(v); }

    pub fn inc_bullets_fired(&mut self) { self.data.bullets_fired = self.data.bullets_fired.wrapping_add(1); }
    pub fn inc_bullets_that_hit(&mut self) { self.data.bullets_that_hit = self.data.bullets_that_hit.wrapping_add(1); }

    pub fn max_longest_time_spent_in_a_vehicle(&mut self, seconds: u32) {
        if self.data.longest_time_in_vehicle < seconds {
            self.data.longest_time_in_vehicle = seconds;
        }
    }

    pub fn inc_people_killed(&mut self) { self.data.people_killed = self.data.people_killed.wrapping_add(1); }
    pub fn inc_people_burnt_out(&mut self) { self.data.people_burnt_out = self.data.people_burnt_out.wrapping_add(1); }
    pub fn inc_near_misses(&mut self) { self.data.near_misses = self.data.near_misses.wrapping_add(1); }
    pub fn inc_lions_collected(&mut self) { self.data.lions_collected = self.data.lions_collected.wrapping_add(1); }
    pub fn inc_cockroaches_squashed(&mut self) { self.data.cockroaches_squashed = self.data.cockroaches_squashed.wrapping_add(1); }
    pub fn inc_vehicles_stolen(&mut self) { self.data.vehicles_stolen = self.data.vehicles_stolen.wrapping_add(1); }

    pub fn add_number_of_wanted_stars_attained(&mut self, n: u32) {
        self.data.wanted_stars_attained = self.data.wanted_stars_attained.wrapping_add(n as u16);
    }
    pub fn add_number_of_wanted_stars_evaded(&mut self, n: u32) {
        self.data.wanted_stars_evaded = self.data.wanted_stars_evaded.wrapping_add(n as u16);
    }
    pub fn inc_criminals_killed(&mut self) { self.data.criminals_killed = self.data.criminals_killed.wrapping_add(1); }

    pub fn max_longest_police_chase(&mut self, t: u32) {
        if (self.data.longest_police_chase as u32) < t {
            self.data.longest_police_chase = t as u16;
        }
    }

    pub fn max_longest_chase_with_a6_star_rating(&mut self, t: u32) {
        if (self.data.longest_chase_with_6_stars as u32) < t {
            self.data.longest_chase_with_6_stars = t as u16;
        }
    }

    pub fn add_units_of_goods_sold(&mut self, n: u32) {
        self.data.units_of_goods_sold = self.data.units_of_goods_sold.wrapping_add(n as u16);
    }
    pub fn add_units_of_goods_bought(&mut self, n: u32) {
        self.data.units_of_goods_bought = self.data.units_of_goods_bought.wrapping_add(n as u16);
    }

    pub fn inc_number_of_cars_and_bikes_destroyed(&mut self) {
        self.data.cars_and_bikes_destroyed = self.data.cars_and_bikes_destroyed.wrapping_add(1);
    }
    pub fn inc_number_of_boats_destroyed(&mut self) {
        self.data.boats_destroyed = self.data.boats_destroyed.wrapping_add(1);
    }

    pub fn inc_melee_kills(&mut self) { self.data.melee_kills = self.data.melee_kills.wrapping_add(1); }
    pub fn inc_gang_kills(&mut self) { self.data.gang_kills = self.data.gang_kills.wrapping_add(1); }

    pub fn inc_weapon_kills(&mut self, weapon_type: usize) {
        if weapon_type < self.data.weapon_kills.len() {
            self.data.weapon_kills[weapon_type] = self.data.weapon_kills[weapon_type].wrapping_add(1);
        }
    }

    pub fn inc_number_of_times_busted(&mut self) {
        self.data.busted = (self.data.busted & 0xF000) | ((self.data.busted.wrapping_add(1)) & 0x0FFF);
    }

    pub fn inc_number_of_times_wasted(&mut self) {
        let mut w = self.data.get_word_c0();
        inc_field(&mut w, 0, 12);
        self.data.set_word_c0(w);
    }

    pub fn inc_unique_commodities_owned(&mut self) {
        let mut w = self.data.get_word_c0();
        inc_field(&mut w, 12, 8);
        self.data.set_word_c0(w);
    }

    pub fn inc_missions_failed(&mut self) {
        let mut w = self.data.get_word_c0();
        inc_field(&mut w, 20, 12);
        self.data.set_word_c0(w);
    }

    pub fn inc_fishes_fed(&mut self) {
        let mut w = self.data.get_word_c0();
        inc_field(&mut w, 32, 12);
        self.data.set_word_c0(w);
    }

    pub fn inc_missions_passed(&mut self) {
        let mut w = self.data.get_word_c0();
        inc_field(&mut w, 44, 8);
        self.data.set_word_c0(w);
    }

    pub fn inc_days_passed_in_game(&mut self) {
        let mut w = self.data.get_word_c0();
        inc_field(&mut w, 52, 12);
        self.data.set_word_c0(w);
    }

    pub fn inc_times_cheated(&mut self) {
        let mut w = self.data.get_word_c8();
        inc_field(&mut w, 0, 12);
        self.data.set_word_c8(w);
    }

    pub fn inc_shipments_robbed(&mut self) {
        let mut w = self.data.get_word_c8();
        inc_field(&mut w, 12, 8);
        self.data.set_word_c8(w);
    }

    pub fn inc_number_of_times_saved(&mut self) {
        let mut w = self.data.get_word_c8();
        inc_field(&mut w, 20, 12);
        self.data.set_word_c8(w);
    }

    pub fn inc_warehouse_raids_pulled_off(&mut self) {
        let mut w = self.data.get_word_c8();
        inc_field(&mut w, 44, 8);
        self.data.set_word_c8(w);
    }

    pub fn warehouse_raids_pulled_off(&self) -> u32 {
        field(self.data.get_word_c8(), 44, 8) as u32
    }

    pub fn inc_explosions(&mut self) {
        let mut w = self.data.get_word_d0();
        inc_field(&mut w, 52, 12);
        self.data.set_word_d0(w);
    }

    pub fn inc_trades_done(&mut self) {
        self.data.trades_done = (self.data.trades_done & 0xFFF00000) | ((self.data.trades_done.wrapping_add(1)) & 0x000FFFFF);
    }

    pub fn add_made_from_trading(&mut self, v: i32) { self.data.made_from_trading = self.data.made_from_trading.wrapping_add(v); }
    pub fn add_spent_on_scratch_cards(&mut self, v: i32) { self.data.spent_on_scratch_cards = self.data.spent_on_scratch_cards.wrapping_add(v); }
    pub fn add_made_on_scratch_cards(&mut self, v: i32) { self.data.made_on_scratch_cards = self.data.made_on_scratch_cards.wrapping_add(v); }

    // Stunts records
    pub fn max_longest_wheelie_distance(&mut self, mut d: u32) {
        if d > 0xFFE { d = 0xFFF; }
        let mut w = self.data.get_word_c8();
        if (field(w, 32, 12) as u32) < d {
            w = with_field(w, 32, 12, d as u64);
            self.data.set_word_c8(w);
        }
    }

    pub fn max_longest_stoppie_distance(&mut self, d: u32) {
        let val = (d & 0xFFF) as u64;
        let mut w = self.data.get_word_c8();
        if field(w, 52, 12) < val {
            w = with_field(w, 52, 12, val);
            self.data.set_word_c8(w);
        }
    }

    pub fn max_longest_2wheel_distance(&mut self, d: u32) {
        let val = (d & 0xFFF) as u64;
        let mut w = self.data.get_word_d0();
        if field(w, 0, 12) < val {
            w = with_field(w, 0, 12, val);
            self.data.set_word_d0(w);
        }
    }

    pub fn max_most_flips_in_one_jump(&mut self, n: u32) {
        let val = (n & 0xFF) as u64;
        let mut w = self.data.get_word_d0();
        if field(w, 12, 8) < val {
            w = with_field(w, 12, 8, val);
            self.data.set_word_d0(w);
        }
    }

    pub fn max_farthest_jump_distance(&mut self, d: u32) {
        let val = (d & 0xFFF) as u64;
        let mut w = self.data.get_word_d0();
        if field(w, 20, 12) < val {
            w = with_field(w, 20, 12, val);
            self.data.set_word_d0(w);
        }
    }

    pub fn max_highest_jump_reached(&mut self, h: u32) {
        let val = (h & 0xFFF) as u64;
        let mut w = self.data.get_word_d0();
        if field(w, 32, 12) < val {
            w = with_field(w, 32, 12, val);
            self.data.set_word_d0(w);
        }
    }

    pub fn max_most_spin_in_one_jump(&mut self, n: u32) {
        let val = (n & 0xFF) as u64;
        let mut w = self.data.get_word_d0();
        if field(w, 44, 8) < val {
            w = with_field(w, 44, 8, val);
            self.data.set_word_d0(w);
        }
    }

    fn word_d8(&self) -> u32 {
        self.data.word_d8[0] as u32
            | ((self.data.word_d8[1] as u32) << 8)
            | ((self.data.word_d8[2] as u32) << 16)
    }

    fn set_word_d8(&mut self, v: u32) {
        self.data.word_d8[0] = (v & 0xFF) as u8;
        self.data.word_d8[1] = ((v >> 8) & 0xFF) as u8;
        self.data.word_d8[2] = ((v >> 16) & 0xFF) as u8;
    }

    pub fn inc_weapons_picked_up(&mut self) {
        let w = self.word_d8();
        self.set_word_d8((w & 0xFFF000) | ((w.wrapping_add(1)) & 0x000FFF));
    }

    pub fn inc_mission_retries(&mut self) {
        let w = self.word_d8();
        self.set_word_d8(w.wrapping_add(0x1000) & 0xFFFFFF);
    }

    pub fn add_radio_time(&mut self, station: usize, t: u32) {
        if station >= 12 { return; }
        if (self.data.radio_time[station] as u64) + (t as u64) > 0xFFFFFFFF {
            for r in &mut self.data.radio_time {
                *r >>= 1;
            }
        }
        self.data.radio_time[station] = self.data.radio_time[station].wrapping_add(t as u16);
    }

    pub fn force_radio_time(&mut self, station: usize, t: u16) {
        if station < 12 {
            self.data.radio_time[station] = t;
        }
    }

    pub fn process_distance(&mut self, vel: &[i32; 3], vehicle_type: i32, swimming: bool) {
        let slot = if vehicle_type < 0 {
            if swimming { 1 } else { 0 }
        } else if vehicle_type <= 2 {
            (2 + vehicle_type) as usize
        } else {
            return;
        };

        let s = (vel[0] as i64) * (vel[0] as i64)
            + (vel[1] as i64) * (vel[1] as i64)
            + (vel[2] as i64) * (vel[2] as i64);
        let len = (s as f64).sqrt() as u32;
        // |v| x 0x880 (17/512 d'une seconde de mouvement, x 16): distance cette frame en 1/16 d'unites
        let delta = ((((len as i64) * 0x880) >> 12) >> 12) as u32;
        self.data.distance[slot] = self.data.distance[slot].wrapping_add(delta);
    }

    pub fn process_wanted_change(&mut self, level: u32, previous: u32) {
        if !self.enabled { return; }
        if level < previous {
            self.data.wanted_stars_evaded = self.data.wanted_stars_evaded.wrapping_add((previous - level) as u16);
        } else if level != previous {
            self.data.wanted_stars_attained = self.data.wanted_stars_attained.wrapping_add((level - previous) as u16);
        }
        if previous < level {
            self.hour_wanted = level;
        }
    }

    pub fn process_end_of_hour(&mut self, wanted_level: u32) {
        if self.data.media_value >= 0xFA { return; }
        let peak = if self.hour_wanted > wanted_level { self.hour_wanted } else { wanted_level };
        self.data.media_value = self.data.media_value.saturating_add(peak);
        self.hour_wanted = 0;
    }

    pub fn calc_criminal_rating(&mut self) -> i32 {
        let fired = self.data.bullets_fired;
        let accuracy = if fired > 0 {
            ((self.data.bullets_that_hit / fired) << 12) as i32
        } else {
            0
        };

        let mut cr = self.data.people_killed as i32;
        cr += (self.data.boats_destroyed as i32 + self.data.cars_and_bikes_destroyed as i32) * 30;
        cr += self.data.made_from_trading;
        if fired > 100 {
            cr += (accuracy >> 12) * 500;
        }
        cr -= (self.times_wasted() as i32) * 3;
        cr -= (self.times_busted() as i32) * 3;
        cr -= (self.times_cheated() as i32) * 1000;

        if self.times_cheated() == 0 {
            if cr < 0 { cr = 0; }
        } else if cr < -6500 {
            cr = -6500;
        }
        if cr > 10_000_000 {
            cr = 10_000_000;
        }

        self.data.criminal_rating = cr;

        if cr < -6000 { return 0x617; }
        if cr < -4000 { return 0x616; }
        if cr < -2000 { return 0x615; }
        if cr < -500 { return 0x614; }
        if cr < 0 { return 0x613; }

        let u = cr as u32;
        if (1150..1300).contains(&u) {
            return if self.times_busted() > 0 { 0x5EA } else { 0x5E9 };
        }

        const RANKS: &[(u32, i32)] = &[
            (20, 0x5D2), (50, 0x5D3), (75, 0x5D4), (100, 0x5D5), (120, 0x5D6), (150, 0x5D7), (200, 0x5D8), (240, 0x5D9),
            (270, 0x5DA), (300, 0x5DB), (335, 0x5DC), (370, 0x5DD), (400, 0x5DE), (450, 0x5DF), (500, 0x5E0),
            (550, 0x5E1), (600, 0x5E2), (610, 0x5E3), (650, 0x5E4), (700, 0x5E5), (850, 0x5E6), (1000, 0x5E7),
            (1005, 0x5E8), (1150, 0x5E9), (1300, 0), (1500, 0x5EB), (1700, 0x5EC), (2000, 0x5ED), (2100, 0x5EE),
            (2300, 0x5EF), (2500, 0x5F0), (2750, 0x5F1), (3000, 0x5F2), (3500, 0x5F3), (4000, 0x5F4), (5000, 0x5F5),
            (7500, 0x5F6), (10000, 0x5F7), (20000, 0x5F8), (30000, 0x5F9), (40000, 0x5FA), (50000, 0x5FB),
            (65000, 0x5FC), (80000, 0x5FD), (100000, 0x5FE), (150000, 0x5FF), (200000, 0x600), (300000, 0x601),
            (375000, 0x602), (500000, 0x603)
        ];

        for &(below, text) in RANKS {
            if u < below {
                return text;
            }
        }

        if self.data.unknown0c != 0 { 0x605 } else { 0x604 }
    }

    pub fn calc_gold_medals(s: &SaveProgress) -> i32 { s.ambient_medal.iter().filter(|&&m| m == 3).count() as i32 }
    pub fn calc_silver_medals(s: &SaveProgress) -> i32 { s.ambient_medal.iter().filter(|&&m| m == 2).count() as i32 }
    pub fn calc_bronze_medals(s: &SaveProgress) -> i32 { s.ambient_medal.iter().filter(|&&m| m == 1).count() as i32 }

    pub fn calc_total_gang_members_wasted(&self) -> u16 { self.data.gang_kills }

    pub fn calc_total_distance_travelled(&self) -> i32 {
        self.data.distance.iter().copied().sum::<u32>() as i32
    }

    pub fn calc_usjs_completed(s: &SaveProgress) -> i32 { s.unique_jump.iter().filter(|&&v| v).count() as i32 }
    pub fn calc_cams_destroyed(s: &SaveProgress) -> i32 { s.package.iter().filter(|&&v| v).count() as i32 }
    pub fn calc_safehouses_owned(s: &SaveProgress) -> i32 { s.safehouse.iter().filter(|&&v| v).count() as i32 }
    pub fn calculate_num_dealers_found(s: &SaveProgress) -> i32 { s.dealer.iter().filter(|&&v| v).count() as i32 }
    pub fn calculate_unique_commodities_owned(s: &SaveProgress) -> i32 { s.item.iter().filter(|&&v| v).count() as i32 }

    pub fn calc_random_encounters_found(s: &SaveProgress) -> i32 {
        s.encounter.iter().filter(|&&e| e != 0x80).count() as i32
    }

    pub fn calc_most_fav_radio_station(&self) -> i32 {
        let mut best = 0usize;
        let mut most = self.data.radio_time[0];
        for i in 1..12 {
            if self.data.radio_time[i] > most {
                most = self.data.radio_time[i];
                best = i;
            }
        }
        if best > 0 { STATION_TEXT_IDS[best - 1] } else { 0xE0 }
    }

    pub fn calc_least_fav_radio_station(&self) -> i32 {
        let mut best = 0usize;
        let mut least = if self.data.radio_time[0] > 0xFF { 0x100 } else { self.data.radio_time[0] };
        for i in 1..12 {
            if self.data.radio_time[i] < least {
                least = self.data.radio_time[i];
                best = i;
            }
        }
        if best > 0 { STATION_TEXT_IDS[best - 1] } else { 0xE0 }
    }

    pub fn calc_average_value_of_goods_sold(&self) -> u32 {
        calc_average(self.data.value_of_goods_sold, self.data.units_of_goods_sold)
    }

    pub fn calc_average_value_of_goods_bought(&self) -> u32 {
        calc_average(self.data.value_of_goods_bought, self.data.units_of_goods_bought)
    }

    pub fn calc_accuracy(&self) -> i32 {
        let fired = self.data.bullets_fired;
        let a = if fired > 0 {
            (((self.data.bullets_that_hit * 100) / fired) << 12) as i32
        } else {
            0
        };
        if a > 0x63FFF { 0x64000 } else { a }
    }

    pub fn calc_favourite_weapon(&self) -> usize {
        let mut best = 0usize;
        let mut most = 0u16;
        for i in 0..24 {
            let k = self.data.weapon_kills[i];
            if k > most {
                best = i;
            }
            if k >= most {
                most = k;
            }
        }
        best
    }

    pub fn calc_media_value(&self, actual_level: i32) -> i32 {
        let v = self.data.media_value;
        const LOW: [u32; 10] = [3, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        for (i, &limit) in LOW.iter().enumerate() {
            if v < limit {
                return 0x553 + (i as i32);
            }
        }
        if actual_level < 5 { return 0x55C; }
        if v < 110 { return 0x55D; }
        if v < 120 { return 0x55E; }
        if v < 130 { return 0x55F; }
        if v < 140 { return 0x560; }
        if v < 150 { return 0x561; }
        if v < 160 || actual_level == 5 { return 0x562; }
        if v < 170 { return 0x563; }
        if v < 180 { return 0x564; }
        if v < 190 { return 0x565; }
        if v >= 200 { 0x567 } else { 0x566 }
    }

    pub fn log_race_time(s: &mut SaveProgress, race: usize, time: u32) {
        if race < 0x4C { return; }
        let idx = race - 0x4C;
        if idx >= 16 { return; }
        if s.race_best_time[idx] > 0 && s.race_best_time[idx] <= time {
            return;
        }
        s.race_best_time[idx] = time;
    }

    pub fn best_race_time(s: &SaveProgress, race: usize) -> u32 {
        if race < 0x4C { return 0; }
        let idx = race - 0x4C;
        if idx < 16 { s.race_best_time[idx] } else { 0 }
    }

    pub fn total_value_drugs_confiscated(&self) -> u32 { self.data.drugs_confiscated }
    pub fn add_total_value_drugs_confiscated(&mut self, v: u32) { self.data.drugs_confiscated = self.data.drugs_confiscated.wrapping_add(v); }
    pub fn lions_collected(&self) -> u16 { self.data.lions_collected }
    pub fn cockroaches_squashed(&self) -> u16 { self.data.cockroaches_squashed }
    pub fn days_passed(&self) -> u32 { (self.data.get_word_c0() >> 52) as u32 }

    // Getters bit-packes
    pub fn times_busted(&self) -> u32 { (self.data.busted & 0xFFF) as u32 }
    pub fn times_wasted(&self) -> u32 { (self.data.get_word_c0() & 0xFFF) as u32 }
    pub fn missions_failed(&self) -> u32 { ((self.data.get_word_c0() >> 20) & 0xFFF) as u32 }
    pub fn fishes_fed(&self) -> u32 { ((self.data.get_word_c0() >> 32) & 0xFFF) as u32 }
    pub fn missions_passed(&self) -> u32 { ((self.data.get_word_c0() >> 44) & 0xFF) as u32 }
    pub fn times_cheated(&self) -> u32 { (self.data.get_word_c8() & 0xFFF) as u32 }
    pub fn times_saved(&self) -> u32 { ((self.data.get_word_c8() >> 20) & 0xFFF) as u32 }
    pub fn explosions(&self) -> u32 { ((self.data.get_word_d0() >> 52) & 0xFFF) as u32 }
    pub fn weapons_picked_up(&self) -> u32 { self.word_d8() & 0xFFF }
    pub fn mission_retries(&self) -> u32 { self.word_d8() >> 12 }
}

#[inline]
fn calc_average(value: u32, units: u16) -> u32 {
    let v = value.wrapping_mul(0x1000);
    if (v as i32) < 1 || units == 0 {
        return 0;
    }
    ((((v as u64) << 32) / ((units as u64) << 12)) >> 32) as u32 & 0xFFFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_packed_counters_wrapping_and_stunt_records() {
        let mut s = Stats::new();
        s.inc_missions_failed();
        for _ in 0..4097 {
            s.inc_number_of_times_wasted();
        }
        assert_eq!(s.times_wasted(), 1, "wasted wraps at 12 bits without touching its neighbours");
        assert_eq!(s.missions_failed(), 1);

        for _ in 0..256 {
            s.inc_missions_passed();
        }
        s.inc_days_passed_in_game();
        assert_eq!(s.missions_passed(), 0, "missions passed wraps at 8 bits");
        assert_eq!(s.days_passed(), 1);

        s.inc_weapons_picked_up();
        for _ in 0..4096 {
            s.inc_mission_retries();
        }
        assert_eq!(s.mission_retries(), 0, "the 24-bit word keeps its two counters apart");
        assert_eq!(s.weapons_picked_up(), 1);

        s.inc_explosions();
        assert_eq!(s.explosions(), 1, "explosions count in the top bits of +0xD0");

        s.max_longest_wheelie_distance(5000);
        assert_eq!((s.data.get_word_c8() >> 32) & 0xFFF, 0xFFF, "a wheelie record saturates at 0xFFF");

        s.max_longest_stoppie_distance(0x1234);
        assert_eq!(s.data.get_word_c8() >> 52, 0x234, "a stoppie record keeps only its 12 bits");

        s.max_most_flips_in_one_jump(3);
        s.max_most_flips_in_one_jump(2);
        assert_eq!((s.data.get_word_d0() >> 12) & 0xFF, 3, "records keep the larger value");
    }

    #[test]
    fn criminal_rating_calculation_parity() {
        let mut r = Stats::new();
        r.data.people_killed = 10;
        r.data.cars_and_bikes_destroyed = 2;
        assert_eq!(r.calc_criminal_rating(), 0x5D4);
        assert_eq!(r.data.criminal_rating, 70, "10 kills and 2 cars rate 70");

        r.data.people_killed = 1200;
        r.data.cars_and_bikes_destroyed = 0;
        assert_eq!(r.calc_criminal_rating(), 0x5E9, "1200 without arrests");

        r.inc_number_of_times_busted();
        assert_eq!(r.calc_criminal_rating(), 0x5EA);
        assert_eq!(r.data.criminal_rating, 1197, "an arrest costs 3 and changes the rank");

        let mut a = Stats::new();
        a.data.bullets_fired = 101;
        a.data.bullets_that_hit = 101;
        assert_eq!(a.calc_criminal_rating(), 0x5E0 + 1);
        assert_eq!(a.data.criminal_rating, 500, "perfect aim over 100 shots adds 500");

        let mut c = Stats::new();
        c.inc_times_cheated();
        assert_eq!(c.calc_criminal_rating(), 0x614);
        assert_eq!(c.data.criminal_rating, -1000, "a cheat costs 1000");

        let mut n = Stats::new();
        n.inc_number_of_times_wasted();
        assert_eq!(n.calc_criminal_rating(), 0x5D2);
        assert_eq!(n.data.criminal_rating, 0, "without cheats the rating stays at 0");

        let mut top = Stats::new();
        top.data.people_killed = 0xFFFF;
        top.data.made_from_trading = 600000;
        assert_eq!(top.calc_criminal_rating(), 0x604, "the top rank");
        top.data.unknown0c = 1;
        assert_eq!(top.calc_criminal_rating(), 0x605, "the top rank with +0xC set");
    }

    #[test]
    fn media_value_and_radio_stats() {
        let mut m = Stats::new();
        assert_eq!(m.calc_media_value(0), 0x553, "no wanted hours");
        m.data.media_value = 105;
        assert_eq!(m.calc_media_value(4), 0x55C);
        assert_eq!(m.calc_media_value(5), 0x55D, "past 100 the wanted level gates the rank");
        m.data.media_value = 165;
        assert_eq!(m.calc_media_value(5), 0x562);
        assert_eq!(m.calc_media_value(6), 0x563, "level 5 stops at 0x562");
        m.data.media_value = 200;
        assert_eq!(m.calc_media_value(6), 0x567, "the top media rank");

        let mut r = Stats::new();
        assert_eq!(r.calc_most_fav_radio_station(), 0xE0, "no radio time: slot 0");
        r.data.radio_time[3] = 5;
        assert_eq!(r.calc_most_fav_radio_station(), 227, "slot 3 is the third station");

        for i in 1..12 {
            r.data.radio_time[i] = 10;
        }
        r.data.radio_time[0] = 300;
        r.data.radio_time[5] = 2;
        assert_eq!(r.calc_least_fav_radio_station(), 231, "the least played slot");
        r.add_radio_time(2, 7);
        assert_eq!(r.data.radio_time[2], 17, "radio time adds up");
    }

    #[test]
    fn distances_wanted_averages_and_race_times() {
        let mut d = Stats::new();
        let vel = [0x3000, 0x4000, 0];
        d.process_distance(&vel, 0, false);
        d.process_distance(&vel, 3, false);
        d.process_distance(&vel, -1, true);
        assert_eq!(d.data.distance[2], 2);
        assert_eq!(d.data.distance[1], 2);
        assert_eq!(d.calc_total_distance_travelled(), 4);

        d.process_wanted_change(2, 0);
        d.process_wanted_change(0, 2);
        assert_eq!(d.data.wanted_stars_attained, 2);
        assert_eq!(d.data.wanted_stars_evaded, 2);

        d.process_end_of_hour(1);
        d.process_end_of_hour(1);
        assert_eq!(d.data.media_value, 3);

        d.update_time(5);
        assert_eq!(d.data.total_playing_time, 5);
        assert_eq!(d.current_playing_time(), 5);

        let mut t = Stats::new();
        t.add_value_of_goods_sold(1000);
        t.add_units_of_goods_sold(3);
        assert_eq!(t.calc_average_value_of_goods_sold(), 333);
        assert_eq!(t.calc_average_value_of_goods_bought(), 0);

        t.data.bullets_fired = 4;
        t.data.bullets_that_hit = 3;
        assert_eq!(t.calc_accuracy(), 75 << 12);

        t.inc_weapon_kills(9);
        for _ in 0..3 {
            t.inc_weapon_kills(5);
            t.inc_weapon_kills(9);
        }
        t.data.weapon_kills[9] = 3;
        assert_eq!(t.calc_favourite_weapon(), 5, "ties keep the first weapon");

        let mut p = SaveProgress::default();
        Stats::log_race_time(&mut p, 0x4C, 100);
        Stats::log_race_time(&mut p, 0x4C, 120);
        assert_eq!(Stats::best_race_time(&p, 0x4C), 100, "slower race time is not logged");
        Stats::log_race_time(&mut p, 0x4C, 90);
        assert_eq!(Stats::best_race_time(&p, 0x4C), 90, "faster race time is logged");

        p.ambient_medal[0] = 3;
        p.ambient_medal[1] = 3;
        p.ambient_medal[2] = 1;
        p.encounter[0] = 0x80;
        p.encounter[1] = 0x80;
        assert_eq!(Stats::calc_gold_medals(&p), 2);
        assert_eq!(Stats::calc_bronze_medals(&p), 1);
        assert_eq!(Stats::calc_silver_medals(&p), 0);
        assert_eq!(Stats::calc_random_encounters_found(&p), 6);
    }
}
