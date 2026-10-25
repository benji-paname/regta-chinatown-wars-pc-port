// SPDX-License-Identifier: GPL-3.0-only
//! PSP vehicle table (PAK 29, 106 entries x 312 bytes), paint swatches (PAK 1 -> 2..28)
//! and 3D vehicle models (PAK 30..198). Dimensions and handling follow ctw-native
//! cVehicle / cWheeledVehicle / cBike::SetPropertiesFromVehicleInfo (C) 2026 Kryo.to.

#![allow(dead_code)]

use crate::{
    collision::{BoxShape, Collision},
    iso::{bytes, u16le, u32le, Disc, Result},
    model::{self, Mesh},
    pak::Pak,
    pavement::Rng,
    texture::{self, Texture},
};
use std::collections::HashMap;

/// cBike::VehicleScale() in vehicle.h (0x14CC / 4096.0 = 1.2998047).
pub const BIKE_SCALE_Q12: i64 = 0x14cc;
pub const BIKE_SCALE: f32 = BIKE_SCALE_Q12 as f32 / 4096.;

#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub info_id: usize,
    pub kind: u16,
    pub model_id: u16,
    pub colour_mask: u32,
    pub raw_dims: [i32; 3],
    pub top_speed: u16,
    pub num_gears: u8,
    pub mass_q12: i32,
    pub max_torque_q12: i32,
    pub brake_force_q12: i32,
    pub drive_split_q12: i16,
    pub grip_q12: i16,
    pub front_axle_q12: i16,
    pub rear_axle_q12: i16,
    pub front_k18_q12: i32,
    pub rear_k18_q12: i32,
    pub steer_lock_q12: i32,
    pub steer_rate_q12: i32,
    pub steer_red_q12: i32,
    pub sway_damping_q12: i32,
    pub seat_y_q12: i32,
    pub flags8e: u16,
    pub name: &'static str,
}
impl Spec {
    pub fn half_extents(&self) -> [f32; 3] {
        let mut hx = ((self.raw_dims[0] * 0x2332 >> 12) >> 1) as i64;
        let mut hy = ((self.raw_dims[1] * 0x2332 >> 12) >> 1) as i64;
        let mut hz = self.raw_dims[2] as i64;
        if self.is_bike() {
            hx = (hx * BIKE_SCALE_Q12) >> 12;
            hy = (hy * BIKE_SCALE_Q12) >> 12;
            hz = (hz * BIKE_SCALE_Q12) >> 12;
        }
        [hx as f32 / 4096., hy as f32 / 4096., hz as f32 / 4096.]
    }
    /// Land vehicles: standard cars/trucks (0), motorcycles (1), and Rhino tank (5).
    /// Excludes boats (2), jetskis (3), and helicopters (4).
    pub fn is_land_vehicle(&self) -> bool {
        matches!(self.kind, 0 | 1 | 5)
    }
    pub fn is_bike(&self) -> bool {
        self.kind == 1
    }
    pub fn is_boat(&self) -> bool {
        self.kind == 2
    }
    pub fn is_jetski(&self) -> bool {
        self.kind == 3
    }
    pub fn is_watercraft(&self) -> bool {
        matches!(self.kind, 2 | 3)
    }
    pub fn is_emergency(&self) -> bool {
        matches!(self.name, "POLICE" | "AMBULANCE" | "FIRETRUCK" | "SWAT" | "FBIRANCHER")
    }
    pub fn mass(&self) -> f32 {
        (self.mass_q12 as f32 / 4096.).max(0.4)
    }
    pub fn max_torque(&self) -> f32 {
        (self.max_torque_q12 as f32 / 4096.).max(10.)
    }
    pub fn brake_force(&self) -> f32 {
        (self.brake_force_q12 as f32 / 4096.).max(50.)
    }
    pub fn drive_split(&self) -> f32 {
        (self.drive_split_q12 as f32 / 4096.).clamp(0., 1.)
    }
    pub fn grip(&self) -> f32 {
        (self.grip_q12 as f32 / 4096.).max(0.5)
    }
    pub fn wheelbase(&self) -> f32 {
        ((self.front_axle_q12 as i32 - self.rear_axle_q12 as i32) as f32 / 4096.).max(1.8)
    }
    pub fn front_stiffness(&self) -> f32 {
        (self.front_k18_q12 as f32 / 4096.).max(1.5)
    }
    pub fn rear_stiffness(&self) -> f32 {
        (self.rear_k18_q12 as f32 / 4096.).max(1.5)
    }
    pub fn steer_lock_deg(&self) -> f32 {
        (self.steer_lock_q12 as f32 / 4096.).clamp(20., 95.)
    }
    pub fn steer_rate_deg(&self) -> f32 {
        (self.steer_rate_q12 as f32 / 4096.).clamp(15., 180.)
    }
    pub fn steer_speed_red(&self) -> f32 {
        (self.steer_red_q12 as f32 / 4096.).clamp(0.02, 0.5)
    }
    pub fn sway_damping(&self) -> f32 {
        (self.sway_damping_q12 as f32 / 4096.).clamp(0.1, 0.95)
    }
    pub fn seat_y(&self) -> f32 {
        self.seat_y_q12 as f32 / 4096.
    }
    pub fn visual_scale(&self) -> f32 {
        if self.is_bike() {
            BIKE_SCALE
        } else {
            1.0
        }
    }
    pub fn sound_type(&self) -> usize {
        if self.is_bike() {
            12
        } else if self.kind == 4 {
            3
        } else if self.kind == 5 {
            7
        } else {
            self.info_id % 20
        }
    }
}

pub const BIKE_POOL: &[usize] = &[12, 89, 91, 93, 97, 98];

#[allow(dead_code)]
pub const TRAFFIC_POOL: &[usize] = &[
    0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 27, 33, 34,
    40, 51, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 89, 93, 98,
];

/// Authentic vehicle lists from PSP popinfo.bin (13 lists, excluding watercraft).
pub const VEH_LISTS: &[&[usize]] = &[
    &[92],                                  // 0: Cabby
    &[10, 19, 54, 55, 56, 52],              // 1: Commercial / Trucks (Flatbed, Yankee, Yankee variants)
    &[12, 93, 91, 89, 98],                  // 2: Bikes (Angel, Sanchez, PCJ600, NRG900, Chopper)
    &[20, 16, 17, 57],                      // 3: Sports (Coquette, Banshee, Comet, Infernus)
    &[],                                    // 4: Boats (excluded from land street traffic)
    &[66],                                  // 5: Limo
    &[0, 5, 14, 59, 62],                    // 6: Common A (Admiral, Buccaneer, Hellenbach, Sabre GT, Peyote)
    &[22, 2, 1, 64, 60, 63],                // 7: Common B (Chavos, Dukes, Blista, Perren, Stallion, Ingot)
    &[],                                    // 8: Streetracers (special missions)
    &[21, 24, 23, 61, 58],                  // 9: Luxury / SUVs (Cognoscenti, EXT, Cavalcade, Sultan, Patriot)
    &[13],                                  // 10: Taxi (Cabbie)
    &[9, 46, 47, 48, 49, 50, 25],           // 11: Vans / Pickups (Burrito & commercial liveries, Bobcat)
];

/// Authentic zone vehicle makeups from PSP popinfo.bin / population_tables.bin (CTWZONE1).
/// Each entry is a list of (veh_list_index, weight).
const ZONE_MAKEUPS: &[&[(usize, u32)]] = &[
    // 0: Default
    &[(3, 40), (6, 50), (1, 15), (9, 50), (10, 150), (11, 30), (5, 5), (2, 10)],
    // 1: Bohan / North Industrial
    &[(3, 30), (6, 50), (1, 30), (7, 40), (9, 5), (10, 100), (11, 50), (2, 10)],
    // 2: Downtown Algonquin / L'Exchange / Chinatown (high taxis, sedans, luxury, bikes)
    &[(3, 40), (6, 50), (1, 15), (9, 50), (10, 150), (11, 30), (5, 5), (2, 10)],
    // 3: Dukes / Residential East
    &[(3, 5), (6, 50), (1, 15), (7, 40), (9, 5), (10, 20), (11, 30), (2, 10)],
    // 4: Industrial South / Docks / Meadows
    &[(6, 25), (1, 100), (7, 20), (10, 1), (11, 80), (2, 8)],
    // 5: General Commercial
    &[(3, 40), (6, 50), (1, 15), (9, 50), (10, 150), (11, 30), (5, 5), (2, 10)],
    // 6: Suburbs / Alderney
    &[(6, 90), (1, 50), (7, 90), (10, 7), (11, 50), (2, 7)],
    // 7: Castle Gardens / Financial / Midtown
    &[(3, 50), (6, 50), (1, 10), (7, 10), (9, 50), (10, 20), (11, 40), (2, 10)],
];

/// Choose a vehicle for ambient traffic following PSP popinfo.bin zone budgets and 5% police chance.
pub fn choose_traffic_vehicle(pos: [f32; 3], rng: &mut Rng) -> usize {
    // 5% ambient patrol police car (PopulationProfile pedType 10 weight 5 / ~100)
    if rng.below(100) < 5 {
        return 18; // POLICE
    }
    let m_idx = if pos[1] > 2000. {
        1 // Bohan
    } else if pos[1] < 800. && pos[0] < 200. {
        7 // Castle Gardens / Financial
    } else if pos[0] > 200. {
        if pos[1] < 800. {
            4 // Docks / Meadows
        } else {
            3 // Dukes
        }
    } else if pos[0] < -700. {
        6 // Alderney / Suburbs
    } else {
        2 // Downtown Algonquin / L'Exchange
    };
    let makeup = ZONE_MAKEUPS[m_idx];
    let total_weight: u32 = makeup.iter().map(|(_, w)| *w).sum();
    if total_weight == 0 {
        return 0; // Admiral fallback
    }
    let mut roll = rng.below(total_weight);
    let mut chosen_list = 6;
    for &(list_id, weight) in makeup {
        if roll < weight {
            chosen_list = list_id;
            break;
        }
        roll -= weight;
    }
    let vids = VEH_LISTS.get(chosen_list).copied().unwrap_or(&[]);
    if vids.is_empty() {
        return 0;
    }
    vids[rng.below(vids.len() as u32) as usize]
}

pub fn spec(info_id: usize) -> &'static Spec {
    &SPECS[info_id.min(SPECS.len() - 1)]
}

pub fn choose_paint(info_id: usize, colour: u8, rng: &mut Rng) -> u8 {
    if colour < 25 {
        return colour;
    }
    let mask = spec(info_id).colour_mask;
    if mask == 0 {
        return 0;
    }
    let mut allowed = [0u8; 27];
    let mut len = 0;
    for c in 0..27u8 {
        if (mask >> c) & 1 != 0 {
            allowed[len] = c;
            len += 1;
        }
    }
    if len == 0 {
        0
    } else {
        allowed[rng.below(len as u32) as usize]
    }
}

pub const SPECS: [Spec; 106] = [
    Spec { info_id:   0, kind: 0, model_id:  30, colour_mask: 0x015806f3, raw_dims: [  4528,  12536,   3362], top_speed: 100, num_gears: 3, mass_q12:  4096, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4534, front_axle_q12:  9367, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 245760, steer_rate_q12: 491520, steer_red_q12:  626, sway_damping_q12: 3252, seat_y_q12:    196, flags8e: 0x2110, name: "ADMIRAL" },
    Spec { info_id:   1, kind: 0, model_id:  32, colour_mask: 0x002999b7, raw_dims: [  4286,   9870,   3130], top_speed: 100, num_gears: 3, mass_q12:  4096, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4534, front_axle_q12:  9367, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 245760, steer_rate_q12: 491520, steer_red_q12:  626, sway_damping_q12: 3252, seat_y_q12:    -57, flags8e: 0x6110, name: "BLISTA" },
    Spec { info_id:   2, kind: 0, model_id:  33, colour_mask: 0x01fffff7, raw_dims: [  4581,  11184,   3029], top_speed: 130, num_gears: 3, mass_q12:  6144, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12:   24, grip_q12: 4538, front_axle_q12:  9744, rear_axle_q12:  -9928, front_k18_q12: 16384, rear_k18_q12: 11452, steer_lock_q12: 253952, steer_rate_q12: 122847, steer_red_q12:  339, sway_damping_q12: 3244, seat_y_q12:    169, flags8e: 0x0310, name: "DUKES" },
    Spec { info_id:   3, kind: 4, model_id:  34, colour_mask: 0x00000000, raw_dims: [  4184,  20829,   6440], top_speed: 120, num_gears: 3, mass_q12: 40960, max_torque_q12: 2457600, brake_force_q12:  819200, drive_split_q12: 2048, grip_q12: 4096, front_axle_q12:     0, rear_axle_q12:      0, front_k18_q12:  4096, rear_k18_q12:  4096, steer_lock_q12: 368640, steer_rate_q12: 368640, steer_red_q12:    0, sway_damping_q12:    0, seat_y_q12:   7380, flags8e: 0x0300, name: "POLMAV" },
    Spec { info_id:   4, kind: 0, model_id:  36, colour_mask: 0x00000000, raw_dims: [  5361,  12893,   5808], top_speed: 100, num_gears: 3, mass_q12:  8192, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4538, front_axle_q12:  9371, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 245760, steer_rate_q12: 491520, steer_red_q12:  684, sway_damping_q12: 3256, seat_y_q12:   5383, flags8e: 0x6111, name: "SWAT" },
    Spec { info_id:   5, kind: 0, model_id:  38, colour_mask: 0x004a1453, raw_dims: [  4425,  12370,   2967], top_speed:  90, num_gears: 3, mass_q12:  6144, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4534, front_axle_q12:  9367, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 245760, steer_rate_q12: 491520, steer_red_q12:  626, sway_damping_q12: 3252, seat_y_q12:  -1306, flags8e: 0x0110, name: "BUCCANEER" },
    Spec { info_id:   6, kind: 0, model_id:  39, colour_mask: 0x00000000, raw_dims: [  6091,  19270,   7107], top_speed:  60, num_gears: 3, mass_q12: 12288, max_torque_q12: 327680, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4526, front_axle_q12: 13455, rear_axle_q12: -17604, front_k18_q12:  9703, rear_k18_q12:  7778, steer_lock_q12: 253952, steer_rate_q12: 122847, steer_red_q12:  282, sway_damping_q12: 3244, seat_y_q12:  13279, flags8e: 0x6141, name: "FIRETRUCK" },
    Spec { info_id:   7, kind: 5, model_id:  41, colour_mask: 0x00000000, raw_dims: [  7610,  18624,   8059], top_speed:  50, num_gears: 3, mass_q12: 20480, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4530, front_axle_q12:  9363, rear_axle_q12:  -9416, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12:  81879, steer_red_q12:  569, sway_damping_q12: 3248, seat_y_q12:  -5954, flags8e: 0xa100, name: "RHINO" },
    Spec { info_id:   8, kind: 0, model_id:  43, colour_mask: 0x00000000, raw_dims: [  4813,  13643,   5039], top_speed: 100, num_gears: 3, mass_q12:  8192, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4534, front_axle_q12:  9367, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 245760, steer_rate_q12: 491520, steer_red_q12:  626, sway_damping_q12: 3252, seat_y_q12:   4323, flags8e: 0x6911, name: "AMBULANCE" },
    Spec { info_id:   9, kind: 0, model_id:  45, colour_mask: 0x01c25ef7, raw_dims: [  5127,  12916,   4855], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4562, front_axle_q12:  9396, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  397, sway_damping_q12: 3268, seat_y_q12:   1149, flags8e: 0x6108, name: "BURRITO" },
    Spec { info_id:  10, kind: 0, model_id:  46, colour_mask: 0x00625452, raw_dims: [  6224,  18954,   7690], top_speed:  70, num_gears: 3, mass_q12: 12288, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   6962, flags8e: 0x6148, name: "FLATBED" },
    Spec { info_id:  11, kind: 2, model_id:  47, colour_mask: 0x01180231, raw_dims: [  6453,  22892,   6574], top_speed: 120, num_gears: 3, mass_q12:  6144, max_torque_q12:   6144, brake_force_q12:    4096, drive_split_q12: 2441, grip_q12: 2048, front_axle_q12:  8192, rear_axle_q12: -16384, front_k18_q12:    16, rear_k18_q12:  2048, steer_lock_q12: 327696, steer_rate_q12:  79585, steer_red_q12:  110, sway_damping_q12: 1024, seat_y_q12:   2714, flags8e: 0x1304, name: "SPEEDER" },
    Spec { info_id:  12, kind: 1, model_id:  49, colour_mask: 0x01804208, raw_dims: [  2872,   7780,   3695], top_speed:  90, num_gears: 3, mass_q12:  2048, max_torque_q12: 245760, brake_force_q12:  409600, drive_split_q12: 2080, grip_q12: 4558, front_axle_q12:  6144, rear_axle_q12:  -6144, front_k18_q12: 22528, rear_k18_q12: 24576, steer_lock_q12: 143360, steer_rate_q12: 204787, steer_red_q12:  282, sway_damping_q12: 1024, seat_y_q12:  -2098, flags8e: 0x0304, name: "ANGEL" },
    Spec { info_id:  13, kind: 0, model_id:  50, colour_mask: 0x00000000, raw_dims: [  4977,  10970,   3698], top_speed: 100, num_gears: 3, mass_q12:  4096, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12:  196, grip_q12: 4530, front_axle_q12:  9363, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 368664, steer_rate_q12: 409579, steer_red_q12: 1372, sway_damping_q12: 3248, seat_y_q12:    382, flags8e: 0x6310, name: "CABBIE" },
    Spec { info_id:  14, kind: 0, model_id:  52, colour_mask: 0x00317c2c, raw_dims: [  4962,  11417,   3384], top_speed: 130, num_gears: 3, mass_q12:  4096, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:   -497, flags8e: 0x6310, name: "HELLENBACH" },
    Spec { info_id:  15, kind: 0, model_id:  53, colour_mask: 0x01bddaf3, raw_dims: [  6079,  29781,   7425], top_speed:  60, num_gears: 3, mass_q12: 12288, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4558, front_axle_q12: 13488, rear_axle_q12: -17608, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 368664, steer_red_q12:  741, sway_damping_q12: 3264, seat_y_q12:  16987, flags8e: 0x6130, name: "TANKER" },
    Spec { info_id:  16, kind: 0, model_id:  54, colour_mask: 0x01a8481a, raw_dims: [  4625,  12222,   3178], top_speed: 130, num_gears: 3, mass_q12:  4096, max_torque_q12: 184320, brake_force_q12: 8192000, drive_split_q12: 2048, grip_q12: 4550, front_axle_q12:  9756, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  397, sway_damping_q12: 3256, seat_y_q12:  -1494, flags8e: 0x4318, name: "BANSHEE" },
    Spec { info_id:  17, kind: 0, model_id:  55, colour_mask: 0x000d6432, raw_dims: [  5339,  11724,   3222], top_speed: 130, num_gears: 3, mass_q12:  4096, max_torque_q12: 204800, brake_force_q12: 2048000, drive_split_q12:  401, grip_q12: 4546, front_axle_q12:  9752, rear_axle_q12:  -9924, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 327696, steer_rate_q12: 491520, steer_red_q12: 1028, sway_damping_q12: 3252, seat_y_q12:  -1325, flags8e: 0x4318, name: "COMET" },
    Spec { info_id:  18, kind: 0, model_id:  56, colour_mask: 0x00000000, raw_dims: [  4576,  12192,   3673], top_speed: 110, num_gears: 3, mass_q12:  4096, max_torque_q12: 163840, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4534, front_axle_q12:  9367, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 327696, steer_rate_q12: 491520, steer_red_q12: 1142, sway_damping_q12: 3252, seat_y_q12:     67, flags8e: 0x4311, name: "POLICE" },
    Spec { info_id:  19, kind: 0, model_id:  58, colour_mask: 0x00621452, raw_dims: [  5299,  15184,   6378], top_speed:  70, num_gears: 3, mass_q12:  8192, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   4209, flags8e: 0x6148, name: "YANKEE" },
    Spec { info_id:  20, kind: 0, model_id:  59, colour_mask: 0x0181b01b, raw_dims: [  4906,  12192,   3286], top_speed: 130, num_gears: 3, mass_q12:  4096, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:   -781, flags8e: 0x4318, name: "COQUETTE" },
    Spec { info_id:  21, kind: 0, model_id:  60, colour_mask: 0x015806f3, raw_dims: [  4776,  12523,   3487], top_speed: 130, num_gears: 3, mass_q12:  6144, max_torque_q12: 184320, brake_force_q12: 6144000, drive_split_q12: 2048, grip_q12: 4550, front_axle_q12:  9756, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  397, sway_damping_q12: 3256, seat_y_q12:    111, flags8e: 0x2318, name: "COGNOSCENTI" },
    Spec { info_id:  22, kind: 0, model_id:  61, colour_mask: 0x01881dee, raw_dims: [  4634,  11761,   3314], top_speed:  70, num_gears: 3, mass_q12:  4096, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4562, front_axle_q12:  9396, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  397, sway_damping_q12: 3268, seat_y_q12:    335, flags8e: 0x0108, name: "CHAVOS" },
    Spec { info_id:  23, kind: 0, model_id:  62, colour_mask: 0x0028a89b, raw_dims: [  4722,  11393,   4088], top_speed: 115, num_gears: 3, mass_q12:  6144, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4534, front_axle_q12:  9367, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 286728, steer_rate_q12: 409579, steer_red_q12: 1028, sway_damping_q12: 3252, seat_y_q12:   -202, flags8e: 0x0110, name: "CAVCADE" },
    Spec { info_id:  24, kind: 0, model_id:  63, colour_mask: 0x007daa9b, raw_dims: [  4540,  11323,   3877], top_speed: 115, num_gears: 3, mass_q12:  6144, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4534, front_axle_q12:  9367, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 286728, steer_rate_q12: 409579, steer_red_q12: 1028, sway_damping_q12: 3252, seat_y_q12:     97, flags8e: 0x2118, name: "EXT" },
    Spec { info_id:  25, kind: 0, model_id:  64, colour_mask: 0x00420473, raw_dims: [  4790,  11849,   4179], top_speed:  70, num_gears: 3, mass_q12:  4096, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4558, front_axle_q12:  9392, rear_axle_q12:  -9416, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 245760, steer_rate_q12: 368664, steer_red_q12:  339, sway_damping_q12: 3264, seat_y_q12:    425, flags8e: 0x6110, name: "BOBCAT" },
    Spec { info_id:  26, kind: 0, model_id:  65, colour_mask: 0x00000000, raw_dims: [  4823,  15770,   5745], top_speed:  70, num_gears: 3, mass_q12: 12288, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4562, front_axle_q12: 13492, rear_axle_q12: -17608, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 184332, steer_rate_q12: 327696, steer_red_q12:  397, sway_damping_q12: 3268, seat_y_q12:  11099, flags8e: 0x6110, name: "RECOVERY" },
    Spec { info_id:  27, kind: 0, model_id:  66, colour_mask: 0x01ac5208, raw_dims: [  5228,  12156,   2544], top_speed: 160, num_gears: 3, mass_q12:  4096, max_torque_q12: 225280, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4550, front_axle_q12:  9756, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 225243, steer_rate_q12: 491520, steer_red_q12:  397, sway_damping_q12: 3256, seat_y_q12:  -1311, flags8e: 0x8604, name: "F1" },
    Spec { info_id:  28, kind: 0, model_id:  67, colour_mask: 0x00000000, raw_dims: [  4687,  11122,   3760], top_speed: 130, num_gears: 3, mass_q12:  4096, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:  -1158, flags8e: 0x4390, name: "STREETRACER1" },
    Spec { info_id:  29, kind: 0, model_id:  69, colour_mask: 0x00000000, raw_dims: [  4862,  12322,   3584], top_speed: 130, num_gears: 3, mass_q12:  4096, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:   -680, flags8e: 0x2390, name: "STREETRACER2" },
    Spec { info_id:  30, kind: 0, model_id:  71, colour_mask: 0x00000000, raw_dims: [  5705,  12201,   3408], top_speed: 130, num_gears: 3, mass_q12:  4096, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:  -1325, flags8e: 0x4390, name: "STREETRACER3" },
    Spec { info_id:  31, kind: 0, model_id:  73, colour_mask: 0x00000000, raw_dims: [  5705,  12201,   3408], top_speed: 130, num_gears: 3, mass_q12:  4096, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:  -1325, flags8e: 0x2398, name: "STREETRACER4" },
    Spec { info_id:  32, kind: 0, model_id:  75, colour_mask: 0x00000000, raw_dims: [  4906,  12682,   3286], top_speed: 130, num_gears: 3, mass_q12:  4096, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:   -264, flags8e: 0x4398, name: "STREETRACER5" },
    Spec { info_id:  33, kind: 0, model_id:  77, colour_mask: 0x00c24440, raw_dims: [  6582,  15914,   6088], top_speed:  70, num_gears: 3, mass_q12: 12288, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4562, front_axle_q12:  9396, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  397, sway_damping_q12: 3268, seat_y_q12:  -1722, flags8e: 0x8100, name: "BULLDOZER" },
    Spec { info_id:  34, kind: 0, model_id:  78, colour_mask: 0x00000000, raw_dims: [  5120,  11799,   4484], top_speed: 110, num_gears: 3, mass_q12:  6144, max_torque_q12: 163840, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4538, front_axle_q12:  9371, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 245760, steer_rate_q12: 491520, steer_red_q12:  684, sway_damping_q12: 3256, seat_y_q12:    283, flags8e: 0x0319, name: "FBIRANCHER" },
    Spec { info_id:  35, kind: 2, model_id:  79, colour_mask: 0x015802f3, raw_dims: [  8419,  30759,  16363], top_speed: 120, num_gears: 3, mass_q12:  8192, max_torque_q12:   4096, brake_force_q12:    4096, drive_split_q12: 3260, grip_q12: 2048, front_axle_q12:  8192, rear_axle_q12: -18034, front_k18_q12:    16, rear_k18_q12:  2048, steer_lock_q12: 532488, steer_rate_q12: 245760, steer_red_q12:  110, sway_damping_q12: 1024, seat_y_q12:    928, flags8e: 0x1300, name: "REEFER" },
    Spec { info_id:  36, kind: 2, model_id:  85, colour_mask: 0x00000000, raw_dims: [  6534,  15664,   4955], top_speed: 120, num_gears: 3, mass_q12:  4096, max_torque_q12:   4096, brake_force_q12:    4096, drive_split_q12: 3260, grip_q12: 2048, front_axle_q12:  8192, rear_axle_q12: -14336, front_k18_q12:    12, rear_k18_q12:  2048, steer_lock_q12: 368664, steer_rate_q12:  81879, steer_red_q12:  110, sway_damping_q12: 1024, seat_y_q12:    -16, flags8e: 0x1304, name: "CGDINGHY" },
    Spec { info_id:  37, kind: 2, model_id:  87, colour_mask: 0x00000000, raw_dims: [  7759,  29624,  11894], top_speed: 120, num_gears: 3, mass_q12:  6144, max_torque_q12:  12288, brake_force_q12:    4096, drive_split_q12: 3264, grip_q12: 2048, front_axle_q12:  8192, rear_axle_q12: -18432, front_k18_q12:     8, rear_k18_q12:  2048, steer_lock_q12: 327696, steer_rate_q12:  79585, steer_red_q12:  167, sway_damping_q12: 1024, seat_y_q12:    456, flags8e: 0x1300, name: "CGLAUNCH" },
    Spec { info_id:  38, kind: 2, model_id:  90, colour_mask: 0x00000000, raw_dims: [  9301,  32753,   9960], top_speed: 120, num_gears: 3, mass_q12:  4096, max_torque_q12:   8192, brake_force_q12:    4096, drive_split_q12: 3252, grip_q12: 2048, front_axle_q12:  8192, rear_axle_q12: -18432, front_k18_q12:     8, rear_k18_q12:  2048, steer_lock_q12: 327696, steer_rate_q12:  79585, steer_red_q12:    0, sway_damping_q12: 1024, seat_y_q12:    -31, flags8e: 0x1100, name: "TORPEDOLAUNCH" },
    Spec { info_id:  39, kind: 2, model_id:  92, colour_mask: 0x015806f3, raw_dims: [  6651,  13773,   2975], top_speed: 120, num_gears: 3, mass_q12:  4096, max_torque_q12:   4096, brake_force_q12:    4096, drive_split_q12: 3260, grip_q12: 2048, front_axle_q12:  8192, rear_axle_q12: -14336, front_k18_q12:    12, rear_k18_q12:  2048, steer_lock_q12: 368664, steer_rate_q12:  81879, steer_red_q12:  110, sway_damping_q12: 1024, seat_y_q12:  -3418, flags8e: 0x1304, name: "DINGHY" },
    Spec { info_id:  40, kind: 0, model_id:  93, colour_mask: 0x00000000, raw_dims: [  5993,  22952,   6305], top_speed:  70, num_gears: 3, mass_q12: 12288, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4558, front_axle_q12: 13488, rear_axle_q12: -17608, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 184332, steer_rate_q12: 327696, steer_red_q12:  339, sway_damping_q12: 3264, seat_y_q12:  19022, flags8e: 0x4140, name: "BUS" },
    Spec { info_id:  41, kind: 0, model_id:  95, colour_mask: 0x00000000, raw_dims: [  3685,   8401,   2648], top_speed: 130, num_gears: 3, mass_q12:  4096, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:  -1796, flags8e: 0x4300, name: "STREETRACER1 MOD" },
    Spec { info_id:  42, kind: 0, model_id:  96, colour_mask: 0x00000000, raw_dims: [  4822,  12403,   8869], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4562, front_axle_q12:  9396, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  397, sway_damping_q12: 3268, seat_y_q12:   1916, flags8e: 0x6100, name: "BURRITO MOD" },
    Spec { info_id:  43, kind: 3, model_id:  97, colour_mask: 0x015806f3, raw_dims: [  4061,   9743,   3162], top_speed: 150, num_gears: 3, mass_q12:  2048, max_torque_q12:   8192, brake_force_q12:    8192, drive_split_q12: 1404, grip_q12: 2048, front_axle_q12:  8192, rear_axle_q12:  -8192, front_k18_q12:    12, rear_k18_q12:     0, steer_lock_q12: 327696, steer_rate_q12:  79585, steer_red_q12:   53, sway_damping_q12: 1024, seat_y_q12:  -3944, flags8e: 0x1304, name: "WETSKI" },
    Spec { info_id:  44, kind: 2, model_id:  98, colour_mask: 0x015802f3, raw_dims: [  7971,  27125,   6257], top_speed: 140, num_gears: 3, mass_q12:  2048, max_torque_q12:  11055, brake_force_q12:    4096, drive_split_q12: 3260, grip_q12: 2048, front_axle_q12:  8192, rear_axle_q12: -28672, front_k18_q12:    12, rear_k18_q12:  2048, steer_lock_q12: 327696, steer_rate_q12:  79585, steer_red_q12:  110, sway_damping_q12: 1024, seat_y_q12:  -4319, flags8e: 0x1304, name: "CRUISER" },
    Spec { info_id:  45, kind: 4, model_id:  99, colour_mask: 0x00000000, raw_dims: [  7547,  19319,   6797], top_speed: 120, num_gears: 3, mass_q12: 40960, max_torque_q12: 2457600, brake_force_q12:  819200, drive_split_q12: 2048, grip_q12: 4096, front_axle_q12:     0, rear_axle_q12:      0, front_k18_q12:  4096, rear_k18_q12:  4096, steer_lock_q12: 368640, steer_rate_q12: 368640, steer_red_q12:    0, sway_damping_q12:    0, seat_y_q12:   3858, flags8e: 0x0300, name: "DEFENDER" },
    Spec { info_id:  46, kind: 0, model_id: 101, colour_mask: 0x00000000, raw_dims: [  5127,  12916,   4855], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4562, front_axle_q12:  9396, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  397, sway_damping_q12: 3268, seat_y_q12:   1149, flags8e: 0x6108, name: "BURRITO BURGER" },
    Spec { info_id:  47, kind: 0, model_id: 103, colour_mask: 0x00000000, raw_dims: [  5127,  12916,   4855], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4562, front_axle_q12:  9396, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  397, sway_damping_q12: 3268, seat_y_q12:   1149, flags8e: 0x6108, name: "BURRITO DEBRIS" },
    Spec { info_id:  48, kind: 0, model_id: 105, colour_mask: 0x00000000, raw_dims: [  5127,  12916,   4855], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4562, front_axle_q12:  9396, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  397, sway_damping_q12: 3268, seat_y_q12:   1149, flags8e: 0x6108, name: "BURRITO FLY" },
    Spec { info_id:  49, kind: 0, model_id: 107, colour_mask: 0x00000000, raw_dims: [  5127,  12916,   4855], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4562, front_axle_q12:  9396, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  397, sway_damping_q12: 3268, seat_y_q12:   1149, flags8e: 0x6108, name: "BURRITO RANCH" },
    Spec { info_id:  50, kind: 0, model_id: 109, colour_mask: 0x00000000, raw_dims: [  5127,  12916,   4855], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4562, front_axle_q12:  9396, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  397, sway_damping_q12: 3268, seat_y_q12:   1149, flags8e: 0x6108, name: "BURRITO SPRUNK" },
    Spec { info_id:  51, kind: 0, model_id: 111, colour_mask: 0x01a8481a, raw_dims: [  3987,   6791,   1816], top_speed: 100, num_gears: 3, mass_q12:  4096, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4538, front_axle_q12:  9371, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 245760, steer_rate_q12: 491520, steer_red_q12:  684, sway_damping_q12: 3256, seat_y_q12:   -456, flags8e: 0x8204, name: "GOKART" },
    Spec { info_id:  52, kind: 0, model_id: 112, colour_mask: 0x00000000, raw_dims: [  5299,  15184,   6378], top_speed:  70, num_gears: 3, mass_q12:  8192, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   4209, flags8e: 0x6148, name: "YANKEE 247" },
    Spec { info_id:  53, kind: 0, model_id: 114, colour_mask: 0x00000000, raw_dims: [  5299,  15184,   6378], top_speed:  70, num_gears: 3, mass_q12:  8192, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   4209, flags8e: 0x0148, name: "YANKEE AMMO" },
    Spec { info_id:  54, kind: 0, model_id: 116, colour_mask: 0x00000000, raw_dims: [  5299,  15184,   6378], top_speed:  70, num_gears: 3, mass_q12:  8192, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   4209, flags8e: 0x0148, name: "YANKEE CLUCKIN" },
    Spec { info_id:  55, kind: 0, model_id: 118, colour_mask: 0x00000000, raw_dims: [  5299,  15184,   6378], top_speed:  70, num_gears: 3, mass_q12:  8192, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   4209, flags8e: 0x6148, name: "YANKEE ECOLA" },
    Spec { info_id:  56, kind: 0, model_id: 120, colour_mask: 0x00000000, raw_dims: [  5299,  15184,   6378], top_speed:  70, num_gears: 3, mass_q12:  8192, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   4209, flags8e: 0x6148, name: "YANKEE HINTER" },
    Spec { info_id:  57, kind: 0, model_id: 122, colour_mask: 0x0181b01b, raw_dims: [  5426,  12140,   3147], top_speed: 130, num_gears: 3, mass_q12:  4096, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:  -1699, flags8e: 0x5318, name: "INFERNUS" },
    Spec { info_id:  58, kind: 0, model_id: 123, colour_mask: 0x007daa9b, raw_dims: [  4998,  11184,   4146], top_speed: 100, num_gears: 3, mass_q12:  6144, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4538, front_axle_q12:  9371, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 245760, steer_rate_q12: 491520, steer_red_q12:  684, sway_damping_q12: 3256, seat_y_q12:    484, flags8e: 0x6318, name: "PATRIOT" },
    Spec { info_id:  59, kind: 0, model_id: 124, colour_mask: 0x00317c2c, raw_dims: [  4772,  11900,   3219], top_speed: 130, num_gears: 3, mass_q12:  4096, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:  -1905, flags8e: 0x4310, name: "SABREGT" },
    Spec { info_id:  60, kind: 0, model_id: 125, colour_mask: 0x00317c2c, raw_dims: [  4807,  12025,   3044], top_speed: 120, num_gears: 3, mass_q12:  4096, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12:   24, grip_q12: 4538, front_axle_q12:  9744, rear_axle_q12:  -9928, front_k18_q12: 16384, rear_k18_q12: 11452, steer_lock_q12: 253952, steer_rate_q12: 122847, steer_red_q12:  339, sway_damping_q12: 3244, seat_y_q12:  -1980, flags8e: 0x6310, name: "STALLION" },
    Spec { info_id:  61, kind: 0, model_id: 126, colour_mask: 0x0181b01b, raw_dims: [  4714,  11758,   3345], top_speed: 130, num_gears: 3, mass_q12:  4096, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:   -337, flags8e: 0x4390, name: "SULTAN" },
    Spec { info_id:  62, kind: 0, model_id: 127, colour_mask: 0x004a1453, raw_dims: [  4756,  12679,   2939], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:  -2802, flags8e: 0x6310, name: "PEYOTE" },
    Spec { info_id:  63, kind: 0, model_id: 128, colour_mask: 0x004a1453, raw_dims: [  4625,  12007,   3392], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   -108, flags8e: 0x2308, name: "INGOT" },
    Spec { info_id:  64, kind: 0, model_id: 129, colour_mask: 0x01881dee, raw_dims: [  4334,  11618,   3978], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   1037, flags8e: 0x0308, name: "PERREN" },
    Spec { info_id:  65, kind: 0, model_id: 130, colour_mask: 0x00000000, raw_dims: [  4514,  12971,   3387], top_speed: 115, num_gears: 3, mass_q12:  6144, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4534, front_axle_q12:  9367, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 286728, steer_rate_q12: 409579, steer_red_q12: 1028, sway_damping_q12: 3252, seat_y_q12:   1384, flags8e: 0x6110, name: "HEARSE" },
    Spec { info_id:  66, kind: 0, model_id: 132, colour_mask: 0x01881dee, raw_dims: [  4894,  16988,   3449], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4562, front_axle_q12: 13492, rear_axle_q12: -17608, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 188399, steer_rate_q12: 327696, steer_red_q12:  397, sway_damping_q12: 3268, seat_y_q12:   6935, flags8e: 0x2318, name: "LIMO" },
    Spec { info_id:  67, kind: 0, model_id: 133, colour_mask: 0x00000000, raw_dims: [  4486,  12419,   3331], top_speed: 100, num_gears: 3, mass_q12:  4096, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4538, front_axle_q12:  9371, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 245760, steer_rate_q12: 491520, steer_red_q12:  684, sway_damping_q12: 3256, seat_y_q12:    152, flags8e: 0x2310, name: "GANG ADMIRAL" },
    Spec { info_id:  68, kind: 0, model_id: 135, colour_mask: 0x00000000, raw_dims: [  4625,  12683,   3188], top_speed: 110, num_gears: 3, mass_q12:  4096, max_torque_q12: 163840, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:  -1494, flags8e: 0x4398, name: "GANG BANSHEE" },
    Spec { info_id:  69, kind: 0, model_id: 137, colour_mask: 0x00000000, raw_dims: [  4822,  12403,   5032], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:    925, flags8e: 0x6308, name: "GANG BURR AFR" },
    Spec { info_id:  70, kind: 0, model_id: 139, colour_mask: 0x00000000, raw_dims: [  4636,  11932,   6124], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:    714, flags8e: 0x6308, name: "GANG BURR CHAN" },
    Spec { info_id:  71, kind: 0, model_id: 141, colour_mask: 0x00000000, raw_dims: [  4822,  12403,   5028], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:    925, flags8e: 0x6308, name: "GANG BURR ANGEL" },
    Spec { info_id:  72, kind: 0, model_id: 143, colour_mask: 0x00000000, raw_dims: [  4790,  11849,   4179], top_speed:  70, num_gears: 3, mass_q12:  4096, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4558, front_axle_q12:  9392, rear_axle_q12:  -9416, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 245760, steer_rate_q12: 368664, steer_red_q12:  339, sway_damping_q12: 3264, seat_y_q12:    341, flags8e: 0x6110, name: "GANG BOBCAT" },
    Spec { info_id:  73, kind: 0, model_id: 145, colour_mask: 0x00000000, raw_dims: [  5127,  12916,   5215], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   1149, flags8e: 0x6308, name: "GANG BURR IRE3" },
    Spec { info_id:  74, kind: 0, model_id: 147, colour_mask: 0x00000000, raw_dims: [  5127,  12916,   4855], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   1149, flags8e: 0x6308, name: "GANG BURR JAM" },
    Spec { info_id:  75, kind: 0, model_id: 149, colour_mask: 0x00000000, raw_dims: [  5127,  12916,   4855], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   1149, flags8e: 0x6308, name: "GANG BURR KEN" },
    Spec { info_id:  76, kind: 0, model_id: 151, colour_mask: 0x00000000, raw_dims: [  5127,  12916,   4855], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   1149, flags8e: 0x6308, name: "GANG BURR KOR" },
    Spec { info_id:  77, kind: 0, model_id: 153, colour_mask: 0x00000000, raw_dims: [  5127,  12916,   4855], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   1149, flags8e: 0x6308, name: "GANG BURR MOB" },
    Spec { info_id:  78, kind: 0, model_id: 155, colour_mask: 0x00000000, raw_dims: [  5127,  12916,   5581], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   1149, flags8e: 0x6308, name: "GANG BURR RUSS" },
    Spec { info_id:  79, kind: 0, model_id: 157, colour_mask: 0x00000000, raw_dims: [  5127,  12916,   4855], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   1149, flags8e: 0x6308, name: "GANG BURR SPAN" },
    Spec { info_id:  80, kind: 0, model_id: 159, colour_mask: 0x00000000, raw_dims: [  5224,  13160,   6397], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:   1149, flags8e: 0x6308, name: "GANG BURR ZHOU" },
    Spec { info_id:  81, kind: 0, model_id: 161, colour_mask: 0x00000000, raw_dims: [  4634,  11761,   3314], top_speed:  70, num_gears: 3, mass_q12:  4096, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4567, front_axle_q12:  9400, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  454, sway_damping_q12: 3272, seat_y_q12:    335, flags8e: 0x0308, name: "GANG CHAVOS" },
    Spec { info_id:  82, kind: 0, model_id: 163, colour_mask: 0x00000000, raw_dims: [  4776,  12523,   3487], top_speed: 130, num_gears: 3, mass_q12:  6144, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:    111, flags8e: 0x2318, name: "GANG COGNOSCENTI" },
    Spec { info_id:  83, kind: 0, model_id: 165, colour_mask: 0x00000000, raw_dims: [  5705,  12201,   3408], top_speed: 110, num_gears: 3, mass_q12:  4096, max_torque_q12: 163840, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4550, front_axle_q12:  9756, rear_axle_q12:  -9924, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 327696, steer_rate_q12: 491520, steer_red_q12: 1085, sway_damping_q12: 3256, seat_y_q12:  -1325, flags8e: 0x4318, name: "GANG COMET" },
    Spec { info_id:  84, kind: 0, model_id: 167, colour_mask: 0x00000000, raw_dims: [  4906,  12192,   3286], top_speed: 130, num_gears: 3, mass_q12:  4096, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:   -781, flags8e: 0x4318, name: "GANG COQUETTE" },
    Spec { info_id:  85, kind: 0, model_id: 169, colour_mask: 0x00000000, raw_dims: [  4936,  12050,   3263], top_speed: 130, num_gears: 3, mass_q12:  6144, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12:   24, grip_q12: 4538, front_axle_q12:  9744, rear_axle_q12:  -9928, front_k18_q12: 16384, rear_k18_q12: 11452, steer_lock_q12: 253952, steer_rate_q12: 122847, steer_red_q12:  339, sway_damping_q12: 3244, seat_y_q12:      0, flags8e: 0x0310, name: "GANG DUKES" },
    Spec { info_id:  86, kind: 0, model_id: 171, colour_mask: 0x00000000, raw_dims: [  4540,  11323,   4575], top_speed: 100, num_gears: 3, mass_q12:  6144, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4538, front_axle_q12:  9371, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 245760, steer_rate_q12: 491520, steer_red_q12:  684, sway_damping_q12: 3256, seat_y_q12:     97, flags8e: 0x2318, name: "GANG EXT" },
    Spec { info_id:  87, kind: 0, model_id: 172, colour_mask: 0x00000000, raw_dims: [  4724,  10873,   3352], top_speed: 130, num_gears: 3, mass_q12:  4096, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:   -653, flags8e: 0x6390, name: "GANG HELLEN" },
    Spec { info_id:  88, kind: 0, model_id: 174, colour_mask: 0x00000000, raw_dims: [  4469,  13155,   3215], top_speed: 100, num_gears: 3, mass_q12:  4096, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4538, front_axle_q12:  9371, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 245760, steer_rate_q12: 491520, steer_red_q12:  684, sway_damping_q12: 3256, seat_y_q12:  -1426, flags8e: 0x0308, name: "GANG MANANA" },
    Spec { info_id:  89, kind: 1, model_id: 176, colour_mask: 0x01804208, raw_dims: [  2244,   7385,   3128], top_speed: 130, num_gears: 3, mass_q12:  2048, max_torque_q12: 286720, brake_force_q12:  409600, drive_split_q12: 2072, grip_q12: 4550, front_axle_q12:  6144, rear_axle_q12:  -6144, front_k18_q12: 22528, rear_k18_q12: 24576, steer_lock_q12: 143360, steer_rate_q12:  81879, steer_red_q12:  282, sway_damping_q12: 1024, seat_y_q12:  -2111, flags8e: 0x0304, name: "NRG900" },
    Spec { info_id:  90, kind: 0, model_id: 177, colour_mask: 0x00000000, raw_dims: [  5047,  13040,   5798], top_speed: 100, num_gears: 3, mass_q12:  8192, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4534, front_axle_q12:  9367, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 245760, steer_rate_q12: 491520, steer_red_q12:  626, sway_damping_q12: 3252, seat_y_q12:   5080, flags8e: 0x6110, name: "ALPHAMAIL" },
    Spec { info_id:  91, kind: 1, model_id: 179, colour_mask: 0x01804208, raw_dims: [  2872,   7780,   3695], top_speed:  90, num_gears: 3, mass_q12:  2048, max_torque_q12: 245760, brake_force_q12:  409600, drive_split_q12: 2080, grip_q12: 4558, front_axle_q12:  6144, rear_axle_q12:  -6144, front_k18_q12: 22528, rear_k18_q12: 24576, steer_lock_q12: 143360, steer_rate_q12: 204787, steer_red_q12:  282, sway_damping_q12: 1024, seat_y_q12:  -2098, flags8e: 0x0304, name: "ANGEL2" },
    Spec { info_id:  92, kind: 0, model_id: 180, colour_mask: 0x00000000, raw_dims: [  3829,   8187,   3198], top_speed:  80, num_gears: 3, mass_q12:  4096, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4534, front_axle_q12:  9367, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 245760, steer_rate_q12: 491520, steer_red_q12:  626, sway_damping_q12: 3252, seat_y_q12:  -4972, flags8e: 0x0104, name: "BAGGAGE" },
    Spec { info_id:  93, kind: 1, model_id: 181, colour_mask: 0x01804208, raw_dims: [  3195,   7994,   3712], top_speed:  95, num_gears: 3, mass_q12:  2048, max_torque_q12: 327680, brake_force_q12:  409600, drive_split_q12: 2072, grip_q12: 4550, front_axle_q12:  6144, rear_axle_q12:  -6144, front_k18_q12: 22528, rear_k18_q12: 24576, steer_lock_q12: 135168, steer_rate_q12:  81920, steer_red_q12:  286, sway_damping_q12: 1024, seat_y_q12:  -1573, flags8e: 0x0304, name: "CHOPPER" },
    Spec { info_id:  94, kind: 0, model_id: 182, colour_mask: 0x00000000, raw_dims: [  4604,  13460,   3109], top_speed: 100, num_gears: 3, mass_q12:  4096, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4538, front_axle_q12:  9371, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 245760, steer_rate_q12: 491520, steer_red_q12:  684, sway_damping_q12: 3256, seat_y_q12:   -314, flags8e: 0x6310, name: "GANG VOODOO" },
    Spec { info_id:  95, kind: 4, model_id: 184, colour_mask: 0x00000000, raw_dims: [  5979,  17558,   5964], top_speed: 120, num_gears: 3, mass_q12: 40960, max_torque_q12: 2457600, brake_force_q12:  819200, drive_split_q12: 2048, grip_q12: 4096, front_axle_q12:     0, rear_axle_q12:      0, front_k18_q12:  4096, rear_k18_q12:  4096, steer_lock_q12: 368640, steer_rate_q12: 368640, steer_red_q12:    0, sway_damping_q12:    0, seat_y_q12:   2204, flags8e: 0x0300, name: "SPARROW" },
    Spec { info_id:  96, kind: 2, model_id: 186, colour_mask: 0x00000000, raw_dims: [  8483, -32132,  15042], top_speed: 120, num_gears: 3, mass_q12:  8192, max_torque_q12:  12288, brake_force_q12:    4096, drive_split_q12: 3260, grip_q12: 2048, front_axle_q12:  8192, rear_axle_q12: -18038, front_k18_q12:    16, rear_k18_q12:  2048, steer_lock_q12: 368664, steer_rate_q12: 122847, steer_red_q12:  110, sway_damping_q12: 1024, seat_y_q12:   6713, flags8e: 0x1300, name: "ZHOUBOAT" },
    Spec { info_id:  97, kind: 1, model_id: 188, colour_mask: 0x00000000, raw_dims: [  2872,   7780,   3695], top_speed:  90, num_gears: 3, mass_q12:  2048, max_torque_q12: 245760, brake_force_q12:  409600, drive_split_q12: 2084, grip_q12: 4562, front_axle_q12:  6144, rear_axle_q12:  -6144, front_k18_q12: 22528, rear_k18_q12: 24576, steer_lock_q12: 143360, steer_rate_q12: 204787, steer_red_q12:  339, sway_damping_q12: 1024, seat_y_q12:  -2098, flags8e: 0x0304, name: "GANG ANGEL" },
    Spec { info_id:  98, kind: 1, model_id: 189, colour_mask: 0x01804208, raw_dims: [  3447,   7322,   3482], top_speed: 110, num_gears: 3, mass_q12:  2048, max_torque_q12: 286720, brake_force_q12:  409600, drive_split_q12: 2080, grip_q12: 4558, front_axle_q12:  6144, rear_axle_q12:  -6144, front_k18_q12: 22528, rear_k18_q12: 24576, steer_lock_q12: 143360, steer_rate_q12: 204787, steer_red_q12:  282, sway_damping_q12: 1024, seat_y_q12:  -1671, flags8e: 0x0304, name: "SANCHEZ" },
    Spec { info_id:  99, kind: 2, model_id: 190, colour_mask: 0x015802f3, raw_dims: [  7487,  25068,   4810], top_speed: 120, num_gears: 3, mass_q12:  6144, max_torque_q12:   4096, brake_force_q12:    4096, drive_split_q12: 2441, grip_q12: 2048, front_axle_q12:  8192, rear_axle_q12: -24576, front_k18_q12:    16, rear_k18_q12:  2048, steer_lock_q12: 409579, steer_rate_q12: 102391, steer_red_q12:  110, sway_damping_q12: 1024, seat_y_q12:  -6191, flags8e: 0x1304, name: "JETMAX" },
    Spec { info_id: 100, kind: 0, model_id: 191, colour_mask: 0x00000000, raw_dims: [  5127,  16202,  10732], top_speed: 100, num_gears: 3, mass_q12:  4096, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12:  196, grip_q12: 4530, front_axle_q12:  9363, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 368664, steer_rate_q12: 409579, steer_red_q12: 1372, sway_damping_q12: 3248, seat_y_q12:   1149, flags8e: 0x6308, name: "BURRITO CHINESE" },
    Spec { info_id: 101, kind: 0, model_id: 193, colour_mask: 0x00000000, raw_dims: [  4776,  12523,   3487], top_speed: 130, num_gears: 3, mass_q12:  6144, max_torque_q12: 184320, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4554, front_axle_q12:  9760, rear_axle_q12:  -9928, front_k18_q12: 24576, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 491520, steer_red_q12:  454, sway_damping_q12: 3260, seat_y_q12:    111, flags8e: 0x2318, name: "GANG COG KEN" },
    Spec { info_id: 102, kind: 0, model_id:  65, colour_mask: 0x00000000, raw_dims: [  4823,  15770,   5745], top_speed: 100, num_gears: 3, mass_q12: 12288, max_torque_q12: 143360, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4562, front_axle_q12: 13492, rear_axle_q12: -17608, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 368664, steer_rate_q12: 409579, steer_red_q12: 1314, sway_damping_q12: 3268, seat_y_q12:  11099, flags8e: 0x6100, name: "RECOV DUMMY" },
    Spec { info_id: 103, kind: 0, model_id: 195, colour_mask: 0x00000000, raw_dims: [  5329,  13046,   4855], top_speed:  70, num_gears: 3, mass_q12:  6144, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4562, front_axle_q12:  9396, rear_axle_q12:  -9420, front_k18_q12: 20480, rear_k18_q12: 16384, steer_lock_q12: 155623, steer_rate_q12: 327696, steer_red_q12:  397, sway_damping_q12: 3268, seat_y_q12:   1149, flags8e: 0x6108, name: "BURRITO MEL" },
    Spec { info_id: 104, kind: 4, model_id: 196, colour_mask: 0x00000000, raw_dims: [  4260,  20803,   6558], top_speed: 120, num_gears: 3, mass_q12: 40960, max_torque_q12: 2457600, brake_force_q12:  819200, drive_split_q12: 2048, grip_q12: 4096, front_axle_q12:     0, rear_axle_q12:      0, front_k18_q12:  4096, rear_k18_q12:  4096, steer_lock_q12: 368640, steer_rate_q12: 368640, steer_red_q12:    0, sway_damping_q12:    0, seat_y_q12:   3846, flags8e: 0x0300, name: "HIGGMAV" },
    Spec { info_id: 105, kind: 0, model_id: 198, colour_mask: 0x00000000, raw_dims: [  6079,  29781,   7425], top_speed:  60, num_gears: 3, mass_q12: 12288, max_torque_q12: 102400, brake_force_q12: 2048000, drive_split_q12: 2048, grip_q12: 4558, front_axle_q12: 13488, rear_axle_q12: -17608, front_k18_q12: 20480, rear_k18_q12: 20480, steer_lock_q12: 204787, steer_rate_q12: 368664, steer_red_q12:  741, sway_damping_q12: 3264, seat_y_q12:  16987, flags8e: 0x6130, name: "TANKER TERROIL" },
];

pub struct TestCar {
    pub mesh: Mesh,
    pub models: HashMap<u16, Mesh>,
    pub textures: HashMap<u16, Texture>,
    pub p: [f32; 3],
    pub angle: f32,
    pub shape: BoxShape,
}
impl TestCar {
    pub fn load(
        disc: &mut Disc,
        pak: &Pak,
        collision: &mut Collision,
        xy: [f32; 2],
        angle: f32,
    ) -> Result<Self> {
        let mut car = Self::load_mesh(disc, pak, xy, angle)?;
        let ground = collision.ground(xy[0], xy[1], 0.);
        if ground.water {
            return Err("Test car must be placed on land".into());
        }
        car.p[2] = ground.z - car.mesh.declared_bounds[0][2];
        car.shape.c[2] = car.p[2] + car.shape.h[2];
        let (cx, cy) = Collision::cell_xy(xy[0], xy[1]).ok_or("Car outside map")?;
        collision.cells[cx * 100 + cy].boxes.push(car.shape.clone());
        Ok(car)
    }
    /// Decode the shared Admiral mesh and the full land vehicle catalog (PAK 29 + PAK 1..28).
    pub fn load_mesh(disc: &mut Disc, pak: &Pak, xy: [f32; 2], angle: f32) -> Result<Self> {
        if !xy.iter().all(|v| v.is_finite()) || !angle.is_finite() {
            return Err("Car position and angle must be finite".into());
        }
        let info = pak.read(disc, 29)?.0;
        let count = u32le(&info, 0)? as usize;
        bytes(&info, 4, count * 312)?;
        if count != SPECS.len()
            || u16le(&info, 6)? != 30
            || !bytes(&info, 12, 32)?.starts_with(b"./Admiral.vehicle\0")
        {
            return Err("Unsupported PSP vehicle table".into());
        }
        for (i, s) in SPECS.iter().enumerate() {
            let base = 4 + i * 312;
            if u16le(&info, base)? != s.kind
                || u16le(&info, base + 2)? != s.model_id
                || u16le(&info, base + 0x4e)? != s.top_speed
                || u32le(&info, base + 0x50)? as i32 != s.mass_q12
                || u32le(&info, base + 0x6c)? as i32 != s.max_torque_q12
                || u16le(&info, base + 0x8e)? != s.flags8e
            {
                return Err("PSP vehicle table entry mismatch".into());
            }
        }
        let mut textures = HashMap::new();
        // PAK 1 lists the 27 PSP vehicle paint swatch textures (PAK 2..=28).
        let paint_dir = pak.read(disc, 1)?.0;
        let paint_count = u16le(&paint_dir, 0)? as usize;
        if paint_count != 27 {
            return Err("Unsupported PSP vehicle paint directory".into());
        }
        for k in 0..paint_count {
            let tid = u16le(&paint_dir, 2 + k * 2)?;
            textures.insert(
                tid,
                texture::decode(tid as usize, &pak.read(disc, tid as usize)?.0)?,
            );
        }
        let mut models = HashMap::new();
        for s in &SPECS {
            if !s.is_land_vehicle() || models.contains_key(&s.model_id) {
                continue;
            }
            if let Ok((raw, _)) = pak.read(disc, s.model_id as usize) {
                if let Ok(m) = model::parse(&raw) {
                    for &id in &m.texture_ids {
                        if !textures.contains_key(&id) {
                            if let Ok((traw, _)) = pak.read(disc, id as usize) {
                                if let Ok(t) = texture::decode(id as usize, &traw) {
                                    textures.insert(id, t);
                                }
                            }
                        }
                    }
                    models.insert(s.model_id, m);
                }
            }
        }
        let mesh = model::parse(&pak.read(disc, 30)?.0)?;
        let p = [xy[0], xy[1], -mesh.declared_bounds[0][2]];
        let h = SPECS[0].half_extents();
        if h.iter().any(|v| *v <= 0. || *v > 10.) {
            return Err("Invalid car dimensions".into());
        }
        let shape = BoxShape {
            c: [p[0], p[1], p[2] + h[2]],
            h,
            angle,
            flags: 0,
        };
        Ok(Self {
            mesh,
            models,
            textures,
            p,
            angle,
            shape,
        })
    }
}

// ============================================================================
// Chinatown Wars authentic Traffic Light & Intersection Manager (cTrafficLight, cTrafficLightManager)
// ============================================================================

/// Chinatown Wars authentic Traffic Light state (cTrafficLight, cTrafficLightManager).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrafficLightState {
    Green = 0,
    Amber = 1,
    Red = 2,
}

impl TrafficLightState {
    pub fn is_stop(&self) -> bool {
        matches!(self, TrafficLightState::Amber | TrafficLightState::Red)
    }

    pub fn color_rgba(&self) -> [f32; 4] {
        match self {
            TrafficLightState::Green => [0.15, 0.95, 0.35, 1.0],
            TrafficLightState::Amber => [1.0, 0.78, 0.12, 1.0],
            TrafficLightState::Red => [0.95, 0.18, 0.12, 1.0],
        }
    }
}

/// A specific traffic light junction in Liberty City (cTrafficLight).
#[derive(Clone, Debug)]
pub struct TrafficLight {
    pub node_id: usize,
    pub pos: [f32; 3],
    pub cycle_offset: f32,
}

/// Global Traffic Light Manager handling intersection signal cycles (cTrafficLightManager).
#[derive(Clone, Debug)]
pub struct TrafficLightManager {
    pub timer: f32,
    pub cycle_duration: f32,
    pub lights: Vec<TrafficLight>,
}

impl Default for TrafficLightManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TrafficLightManager {
    pub const CYCLE_TIME: f32 = 28.0;
    pub const GREEN_TIME: f32 = 11.5;
    pub const AMBER_TIME: f32 = 2.5;

    pub fn new() -> Self {
        Self {
            timer: 0.0,
            cycle_duration: Self::CYCLE_TIME,
            lights: Vec::new(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.timer = (self.timer + dt) % self.cycle_duration;
    }

    /// Evaluates signal state for approach direction (angle in radians).
    /// Phase 0 (North-South): Green 0..11.5s, Amber 11.5..14s, Red 14..28s.
    /// Phase 1 (East-West): Red 0..14s, Green 14..25.5s, Amber 25.5..28s.
    pub fn signal_for_approach(&self, cycle_offset: f32, approach_angle: f32) -> TrafficLightState {
        let t = (self.timer + cycle_offset) % self.cycle_duration;
        let is_east_west = (approach_angle.sin().abs()) > (approach_angle.cos().abs());

        if !is_east_west {
            if t < Self::GREEN_TIME {
                TrafficLightState::Green
            } else if t < Self::GREEN_TIME + Self::AMBER_TIME {
                TrafficLightState::Amber
            } else {
                TrafficLightState::Red
            }
        } else {
            let p1_start = Self::GREEN_TIME + Self::AMBER_TIME;
            if t < p1_start {
                TrafficLightState::Red
            } else if t < p1_start + Self::GREEN_TIME {
                TrafficLightState::Green
            } else {
                TrafficLightState::Amber
            }
        }
    }
}

// ============================================================================
// Chinatown Wars authentic Traffic & Pedestrian Reaction Manager (cTrafficManager, cWanderRoads)
// ============================================================================

/// Chinatown Wars authentic Traffic & Pedestrian Reaction Manager (cTrafficManager).
#[derive(Clone, Debug, Default)]
pub struct TrafficManager {
    pub siren_yield_timer: f32,
    pub emergency_near: bool,
    pub emergency_pos: [f32; 3],
}

impl TrafficManager {
    pub fn new() -> Self {
        Self {
            siren_yield_timer: 0.0,
            emergency_near: false,
            emergency_pos: [0.0; 3],
        }
    }

    /// Scans for active emergency sirens (police in pursuit or player ambulance/fire/police)
    pub fn update_siren_proximity(&mut self, dt: f32, cars: &[crate::actors::Car]) {
        self.emergency_near = false;
        for car in cars {
            if car.siren_active {
                self.emergency_near = true;
                self.emergency_pos = car.p;
                self.siren_yield_timer = 2.0;
                break;
            }
        }
        if !self.emergency_near && self.siren_yield_timer > 0.0 {
            self.siren_yield_timer = (self.siren_yield_timer - dt).max(0.0);
        }
    }

    /// Evaluates whether an NPC car should pull over / yield to an emergency siren.
    pub fn should_yield_to_siren(&self, car_pos: [f32; 3]) -> bool {
        if self.siren_yield_timer <= 0.0 {
            return false;
        }
        let dx = car_pos[0] - self.emergency_pos[0];
        let dy = car_pos[1] - self.emergency_pos[1];
        dx * dx + dy * dy < 35.0 * 35.0
    }
}

// ============================================================================
// Chinatown Wars authentic Pay 'n' Spray Respray Garage System (cPayNSpray, cPayNSprayManager)
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayNSprayState {
    Open,
    Closing,
    Spraying,
    Opening,
}

#[derive(Clone, Debug)]
pub struct PayNSpray {
    pub id: usize,
    pub name: &'static str,
    pub p: [f32; 3],
    pub bounds: [f32; 3], // half-width, half-depth, height
    pub facing_angle: f32,
    pub door_progress: f32, // 0.0 = fully open, 1.0 = fully closed
    pub state: PayNSprayState,
    pub timer: f32,
}

impl PayNSpray {
    pub fn new(id: usize, name: &'static str, p: [f32; 3], facing_angle: f32) -> Self {
        Self {
            id,
            name,
            p,
            bounds: [4.5, 6.0, 3.8],
            facing_angle,
            door_progress: 0.0,
            state: PayNSprayState::Open,
            timer: 0.0,
        }
    }

    pub fn is_car_inside(&self, car_p: [f32; 3]) -> bool {
        let dx = (car_p[0] - self.p[0]).abs();
        let dy = (car_p[1] - self.p[1]).abs();
        let dz = (car_p[2] - self.p[2]).abs();
        dx < self.bounds[0] && dy < self.bounds[1] && dz < self.bounds[2]
    }
}

#[derive(Clone, Debug)]
pub struct PayNSprayManager {
    pub garages: Vec<PayNSpray>,
    pub active_garage: Option<usize>,
}

impl Default for PayNSprayManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PayNSprayManager {
    pub const RESIGN_COST: u32 = 100;

    pub fn new() -> Self {
        Self {
            garages: vec![
                PayNSpray::new(0, "Meadows Park (Dukes)", [465.0, 180.0, 0.0], 0.0),
                PayNSpray::new(1, "Cerveza Heights (Broker)", [520.0, -280.0, 0.0], std::f32::consts::FRAC_PI_2),
                PayNSpray::new(2, "Northwood (Algonquin)", [-310.0, 720.0, 0.0], std::f32::consts::PI),
                PayNSpray::new(3, "Suffolk (Lower Algonquin)", [-380.0, -420.0, 0.0], -std::f32::consts::FRAC_PI_2),
                PayNSpray::new(4, "BOABO (Broker)", [290.0, -110.0, 0.0], 0.0),
                PayNSpray::new(5, "Chase Point (Bohan)", [150.0, 580.0, 0.0], std::f32::consts::FRAC_PI_2),
            ],
            active_garage: None,
        }
    }

    /// Updates garage doors and checks car entry/exit and respray service.
    pub fn update(
        &mut self,
        dt: f32,
        mut player_car: Option<&mut crate::actors::Car>,
        player_cash: &mut u32,
        police: &mut crate::police::PoliceManager,
        spent_on_pay_n_spray: &mut u32,
    ) -> bool {
        let mut did_respray = false;

        for (idx, garage) in self.garages.iter_mut().enumerate() {
            match garage.state {
                PayNSprayState::Open => {
                    garage.door_progress = (garage.door_progress - dt * 2.5).max(0.0);
                    if let Some(ref car) = player_car {
                        if garage.is_car_inside(car.p) && car.speed.abs() < 4.0 {
                            garage.state = PayNSprayState::Closing;
                            garage.timer = 0.0;
                            self.active_garage = Some(idx);
                        }
                    }
                }
                PayNSprayState::Closing => {
                    garage.door_progress = (garage.door_progress + dt * 2.0).min(1.0);
                    if garage.door_progress >= 1.0 {
                        garage.state = PayNSprayState::Spraying;
                        garage.timer = 0.0;
                    }
                    if let Some(ref mut car) = player_car {
                        car.speed = 0.0;
                    }
                }
                PayNSprayState::Spraying => {
                    garage.timer += dt;
                    if let Some(ref mut car) = player_car {
                        car.speed = 0.0;
                    }
                    if garage.timer >= 1.6 {
                        // Service complete!
                        if let Some(ref mut car) = player_car {
                            car.health = 1000.0;
                            car.flat_tires = [false; 4];
                            car.disabled = false;
                            car.paint = (car.paint + 1) % 16;
                            car.respray_timer = 3.0; // exit sparkles
                        }
                        if *player_cash >= Self::RESIGN_COST {
                            *player_cash -= Self::RESIGN_COST;
                            *spent_on_pay_n_spray = spent_on_pay_n_spray.wrapping_add(Self::RESIGN_COST);
                        }
                        police.start_cooldown(15.0);
                        garage.state = PayNSprayState::Opening;
                        garage.timer = 0.0;
                        did_respray = true;
                    }
                }
                PayNSprayState::Opening => {
                    garage.door_progress = (garage.door_progress - dt * 2.0).max(0.0);
                    if garage.door_progress <= 0.0 {
                        garage.state = PayNSprayState::Open;
                        self.active_garage = None;
                    }
                }
            }
        }

        did_respray
    }

    /// Finds nearest Pay 'n' Spray garage coordinates for radar blip.
    pub fn nearest_blip(&self, player_p: [f32; 3]) -> Option<([f32; 2], [f32; 4])> {
        let mut nearest: Option<([f32; 2], [f32; 4])> = None;
        let mut min_d = f32::MAX;
        for g in &self.garages {
            let dx = g.p[0] - player_p[0];
            let dy = g.p[1] - player_p[1];
            let d = dx * dx + dy * dy;
            if d < min_d {
                min_d = d;
                // Yellow/Orange spray-can radar icon color
                nearest = Some(([g.p[0], g.p[1]], [1.0, 0.85, 0.15, 1.0]));
            }
        }
        nearest
    }
}

/// Helper for linear scalar approach towards a target.
fn move_towards_scalar(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else {
        current + (target - current).signum() * max_delta
    }
}

/// Individual node in a boat's trailing hydrodynamic wake ribbon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WakeSegment {
    pub pos: [f32; 3],
    pub left_edge: [f32; 3],
    pub right_edge: [f32; 3],
    pub width: f32,
    pub alpha: f32,
    pub life: f32,
}

/// Dynamic dual-trail hydrodynamic wake emitter (`cBoatWakeEmitter`).
#[derive(Debug, Clone, Default)]
pub struct BoatWakeEmitter {
    pub segments: Vec<WakeSegment>,
    pub spawn_timer: f32,
}

impl BoatWakeEmitter {
    pub fn new() -> Self {
        Self {
            segments: Vec::with_capacity(64),
            spawn_timer: 0.0,
        }
    }

    /// Emits expanding foam ribbon segments behind moving boat stern.
    pub fn update(&mut self, dt: f32, boat_pos: [f32; 3], boat_angle: f32, speed: f32, is_jetski: bool) {
        for seg in &mut self.segments {
            seg.life += dt;
            seg.alpha = (1.0 - seg.life / 3.2).max(0.0);
            seg.width += 2.0 * dt;
            let perp_x = -boat_angle.cos();
            let perp_y = boat_angle.sin();
            seg.left_edge[0] = seg.pos[0] - perp_x * seg.width;
            seg.left_edge[1] = seg.pos[1] - perp_y * seg.width;
            seg.right_edge[0] = seg.pos[0] + perp_x * seg.width;
            seg.right_edge[1] = seg.pos[1] + perp_y * seg.width;
        }
        self.segments.retain(|s| s.alpha > 0.02);

        if speed.abs() > 1.2 {
            self.spawn_timer += dt;
            let interval = if is_jetski { 0.045 } else { 0.065 };
            if self.spawn_timer >= interval {
                self.spawn_timer = 0.0;
                let stern_offset = if is_jetski { -1.1 } else { -2.4 };
                let stern_x = boat_pos[0] + boat_angle.sin() * stern_offset;
                let stern_y = boat_pos[1] + boat_angle.cos() * stern_offset;
                let initial_w = if is_jetski { 0.55 } else { 1.15 };
                let perp_x = -boat_angle.cos();
                let perp_y = boat_angle.sin();
                self.segments.push(WakeSegment {
                    pos: [stern_x, stern_y, -7.38],
                    left_edge: [stern_x - perp_x * initial_w, stern_y - perp_y * initial_w, -7.38],
                    right_edge: [stern_x + perp_x * initial_w, stern_y + perp_y * initial_w, -7.38],
                    width: initial_w,
                    alpha: 0.85,
                    life: 0.0,
                });
            }
        }
    }
}

/// Bow spray water droplets (`cBoatSprayEmitter`).
#[derive(Debug, Clone, Copy)]
pub struct BoatSprayParticle {
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub life: f32,
    pub alpha: f32,
    pub size: f32,
}

#[derive(Debug, Clone, Default)]
pub struct BoatSprayEmitter {
    pub particles: Vec<BoatSprayParticle>,
}

impl BoatSprayEmitter {
    pub fn new() -> Self {
        Self { particles: Vec::with_capacity(48) }
    }

    pub fn update(&mut self, dt: f32, bow_pos: [f32; 3], angle: f32, speed: f32, is_jetski: bool) {
        for p in &mut self.particles {
            p.life += dt;
            p.pos[0] += p.vel[0] * dt;
            p.pos[1] += p.vel[1] * dt;
            p.pos[2] += p.vel[2] * dt;
            p.vel[2] -= 9.8 * dt;
            p.alpha = (1.0 - p.life / 0.42).max(0.0);
            p.size += 0.7 * dt;
        }
        self.particles.retain(|p| p.alpha > 0.05 && p.pos[2] >= -7.5);

        if speed > 3.5 && self.particles.len() < 36 {
            let fwd = [angle.sin(), angle.cos()];
            let right = [fwd[1], -fwd[0]];
            let bow_len = if is_jetski { 0.95 } else { 2.3 };
            let spray_speed = speed * 0.42;

            // Port bow spray
            self.particles.push(BoatSprayParticle {
                pos: [bow_pos[0] + fwd[0] * bow_len - right[0] * 0.6, bow_pos[1] + fwd[1] * bow_len - right[1] * 0.6, -7.32],
                vel: [-right[0] * spray_speed + fwd[0] * 1.2, -right[1] * spray_speed + fwd[1] * 1.2, 2.6],
                life: 0.0,
                alpha: 0.75,
                size: 0.22,
            });
            // Starboard bow spray
            self.particles.push(BoatSprayParticle {
                pos: [bow_pos[0] + fwd[0] * bow_len + right[0] * 0.6, bow_pos[1] + fwd[1] * bow_len + right[1] * 0.6, -7.32],
                vel: [right[0] * spray_speed + fwd[0] * 1.2, right[1] * spray_speed + fwd[1] * 1.2, 2.6],
                life: 0.0,
                alpha: 0.75,
                size: 0.22,
            });
        }
    }
}

/// Authentic hydrodynamic boat engine with water drag, hydroplane lift, and wave oscillation (`cBoatEngine`).
#[derive(Debug, Clone)]
pub struct BoatEngine {
    pub throttle: f32,
    pub steer: f32,
    pub water_level: f32,
    pub wave_phase: f32,
    pub buoyant_force: f32,
    pub planing_lift: f32,
}

impl Default for BoatEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl BoatEngine {
    pub fn new() -> Self {
        Self {
            throttle: 0.0,
            steer: 0.0,
            water_level: -7.42,
            wave_phase: 0.0,
            buoyant_force: 1.0,
            planing_lift: 0.0,
        }
    }

    /// Computes hydrodynamics step: water resistance, propeller thrust, hydroplane planing.
    pub fn step(
        &mut self,
        dt: f32,
        speed: &mut f32,
        angle: &mut f32,
        pitch: &mut f32,
        roll: &mut f32,
        p: &mut [f32; 3],
        input_throttle: f32,
        input_steer: f32,
        is_jetski: bool,
    ) {
        self.throttle = input_throttle.clamp(-1.0, 1.0);
        self.steer = input_steer.clamp(-1.0, 1.0);
        self.wave_phase += dt * 2.2;

        let max_speed = if is_jetski { 32.0 } else { 26.0 };
        let accel = if is_jetski { 18.0 } else { 12.0 };
        let water_drag = if is_jetski { 4.5 } else { 3.2 };

        if self.throttle.abs() > 0.05 {
            *speed += self.throttle * accel * dt;
        } else {
            *speed = move_towards_scalar(*speed, 0.0, water_drag * dt);
        }
        *speed = speed.clamp(-8.0, max_speed);

        let steer_authority = (speed.abs() / 4.0).min(1.0).max(0.2);
        let turn_rate = if is_jetski { 2.2 } else { 1.4 };
        *angle -= self.steer * turn_rate * steer_authority * dt;

        let target_pitch = if *speed > 3.0 {
            let bow_rise = (*speed / max_speed) * if is_jetski { 0.16 } else { 0.10 };
            -bow_rise
        } else {
            0.02 * (self.wave_phase * 1.5).sin()
        };
        *pitch += (target_pitch - *pitch) * (6.0 * dt).min(1.0);

        let target_roll = if is_jetski {
            self.steer * 0.35 * (*speed / max_speed).min(1.0)
        } else {
            -self.steer * 0.12 * (*speed / max_speed).min(1.0) + 0.03 * (self.wave_phase * 1.8).cos()
        };
        *roll += (target_roll - *roll) * (8.0 * dt).min(1.0);

        let wave_bob = 0.06 * (self.wave_phase).sin() + 0.04 * (self.wave_phase * 1.7 + 0.5).cos();
        p[2] = self.water_level + wave_bob;
    }
}

/// Tracks a land vehicle submerging into water (`cCarInWater`).
#[derive(Debug, Clone)]
pub struct CarInWater {
    pub submersion: f32,
    pub bubble_timer: f32,
    pub engine_stalled: bool,
    pub trigger_sinking_minigame: bool,
}

impl Default for CarInWater {
    fn default() -> Self {
        Self::new()
    }
}

impl CarInWater {
    pub fn new() -> Self {
        Self {
            submersion: 0.0,
            bubble_timer: 0.0,
            engine_stalled: false,
            trigger_sinking_minigame: false,
        }
    }

    pub fn update(&mut self, dt: f32, is_player_driving: bool) {
        self.submersion = (self.submersion + dt * 0.35).min(1.0);
        if self.submersion > 0.15 {
            self.engine_stalled = true;
        }
        if is_player_driving && self.submersion > 0.40 && !self.trigger_sinking_minigame {
            self.trigger_sinking_minigame = true;
        }
    }
}

/// Ambient watercraft waypoint on Liberty City navigable waterways (`cAIBoat`).
#[derive(Debug, Clone, Copy)]
pub struct WaterWaypoint {
    pub p: [f32; 2],
    pub name: &'static str,
}

/// Liberty City authentic navigable waterways channels:
/// East River, West River, North Bohan Canal, Humboldt Bay.
pub const WATER_WAYPOINTS: &[WaterWaypoint] = &[
    WaterWaypoint { p: [ 600.0, -1200.0], name: "East River South" },
    WaterWaypoint { p: [ 750.0,   -200.0], name: "East River Mid" },
    WaterWaypoint { p: [ 680.0,    800.0], name: "East River North" },
    WaterWaypoint { p: [ 300.0,   1600.0], name: "Humboldt River" },
    WaterWaypoint { p: [-200.0,   2100.0], name: "Bohan Channel" },
    WaterWaypoint { p: [-950.0,   1500.0], name: "West River North" },
    WaterWaypoint { p: [-1100.0,   200.0], name: "West River Mid" },
    WaterWaypoint { p: [-900.0,  -1100.0], name: "West River South" },
];

/// Ambient boat and police patrol boat management system (`cAmbientBoatManager`, `cPoliceBoat`).
#[derive(Debug, Clone)]
pub struct AmbientBoatManager {
    pub spawn_timer: f32,
    pub active_patrols: usize,
}

impl Default for AmbientBoatManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AmbientBoatManager {
    pub fn new() -> Self {
        Self {
            spawn_timer: 0.0,
            active_patrols: 0,
        }
    }

    /// Spawns ambient boats or police watercraft depending on player location and wanted level.
    pub fn update(
        &mut self,
        dt: f32,
        player_p: [f32; 3],
        wanted_stars: u8,
        cars: &mut Vec<crate::actors::Car>,
        rng: &mut Rng,
    ) {
        self.spawn_timer += dt;
        if self.spawn_timer >= 5.0 {
            self.spawn_timer = 0.0;

            let water_boats_count = cars.iter().filter(|c| c.is_watercraft()).count();
            if water_boats_count < 4 {
                let mut best_pt = None;
                let mut best_d = 350.0 * 350.0;
                for wp in WATER_WAYPOINTS {
                    let dx = wp.p[0] - player_p[0];
                    let dy = wp.p[1] - player_p[1];
                    let d = dx * dx + dy * dy;
                    if d < best_d && d > 60.0 * 60.0 {
                        best_d = d;
                        best_pt = Some(wp.p);
                    }
                }

                if let Some(pos) = best_pt {
                    let (info_id, model_id, name) = if wanted_stars >= 2 {
                        if rng.below(2) == 0 {
                            (36, 85, "CGDINGHY")
                        } else {
                            (37, 87, "CGLAUNCH")
                        }
                    } else {
                        match rng.below(4) {
                            0 => (11, 47, "SPEEDER"),
                            1 => (43, 97, "WETSKI"),
                            2 => (39, 92, "DINGHY"),
                            _ => (99, 190, "JETMAX"),
                        }
                    };

                    let angle = (rng.below(628) as f32) / 100.0;
                    let mut boat = crate::actors::make_vehicle(
                        [pos[0], pos[1], -7.42],
                        angle,
                        0,
                        0,
                        info_id,
                        model_id,
                        false,
                        false,
                    );
                    boat.name = name;
                    boat.speed = 8.0 + (rng.below(8) as f32);
                    boat.wake_emitter = Some(BoatWakeEmitter::new());
                    boat.spray_emitter = Some(BoatSprayEmitter::new());
                    boat.boat_engine = Some(BoatEngine::new());
                    cars.push(boat);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn psp_vehicle_specs_and_paint_masks_match_pak29_reference() {
        let admiral = spec(0);
        assert_eq!(admiral.model_id, 30);
        assert_eq!(admiral.name, "ADMIRAL");
        let h = admiral.half_extents();
        assert!((h[0] - 1.2158203).abs() < 1e-4);
        assert!((h[1] - 3.3659668).abs() < 1e-4);
        assert!((h[2] - 0.8208008).abs() < 1e-4);
        assert!((admiral.mass() - 1.0).abs() < 1e-4);
        assert!((admiral.max_torque() - 35.0).abs() < 1e-4);

        let nrg = spec(89);
        assert!(nrg.is_bike() && nrg.is_land_vehicle());
        assert_eq!(nrg.name, "NRG900");
        assert!((nrg.mass() - 0.5).abs() < 1e-4);
        assert!((nrg.max_torque() - 70.0).abs() < 1e-4);
        assert!((nrg.visual_scale() - BIKE_SCALE).abs() < 1e-5);

        let rhino = spec(7);
        assert!(rhino.kind == 5 && rhino.is_land_vehicle());
        assert_eq!(rhino.name, "RHINO");
        assert!((rhino.mass() - 5.0).abs() < 1e-4);

        let speeder = spec(11);
        assert!(!speeder.is_land_vehicle());

        let mut rng = Rng::new(42);
        for _ in 0..32 {
            let c = choose_paint(0, 25, &mut rng);
            assert!((admiral.colour_mask >> c) & 1 != 0);
        }
        assert_eq!(choose_paint(0, 3, &mut rng), 3);
        assert_eq!(choose_paint(18, 25, &mut rng), 0);
    }

    #[test]
    fn traffic_light_signal_cycle_alternation_and_stop_detection() {
        let mut tlm = TrafficLightManager::new();
        assert_eq!(tlm.timer, 0.0);

        // At t=0s, North-South is Green (0 rad = facing along Y), East-West is Red (PI/2 rad)
        let ns_signal = tlm.signal_for_approach(0.0, 0.0);
        let ew_signal = tlm.signal_for_approach(0.0, std::f32::consts::FRAC_PI_2);
        assert_eq!(ns_signal, TrafficLightState::Green);
        assert!(!ns_signal.is_stop());
        assert_eq!(ew_signal, TrafficLightState::Red);
        assert!(ew_signal.is_stop());

        // Step to t=12.0s: North-South turns Amber, East-West remains Red
        tlm.update(12.0);
        let ns_amber = tlm.signal_for_approach(0.0, 0.0);
        assert_eq!(ns_amber, TrafficLightState::Amber);
        assert!(ns_amber.is_stop());

        // Step to t=16.0s (12+4): North-South turns Red, East-West turns Green!
        tlm.update(4.0);
        let ns_red = tlm.signal_for_approach(0.0, 0.0);
        let ew_green = tlm.signal_for_approach(0.0, std::f32::consts::FRAC_PI_2);
        assert_eq!(ns_red, TrafficLightState::Red);
        assert_eq!(ew_green, TrafficLightState::Green);
    }

    #[test]
    fn traffic_manager_siren_proximity_and_yield_rules() {
        let mut tm = TrafficManager::new();
        assert!(!tm.emergency_near);

        // Create a dummy cop car with siren active
        let mut cop = crate::actors::make_car([50.0, 50.0, 0.0], 0.0, 0, 0, 0);
        cop.siren_active = true;
        let cars = vec![cop];

        tm.update_siren_proximity(0.1, &cars);
        assert!(tm.emergency_near);
        assert_eq!(tm.emergency_pos, [50.0, 50.0, 0.0]);

        // Civilian car at 20m should yield to siren
        assert!(tm.should_yield_to_siren([50.0, 70.0, 0.0]));

        // Distant car at 100m does not need to yield
        assert!(!tm.should_yield_to_siren([50.0, 150.0, 0.0]));
    }

    #[test]
    fn pay_n_spray_garage_service_and_wanted_cooldown() {
        let mut pns = PayNSprayManager::new();
        assert_eq!(pns.garages.len(), 6, "6 authentic Liberty City garages");

        let mut car = crate::actors::make_car([465.0, 180.0, 0.0], 0.0, 0, 0, 0);
        car.health = 200.0;
        car.flat_tires = [true, true, false, false];
        car.paint = 2;
        let mut cash = 500;
        let mut police = crate::police::PoliceManager::new();
        police.set_stars(2);
        let mut spent = 0;

        // Step 1: Car inside garage at slow speed triggers closing door
        pns.update(0.1, Some(&mut car), &mut cash, &mut police, &mut spent);
        assert_eq!(pns.garages[0].state, PayNSprayState::Closing);

        // Step 2: Door closes
        pns.update(0.6, Some(&mut car), &mut cash, &mut police, &mut spent);
        assert_eq!(pns.garages[0].state, PayNSprayState::Spraying);

        // Step 3: Spraying finishes -> repairs car, repaints, deducts cash, cools down police
        let finished = pns.update(1.8, Some(&mut car), &mut cash, &mut police, &mut spent);
        assert!(finished, "Respray completed");
        assert_eq!(car.health, 1000.0, "Car fully repaired");
        assert_eq!(car.flat_tires, [false; 4], "All tires restored");
        assert_eq!(car.paint, 3, "Repainted to next palette");
        assert_eq!(cash, 400, "100$ deducted");
        assert_eq!(spent, 100, "100$ tracked in stats");
        assert!(police.is_cooling_down(), "Police evasion cooldown started");
        assert_eq!(pns.garages[0].state, PayNSprayState::Opening);
    }

    #[test]
    fn boat_specs_buoyancy_and_hydrodynamics() {
        // 1. Spec watercraft classifications
        let speeder = spec(11);
        assert!(speeder.is_boat() && speeder.is_watercraft() && !speeder.is_land_vehicle());
        let wetski = spec(43);
        assert!(wetski.is_jetski() && wetski.is_watercraft() && !wetski.is_land_vehicle());
        let admiral = spec(0);
        assert!(!admiral.is_watercraft() && admiral.is_land_vehicle());

        // 2. Boat engine hydrodynamics step
        let mut engine = BoatEngine::new();
        let mut speed = 0.0;
        let mut angle = 0.0;
        let mut pitch = 0.0;
        let mut roll = 0.0;
        let mut pos = [500.0, -1000.0, -7.42];

        // Accelerate forward
        engine.step(0.1, &mut speed, &mut angle, &mut pitch, &mut roll, &mut pos, 1.0, 0.0, false);
        assert!(speed > 0.0, "Boat accelerates forward");

        // Rudder steering under speed
        let prev_angle = angle;
        engine.step(0.1, &mut speed, &mut angle, &mut pitch, &mut roll, &mut pos, 1.0, 1.0, false);
        assert!(angle != prev_angle, "Rudder steers boat");

        // Decelerate with natural water resistance
        let cruise_speed = speed;
        engine.step(0.5, &mut speed, &mut angle, &mut pitch, &mut roll, &mut pos, 0.0, 0.0, false);
        assert!(speed < cruise_speed, "Water drag decelerates unpowered boat");

        // 3. Wake emitter
        let mut wake = BoatWakeEmitter::new();
        assert!(wake.segments.is_empty());
        wake.update(0.1, pos, angle, 12.0, false);
        assert!(!wake.segments.is_empty(), "Wake segment spawned at speed");
        let initial_width = wake.segments[0].width;
        wake.update(0.2, pos, angle, 12.0, false);
        assert!(wake.segments[0].width > initial_width, "Wake segment expands as it trails");

        // 4. Bow spray emitter
        let mut spray = BoatSprayEmitter::new();
        spray.update(0.05, pos, angle, 14.0, false);
        assert!(!spray.particles.is_empty(), "Bow spray particles emitted at planing speed");

        // 5. Land car sinking in water
        let mut car_in_water = CarInWater::new();
        assert!(!car_in_water.engine_stalled);
        assert!(!car_in_water.trigger_sinking_minigame);
        car_in_water.update(0.5, true);
        assert!(car_in_water.engine_stalled, "Engine stalls in water");
        car_in_water.update(1.0, true);
        assert!(car_in_water.trigger_sinking_minigame, "Sinking car minigame triggered for player");
    }
}
