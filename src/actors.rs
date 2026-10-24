// SPDX-License-Identifier: GPL-3.0-only
//! Small native slice simulation: arcade cars, civilians and melee reactions.
//! Placement uses PSP roads/collisions; behaviour is reconstructed, not PSP parity.
//! Civilians wander and flee on the PSP pavement network (see pavement.rs);
//! the radar-colour areas remain only for civilians without a nearby node.
use crate::{
    collision::{BoxShape, Collision},
    gps::{distance, Roads},
    pavement::{Network, Rng, ARRIVE, FLEE_ARRIVE},
    ped::{npc_is_jogger, npc_is_male, PedAssets, Pose},
    roll::{Roll, DURATION as ROLL_DURATION},
};
use std::f32::consts::TAU;
use std::sync::Arc;

pub struct PedestrianAreas {
    width: usize,
    height: usize,
    allowed: Vec<bool>,
}
impl PedestrianAreas {
    pub fn from_radar(t: &crate::texture::Texture) -> Self {
        Self {
            width: t.info.width,
            height: t.info.height,
            allowed: t
                .rgba
                .iter()
                .map(|&[r, g, b, _]| {
                    // PSP radar: beige public ground and green parks; grey roads and
                    // buildings, purple water. A margin keeps walkers off the kerb.
                    (r > 150 && r > g && g > b && r - b > 20)
                        || (g > r.saturating_add(20) && g > b.saturating_add(20))
                })
                .collect(),
        }
    }
    pub fn contains(&self, p: [f32; 3]) -> bool {
        if self.allowed.is_empty() {
            return true;
        }
        [(0., 0.), (-1.4, 0.), (1.4, 0.), (0., -1.4), (0., 1.4)]
            .iter()
            .all(|&(dx, dy)| {
                let x = ((p[0] + dx + 3500.) / 7000. * self.width as f32).floor() as i32;
                let y = ((2500. - p[1] - dy) / 5000. * self.height as f32).floor() as i32;
                x >= 0
                    && y >= 0
                    && x < self.width as i32
                    && y < self.height as i32
                    && self.allowed[y as usize * self.width + x as usize]
            })
    }
}

pub const CAR_HALF_WIDTH: f32 = 1.21582;
const CAR_HEIGHT: f32 = 1.9854;

#[derive(Default, Clone, Copy)]
pub struct DriveInput {
    /// Forward accelerator positive, reverse negative.
    pub throttle: f32,
    /// Positive turns right, negative turns left.
    pub steer: f32,
    pub brake: bool,
}

#[derive(Clone, Debug)]
pub struct SkidStrip {
    pub points: Vec<([f32; 3], [f32; 3])>,
    pub car_idx: usize,
    pub wheel: u8,
    pub active: bool,
    pub hold_timer: f32,
    pub fade_timer: f32,
}
impl SkidStrip {
    pub fn alpha(&self) -> f32 {
        let base_alpha = 12.0 / 31.0;
        if self.hold_timer > 0.0 {
            base_alpha
        } else {
            (self.fade_timer / 4.0).clamp(0.0, 1.0) * base_alpha
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub p: [f32; 3],
    pub v: [f32; 3],
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub grow: f32,
    pub kind: u8,
}

pub struct Car {
    /// Ground position; renderer adds the vehicle mesh's base offset.
    pub p: [f32; 3],
    pub angle: f32,
    /// Longitudinal speed along the vehicle heading (u/s).
    pub speed: f32,
    /// Lateral slip speed perpendicular to heading (u/s, positive = right).
    pub lat_vel: f32,
    /// Angular yaw rate (rad/s).
    pub yaw_rate: f32,
    /// Normalized steering wheel angle.
    pub steer_angle: f32,
    /// Transmission gear (-1 = reverse, 0 = neutral, 1..=num_gears = forward).
    pub gear: i8,
    /// Normalized engine RPM (0.2 .. 1.0).
    pub rpm: f32,
    /// Body roll angle (rad): outward suspension sway for cars; inward lean for bikes (cBike::bikeLean_).
    pub roll: f32,
    /// Body pitch angle (rad): squat/dive for cars; wheelie/stoppie for bikes (cBike::bikePitch_).
    pub pitch: f32,
    pub roll_vel: f32,
    pub pitch_vel: f32,
    pub wheel_spin: bool,
    pub front_skidding: bool,
    pub rear_skidding: bool,
    pub braking: bool,
    pub headlights: bool,
    pub info_id: usize,
    pub model_id: u16,
    pub name: &'static str,
    pub h: [f32; 3],
    pub paint: u8,
    pub from_cargen: bool,
    pub parked: bool,
    pub lane: u8,
    pub wobble: f32,
    pub next_lane: u8,
    pub next_wobble: f32,
    pub rail_dist: f32,
    pub next_target: usize,
    pub on_rail: bool,
    pub start_delay: f32,
    pub driver_appearance: Option<usize>,
    pub driver_pose: Pose,
    pub target: usize,
    pub previous: usize,
    pub blocked: f32,
}
impl Car {
    pub fn spec(&self) -> &'static crate::vehicle::Spec {
        crate::vehicle::spec(self.info_id)
    }
    pub fn is_bike(&self) -> bool {
        self.spec().is_bike()
    }
    pub fn visual_scale(&self) -> f32 {
        self.spec().visual_scale()
    }
    /// Rider world position when seated on an open vehicle (motorcycle / gokart).
    pub fn rider_pos(&self) -> Option<[f32; 3]> {
        if !self.is_bike() && self.info_id != 51 {
            return None;
        }
        let f = forward(self.angle);
        let side = [f[1], -f[0]];
        let seat_y = (self.spec().seat_y() * 0.32).clamp(-0.65, 0.45);
        let lean_x = self.roll.sin() * 0.42;
        let z_off = if self.is_bike() { 0.34 } else { 0.16 };
        Some([
            self.p[0] + f[0] * seat_y + side[0] * lean_x,
            self.p[1] + f[1] * seat_y + side[1] * lean_x,
            self.p[2] + z_off,
        ])
    }
    /// Authentic 3D rider positions for seated bike rider (cBike::GetPedRenderPos).
    /// Returns (upper_pos, legs_pos).
    pub fn rider_render_pos(&self) -> Option<([f32; 3], [f32; 3])> {
        if !self.is_bike() && self.info_id != 51 {
            return None;
        }
        let sy = self.spec().seat_y();
        let u_local = [0.0, sy, 0x1333 as f32 / 4096.0];
        let l_local = [0.0, sy + 0x0e00 as f32 / 4096.0, 0x0c00 as f32 / 4096.0];
        let u = transform_bike_point(self, u_local);
        let l = transform_bike_point(self, l_local);
        Some((u, l))
    }
}

fn transform_bike_point(car: &Car, local: [f32; 3]) -> [f32; 3] {
    let scale = car.visual_scale();
    let (px, py, pz) = (local[0] * scale, local[1] * scale, local[2] * scale);
    let (sr, cr) = car.roll.sin_cos();
    let x1 = px * cr + pz * sr;
    let y1 = py;
    let z1 = -px * sr + pz * cr;
    let (sp, cp) = car.pitch.sin_cos();
    let x2 = x1;
    let y2 = y1 * cp - z1 * sp;
    let z2 = y1 * sp + z1 * cp;
    let (sa, ca) = car.angle.sin_cos();
    let xw = x2 * ca - y2 * sa;
    let yw = x2 * sa + y2 * ca;
    [car.p[0] + xw, car.p[1] + yw, car.p[2] + z2]
}
pub struct Ped {
    pub p: [f32; 3],
    pub heading: f32,
    pub level: u8,
    pub pose: Pose,
    pub health: f32,
    pub hit_flash: f32,
    pub appearance: usize,
    pub attacking: f32,
    pub fall_forward: bool,
    pub dead_timer: f32,
    pub evade_roll: Option<Roll>,
    pub evading: bool,
    target: [f32; 3],
    home: [f32; 3],
    panic: f32,
    pub hostile: bool,
    cooldown: f32,
    blocked: f32,
    /// cWanderPath: node being walked to and the node it came from.
    cur: Option<u32>,
    prev: Option<u32>,
    mode: Mode,
}
#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Wander,
    /// cFleeOnFoot: along the network, or straight away when off it.
    Flee { straight: bool },
    Chase,
}
impl Ped {
    /// Walking along a pavement link (the radar-area constraint is not used).
    pub fn on_pavement(&self) -> bool {
        self.cur.is_some() && self.prev.is_some() && !matches!(self.mode, Mode::Flee { straight: true } | Mode::Chase)
    }
}
/// PSP pedestrian turn rate: 4000 angle units (65536 per turn) per 30 Hz tick.
const PED_TURN: f32 = 4000. / 65536. * TAU * 30.;
fn planar(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
pub struct Scene {
    pub pedestrian_areas: Arc<PedestrianAreas>,
    pub pavement: Arc<Network>,
    rng: Rng,
    pub cars: Vec<Car>,
    pub peds: Vec<Ped>,
    pub driving: Option<usize>,
    pub health: f32,
    pub armour: u8,
    pub hit_flash: f32,
    pub attack_timer: f32,
    pub kills: usize,
    pub hits: usize,
    pub vehicle_entries: usize,
    pub vehicle_exits: usize,
    pub collisions: usize,
    pub skidmarks: Vec<SkidStrip>,
    pub particles: Vec<Particle>,
    pub props: crate::props::PropsManager,
    pub bullet_traces: Vec<crate::weapons::BulletTrace>,
    pub targeting: crate::weapons::TargetingSystem,
    pub weapons: Option<crate::weapons::WeaponManager>,
    pub police: crate::police::PoliceManager,
    pub explosions: crate::explosions::ExplosionManager,
    pub radio: crate::radio::RadioManager,
    pub weather: crate::weather::WeatherManager,
    pub missions: crate::missions::MissionEngine,
    pub drugs: crate::drugs::DrugEconomy,
    pub safehouses: crate::safehouse::SafehouseManager,
    pub stats: crate::stats::Stats,
    pub save_progress: crate::stats::SaveProgress,
    pub pda: crate::pda::PdaEngine,
    pub minigame: Option<crate::minigames::MinigameSession>,
    pub jobs: crate::jobs::JobManager,
    pub collectibles: crate::collectibles::CollectibleManager,
    pub races: crate::races::RaceManager,
    pub active_race: Option<crate::races::RaceSession>,
    pub cash: u32,
    pub prompt: Option<String>,
    pub banner: Option<(String, f32)>,
    stream_timer: f32,
}

/// PSP ReduceArmour 0x089c967c: normal armour absorbs damage first and
/// returns only overflow to health. Double-armour reward is not implemented.
fn damage_player(health: &mut f32, armour: &mut u8, damage: u8) {
    let absorbed = (*armour).min(damage);
    *armour -= absorbed;
    *health = (*health - (damage - absorbed) as f32).max(0.);
}
pub(crate) fn forward(angle: f32) -> [f32; 2] {
    [-angle.sin(), angle.cos()]
}
fn heading(a: [f32; 3], b: [f32; 3]) -> f32 {
    -(b[0] - a[0]).atan2(b[1] - a[1])
}
fn wrapped(a: f32) -> f32 {
    (a + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI
}
fn move_towards(value: f32, target: f32, step: f32) -> f32 {
    value + (target - value).clamp(-step, step)
}

fn person_position(c: &Collision, mut p: [f32; 3]) -> Option<[f32; 3]> {
    let ground = c.ground(p[0], p[1], p[2]);
    if ground.water || (ground.z - p[2]).abs() > 0.55 {
        return None;
    }
    p[2] = ground.z;
    c.body_clear(p, 0.7, 2.).then_some(p)
}

fn car_position(c: &Collision, p: [f32; 3], angle: f32) -> Option<[f32; 3]> {
    car_position_for(c, p, angle, 0)
}

fn car_position_for(
    c: &Collision,
    mut p: [f32; 3],
    angle: f32,
    info_id: usize,
) -> Option<[f32; 3]> {
    let ground = c.ground(p[0], p[1], p[2]);
    if ground.water || (ground.z - p[2]).abs() > 0.55 {
        return None;
    }
    p[2] = ground.z;
    let f = forward(angle);
    let side = [f[1], -f[0]];
    let spec = crate::vehicle::spec(info_id);
    if spec.is_bike() || info_id == 51 {
        // Motorcycles (cBike) and Gokart have a narrower/shorter footprint.
        for longitudinal in [-1.65, 0., 1.65] {
            for lateral in [-0.20, 0.20] {
                let q = [
                    p[0] + f[0] * longitudinal + side[0] * lateral,
                    p[1] + f[1] * longitudinal + side[1] * lateral,
                    p[2],
                ];
                let g = c.ground(q[0], q[1], p[2]);
                if g.water || (g.z - p[2]).abs() > 0.55 || !c.body_clear(q, 0.55, CAR_HEIGHT) {
                    return None;
                }
            }
        }
        return Some(p);
    }
    // Overlapping circles cover the long Admiral footprint, not just its centre.
    for longitudinal in [-2.55, 0., 2.55] {
        for lateral in [-0.42, 0.42] {
            let q = [
                p[0] + f[0] * longitudinal + side[0] * lateral,
                p[1] + f[1] * longitudinal + side[1] * lateral,
                p[2],
            ];
            let g = c.ground(q[0], q[1], p[2]);
            if g.water || (g.z - p[2]).abs() > 0.55 || !c.body_clear(q, 0.8, CAR_HEIGHT) {
                return None;
            }
        }
    }
    Some(p)
}

fn cargen_position(
    c: &Collision,
    p: [f32; 3],
    angle: f32,
    ref_z: f32,
    info_id: usize,
) -> Option<[f32; 3]> {
    let z_hint = if p[2].abs() < 0.05 { ref_z } else { p[2] };
    let g0 = c.ground(p[0], p[1], z_hint);
    if g0.water || (g0.z - ref_z).abs() > 8.0 {
        return None;
    }
    let f = forward(angle);
    let side = [f[1], -f[0]];
    for (longitudinal, lateral) in [
        (0., 0.),
        (0., -0.5),
        (0., 0.5),
        (-0.6, 0.),
        (0.6, 0.),
        (0., -1.0),
        (0., 1.0),
    ] {
        let q = [
            p[0] + f[0] * longitudinal + side[0] * lateral,
            p[1] + f[1] * longitudinal + side[1] * lateral,
            g0.z,
        ];
        if let Some(valid) = car_position_for(c, q, angle, info_id) {
            return Some(valid);
        }
    }
    None
}

pub(crate) fn make_vehicle(
    p: [f32; 3],
    angle: f32,
    target: usize,
    previous: usize,
    info_id: usize,
    paint: u8,
    parked: bool,
    from_cargen: bool,
) -> Car {
    let spec = crate::vehicle::spec(info_id);
    let is_bike = spec.is_bike();
    let driver_app = if is_bike && !parked {
        Some(paint as usize % 12)
    } else {
        None
    };
    let driver_pose = if let Some(app) = driver_app {
        let mut pose = Pose::for_appearance(app);
        let base = if npc_is_male(app) { crate::ped::BODY_STRIDE } else { 0 };
        pose.upper = base + crate::ped::BIKE_RIDE_OFFSET;
        pose.legs = base + crate::ped::BIKE_RIDE_OFFSET + 1;
        pose
    } else {
        Pose::default()
    };
    Car {
        p,
        angle,
        speed: 0.,
        lat_vel: 0.,
        yaw_rate: 0.,
        steer_angle: 0.,
        gear: 0,
        rpm: 0.22,
        roll: 0.,
        pitch: 0.,
        roll_vel: 0.,
        pitch_vel: 0.,
        wheel_spin: false,
        front_skidding: false,
        rear_skidding: false,
        braking: false,
        headlights: true,
        info_id: spec.info_id,
        model_id: spec.model_id,
        name: spec.name,
        h: spec.half_extents(),
        paint,
        from_cargen,
        parked,
        lane: 0,
        wobble: 0.,
        next_lane: 0,
        next_wobble: 0.,
        rail_dist: 0.,
        next_target: target,
        on_rail: !parked,
        start_delay: 0.,
        driver_appearance: driver_app,
        driver_pose,
        target,
        previous,
        blocked: 0.,
    }
}

#[cfg(test)]
fn make_car(p: [f32; 3], angle: f32, target: usize, previous: usize, index: usize) -> Car {
    make_vehicle(p, angle, target, previous, 0, 0, index == 0, false)
}
pub(crate) fn make_ped(p: [f32; 3], target: [f32; 3], appearance: usize) -> Ped {
    Ped {
        p,
        target,
        home: p,
        heading: heading(p, target),
        level: if npc_is_jogger(appearance) { 2 } else { 1 },
        pose: Pose::for_appearance(appearance),
        health: 60.,
        hit_flash: 0.,
        appearance,
        attacking: 0.,
        fall_forward: false,
        dead_timer: 0.,
        evade_roll: None,
        evading: false,
        panic: 0.,
        hostile: false,
        cooldown: 0.,
        blocked: 0.,
        cur: None,
        prev: None,
        mode: Mode::Wander,
    }
}

impl Scene {
    pub fn new(
        c: &Collision,
        roads: &Roads,
        player: [f32; 3],
        areas: Arc<PedestrianAreas>,
        pavement: Arc<Network>,
    ) -> Self {
        // Deterministic per start point, so validation captures are reproducible.
        let seed = ((player[0] * 8.) as i32 as u32) ^ ((player[1] * 8.) as i32 as u32).rotate_left(16) ^ 0x9e37_79b9;
        let mut scene = Self {
            pedestrian_areas: areas.clone(),
            pavement: pavement.clone(),
            rng: Rng::new(seed),
            cars: Vec::new(),
            peds: Vec::new(),
            skidmarks: Vec::new(),
            particles: Vec::new(),
            props: crate::props::PropsManager::new(),
            bullet_traces: Vec::new(),
            targeting: crate::weapons::TargetingSystem::new(),
            weapons: None,
            police: crate::police::PoliceManager::new(),
            explosions: crate::explosions::ExplosionManager::new(),
            radio: crate::radio::RadioManager::new(None),
            weather: crate::weather::WeatherManager::new(None),
            missions: crate::missions::MissionEngine::new(),
            drugs: crate::drugs::DrugEconomy::new(),
            safehouses: crate::safehouse::SafehouseManager::new(),
            stats: crate::stats::Stats::new(),
            save_progress: crate::stats::SaveProgress::default(),
            pda: crate::pda::PdaEngine::new(),
            minigame: None,
            jobs: crate::jobs::JobManager::new(),
            collectibles: crate::collectibles::CollectibleManager::new(),
            races: crate::races::RaceManager::new(),
            active_race: None,
            cash: 500,
            prompt: None,
            banner: None,
            driving: None,
            health: 127.,
            armour: 0,
            hit_flash: 0.,
            attack_timer: 0.,
            kills: 0,
            hits: 0,
            vehicle_entries: 0,
            vehicle_exits: 0,
            collisions: 0,
            stream_timer: 0.,
        };
        let mut candidates = Vec::new();
        for (a, b) in roads.edges() {
            let x = roads.nodes[a].p;
            let y = roads.nodes[b].p;
            let l2 = (y[0] - x[0]).powi(2) + (y[1] - x[1]).powi(2);
            if l2 < 16. {
                continue;
            }
            let projection =
                (((player[0] - x[0]) * (y[0] - x[0]) + (player[1] - x[1]) * (y[1] - x[1])) / l2)
                    .clamp(0.08, 0.92);
            for t in [projection, 0.22, 0.5, 0.78] {
                let p = std::array::from_fn(|i| x[i] + (y[i] - x[i]) * t);
                let d = distance(player, p);
                if d < 100. {
                    candidates.push((d, p, heading(x, y), a, b));
                }
            }
        }
        candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut car_rng = Rng::new(seed ^ 0x5bd1_e995);
        // The first parked car is accessible immediately, even when the closest
        // road node is far down a long street. Use world clearance, never teleport
        // a car into a wall or the water to satisfy a visual count.
        let road_angle = candidates.first().map(|v| v.2).unwrap_or(0.);
        for radius in [7., 10., 14., 18.] {
            for i in 0..12 {
                let a = i as f32 * TAU / 12.;
                let p = [
                    player[0] + a.sin() * radius,
                    player[1] + a.cos() * radius,
                    player[2],
                ];
                if let Some(p) = car_position(c, p, road_angle) {
                    let target = candidates.first().map(|v| v.4).unwrap_or(0);
                    let previous = candidates.first().map(|v| v.3).unwrap_or(0);
                    let paint = crate::vehicle::choose_paint(0, 0xff, &mut car_rng);
                    scene.cars.push(make_vehicle(
                        p, road_angle, target, previous, 0, paint, true, false,
                    ));
                    break;
                }
            }
            if !scene.cars.is_empty() {
                break;
            }
        }
        // Also place a parked motorcycle (cBike: NRG900 / Sanchez / Angel / Chopper)
        // on the street behind the first car so the player can test motorcycle physics
        // immediately without blocking the Admiral's forward lane.
        let bike_id = crate::vehicle::BIKE_POOL
            [car_rng.below(crate::vehicle::BIKE_POOL.len() as u32) as usize];
        let bike_paint = crate::vehicle::choose_paint(bike_id, 0xff, &mut car_rng);
        let rf = forward(road_angle);
        let rside = [rf[1], -rf[0]];
        let base_p = scene.cars.first().map(|c| c.p).unwrap_or(player);
        for back in [8.5, 11.0, 14.0, 17.5] {
            let mut placed = false;
            for lat in [0.0, 2.2, -2.2] {
                let p = [
                    base_p[0] - rf[0] * back + rside[0] * lat,
                    base_p[1] - rf[1] * back + rside[1] * lat,
                    player[2],
                ];
                if scene.cars.iter().any(|car| distance(car.p, p) < 7.5) {
                    continue;
                }
                if let Some(p) = car_position_for(c, p, road_angle, bike_id) {
                    let target = candidates.first().map(|v| v.4).unwrap_or(0);
                    let previous = candidates.first().map(|v| v.3).unwrap_or(0);
                    scene.cars.push(make_vehicle(
                        p, road_angle, target, previous, bike_id, bike_paint, true, false,
                    ));
                    placed = true;
                    break;
                }
            }
            if placed {
                break;
            }
        }
        // GenerateRandomPed / GetCreatePosition: civilians start on a pavement
        // link and walk from its first node to the second (cWanderPath). The
        // slice fills a 9-90 unit ring at once instead of the original trickle.
        for (q, from, to) in pavement.spawn_points(player, 9., 90., &mut scene.rng) {
            if scene.peds.len() >= 20 {
                break;
            }
            if scene.peds.iter().any(|ped| distance(ped.p, q) < 8.)
                || scene.cars.iter().any(|car| point_in_car(car, q, 1.5))
            {
                continue;
            }
            if let Some(q) = person_position(c, q) {
                let target = pavement.wander_target(to, &mut scene.rng);
                let mut ped = make_ped(q, target, scene.peds.len());
                ped.prev = Some(from);
                ped.cur = Some(to);
                scene.peds.push(ped);
            }
        }
        for &(_, p, a, from, to) in &candidates {
            if scene.cars.len() < 9
                && distance(p, player) > 12.
                && scene.cars.iter().all(|car| distance(car.p, p) > 12.)
                && scene.peds.iter().all(|ped| distance(ped.p, p) > 4.5)
            {
                let p_from = roads.nodes[from].p;
                let p_to = roads.nodes[to].p;
                let seg_len = (p_to[0] - p_from[0]).hypot(p_to[1] - p_from[1]).max(0.1);
                let (lane, wobble) = roads.select_lane(from, to, car_rng.below(0xffff_ffff));
                let t = 0.5;
                let p_lane = roads.lane_world_pos(from, to, t, lane, wobble);
                let a_lane = heading(p_from, p_to);
                let info_id = crate::vehicle::choose_traffic_vehicle(p_lane, &mut car_rng);
                if let Some(valid_p) = car_position_for(c, p_lane, a_lane, info_id) {
                    let paint = crate::vehicle::choose_paint(info_id, 0xff, &mut car_rng);
                    let mut veh = make_vehicle(valid_p, a_lane, to, from, info_id, paint, false, false);
                    veh.lane = lane;
                    veh.wobble = wobble;
                    veh.rail_dist = seg_len * t;
                    veh.on_rail = true;
                    if let Some(to_node) = roads.nodes.get(to) {
                        let next_cand = to_node
                            .links
                            .iter()
                            .copied()
                            .filter(|&n| roads.nodes[n].land() && n != from)
                            .nth(scene.cars.len() % to_node.links.len().max(1))
                            .or_else(|| to_node.links.iter().copied().find(|&n| roads.nodes[n].land()));
                        if let Some(nt) = next_cand {
                            veh.next_target = nt;
                            let (nl, nw) = roads.convert_lane(from, to, nt, lane, car_rng.below(0xffff_ffff));
                            veh.next_lane = nl;
                            veh.next_wobble = nw;
                        }
                    }
                    scene.cars.push(veh);
                }
            }
            if scene.peds.len() < 12 {
                let f = forward(a);
                for side in [-22., -17., -12., 12., 17., 22.] {
                    let q = [p[0] + f[1] * side, p[1] - f[0] * side, p[2]];
                    if distance(q, player) > 9.
                        && scene.peds.iter().all(|ped| distance(ped.p, q) > 8.)
                        && scene.cars.iter().all(|car| distance(car.p, q) > 4.5)
                    {
                        if let Some(q) = person_position(c, q).filter(|p| areas.contains(*p)) {
                            let target = [q[0] + f[0] * 12., q[1] + f[1] * 12., q[2]];
                            scene.peds.push(make_ped(q, target, scene.peds.len()));
                        }
                    }
                }
            }
            if scene.cars.len() >= 9 && scene.peds.len() >= 20 {
                break;
            }
        }
        // Populate nearby walkable ground as well as road edges, so a plaza or
        // car park is not empty at the initial presentation viewpoint.
        for radius in [18., 28., 40., 55., 70., 90.] {
            for i in 0..16 {
                if scene.peds.len() >= 20 {
                    break;
                }
                let a = (i as f32 + 0.3) * TAU / 16.;
                let p = [
                    player[0] + a.sin() * radius,
                    player[1] + a.cos() * radius,
                    player[2],
                ];
                if scene.peds.iter().any(|ped| distance(ped.p, p) < 9.)
                    || scene.cars.iter().any(|car| distance(car.p, p) < 5.)
                {
                    continue;
                }
                if let Some(p) = person_position(c, p).filter(|p| areas.contains(*p)) {
                    let target = [p[0] + a.cos() * 8., p[1] - a.sin() * 8., p[2]];
                    scene.peds.push(make_ped(p, target, scene.peds.len()));
                }
            }
        }
        // cWorldStream::ProcessCarGen: spawn parked vehicles from world.bin
        // section 7 (CarGen) within 150 units of the player, preserving their
        // PSP vehicle type (cars, motorcycles, special land vehicles & tank), heading and paint.
        let mut cargens: Vec<_> = c
            .cells
            .iter()
            .flat_map(|cell| &cell.car_gens)
            .filter(|gen| gen.flags & 0x80 == 0 && gen.p[2] >= -1.0)
            .filter_map(|gen| {
                let d = planar(gen.p, player);
                (10.0..=150.0).contains(&d).then_some((d, *gen))
            })
            .collect();
        cargens.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut cargen_spawned = 0usize;
        for (_, gen) in cargens {
            if cargen_spawned >= 4 || scene.cars.len() >= 13 {
                break;
            }
            let info_id = if gen.flags & 1 != 0
                || usize::from(gen.vehicle) >= crate::vehicle::SPECS.len()
                || !crate::vehicle::spec(usize::from(gen.vehicle)).is_land_vehicle()
            {
                crate::vehicle::choose_traffic_vehicle(gen.p, &mut car_rng)
            } else {
                usize::from(gen.vehicle)
            };
            let paint = crate::vehicle::choose_paint(info_id, gen.colour, &mut car_rng);
            let mut angle = gen.angle();
            if gen.flags & 2 != 0 {
                angle = wrapped(angle + (car_rng.below(21) as i32 - 10) as f32 * 0.015);
            }
            if let Some(valid) = cargen_position(c, gen.p, angle, player[2], info_id) {
                let target = candidates.first().map(|v| v.4).unwrap_or(0);
                let previous = candidates.first().map(|v| v.3).unwrap_or(0);
                let candidate = make_vehicle(
                    valid, angle, target, previous, info_id, paint, true, true,
                );
                if scene
                    .cars
                    .iter()
                    .all(|car| distance(car.p, valid) > 3.2 && !cars_overlap(car, &candidate))
                    && scene.peds.iter().all(|ped| distance(ped.p, valid) > 4.0)
                {
                    scene.cars.push(candidate);
                    cargen_spawned += 1;
                }
            }
        }
        scene
    }

    /// Temporarily expose the moving cars to the existing foot/vault controller.
    /// Restore the original cell contents before updating the vehicle simulation.
    pub fn with_car_collision<T>(
        &self,
        collision: &mut Collision,
        step: impl FnOnce(&Collision) -> T,
    ) -> T {
        let mut restore = Vec::new();
        for (i, car) in self.cars.iter().enumerate() {
            if self.driving == Some(i) {
                continue;
            }
            if let Some((x, y)) = Collision::cell_xy(car.p[0], car.p[1]) {
                let index = x * 100 + y;
                if !restore.iter().any(|&(cell, _)| cell == index) {
                    restore.push((index, collision.cells[index].boxes.len()));
                }
                collision.cells[index].boxes.push(BoxShape {
                    c: [car.p[0], car.p[1], car.p[2] + 1.16458],
                    h: [car.h[0], car.h[1], 0.8208],
                    angle: car.angle,
                    flags: 0,
                });
            }
        }
        let result = step(collision);
        for (index, len) in restore {
            collision.cells[index].boxes.truncate(len);
        }
        result
    }

    pub fn nearest_car(&self, player: [f32; 3]) -> Option<usize> {
        self.cars
            .iter()
            .enumerate()
            .filter(|(_, car)| car.speed.abs() < 4. && (car.p[2] - player[2]).abs() < 2.)
            .filter_map(|(i, car)| {
                let d = distance(car.p, player);
                (d < 6.5).then_some((i, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|v| v.0)
    }

    pub fn toggle_vehicle(&mut self, c: &Collision, player: [f32; 3]) -> Option<[f32; 3]> {
        if let Some(index) = self.driving {
            let car = &self.cars[index];
            if car.speed.abs() > 4. {
                return None;
            }
            let f = forward(car.angle);
            for (side, longitudinal) in [(-3., 0.), (3., 0.), (-3., -3.), (3., -3.), (0., -5.5)] {
                let p = [
                    car.p[0] + f[1] * side + f[0] * longitudinal,
                    car.p[1] - f[0] * side + f[1] * longitudinal,
                    car.p[2],
                ];
                if self
                    .cars
                    .iter()
                    .enumerate()
                    .any(|(i, c)| i != index && point_in_car(c, p, 0.9))
                {
                    continue;
                }
                if let Some(p) = person_position(c, p) {
                    let car = &mut self.cars[index];
                    car.speed = 0.;
                    car.lat_vel = 0.;
                    car.yaw_rate = 0.;
                    car.steer_angle = 0.;
                    car.roll = 0.;
                    car.pitch = 0.;
                    car.roll_vel = 0.;
                    car.pitch_vel = 0.;
                    car.wheel_spin = false;
                    car.parked = true;
                    self.driving = None;
                    self.vehicle_exits += 1;
                    return Some(p);
                }
            }
            None
        } else {
            let index = self.nearest_car(player)?;
            // Entry must not reach through a building wall.
            let delta = [
                self.cars[index].p[0] - player[0],
                self.cars[index].p[1] - player[1],
            ];
            let steps = (delta[0].hypot(delta[1]) / 0.5).ceil() as usize;
            if (1..=steps).any(|i| {
                let t = i as f32 / steps.max(1) as f32;
                person_position(
                    c,
                    [
                        player[0] + delta[0] * t,
                        player[1] + delta[1] * t,
                        player[2],
                    ],
                )
                .is_none()
            }) {
                return None;
            }
            self.driving = Some(index);
            let car = &mut self.cars[index];
            car.parked = false;
            car.speed = 0.;
            car.lat_vel = 0.;
            car.yaw_rate = 0.;
            car.wheel_spin = false;
            self.attack_timer = 0.;
            self.vehicle_entries += 1;
            Some(self.cars[index].p)
        }
    }

    /// Cooldown and a forward cone prevent a held button or distant NPC from
    /// causing an instant kill. Collision samples reject attacks through walls.
    pub fn attack(&mut self, c: &Collision, player: [f32; 3], angle: f32) -> bool {
        if self.driving.is_some() || self.health <= 0. || self.attack_timer > 0. {
            return false;
        }
        let f = forward(angle);

        let is_firearm = self
            .weapons
            .as_ref()
            .and_then(|wm| wm.current())
            .is_some_and(|w| !w.is_melee());

        if is_firearm {
            if let Some(wm) = &mut self.weapons {
                let Some(w) = wm.current_mut() else {
                    return false;
                };
                if w.timer > 0.0 {
                    return false;
                }
                if w.clip == 0 {
                    if w.ammo != 0 {
                        w.state = crate::weapons::WeaponState::Reloading;
                        w.timer = 1.2;
                    }
                    return false;
                }
                w.clip -= 1;
                if w.ammo > 0 {
                    w.ammo -= 1;
                }
                w.timer = 0.18;
                w.state = crate::weapons::WeaponState::FireDelay;
                self.attack_timer = 0.18;
            }

            let muzzle = [player[0] + f[0] * 0.9, player[1] + f[1] * 0.9, player[2] + 1.2];
            let (target_pos, hit_idx) = match self.targeting.locked_target {
                Some(crate::weapons::TargetKind::Ped(idx)) if idx < self.peds.len() => {
                    (self.peds[idx].p, Some(idx))
                }
                _ => (
                    [player[0] + f[0] * 32.0, player[1] + f[1] * 32.0, player[2] + 1.0],
                    None,
                ),
            };

            self.bullet_traces.push(crate::weapons::BulletTrace {
                start: muzzle,
                end: target_pos,
                life: 0.12,
                max_life: 0.12,
                kind: 0,
            });

            if self.particles.len() < 160 {
                self.particles.push(Particle {
                    p: muzzle,
                    v: [f[0] * 0.5, f[1] * 0.5, 0.2],
                    life: 0.0,
                    max_life: 0.06,
                    size: 0.22,
                    grow: 0.5,
                    kind: 0,
                });
            }

            if let Some(idx) = hit_idx {
                let ped = &mut self.peds[idx];
                ped.health = (ped.health - 45.0).max(0.0);
                ped.hit_flash = 0.25;
                ped.panic = 8.0;
                self.hits += 1;
                self.police.report_crime(crate::police::CrimeType::PedHit);
                if ped.health <= 0.0 {
                    let pf = forward(ped.heading);
                    ped.fall_forward = f[0] * pf[0] + f[1] * pf[1] > 0.0;
                    ped.dead_timer = 0.0;
                    ped.evade_roll = None;
                    self.kills += 1;
                    self.police.report_crime(crate::police::CrimeType::PedKill);
                }
            }
            return true;
        }

        self.attack_timer = 0.48;
        let target = self
            .peds
            .iter()
            .enumerate()
            .filter_map(|(i, ped)| {
                let dx = ped.p[0] - player[0];
                let dy = ped.p[1] - player[1];
                let d = dx.hypot(dy);
                let visible = (1..=5).all(|step| {
                    let t = step as f32 / 5.;
                    c.body_clear(
                        [player[0] + dx * t, player[1] + dy * t, player[2]],
                        0.2,
                        1.5,
                    )
                });
                (ped.health > 0.
                    && d < 3.4
                    && (ped.p[2] - player[2]).abs() < 1.5
                    && (d < 0.8 || (dx * f[0] + dy * f[1]) / d > 0.2)
                    && visible)
                    .then_some((i, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let Some((index, _)) = target else {
            return false;
        };
        let ped = &mut self.peds[index];
        ped.health = (ped.health - 30.).max(0.);
        ped.hit_flash = 0.25;
        ped.panic = 8.;
        // cPed::GetNewState: female civilians (bodySet 0) always flee instead of fighting.
        ped.hostile = npc_is_male(ped.appearance) && ped.appearance % 3 == 0;
        ped.cooldown = 0.75;
        let pushed = [ped.p[0] + f[0] * 0.7, ped.p[1] + f[1] * 0.7, ped.p[2]];
        let on_pavement = ped.on_pavement();
        if let Some(p) = person_position(c, pushed)
            .filter(|p| on_pavement || self.pedestrian_areas.contains(*p))
        {
            ped.p = p;
        }
        self.hits += 1;
        self.police.report_crime(crate::police::CrimeType::PedHit);
        if ped.health <= 0. {
            let pf = forward(ped.heading);
            ped.fall_forward = f[0] * pf[0] + f[1] * pf[1] > 0.;
            ped.dead_timer = 0.;
            ped.evade_roll = None;
            self.kills += 1;
            self.police.report_crime(crate::police::CrimeType::PedKill);
        }
        for other in &mut self.peds {
            if distance(other.p, player) < 14. && other.health > 0. && !other.hostile {
                other.panic = 5.;
            }
        }
        true
    }

    /// Handles world interaction on foot with Story Mission contacts, Safehouses, and Drug Dealers.
    pub fn interact(&mut self, player_pos: [f32; 3]) -> bool {
        // 0. Active Minigame action trigger (strike glass, twist wire, pump fuel, scratch)
        if let Some(minigame) = &mut self.minigame {
            let done = minigame.trigger_action();
            if let Some(msg) = &minigame.reward_message {
                self.banner = Some((msg.clone(), 3.5));
            }
            if done {
                self.cash = self.cash.saturating_add(minigame.reward_cash);
                self.minigame = None;
                if let crate::missions::MissionState::InProgress { objective: crate::missions::MissionObjective::CompleteMinigame { .. }, .. } = &self.missions.state {
                    self.missions.advance_objective(&mut self.cash);
                }
            }
            return true;
        }

        // 1. Story Mission Contacts
        for &contact in &[
            crate::missions::Contact::UncleKenny,
            crate::missions::Contact::ChanJaoming,
            crate::missions::Contact::ZhouMing,
            crate::missions::Contact::HsinJaoming,
            crate::missions::Contact::WadeHeston,
            crate::missions::Contact::LesterLeroc,
            crate::missions::Contact::RudyDAvanzo,
            crate::missions::Contact::XinShan,
        ] {
            let m_pos = contact.meeting_pos();
            if planar(player_pos, m_pos) < 6.0 {
                if let Some(m) = self.missions.catalog.iter().find(|m| m.contact == contact) {
                    let m_id = m.id;
                    let m_title = m.title;
                    if self.missions.start_mission(m_id).is_ok() {
                        let _ = self.missions.begin_gameplay();
                        if let crate::missions::MissionState::InProgress { objective: crate::missions::MissionObjective::CompleteMinigame { overlay_name }, .. } = &self.missions.state {
                            if *overlay_name == "sinkingcarapp" {
                                self.minigame = Some(crate::minigames::MinigameSession::new_sinking_car());
                            } else if *overlay_name == "hotwiringapp" {
                                self.minigame = Some(crate::minigames::MinigameSession::new_hotwire(crate::minigames::HotwireMethod::WiresTwist));
                            }
                        }
                        self.banner = Some((format!("MISSION: {} ({})", m_title, contact.name()), 4.0));
                        return true;
                    }
                }
            }
        }

        // 2. Safehouses (Sleep in bed to save game & restore health, or purchase unowned real estate)
        for safehouse in &mut self.safehouses.properties {
            if planar(player_pos, safehouse.pos) < 5.5 {
                if safehouse.owned {
                    self.health = 127.0;
                    self.stats.inc_number_of_times_saved();
                    self.banner = Some(("LIT DE SAUVEGARDE: SANTE RESTAUREE (+6H)".to_string(), 3.5));
                    return true;
                } else if self.cash >= safehouse.price {
                    let price = safehouse.price;
                    safehouse.owned = true;
                    self.cash -= price;
                    self.stats.add_spent_on_property(price);
                    self.banner = Some((format!("PLANQUE ACHETEE: {} (-${})", safehouse.name, price), 4.0));
                    return true;
                } else {
                    self.banner = Some((format!("FONDS INSUFFISANTS: {} COUTE ${}", safehouse.name, safehouse.price), 3.0));
                    return true;
                }
            }
        }

        // 3. Drug Dealers (Trade wholesale commodities)
        for idx in 0..self.drugs.dealers.len() {
            let d_pos = self.drugs.dealers[idx].pos;
            if planar(player_pos, d_pos) < 4.0 {
                let dealer_gang = self.drugs.dealers[idx].gang;
                let dealer_name = self.drugs.dealers[idx].name;
                let want = dealer_gang.specialty_buy();
                let sell = dealer_gang.specialty_sale();
                if self.drugs.player_bag.get(want) > 0 {
                    let qty = self.drugs.player_bag.get(want).min(10);
                    if let Ok(earned) = self.drugs.player_sell(idx, want, qty, &mut self.cash) {
                        self.stats.inc_trades_done();
                        self.stats.add_units_of_goods_sold(qty);
                        self.stats.add_value_of_goods_sold(earned);
                        self.banner = Some((format!("VENTE {} x{}: +${} ({})", want.name(), qty, earned, dealer_name), 3.5));
                        return true;
                    }
                }
                if self.cash >= 200 && self.drugs.dealers[idx].stock.get(sell) > 0 {
                    let qty = 5.min(self.drugs.dealers[idx].stock.get(sell));
                    if let Ok(cost) = self.drugs.player_buy(idx, sell, qty, &mut self.cash) {
                        self.stats.inc_trades_done();
                        self.stats.add_units_of_goods_bought(qty);
                        self.stats.add_value_of_goods_bought(cost);
                        self.banner = Some((format!("ACHAT {} x{}: -${} ({})", sell.name(), qty, cost, dealer_name), 3.5));
                        return true;
                    }
                }
                self.banner = Some((format!("DEALER: {} ({}) - STOCK EPUISE", dealer_name, dealer_gang.name()), 2.5));
                return true;
            }
        }

        // 4. Vehicle Ambient Jobs (Taxi, Vigilante, Firefighter, Paramedic, Noodle Delivery)
        if let Some(car_idx) = self.driving {
            let car_name = self.cars[car_idx].name;
            if self.jobs.is_active() {
                self.jobs.cancel_job();
                self.banner = Some(("MISSION DE SERVICE ANNULÉE".to_string(), 3.0));
                return true;
            } else if let Some(job_kind) = crate::jobs::JobKind::from_vehicle(car_name) {
                if self.jobs.try_start_job(job_kind, car_name, player_pos) {
                    if let Some(msg) = self.jobs.banner_message.take() {
                        self.banner = Some(msg);
                    }
                    return true;
                }
            }
        }

        // 4b. Street Races (Overlay 140 streetrace)
        if self.driving.is_some() {
            if let Some(race) = &self.active_race {
                if matches!(race.state, crate::races::RaceState::Finished { .. } | crate::races::RaceState::Failed { .. }) {
                    self.active_race = None;
                    return true;
                }
            } else if !self.jobs.is_active() {
                for circuit in &self.races.circuits {
                    let start_cp = circuit.checkpoints[0];
                    if planar(player_pos, start_cp) < 20.0 {
                        let id = circuit.id;
                        match self.races.start_race(id, &mut self.cash) {
                            Ok(session) => {
                                self.banner = session.banner_message.clone();
                                self.active_race = Some(session);
                                return true;
                            }
                            Err(err) => {
                                self.banner = Some((err.to_string(), 3.0));
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // 5. Dumpster Stash Search
        if self.driving.is_none() {
            if let Some((loot_name, _kind)) = self.collectibles.try_search_dumpster(player_pos, &mut self.cash) {
                if let Some(msg) = self.collectibles.banner_message.take() {
                    self.banner = Some(msg);
                } else {
                    self.banner = Some((format!("FOUILLE DE BENNE : {}", loot_name), 3.5));
                }
                return true;
            }
        }

        false
    }

    pub fn tick(
        &mut self,
        c: &Collision,
        roads: &Roads,
        assets: &PedAssets,
        player: [f32; 3],
        dt: f32,
        input: DriveInput,
    ) -> Option<[f32; 3]> {
        let dt = dt.clamp(0., 0.1);
        self.attack_timer = (self.attack_timer - dt).max(0.);
        self.hit_flash = (self.hit_flash - dt).max(0.);
        let previous: Vec<_> = self.cars.iter().map(|car| (car.p, car.angle, car.h[0])).collect();
        for i in 0..self.cars.len() {
            if self.driving == Some(i) {
                let control = if self.health > 0. {
                    input
                } else {
                    DriveInput {
                        brake: true,
                        ..DriveInput::default()
                    }
                };
                let prev_speed = self.cars[i].speed;
                if step_car(&mut self.cars[i], c, control, dt) {
                    self.collisions += 1;
                    if self.cars[i].is_bike() && prev_speed.abs() > 7.5 {
                        let bike = &mut self.cars[i];
                        bike.roll = 0.58;
                        bike.parked = true;
                        bike.speed = 0.;
                        bike.lat_vel = 0.;
                        bike.yaw_rate = 0.;
                        let crash_damage = ((prev_speed.abs() - 7.0) * 3.2).clamp(12.0, 60.0) as u8;
                        damage_player(&mut self.health, &mut self.armour, crash_damage);
                        self.hit_flash = 0.35;
                        self.driving = None;
                    }
                }
                if let Some(driving_idx) = self.driving {
                    self.props.check_vehicle_impact(&mut self.cars[driving_idx], dt, &mut self.particles);
                }
            } else if !self.cars[i].parked {
                let car = &mut self.cars[i];
                if car.on_rail {
                    if car.start_delay > 0.0 {
                        car.start_delay = (car.start_delay - dt).max(0.0);
                        car.speed = 0.0;
                    }
                    let look_ahead = 5.0 + (car.speed.max(0.0) * 0.95);
                    let cf = forward(car.angle);
                    let mut obstacle = false;

                    // Obstacle: player
                    let p_dx = player[0] - car.p[0];
                    let p_dy = player[1] - car.p[1];
                    let p_dot = p_dx * cf[0] + p_dy * cf[1];
                    if p_dot > 0.0 && p_dot < look_ahead + 2.0 {
                        let p_lat = (p_dx * cf[1] - p_dy * cf[0]).abs();
                        if p_lat < CAR_HALF_WIDTH + 1.2 {
                            obstacle = true;
                        }
                    }

                    // Obstacle: peds
                    if !obstacle {
                        for ped in &self.peds {
                            if ped.health <= 0.0 {
                                continue;
                            }
                            let dx = ped.p[0] - car.p[0];
                            let dy = ped.p[1] - car.p[1];
                            let dot = dx * cf[0] + dy * cf[1];
                            if dot > 0.0 && dot < look_ahead {
                                let lat = (dx * cf[1] - dy * cf[0]).abs();
                                if lat < CAR_HALF_WIDTH + 0.95 {
                                    obstacle = true;
                                    break;
                                }
                            }
                        }
                    }

                    // Obstacle: other cars
                    if !obstacle {
                        for (j, &(other_p, _, other_hx)) in previous.iter().enumerate() {
                            if j == i {
                                continue;
                            }
                            let dx = other_p[0] - car.p[0];
                            let dy = other_p[1] - car.p[1];
                            let dot = dx * cf[0] + dy * cf[1];
                            if dot > 0.0 && dot < look_ahead + 2.5 {
                                let lat = (dx * cf[1] - dy * cf[0]).abs();
                                if lat < CAR_HALF_WIDTH + other_hx + 0.45 {
                                    obstacle = true;
                                    break;
                                }
                            }
                        }
                    }

                    if obstacle {
                        car.blocked += dt;
                        car.braking = true;
                        car.speed = move_towards(car.speed, 0.0, 16.0 * dt);
                        if car.speed.abs() < 0.25 {
                            car.speed = 0.0;
                            car.start_delay = 0.35;
                        }
                        if car.blocked > 0.5 {
                            if let Some(target_node) = roads.nodes.get(car.target) {
                                let m = target_node.meta();
                                if m.lanes > 1 && !m.flag5 {
                                    let alt_lane = if car.lane == 0 { 1 } else { 0 };
                                    let alt_clear = previous.iter().enumerate().all(|(j, &(other_p, _, _))| {
                                        if j == i {
                                            return true;
                                        }
                                        let dx = other_p[0] - car.p[0];
                                        let dy = other_p[1] - car.p[1];
                                        let dot = dx * cf[0] + dy * cf[1];
                                        if dot > -2.0 && dot < look_ahead + 2.0 {
                                            let lat = (dx * cf[1] - dy * cf[0]).abs();
                                            lat > m.half_width * 0.70
                                        } else {
                                            true
                                        }
                                    });
                                    if alt_clear {
                                        car.lane = alt_lane;
                                        car.next_lane = alt_lane;
                                        car.wobble = 0.0;
                                        car.blocked = 0.0;
                                        car.start_delay = 0.0;
                                    }
                                }
                            }
                        }
                    } else if car.start_delay <= 0.0 {
                        car.blocked = 0.0;
                        car.braking = false;
                        let cruise_speed = 9.0;
                        car.speed = move_towards(car.speed, cruise_speed, 6.0 * dt);
                        car.rail_dist += car.speed * dt;
                        let (new_p, new_angle, total_len, _) = roads.rail_pose(
                            car.previous,
                            car.target,
                            car.next_target,
                            car.rail_dist,
                            car.lane,
                            car.wobble,
                            car.next_lane,
                            car.next_wobble,
                        );
                        if car.rail_dist >= total_len {
                            let a = car.previous;
                            let b = car.target;
                            let c_target = car.next_target;
                            if let Some(node) = roads.nodes.get(c_target) {
                                let next_cand = node
                                    .links
                                    .iter()
                                    .copied()
                                    .filter(|&n| roads.nodes[n].land() && n != b)
                                    .nth(i % node.links.len().max(1))
                                    .or_else(|| {
                                        node.links.iter().copied().find(|&n| roads.nodes[n].land())
                                    });
                                if let Some(nt) = next_cand {
                                    let entry_off = roads.turn_entry_offset(
                                        a,
                                        b,
                                        c_target,
                                        car.lane,
                                        car.next_lane,
                                    );
                                    car.previous = b;
                                    car.target = c_target;
                                    car.lane = car.next_lane;
                                    car.wobble = car.next_wobble;
                                    car.next_target = nt;
                                    let (nl, nw) =
                                        roads.convert_lane(b, c_target, nt, car.lane, (i as u32).wrapping_mul(0x9e37_79b9));
                                    car.next_lane = nl;
                                    car.next_wobble = nw;
                                    car.rail_dist = entry_off;
                                } else {
                                    car.parked = true;
                                    car.on_rail = false;
                                }
                            } else {
                                car.parked = true;
                                car.on_rail = false;
                            }
                        }
                        if let Some(valid) = car_position_for(c, new_p, new_angle, car.info_id) {
                            car.p = valid;
                            car.angle = new_angle;
                        }
                    }
                } else {
                    if let Some(node) = roads.nodes.get(car.target) {
                        if distance(car.p, node.p) < 6.0 {
                            let next = node
                                .links
                                .iter()
                                .copied()
                                .filter(|&n| roads.nodes[n].land() && n != car.previous)
                                .nth(i % node.links.len().max(1))
                                .or_else(|| {
                                    node.links.iter().copied().find(|&n| roads.nodes[n].land())
                                });
                            if let Some(next) = next {
                                car.previous = car.target;
                                car.target = next;
                                car.lane = 0;
                                car.wobble = 0.0;
                                car.rail_dist = 0.0;
                                car.next_target = next;
                                car.on_rail = true;
                            } else {
                                car.parked = true;
                            }
                        }
                    }
                    if let Some(target) = roads.nodes.get(car.target) {
                        let diff = wrapped(heading(car.p, target.p) - car.angle);
                        let steer = (-diff * 1.6).clamp(-1., 1.);
                        let throttle = if car.speed < 8.5 { 0.55 } else { 0. };
                        if step_car(
                            car,
                            c,
                            DriveInput {
                                throttle,
                                steer,
                                brake: diff.abs() > 1.7,
                            },
                            dt,
                        ) {
                            car.blocked += dt;
                        } else {
                            car.blocked = 0.;
                        }
                        if car.blocked > 2.0 {
                            car.parked = true;
                        }
                    }
                }
            }
        }
        // Resolve vehicle-to-vehicle contacts with mass-weighted momentum transfer
        // (calcImpactCar: PAK 29 mass differentiates bikes, cars, trucks and Rhino tank).
        for i in 0..self.cars.len() {
            for j in i + 1..self.cars.len() {
                if cars_overlap(&self.cars[i], &self.cars[j]) {
                    let vi = self.cars[i].speed;
                    let vj = self.cars[j].speed;
                    let mi = self.cars[i].spec().mass();
                    let mj = self.cars[j].spec().mass();
                    let mtot = (mi + mj).max(0.5);
                    (self.cars[i].p, self.cars[i].angle) = (previous[i].0, previous[i].1);
                    (self.cars[j].p, self.cars[j].angle) = (previous[j].0, previous[j].1);
                    let dx = self.cars[j].p[0] - self.cars[i].p[0];
                    let dy = self.cars[j].p[1] - self.cars[i].p[1];
                    let d = dx.hypot(dy).max(1e-4);
                    let (nx, ny) = (dx / d, dy / d);
                    self.cars[i].lat_vel = 0.;
                    self.cars[j].lat_vel = 0.;
                    self.cars[i].pitch_vel -= 0.45 / mi.sqrt();
                    self.cars[j].pitch_vel -= 0.45 / mj.sqrt();
                    if mi > mj * 1.35 && vi.abs() > vj.abs() && vi.abs() > 2.0 {
                        let push = (1.35 * mi / mtot).clamp(0.4, 1.5);
                        let pushed = [
                            self.cars[j].p[0] + nx * push,
                            self.cars[j].p[1] + ny * push,
                            self.cars[j].p[2],
                        ];
                        let info_j = self.cars[j].info_id;
                        let angle_j = self.cars[j].angle;
                        if let Some(valid) = car_position_for(c, pushed, angle_j, info_j) {
                            let mut probe = make_vehicle(
                                valid, angle_j, 0, 0, info_j, 0, true, false,
                            );
                            probe.h = self.cars[j].h;
                            let clear = self
                                .cars
                                .iter()
                                .enumerate()
                                .all(|(k, other)| k == j || !cars_overlap(&probe, other))
                                && self.peds.iter().all(|ped| !point_in_car(&probe, ped.p, 0.65));
                            if clear {
                                self.cars[j].p = valid;
                            }
                        }
                        self.cars[i].speed = vi * ((mi - mj) / mtot).clamp(0.18, 0.72);
                        self.cars[j].speed = 0.;
                    } else if mj > mi * 1.35 && vj.abs() > vi.abs() && vj.abs() > 2.0 {
                        let push = (1.35 * mj / mtot).clamp(0.4, 1.5);
                        let pushed = [
                            self.cars[i].p[0] - nx * push,
                            self.cars[i].p[1] - ny * push,
                            self.cars[i].p[2],
                        ];
                        let info_i = self.cars[i].info_id;
                        let angle_i = self.cars[i].angle;
                        if let Some(valid) = car_position_for(c, pushed, angle_i, info_i) {
                            let mut probe = make_vehicle(
                                valid, angle_i, 0, 0, info_i, 0, true, false,
                            );
                            probe.h = self.cars[i].h;
                            let clear = self
                                .cars
                                .iter()
                                .enumerate()
                                .all(|(k, other)| k == i || !cars_overlap(&probe, other))
                                && self.peds.iter().all(|ped| !point_in_car(&probe, ped.p, 0.65));
                            if clear {
                                self.cars[i].p = valid;
                            }
                        }
                        self.cars[j].speed = vj * ((mj - mi) / mtot).clamp(0.18, 0.72);
                        self.cars[i].speed = 0.;
                    } else {
                        self.cars[i].speed = 0.;
                        self.cars[j].speed = 0.;
                    }
                    if self.driving == Some(i) || self.driving == Some(j) {
                        self.collisions += 1;
                        let driver_idx = self.driving.unwrap();
                        if self.cars[driver_idx].is_bike() {
                            let rel_speed = (vi - vj).abs();
                            if rel_speed > 7.5 {
                                let bike = &mut self.cars[driver_idx];
                                bike.roll = 0.58;
                                bike.parked = true;
                                bike.speed = 0.;
                                bike.lat_vel = 0.;
                                bike.yaw_rate = 0.;
                                let crash_damage = ((rel_speed - 7.0) * 3.2).clamp(12.0, 60.0) as u8;
                                damage_player(&mut self.health, &mut self.armour, crash_damage);
                                self.hit_flash = 0.35;
                                self.driving = None;
                            }
                        }
                    }
                    for k in [i, j] {
                        if self.cars[k].is_bike() && self.cars[k].driver_appearance.is_some() && (vi - vj).abs() > 7.5 {
                            self.cars[k].roll = 0.58;
                            self.cars[k].parked = true;
                            self.cars[k].on_rail = false;
                            self.cars[k].driver_appearance = None;
                        }
                    }
                    self.cars[i].on_rail = false;
                    self.cars[j].on_rail = false;
                }
            }
        }
        for car in &mut self.cars {
            if car.driver_appearance.is_some() {
                car.driver_pose.ride(
                    assets,
                    car.speed,
                    car.speed < -0.2,
                    car.speed.abs() < 1.0,
                );
            }
        }
        for (i, car) in self.cars.iter().enumerate() {
            let skidding_rear = car.rear_skidding || (car.wheel_spin && car.speed.abs() > 2.0);
            let skidding_front = car.front_skidding;
            let cf = forward(car.angle);
            let cside = [cf[1], -cf[0]];
            if car.is_bike() {
                let wheels = [
                    (0u8, car.h[1] * 0.65, 0.0, skidding_front, 0.11),
                    (1u8, -car.h[1] * 0.65, 0.0, skidding_rear, 0.11),
                ];
                for (w_id, long_off, lat_off, skidding, tw) in wheels {
                    let wp = [
                        car.p[0] + cf[0] * long_off + cside[0] * lat_off,
                        car.p[1] + cf[1] * long_off + cside[1] * lat_off,
                        car.p[2] + 0.02,
                    ];
                    let left_edge = [wp[0] - cside[0] * tw, wp[1] - cside[1] * tw, wp[2]];
                    let right_edge = [wp[0] + cside[0] * tw, wp[1] + cside[1] * tw, wp[2]];
                    if skidding {
                        if self.particles.len() < 160 && (self.rng.below(4) < 3) {
                            let rx = (self.rng.below(100) as f32 / 50.0 - 1.0) * 0.12;
                            let ry = (self.rng.below(100) as f32 / 50.0 - 1.0) * 0.12;
                            let rz = self.rng.below(100) as f32 / 200.0;
                            let drift_vx = -cf[0] * car.speed * 0.15 + rx * 0.5;
                            let drift_vy = -cf[1] * car.speed * 0.15 + ry * 0.5;
                            let drift_vz = 0.65 + rz * 0.8;
                            self.particles.push(Particle {
                                p: [wp[0] + rx, wp[1] + ry, wp[2] + 0.08 + rz],
                                v: [drift_vx, drift_vy, drift_vz],
                                life: 0.0,
                                max_life: 0.38 + (self.rng.below(30) as f32 / 100.0),
                                size: 0.30,
                                grow: 1.35,
                                kind: 0,
                            });
                        }
                        if let Some(strip) = self
                            .skidmarks
                            .iter_mut()
                            .find(|s| s.car_idx == i && s.wheel == w_id && s.active)
                        {
                            let last = strip.points.last().unwrap().0;
                            if (wp[0] - last[0]).powi(2) + (wp[1] - last[1]).powi(2) >= 0.16 {
                                strip.points.push((left_edge, right_edge));
                                if strip.points.len() >= 64 {
                                    strip.active = false;
                                }
                            }
                        } else {
                            self.skidmarks.push(SkidStrip {
                                points: vec![(left_edge, right_edge)],
                                car_idx: i,
                                wheel: w_id,
                                active: true,
                                hold_timer: 15.0,
                                fade_timer: 4.0,
                            });
                        }
                    } else if let Some(strip) = self
                        .skidmarks
                        .iter_mut()
                        .find(|s| s.car_idx == i && s.wheel == w_id && s.active)
                    {
                        strip.active = false;
                    }
                }
            } else {
                let wheels = [
                    (0u8, car.h[1] * 0.68, -car.h[0] * 0.82, skidding_front, 0.20),
                    (1u8, car.h[1] * 0.68, car.h[0] * 0.82, skidding_front, 0.20),
                    (2u8, -car.h[1] * 0.68, -car.h[0] * 0.82, skidding_rear, 0.20),
                    (3u8, -car.h[1] * 0.68, car.h[0] * 0.82, skidding_rear, 0.20),
                ];
                for (w_id, long_off, lat_off, skidding, tw) in wheels {
                    let wp = [
                        car.p[0] + cf[0] * long_off + cside[0] * lat_off,
                        car.p[1] + cf[1] * long_off + cside[1] * lat_off,
                        car.p[2] + 0.02,
                    ];
                    let left_edge = [wp[0] - cside[0] * tw, wp[1] - cside[1] * tw, wp[2]];
                    let right_edge = [wp[0] + cside[0] * tw, wp[1] + cside[1] * tw, wp[2]];
                    if skidding {
                        if self.particles.len() < 160 && (self.rng.below(4) < 3) {
                            let rx = (self.rng.below(100) as f32 / 50.0 - 1.0) * 0.12;
                            let ry = (self.rng.below(100) as f32 / 50.0 - 1.0) * 0.12;
                            let rz = self.rng.below(100) as f32 / 200.0;
                            let drift_vx = -cf[0] * car.speed * 0.15 + rx * 0.5;
                            let drift_vy = -cf[1] * car.speed * 0.15 + ry * 0.5;
                            let drift_vz = 0.65 + rz * 0.8;
                            self.particles.push(Particle {
                                p: [wp[0] + rx, wp[1] + ry, wp[2] + 0.08 + rz],
                                v: [drift_vx, drift_vy, drift_vz],
                                life: 0.0,
                                max_life: 0.38 + (self.rng.below(30) as f32 / 100.0),
                                size: 0.30,
                                grow: 1.35,
                                kind: 0,
                            });
                        }
                        if let Some(strip) = self
                            .skidmarks
                            .iter_mut()
                            .find(|s| s.car_idx == i && s.wheel == w_id && s.active)
                        {
                            let last = strip.points.last().unwrap().0;
                            if (wp[0] - last[0]).powi(2) + (wp[1] - last[1]).powi(2) >= 0.16 {
                                strip.points.push((left_edge, right_edge));
                                if strip.points.len() >= 64 {
                                    strip.active = false;
                                }
                            }
                        } else {
                            self.skidmarks.push(SkidStrip {
                                points: vec![(left_edge, right_edge)],
                                car_idx: i,
                                wheel: w_id,
                                active: true,
                                hold_timer: 15.0,
                                fade_timer: 4.0,
                            });
                        }
                    } else if let Some(strip) = self
                        .skidmarks
                        .iter_mut()
                        .find(|s| s.car_idx == i && s.wheel == w_id && s.active)
                    {
                        strip.active = false;
                    }
                }
            }
        }
        for strip in &mut self.skidmarks {
            if !strip.active {
                if strip.hold_timer > 0.0 {
                    strip.hold_timer = (strip.hold_timer - dt).max(0.0);
                } else {
                    strip.fade_timer = (strip.fade_timer - dt).max(0.0);
                }
            }
        }
        self.skidmarks.retain(|s| s.active || s.fade_timer > 0.0);
        if self.skidmarks.len() > 128 {
            let excess = self.skidmarks.len() - 128;
            let mut dropped = 0;
            self.skidmarks.retain(|s| {
                if dropped < excess && !s.active {
                    dropped += 1;
                    false
                } else {
                    true
                }
            });
        }
        for p in &mut self.particles {
            p.life += dt;
            p.p[0] += p.v[0] * dt;
            p.p[1] += p.v[1] * dt;
            p.p[2] += p.v[2] * dt;
            p.v[0] *= (1.0 - 2.5 * dt).max(0.0);
            p.v[1] *= (1.0 - 2.5 * dt).max(0.0);
            p.size += p.grow * dt;
        }
        self.particles.retain(|p| p.life < p.max_life);
        if self.particles.len() > 160 {
            self.particles.drain(..self.particles.len() - 160);
        }
        let focus = self.driving.map(|i| self.cars[i].p).unwrap_or(player);
        self.props.sync_area(c, focus);
        self.props.update(dt, &mut self.particles);
        for trace in &mut self.bullet_traces {
            trace.life = (trace.life - dt).max(0.0);
        }
        self.bullet_traces.retain(|t| t.life > 0.0);
        let fwd = self.driving.map(|i| forward(self.cars[i].angle)).unwrap_or([0.0, 1.0]);
        self.targeting.update(focus, fwd, &self.peds, &self.cars);
        for ped in &mut self.peds {
            ped.hit_flash = (ped.hit_flash - dt).max(0.);
            ped.cooldown = (ped.cooldown - dt).max(0.);
            ped.attacking = (ped.attacking - dt).max(0.);
            ped.panic = (ped.panic - dt).max(0.);
            if ped.health <= 0. {
                ped.dead_timer += dt;
                ped.pose.fall(assets, ped.fall_forward, (ped.dead_timer / 0.35).min(1.));
                ped.level = 0;
                continue;
            }
            for car in &self.cars {
                if point_in_car(car, ped.p, 0.6) {
                    let speed = car.speed.abs();
                    let cf = forward(car.angle);
                    let pf = forward(ped.heading);
                    let dir = car.speed.signum();
                    if speed > 5.5 {
                        ped.fall_forward = (cf[0] * pf[0] + cf[1] * pf[1]) * dir > 0.;
                        ped.dead_timer = 0.;
                        ped.evade_roll = None;
                        ped.health = 0.;
                        ped.hit_flash = 0.3;
                        ped.pose.fall(assets, ped.fall_forward, 0.);
                        self.kills += 1;
                        break;
                    } else if speed > 1.2 {
                        let push_dir = [cf[0] * dir, cf[1] * dir];
                        let nudge_p = [ped.p[0] + push_dir[0] * 0.9, ped.p[1] + push_dir[1] * 0.9, ped.p[2]];
                        if let Some(valid) = person_position(c, nudge_p) {
                            ped.p = valid;
                        }
                        ped.health = (ped.health - (speed * 4.5).clamp(10.0, 45.0)).max(0.);
                        ped.hit_flash = 0.25;
                        ped.panic = 3.0;
                        if ped.health <= 0. {
                            ped.fall_forward = (cf[0] * pf[0] + cf[1] * pf[1]) * dir > 0.;
                            ped.dead_timer = 0.;
                            ped.evade_roll = None;
                            ped.pose.fall(assets, ped.fall_forward, 0.);
                            self.kills += 1;
                        }
                        break;
                    }
                }
            }
            if ped.health <= 0. {
                ped.level = 0;
                continue;
            }
            // HandleEvasiveManeuvers (peds.cpp): dive sideways out of the path
            // of an oncoming car (>= 8.0 u/s within 22 u, closest pass <= 1.0 s).
            if ped.evade_roll.is_none() {
                for car in &self.cars {
                    if car.speed.abs() < 8.0 || (car.p[2] - ped.p[2]).abs() >= CAR_HEIGHT {
                        continue;
                    }
                    let ped_on_pavement = ped.on_pavement() || self.pedestrian_areas.contains(ped.p);
                    let car_on_road = !self.pedestrian_areas.contains(car.p);
                    if ped_on_pavement && car_on_road {
                        continue;
                    }
                    let rx = ped.p[0] - car.p[0];
                    let ry = ped.p[1] - car.p[1];
                    if rx.hypot(ry) > 22. {
                        continue;
                    }
                    let cf = forward(car.angle);
                    let vx = cf[0] * car.speed;
                    let vy = cf[1] * car.speed;
                    let v2 = vx * vx + vy * vy;
                    let dot = rx * vx + ry * vy;
                    if dot <= 0. {
                        continue;
                    }
                    let t = (dot / v2).clamp(0., 1.);
                    let cx = rx - vx * t;
                    let cy = ry - vy * t;
                    let reach = car.h[0] + 0.95;
                    if cx.hypot(cy) <= reach {
                        let v_len = v2.sqrt();
                        let right = [vy / v_len, -vx / v_len];
                        let lat = rx * right[0] + ry * right[1];
                        let sign = if lat > 0.05 {
                            1.
                        } else if lat < -0.05 {
                            -1.
                        } else if self.rng.below(2) == 0 {
                            1.
                        } else {
                            -1.
                        };
                        let dir = [right[0] * sign, right[1] * sign];
                        let roll_heading = heading([0.; 3], [dir[0], dir[1], 0.]);
                        ped.heading = roll_heading;
                        ped.evade_roll = Some(Roll::with_speed(roll_heading, 2, 5));
                        ped.evading = true;
                        ped.hostile = false;
                        ped.panic = ped.panic.max(2.5);
                        break;
                    }
                }
            }
            if let Some(mut roll) = ped.evade_roll.take() {
                roll.elapsed += dt;
                let mut next = ped.p;
                if roll.travel(c, &mut next, dt)
                    && self.cars.iter().all(|car| !point_in_car(car, next, 0.65))
                {
                    ped.p = next;
                } else {
                    roll.blocked = true;
                }
                ped.pose.roll(assets, roll.progress());
                ped.level = 2;
                if roll.elapsed < ROLL_DURATION {
                    ped.evade_roll = Some(roll);
                }
                continue;
            }
            if ped.evading {
                ped.evading = self
                    .cars
                    .iter()
                    .any(|car| car.speed.abs() > 1. && distance(car.p, ped.p) < 25.);
                if ped.evading && ped.panic <= 0. {
                    ped.panic = 1.5;
                }
            }
            let mut target = ped.target;
            let mut speed = if npc_is_jogger(ped.appearance) { 4.2 } else { 2.2 };
            let net = &*self.pavement;
            let near = distance(ped.p, focus);
            if ped.panic > 0. {
                if ped.hostile && self.driving.is_none() && self.health > 0. {
                    ped.mode = Mode::Chase;
                    target = focus;
                    speed = 5.;
                    if near < 2.8 {
                        speed = 0.;
                        if ped.cooldown <= 0. {
                            let midpoint = [
                                (ped.p[0] + focus[0]) * 0.5,
                                (ped.p[1] + focus[1]) * 0.5,
                                focus[2],
                            ];
                            if c.body_clear(midpoint, 0.2, 1.5) {
                                damage_player(&mut self.health, &mut self.armour, 8);
                                self.hit_flash = 0.25;
                                ped.cooldown = 1.1;
                                ped.attacking = 0.4;
                            }
                        }
                    }
                } else {
                    speed = 7.;
                    if !matches!(ped.mode, Mode::Flee { .. }) {
                        // cFleeOnFoot::SetupFlee: follow the current link from
                        // whichever end leads away; without one, run straight.
                        ped.mode = Mode::Flee { straight: true };
                        if let (Some(a), Some(b)) = (ped.prev, ped.cur) {
                            let away = [ped.p[0] - focus[0], ped.p[1] - focus[1]];
                            let (pa, pb) = (net.position(a), net.position(b));
                            let to_a = (pa[0] - ped.p[0]) * away[0] + (pa[1] - ped.p[1]) * away[1];
                            let to_b = (pb[0] - ped.p[0]) * away[0] + (pb[1] - ped.p[1]) * away[1];
                            if to_a > to_b {
                                ped.prev = Some(b);
                                ped.cur = Some(a);
                            }
                            ped.target = net.position(ped.cur.unwrap_or(b));
                            ped.mode = Mode::Flee { straight: false };
                        }
                    }
                    if ped.mode == (Mode::Flee { straight: false }) && planar(ped.p, ped.target) <= FLEE_ARRIVE {
                        // cFleeOnFoot::GetNextNode: the link best leading away from the threat.
                        let next = ped.cur.and_then(|cur| {
                            let here = net.position(cur);
                            let away = [here[0] - focus[0], here[1] - focus[1]];
                            let l = away[0].hypot(away[1]).max(1e-6);
                            net.next(cur, focus, Some([away[0] / l, away[1] / l]), &mut self.rng)
                                .map(|next| (cur, next))
                        });
                        match next {
                            Some((cur, next)) => {
                                ped.prev = Some(cur);
                                ped.cur = Some(next);
                                ped.target = net.position(next);
                            }
                            None => ped.mode = Mode::Flee { straight: true },
                        }
                    }
                    target = if ped.mode == (Mode::Flee { straight: true }) {
                        [
                            ped.p[0] + (ped.p[0] - focus[0]),
                            ped.p[1] + (ped.p[1] - focus[1]),
                            ped.p[2],
                        ]
                    } else {
                        ped.target
                    };
                }
            } else {
                if ped.mode != Mode::Wander {
                    // Calm again: rejoin the closest pavement node, approached
                    // under the radar-area constraint until it is reached.
                    ped.mode = Mode::Wander;
                    ped.prev = None;
                    ped.cur = net.nearest(ped.p, 15.);
                    ped.blocked = 0.;
                    if let Some(cur) = ped.cur {
                        ped.target = net.wander_target(cur, &mut self.rng);
                    } else {
                        let f = forward(ped.heading);
                        ped.home = ped.p;
                        ped.target = [ped.p[0] + f[0] * 8., ped.p[1] + f[1] * 8., ped.p[2]];
                    }
                }
                if let Some(cur) = ped.cur {
                    if planar(ped.p, ped.target) <= ARRIVE {
                        // cWanderPath::GetNextNode, arriving from the previous node.
                        let from = net.position(ped.prev.unwrap_or(cur));
                        if let Some(next) = net.next(cur, from, None, &mut self.rng) {
                            ped.prev = Some(cur);
                            ped.cur = Some(next);
                            ped.target = net.wander_target(next, &mut self.rng);
                        } else {
                            ped.prev = Some(cur);
                        }
                    }
                    target = ped.target;
                } else if distance(ped.p, target) < 1.5 {
                    std::mem::swap(&mut ped.target, &mut ped.home);
                    target = ped.target;
                }
            }
            let on_pavement = ped.on_pavement();
            let desired = heading(ped.p, target);
            ped.heading += wrapped(desired - ped.heading).clamp(-PED_TURN * dt, PED_TURN * dt);
            let f = forward(ped.heading);
            let next = [
                ped.p[0] + f[0] * speed * dt,
                ped.p[1] + f[1] * speed * dt,
                ped.p[2],
            ];
            // On a pavement link the PSP network decides where to walk (it
            // includes crossings). Off it, stay in radar pedestrian areas, but
            // let a civilian already standing on a road walk off it.
            let areas = &self.pedestrian_areas;
            if let Some(next) = person_position(c, next)
                .filter(|p| on_pavement || areas.contains(*p) || !areas.contains(ped.p))
                .filter(|p| self.cars.iter().all(|car| !point_in_car(car, *p, 0.65)))
            {
                ped.p = next;
                ped.blocked = 0.;
                ped.level = if speed > 4. {
                    2
                } else if speed > 0. {
                    1
                } else {
                    0
                };
            } else {
                ped.level = 0;
                ped.blocked += dt;
                let on_route = ped.cur.is_some() && ped.mode != Mode::Chase;
                if on_route && ped.blocked > 1.5 {
                    // PC approximation from ctw-native: no progress for 1.5 s
                    // (45 ticks) turns round on the link instead of walking on the spot.
                    ped.blocked = 0.;
                    match (ped.mode, ped.prev, ped.cur) {
                        (Mode::Flee { straight: false }, _, _) => {
                            ped.mode = Mode::Flee { straight: true };
                        }
                        (_, Some(prev), Some(cur)) => {
                            ped.prev = Some(cur);
                            ped.cur = Some(prev);
                            ped.target = if ped.mode == Mode::Wander {
                                net.wander_target(prev, &mut self.rng)
                            } else {
                                ped.mode = Mode::Flee { straight: false };
                                net.position(prev)
                            };
                        }
                        _ => {
                            // Could not reach the first node: wander locally instead.
                            ped.cur = None;
                            ped.home = ped.p;
                            let f = forward(ped.heading + 1.7);
                            ped.target = [ped.p[0] + f[0] * 8., ped.p[1] + f[1] * 8., ped.p[2]];
                        }
                    }
                } else if !on_route && ped.blocked > 0.6 {
                    ped.heading += 1.7;
                    let f = forward(ped.heading);
                    ped.target = [ped.p[0] + f[0] * 8., ped.p[1] + f[1] * 8., ped.p[2]];
                    ped.home = ped.p;
                    ped.blocked = 0.;
                }
            }
            if ped.attacking > 0. {
                ped.pose.attack(assets, 1. - ped.attacking / 0.4);
            } else {
                ped.pose.step(assets, ped.level, (speed * 4096.) as i32);
            }
        }
        // Pedestrian-pedestrian and pedestrian-player sphere separation so
        // civilians do not walk through each other or through the player.
        for i in 0..self.peds.len() {
            if self.peds[i].health <= 0. {
                continue;
            }
            for j in i + 1..self.peds.len() {
                if self.peds[j].health <= 0.
                    || (self.peds[i].p[2] - self.peds[j].p[2]).abs() >= 1.8
                {
                    continue;
                }
                let dx = self.peds[j].p[0] - self.peds[i].p[0];
                let dy = self.peds[j].p[1] - self.peds[i].p[1];
                let d = dx.hypot(dy);
                if d < 0.96 {
                    let (nx, ny) = if d > 1e-4 { (dx / d, dy / d) } else { (1., 0.) };
                    let push = (0.96 - d) * 0.5;
                    let pi = [
                        self.peds[i].p[0] - nx * push,
                        self.peds[i].p[1] - ny * push,
                        self.peds[i].p[2],
                    ];
                    let pj = [
                        self.peds[j].p[0] + nx * push,
                        self.peds[j].p[1] + ny * push,
                        self.peds[j].p[2],
                    ];
                    if let Some(pi) = person_position(c, pi)
                        .filter(|p| self.cars.iter().all(|car| !point_in_car(car, *p, 0.65)))
                    {
                        self.peds[i].p = pi;
                    }
                    if let Some(pj) = person_position(c, pj)
                        .filter(|p| self.cars.iter().all(|car| !point_in_car(car, *p, 0.65)))
                    {
                        self.peds[j].p = pj;
                    }
                }
            }
            if self.driving.is_none()
                && self.health > 0.
                && (self.peds[i].p[2] - player[2]).abs() < 1.8
            {
                let dx = self.peds[i].p[0] - player[0];
                let dy = self.peds[i].p[1] - player[1];
                let d = dx.hypot(dy);
                if d < 0.96 {
                    let (nx, ny) = if d > 1e-4 { (dx / d, dy / d) } else { (1., 0.) };
                    let push = 0.96 - d;
                    let pi = [
                        self.peds[i].p[0] + nx * push,
                        self.peds[i].p[1] + ny * push,
                        self.peds[i].p[2],
                    ];
                    if let Some(pi) = person_position(c, pi)
                        .filter(|p| self.cars.iter().all(|car| !point_in_car(car, *p, 0.65)))
                    {
                        self.peds[i].p = pi;
                    }
                }
            }
        }
        self.stream_timer += dt;
        if self.stream_timer > 3. {
            self.stream_timer = 0.;
            if self
                .peds
                .iter()
                .filter(|p| distance(p.p, focus) < 85.)
                .count()
                < 8
            {
                let mut fresh = Self::new(
                    c,
                    roads,
                    focus,
                    self.pedestrian_areas.clone(),
                    self.pavement.clone(),
                );
                for ped in &mut self.peds {
                    if distance(ped.p, focus) > 85. {
                        if let Some(replacement) = fresh.peds.pop() {
                            *ped = replacement;
                        }
                    }
                }
                for (i, car) in self.cars.iter_mut().enumerate() {
                    if Some(i) != self.driving && distance(car.p, focus) > 155. {
                        if let Some(replacement) = fresh.cars.pop() {
                            *car = replacement;
                        }
                    }
                }
            }
        }

        // Chinatown Wars authentic Explosion Shockwaves & Blast Resolution
        self.explosions.update(dt);
        let mut new_blasts = Vec::new();
        for exp in &mut self.explosions.active {
            if !exp.blast_applied {
                exp.blast_applied = true;
                new_blasts.push((exp.pos, exp.kind, exp.scale));
            }
        }

        for (blast_pos, blast_kind, blast_scale) in new_blasts {
            let blast = crate::explosions::ExplosionBlast::new(blast_pos, blast_kind, blast_scale);

            // 1. Affect Pedestrians
            for ped in &mut self.peds {
                let (hit, dmg) = blast.ped_hit(ped.p, false);
                if hit {
                    if dmg > 0.0 {
                        ped.health = (ped.health - dmg).max(0.0);
                        ped.hit_flash = 0.35;
                        ped.panic = 5.0;
                        if ped.health <= 0.0 {
                            ped.dead_timer = 0.0;
                            ped.pose.fall(assets, false, 0.0);
                            self.kills += 1;
                        }
                    }
                    let dx = ped.p[0] - blast_pos[0];
                    let dy = ped.p[1] - blast_pos[1];
                    let dist = dx.hypot(dy).max(0.1);
                    let push = (4.0 * (1.0 - (dist / blast.force_radius).min(1.0))).max(0.4);
                    let target_p = [ped.p[0] + (dx / dist) * push, ped.p[1] + (dy / dist) * push, ped.p[2]];
                    if let Some(valid) = person_position(c, target_p) {
                        ped.p = valid;
                    }
                }
            }

            // 2. Affect Player
            if self.driving.is_none() {
                let (hit, dmg) = blast.ped_hit(focus, true);
                if hit && dmg > 0.0 {
                    damage_player(&mut self.health, &mut self.armour, dmg as u8);
                    self.hit_flash = 0.45;
                }
            }

            // 3. Affect Vehicles (damage, momentum impulse, and flying debris)
            for (idx, car) in self.cars.iter_mut().enumerate() {
                let occupied = self.driving == Some(idx);
                let (dmg, impulse) = blast.vehicle_hit(car.p, car.h[0], occupied);
                if impulse != [0.0; 3] {
                    car.speed += (impulse[0] * impulse[0] + impulse[1] * impulse[1]).sqrt() * 0.45;
                    car.pitch_vel += impulse[2] * 0.25;
                    if dmg > 120.0 {
                        car.parked = true;
                    }
                }
            }

            // 4. Report crime to police
            self.police.report_crime(crate::police::CrimeType::Explosion);

            // 5. Check damage against Security Cameras
            self.collectibles.check_explosion_against_cameras(
                blast_pos,
                blast.damage_radius,
                &mut self.save_progress,
                &mut self.cash,
            );
        }

        // Chinatown Wars authentic Pursuit AI and Takedowns
        self.police.update(dt, focus, &mut self.cars);

        // Chinatown Wars authentic In-Car Radio & Dynamic Weather updates
        self.radio.update(dt, self.driving.is_some());
        self.weather.update(dt);

        // Update banner message countdown timer
        if let Some((_, timer)) = &mut self.banner {
            *timer -= dt;
            if *timer <= 0.0 {
                self.banner = None;
            }
        }

        // Update interaction prompt based on world proximity
        self.prompt = None;
        if let Some(m) = &self.minigame {
            let action_hint = match &m.kind {
                crate::minigames::MinigameKind::SinkingCar { hits_done, hits_required, .. } => {
                    format!("[ E / Y ] FRAPPER LE PARE-BRISE ({}/{})", hits_done, hits_required)
                }
                crate::minigames::MinigameKind::Hotwire { method: _, progress } => {
                    format!("[ E / Y ] DEMARRER AUX FILS ({}%)", (progress * 100.0) as u32)
                }
                crate::minigames::MinigameKind::MolotovFilling { bottles_filled, is_pumping, .. } => {
                    if *is_pumping {
                        format!("[ E / Y ] POMPER LE CARBURANT (BOUTEILLES: {}/3)", bottles_filled)
                    } else {
                        format!("[ E / Y ] COMMENCER LE REMPLISSAGE (BOUTEILLES: {}/3)", bottles_filled)
                    }
                }
                crate::minigames::MinigameKind::ScratchCard { card_name, .. } => {
                    format!("[ E / Y ] GRATTER UNE CASE DU TICKET ({})", card_name)
                }
                crate::minigames::MinigameKind::SafeCracking { unlocked_tumblers, .. } => {
                    format!("[ E / Y ] VERIFIER LA COMBINAISON ({}/3)", unlocked_tumblers)
                }
                crate::minigames::MinigameKind::DumpsterSearch { .. } => {
                    "[ E / Y ] FOUILLER LA BENNE".to_string()
                }
                crate::minigames::MinigameKind::CarBombFitting { wires_connected, .. } => {
                    format!("[ E / Y ] RACCORDER LES FILS DU DÉTONATEUR ({}/3)", wires_connected)
                }
                crate::minigames::MinigameKind::Tattooing { stencil_progress, ink_level, .. } => {
                    if *ink_level <= 0.0 {
                        "[ E / Y ] TREMPER L'AIGUILLE DANS L'ENCRE".to_string()
                    } else {
                        format!("[ E / Y ] TATOUER LE POCHOIR ({}%)", (stencil_progress * 100.0) as u32)
                    }
                }
            };
            self.prompt = Some(action_hint);
        } else if self.driving.is_none() {
            // Check story contacts
            for &contact in &[
                crate::missions::Contact::UncleKenny,
                crate::missions::Contact::ChanJaoming,
                crate::missions::Contact::ZhouMing,
                crate::missions::Contact::HsinJaoming,
                crate::missions::Contact::WadeHeston,
                crate::missions::Contact::LesterLeroc,
                crate::missions::Contact::RudyDAvanzo,
                crate::missions::Contact::XinShan,
            ] {
                if planar(focus, contact.meeting_pos()) < 6.0 {
                    self.prompt = Some(format!("[ E / Y ] MISSION : {}", contact.name()));
                    break;
                }
            }

            // Check safehouses
            if self.prompt.is_none() {
                for s in &self.safehouses.properties {
                    if planar(focus, s.pos) < 5.5 {
                        if s.owned {
                            self.prompt = Some("[ E / Y ] PLANQUE : DORMIR / SAUVEGARDER".to_string());
                        } else {
                            self.prompt = Some(format!("[ E / Y ] ACHETER {} (${})", s.name, s.price));
                        }
                        break;
                    }
                }
            }

            // Check drug dealers
            if self.prompt.is_none() {
                for d in &self.drugs.dealers {
                    if planar(focus, d.pos) < 4.0 {
                        self.prompt = Some(format!("[ E / Y ] DEALER AVEC {} ({})", d.name, d.gang.name()));
                        break;
                    }
                }
            }

            // Check dumpsters
            if self.prompt.is_none() {
                for d in &self.collectibles.dumpsters {
                    if !d.searched && planar(focus, d.pos) < 3.5 {
                        self.prompt = Some("[ E / Y ] FOUILLER LA BENNE".to_string());
                        break;
                    }
                }
            }
        } else if let Some(car_idx) = self.driving {
            if let Some(race) = &self.active_race {
                match &race.state {
                    crate::races::RaceState::Countdown(_) => {
                        self.prompt = Some("ATTENTION AU DÉPART !".to_string());
                    }
                    crate::races::RaceState::Racing => {
                        self.prompt = Some(format!("{} - POS : {}", race.lap_display(), race.rank_display()));
                    }
                    crate::races::RaceState::Finished { rank, reward, .. } => {
                        self.prompt = Some(format!("[ E / Y ] TERMINER LA COURSE ({}e place, +${})", rank, reward));
                    }
                    crate::races::RaceState::Failed { .. } => {
                        self.prompt = Some("[ E / Y ] QUITTER LA COURSE ÉCHOUÉE".to_string());
                    }
                }
            } else if self.jobs.is_active() {
                self.prompt = Some("[ E / Y ] QUITTER LE SERVICE".to_string());
            } else if let Some(job_kind) = crate::jobs::JobKind::from_vehicle(self.cars[car_idx].name) {
                self.prompt = Some(job_kind.start_prompt().to_string());
            } else {
                for circuit in &self.races.circuits {
                    let start_cp = circuit.checkpoints[0];
                    if planar(focus, start_cp) < 20.0 {
                        self.prompt = Some(format!(
                            "[ E / Y ] COURSE DE RUE : {} (Frais: ${})",
                            circuit.name, circuit.entry_fee
                        ));
                        break;
                    }
                }
            }
        }

        // Check active story mission objective progress
        let mut advance_mission = false;
        if let crate::missions::MissionState::InProgress { objective, .. } = &mut self.missions.state {
            match objective {
                crate::missions::MissionObjective::GoToLocation { target, radius, .. } => {
                    if planar(focus, *target) < *radius {
                        advance_mission = true;
                    }
                }
                crate::missions::MissionObjective::LoseWantedLevel { .. } => {
                    if self.police.stars == 0 {
                        advance_mission = true;
                    }
                }
                crate::missions::MissionObjective::SurviveFrames { frames_left } => {
                    if *frames_left <= 1 {
                        advance_mission = true;
                    } else {
                        *frames_left -= 1;
                    }
                }
                crate::missions::MissionObjective::CompleteMinigame { overlay_name } => {
                    if self.minigame.is_none() {
                        if *overlay_name == "sinkingcarapp" {
                            self.minigame = Some(crate::minigames::MinigameSession::new_sinking_car());
                        } else if *overlay_name == "hotwiringapp" {
                            self.minigame = Some(crate::minigames::MinigameSession::new_hotwire(crate::minigames::HotwireMethod::WiresTwist));
                        }
                    }
                    if let Some(m) = &mut self.minigame {
                        m.tick(dt);
                        if m.completed {
                            advance_mission = true;
                            self.minigame = None;
                        }
                    }
                }
                _ => {}
            }
        }
        if advance_mission {
            if self.missions.advance_objective(&mut self.cash) {
                self.stats.inc_missions_passed();
                self.banner = Some(("MISSION REUSSIE !".to_string(), 4.5));
            } else {
                self.banner = Some(("OBJECTIF ATTEINT !".to_string(), 3.0));
            }
        }

        // Check drug dealer proximity discovery
        for d in &mut self.drugs.dealers {
            if !d.discovered && planar(focus, d.pos) < 25.0 {
                d.discovered = true;
                if (d.id as usize) < self.save_progress.dealer.len() {
                    self.save_progress.dealer[d.id as usize] = true;
                }
                self.banner = Some((format!("DEALER DECOUVERT: {} ({})", d.name, d.district), 3.0));
            }
        }

        // Update vehicle ambient jobs
        let player_in_vehicle = self.driving.is_some();
        let vehicle_speed = self.driving.map(|i| self.cars[i].speed).unwrap_or(0.0);
        let vehicle_health = 1000.0;
        self.jobs.update(
            dt,
            focus,
            player_in_vehicle,
            vehicle_health,
            vehicle_speed,
            false,
            &mut self.cash,
            &mut self.save_progress,
        );
        if let Some(msg) = self.jobs.banner_message.take() {
            self.banner = Some(msg);
        }

        // Update collectibles, stunt jumps, and rampages
        let vehicle_pos = self.driving.map(|i| self.cars[i].p);
        let vehicle_roll = self.driving.map(|i| self.cars[i].roll).unwrap_or(0.0);
        let vehicle_pitch = self.driving.map(|i| self.cars[i].pitch).unwrap_or(0.0);
        self.collectibles.update(
            dt,
            focus,
            vehicle_pos,
            vehicle_speed,
            false,
            vehicle_roll,
            vehicle_pitch,
            &mut self.cash,
            &mut self.save_progress,
        );
        if let Some(msg) = self.collectibles.banner_message.take() {
            self.banner = Some(msg);
        }

        // Update Street Races engine (Overlay 140 streetrace)
        if let Some(race) = &mut self.active_race {
            race.update(
                dt,
                focus,
                player_in_vehicle,
                &mut self.save_progress,
                &mut self.cash,
            );
            if let Some(msg) = race.banner_message.take() {
                self.banner = Some(msg);
            }
        }

        self.driving.map(|i| self.cars[i].p)
    }
}

fn cars_overlap(a: &Car, b: &Car) -> bool {
    if (a.p[2] - b.p[2]).abs() >= CAR_HEIGHT {
        return false;
    }
    let af = forward(a.angle);
    let bf = forward(b.angle);
    let aside = [af[1], -af[0]];
    let bside = [bf[1], -bf[0]];
    let dot = |u: [f32; 2], v: [f32; 2]| u[0] * v[0] + u[1] * v[1];
    let d = [b.p[0] - a.p[0], b.p[1] - a.p[1]];
    [af, aside, bf, bside].iter().all(|&axis| {
        let ra = dot(af, axis).abs() * a.h[1] + dot(aside, axis).abs() * a.h[0];
        let rb = dot(bf, axis).abs() * b.h[1] + dot(bside, axis).abs() * b.h[0];
        dot(d, axis).abs() < ra + rb
    })
}
fn point_in_car(car: &Car, p: [f32; 3], radius: f32) -> bool {
    if p[2] >= car.p[2] + CAR_HEIGHT || p[2] + 2. <= car.p[2] {
        return false;
    }
    let f = forward(car.angle);
    let d = [p[0] - car.p[0], p[1] - car.p[1]];
    let x = d[0] * f[1] - d[1] * f[0];
    let y = d[0] * f[0] + d[1] * f[1];
    x.abs() < car.h[0] + radius && y.abs() < car.h[1] + radius
}

/// Returns true on collision. Fixed small spatial steps prevent high-speed
/// movement from tunnelling through collision boxes between rendering frames.
/// Uses per-vehicle PAK 29 parameters (cTransmission::DriveEngine,
/// cWheeledVehicle::CalcHandling / UpdateSway, and cBike::UpdateModelMatrix).
fn step_car(car: &mut Car, c: &Collision, input: DriveInput, dt: f32) -> bool {
    let spec = car.spec();
    let mass = spec.mass();
    let max_fwd = (spec.top_speed as f32 * 0.44).clamp(22.0, 68.0);
    let max_rev = -(max_fwd * 0.32).clamp(10.0, 16.0);
    let torque_ratio = ((spec.max_torque() / 35.0) / mass.powf(0.55)).clamp(0.38, 2.25);
    let throttle = input.throttle.clamp(-1., 1.);
    let steer_in = input.steer.clamp(-1., 1.);
    let prev_speed = car.speed;
    let handbrake_turn = input.brake && (steer_in.abs() > 0.10 || car.lat_vel.abs() > 1.2) && car.speed.abs() > 3.0;
    car.braking = input.brake || (car.speed * throttle < -0.15);

    if input.brake {
        let base_brake =
            (32.0 * (spec.brake_force() / 500.0).sqrt().clamp(0.75, 1.55) / mass.powf(0.25))
                .max(28.0);
        if handbrake_turn {
            car.rear_skidding = true;
            car.front_skidding = car.speed.abs() > 12.0 && steer_in.abs() > 0.35;
            car.wheel_spin = throttle > 0.2;
            if throttle > 0.1 {
                let acceleration = 15.0 * torque_ratio;
                car.speed = (car.speed + throttle * acceleration * dt).clamp(max_rev, max_fwd);
            } else {
                let decel = base_brake * 0.42;
                car.speed = move_towards(car.speed, 0., decel * dt);
            }
            car.rpm = (0.55 + throttle.abs() * 0.45).clamp(0.28, 1.0);
        } else {
            car.rear_skidding = car.speed.abs() > 4.5;
            car.front_skidding = car.speed.abs() > 8.0;
            car.wheel_spin = false;
            car.speed = move_towards(car.speed, 0., base_brake * dt);
            car.rpm = move_towards(car.rpm, 0.22, 2.5 * dt);
        }
        if car.speed.abs() < 0.05 {
            car.gear = 0;
            car.rear_skidding = false;
            car.front_skidding = false;
        }
    } else if throttle.abs() > 0.05 {
        car.rear_skidding = car.lat_vel.abs() > 2.2 && car.speed.abs() > 5.0;
        car.front_skidding = car.lat_vel.abs() > 3.6 && car.speed.abs() > 8.0;
        if car.speed * throttle < -0.15 {
            let oppose = (30.0 / mass.powf(0.25)).max(22.0);
            car.speed = (car.speed + throttle * oppose * dt).clamp(max_rev, max_fwd);
            car.wheel_spin = car.speed.abs() > 3.5;
            if car.wheel_spin {
                car.rear_skidding = true;
            }
        } else {
            let num_gears = spec.num_gears.max(1) as i8;
            let gear_mul = if throttle < 0. {
                car.gear = -1;
                car.rpm = (car.speed.abs() / max_rev.abs()).clamp(0.25, 0.98);
                1.0
            } else {
                let sr = (car.speed.max(0.) / max_fwd).clamp(0., 1.);
                car.gear = if sr < 0.36 {
                    1
                } else if sr < 0.70 {
                    2.min(num_gears)
                } else {
                    num_gears
                };
                let band_lo = match car.gear {
                    1 => 0.0,
                    2 => 0.30,
                    _ => 0.62,
                };
                let band_hi = match car.gear {
                    1 => 0.40,
                    2 => 0.74,
                    _ => 1.0,
                };
                car.rpm = (0.32 + (sr - band_lo) / (band_hi - band_lo).max(0.1) * 0.66)
                    .clamp(0.28, 1.0);
                match car.gear {
                    1 => 1.15,
                    2 => 1.00,
                    _ => 0.88,
                }
            };
            let acceleration = 17.0 * torque_ratio * gear_mul;
            car.speed = (car.speed + throttle * acceleration * dt).clamp(max_rev, max_fwd);
            car.wheel_spin = car.speed.abs() < 6.0
                && throttle > 0.8
                && (spec.drive_split() < 0.25 || torque_ratio > 1.25);
            if car.wheel_spin {
                car.rear_skidding = true;
            }
        }
    } else {
        car.rear_skidding = car.lat_vel.abs() > 2.4 && car.speed.abs() > 5.0;
        car.front_skidding = car.lat_vel.abs() > 3.8 && car.speed.abs() > 8.0;
        let coast = (3.0 + (car.speed.abs() / max_fwd) * 1.4) / mass.powf(0.2);
        car.speed = move_towards(car.speed, 0., coast * dt);
        car.rpm = move_towards(car.rpm, 0.22, 1.8 * dt);
        car.wheel_spin = false;
        if car.speed.abs() < 0.05 {
            car.gear = 0;
            car.rear_skidding = false;
            car.front_skidding = false;
        }
    }

    // Steering response from PAK 29 steerLock, steerRate and steerSpeedRed.
    let speed_norm = (car.speed.abs() / max_fwd).clamp(0., 1.);
    let speed_red = (1.0 - speed_norm * spec.steer_speed_red() * 1.5).clamp(0.40, 1.0);
    let lock_factor = (spec.steer_lock_deg() / 60.0).clamp(0.55, 1.45);
    let steer_rate = (spec.steer_rate_deg() / 15.0).clamp(4.0, 12.0);
    if steer_in.abs() < 0.02 {
        car.steer_angle = move_towards(car.steer_angle, 0., steer_rate * 1.6 * dt);
    } else {
        car.steer_angle = move_towards(
            car.steer_angle,
            steer_in * lock_factor * speed_red,
            steer_rate * dt,
        );
    }

    let wb_factor = (4.59 / spec.wheelbase()).clamp(0.58, 1.52);
    let oversteer = if handbrake_turn {
        1.62
    } else if throttle > 0.55 && spec.drive_split() < 0.20 {
        1.18
    } else {
        1.0
    };
    let speed_engage = (car.speed.abs() / 7.0).min(1.0) * car.speed.signum();
    let front_bite = (spec.front_stiffness() / 5.0).clamp(0.65, 1.25);
    let target_yaw = -car.steer_angle * 1.75 * wb_factor * oversteer * speed_engage;
    car.yaw_rate = move_towards(car.yaw_rate, target_yaw, 18.0 * spec.grip() * front_bite * dt);

    let slip_factor = if handbrake_turn {
        0.45
    } else if spec.drive_split() < 0.20 && throttle > 0.5 {
        0.18
    } else if spec.is_bike() {
        0.04
    } else {
        0.08
    };
    let target_lat = if steer_in.abs() < 0.02 && !handbrake_turn && car.lat_vel.abs() < 0.6 {
        0.0
    } else {
        -car.yaw_rate * car.speed.abs() * slip_factor
    };
    let lat_grip = (if handbrake_turn { 4.5 } else { 15.0 })
        * spec.grip()
        * (spec.rear_stiffness() / 5.0).clamp(0.55, 1.3);
    car.lat_vel = move_towards(car.lat_vel, target_lat, lat_grip * dt);
    if car.speed.abs() < 0.2 {
        car.lat_vel = 0.;
        car.yaw_rate = 0.;
        car.rear_skidding = false;
        car.front_skidding = false;
    }

    // Body suspension sway (cars/trucks/tank) vs cornering lean & wheelie/stoppie (cBike).
    if spec.is_bike() {
        let target_roll = if car.speed.abs() > 0.4 {
            steer_in * 0.65 * (car.speed.abs() / 16.0).min(1.0)
        } else if !car.parked {
            0.095
        } else {
            0.0
        };
        car.roll += (target_roll - car.roll) * (8.5 * dt).min(1.0);
        let target_pitch = if input.brake && car.speed > 3.0 {
            -0.22 * (car.speed / 18.0).min(1.0)
        } else if throttle > 0.5 && car.speed >= 0.0 && car.speed < max_fwd * 0.72 {
            0.28 * throttle * (1.0 - car.speed / (max_fwd * 0.85)).max(0.2)
        } else {
            0.0
        };
        car.pitch += (target_pitch - car.pitch) * (7.5 * dt).min(1.0);
    } else {
        let lat_accel = -car.yaw_rate * car.speed;
        let long_accel = (car.speed - prev_speed) / dt.max(1e-4);
        let target_roll = (-lat_accel * 0.0045 / mass.sqrt()).clamp(-0.13, 0.13);
        let target_pitch = (long_accel * 0.0026 / mass.sqrt()).clamp(-0.09, 0.07);
        let damping = (spec.sway_damping() * 10.0).clamp(4.5, 9.0);
        car.roll_vel += (target_roll - car.roll) * 42.0 * dt - car.roll_vel * damping * dt;
        car.roll = (car.roll + car.roll_vel * dt).clamp(-0.16, 0.16);
        car.pitch_vel += (target_pitch - car.pitch) * 42.0 * dt - car.pitch_vel * damping * dt;
        car.pitch = (car.pitch + car.pitch_vel * dt).clamp(-0.12, 0.12);
    }

    let angle = car.angle + car.yaw_rate * dt;
    if car_position_for(c, car.p, angle, car.info_id).is_some() {
        car.angle = wrapped(angle);
    }
    let f = forward(car.angle);
    let side = [f[1], -f[0]];
    let dx = (f[0] * car.speed + side[0] * car.lat_vel) * dt;
    let dy = (f[1] * car.speed + side[1] * car.lat_vel) * dt;
    let distance = dx.hypot(dy);
    let count = (distance / 0.35).ceil().max(1.) as usize;
    let mut collided = false;
    for _ in 0..count {
        let sx = dx / count as f32;
        let sy = dy / count as f32;
        let p = [car.p[0] + sx, car.p[1] + sy, car.p[2]];
        if let Some(p) = car_position_for(c, p, car.angle, car.info_id) {
            car.p = p;
        } else {
            // Allow glancing wall slide only when moving obliquely to the obstacle.
            let oblique = sx.abs() > 0.25 * (sx.hypot(sy)) && sy.abs() > 0.25 * (sx.hypot(sy));
            if oblique {
                let px = [car.p[0] + sx, car.p[1], car.p[2]];
                let py = [car.p[0], car.p[1] + sy, car.p[2]];
                if let Some(valid) = car_position_for(c, px, car.angle, car.info_id)
                    .or_else(|| car_position_for(c, py, car.angle, car.info_id))
                {
                    car.p = valid;
                    car.speed *= 0.72;
                    car.lat_vel = 0.;
                    collided = true;
                    continue;
                }
            }
            car.speed = 0.;
            car.lat_vel = 0.;
            car.pitch_vel -= 0.65 / mass.sqrt();
            return true;
        }
    }
    collided
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::{BoxShape, Cell};
    #[test]
    fn pedestrian_areas_reject_road_and_water_accept_pavement_and_park() {
        let t = crate::texture::Texture {
            high_res: None,
            info: crate::texture::Info {
                id: 0,
                width: 4,
                height: 1,
                bits: 32,
                colors: 0,
            },
            rgba: vec![
                [91, 91, 91, 255],
                [94, 94, 120, 255],
                [195, 184, 166, 255],
                [102, 147, 94, 255],
            ],
        };
        let areas = PedestrianAreas::from_radar(&t);
        assert!(!areas.contains([-2625., 0., 0.]));
        assert!(!areas.contains([-875., 0., 0.]));
        assert!(areas.contains([875., 0., 0.]));
        assert!(areas.contains([2625., 0., 0.]));
        assert!(!areas.contains([4000., 0., 0.]));
    }
    #[test]
    fn car_contacts_include_bumpers_rotation_and_height_separation() {
        let a = make_car([0.; 3], 0., 0, 0, 0);
        let mut b = make_car([0., 6.5, 0.], 0., 0, 0, 1);
        assert!(cars_overlap(&a, &b));
        b.p[1] = 7.;
        assert!(!cars_overlap(&a, &b));
        b.p = [3., 0., 0.];
        assert!(!cars_overlap(&a, &b));
        b.angle = std::f32::consts::FRAC_PI_2;
        assert!(cars_overlap(&a, &b));
        b.p[2] = 3.;
        assert!(!cars_overlap(&a, &b));
    }
    fn flat() -> Collision {
        Collision {
            cells: (0..14000)
                .map(|_| Cell {
                    ground: vec![0x55; 400],
                    ..Cell::default()
                })
                .collect(),
        }
    }
    fn empty() -> Scene {
        Scene {
            pedestrian_areas: Arc::new(PedestrianAreas {
                width: 0,
                height: 0,
                allowed: vec![],
            }),
            pavement: Arc::new(Network::default()),
            rng: Rng::new(1),
            cars: vec![],
            peds: vec![],
            skidmarks: vec![],
            particles: vec![],
            props: crate::props::PropsManager::new(),
            bullet_traces: vec![],
            targeting: crate::weapons::TargetingSystem::new(),
            weapons: None,
            police: crate::police::PoliceManager::new(),
            explosions: crate::explosions::ExplosionManager::new(),
            radio: crate::radio::RadioManager::new(None),
            weather: crate::weather::WeatherManager::new(None),
            missions: crate::missions::MissionEngine::new(),
            drugs: crate::drugs::DrugEconomy::new(),
            safehouses: crate::safehouse::SafehouseManager::new(),
            stats: crate::stats::Stats::new(),
            save_progress: crate::stats::SaveProgress::default(),
            pda: crate::pda::PdaEngine::new(),
            minigame: None,
            jobs: crate::jobs::JobManager::new(),
            collectibles: crate::collectibles::CollectibleManager::new(),
            races: crate::races::RaceManager::new(),
            active_race: None,
            cash: 500,
            prompt: None,
            banner: None,
            driving: None,
            health: 127.,
            armour: 0,
            hit_flash: 0.,
            attack_timer: 0.,
            kills: 0,
            hits: 0,
            vehicle_entries: 0,
            vehicle_exits: 0,
            collisions: 0,
            stream_timer: 0.,
        }
    }
    #[test]
    fn original_armour_absorbs_damage_and_passes_only_overflow_to_health() {
        let (mut health, mut armour) = (127., 127);
        damage_player(&mut health, &mut armour, 8);
        assert_eq!((health, armour), (127., 119));
        damage_player(&mut health, &mut armour, 119);
        assert_eq!((health, armour), (127., 0));
        armour = 3;
        damage_player(&mut health, &mut armour, 8);
        assert_eq!((health, armour), (122., 0));
        damage_player(&mut health, &mut armour, 127);
        assert_eq!((health, armour), (0., 0));
    }
    #[test]
    fn drive_accelerates_brakes_reverses_and_does_not_tunnel_through_walls() {
        let mut c = flat();
        let mut car = make_car([0., 0., 0.], 0., 0, 0, 0);
        for _ in 0..60 {
            assert!(!step_car(
                &mut car,
                &c,
                DriveInput {
                    throttle: 1.,
                    ..DriveInput::default()
                },
                1. / 30.
            ));
        }
        assert!(car.p[1] > 30. && car.speed > 30.);
        for _ in 0..40 {
            step_car(
                &mut car,
                &c,
                DriveInput {
                    brake: true,
                    ..DriveInput::default()
                },
                1. / 30.,
            );
        }
        assert_eq!(car.speed, 0.);
        let stopped = car.p;
        for _ in 0..30 {
            step_car(
                &mut car,
                &c,
                DriveInput {
                    throttle: -1.,
                    ..DriveInput::default()
                },
                1. / 30.,
            );
        }
        assert!(car.p[1] < stopped[1] - 5.);
        car.p = [0., 0., 0.];
        car.speed = 44.;
        let (x, y) = Collision::cell_xy(0., 5.).unwrap();
        c.cells[x * 100 + y].boxes.push(BoxShape {
            c: [0., 5., 1.],
            h: [8., 0.12, 1.],
            angle: 0.,
            flags: 0,
        });
        assert!(step_car(
            &mut car,
            &c,
            DriveInput {
                throttle: 1.,
                ..DriveInput::default()
            },
            0.1
        ));
        assert!(car.p[1] < 2. && car.speed == 0.);
    }
    #[test]
    fn melee_requires_range_front_and_clear_line_and_respects_cooldown() {
        let mut c = flat();
        let mut s = empty();
        s.peds.push(make_ped([0., 5., 0.], [0.; 3], 0));
        assert!(!s.attack(&c, [0.; 3], 0.));
        s.attack_timer = 0.;
        s.peds[0].p = [0., -2., 0.];
        assert!(!s.attack(&c, [0.; 3], 0.));
        s.attack_timer = 0.;
        s.peds[0].p = [0., 2., 0.];
        assert!(s.attack(&c, [0.; 3], 0.));
        assert_eq!(s.peds[0].health, 30.);
        assert!(!s.attack(&c, [0.; 3], 0.));
        s.attack_timer = 0.;
        let (x, y) = Collision::cell_xy(0., 1.).unwrap();
        c.cells[x * 100 + y].boxes.push(BoxShape {
            c: [0., 1., 1.],
            h: [3., 0.2, 1.],
            angle: 0.,
            flags: 0,
        });
        assert!(!s.attack(&c, [0.; 3], 0.));
        assert_eq!(s.peds[0].health, 30.);
    }
    #[test]
    fn exit_stays_in_vehicle_when_all_exits_blocked_and_refuses_exit_at_speed() {
        let mut c = flat();
        let mut s = empty();
        s.cars.push(make_car([0.; 3], 0., 0, 0, 0));
        assert_eq!(s.toggle_vehicle(&c, [3., 0., 0.]), Some([0.; 3]));
        s.cars[0].speed = 10.;
        assert!(s.toggle_vehicle(&c, [0.; 3]).is_none());
        s.cars[0].speed = 0.;
        let (x, y) = Collision::cell_xy(0., 0.).unwrap();
        c.cells[x * 100 + y].boxes.push(BoxShape {
            c: [0., 0., 1.],
            h: [10., 10., 1.],
            angle: 0.,
            flags: 0,
        });
        assert!(s.toggle_vehicle(&c, [0.; 3]).is_none());
        assert_eq!(s.driving, Some(0));
        c.cells[x * 100 + y].boxes.clear();
        let p = s.toggle_vehicle(&c, [0.; 3]).unwrap();
        assert!(c.body_clear(p, 0.875, 2.));
        assert!(!point_in_car(&s.cars[0], p, 0.875));
        assert_eq!(s.driving, None);
    }
    #[test]
    fn dynamic_car_shapes_support_vaults_then_restore_static_collision() {
        let mut c = flat();
        let mut s = empty();
        s.cars.push(make_car([10., 14., 0.], 0., 0, 0, 0));
        let count = c.cells.iter().map(|v| v.boxes.len()).sum::<usize>();
        s.with_car_collision(&mut c, |c| {
            assert!(!c.body_clear([10., 14., 0.], 0.875, 2.));
            let plan = crate::vault::plan(c, [10., 9.5, 0.], 0.)
                .expect("The Admiral roof remains vaultable");
            assert!((plan.top - CAR_HEIGHT).abs() < 0.01);
        });
        assert_eq!(c.cells.iter().map(|v| v.boxes.len()).sum::<usize>(), count);
        assert!(c.body_clear([10., 14., 0.], 0.875, 2.));
        s.driving = Some(0);
        s.with_car_collision(&mut c, |c| assert!(c.body_clear([10., 14., 0.], 0.875, 2.)));
    }
    fn assets() -> PedAssets {
        PedAssets {
            animations: (0..552)
                .map(|_| crate::ped::Animation {
                    rate: 0,
                    angle: 0,
                    flags: 0,
                    frames: vec![],
                })
                .collect(),
            textures: Default::default(),
        }
    }
    fn net(points: &[[f32; 2]], links: &[&[u32]]) -> Network {
        Network::from_nodes(
            points
                .iter()
                .zip(links)
                .map(|(p, l)| crate::pavement::NetNode {
                    p: [p[0], p[1], 0.],
                    flags: 0,
                    links: l.to_vec(),
                })
                .collect(),
        )
    }
    fn segment_distance(p: [f32; 3], a: [f32; 3], b: [f32; 3]) -> f32 {
        let d = [b[0] - a[0], b[1] - a[1]];
        let t = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1])).clamp(0., 1.);
        (p[0] - a[0] - d[0] * t).hypot(p[1] - a[1] - d[1] * t)
    }
    #[test]
    fn civilians_walk_node_to_node_on_the_pavement_graph_even_where_radar_says_road() {
        let c = flat();
        let mut s = empty();
        // The radar mask rejects everything: only the PSP network may lead the walk.
        s.pedestrian_areas = Arc::new(PedestrianAreas {
            width: 1,
            height: 1,
            allowed: vec![false],
        });
        s.pavement = Arc::new(net(
            &[[0., 0.], [20., 0.], [20., 20.], [0., 20.]],
            &[&[1, 3], &[0, 2], &[1, 3], &[2, 0]],
        ));
        let mut ped = make_ped([1., 0., 0.], [20., 0., 0.], 0);
        ped.prev = Some(0);
        ped.cur = Some(1);
        s.peds.push(ped);
        let roads = Roads { nodes: vec![] };
        let assets = assets();
        let mut visited = std::collections::HashSet::new();
        let corners = [[0., 0., 0.], [20., 0., 0.], [20., 20., 0.], [0., 20., 0.]];
        for _ in 0..1200 {
            s.tick(&c, &roads, &assets, [200., 200., 0.], 1. / 30., DriveInput::default());
            let ped = &s.peds[0];
            visited.insert(ped.cur.unwrap());
            assert!(ped.on_pavement());
            let off = (0..4)
                .map(|i| segment_distance(ped.p, corners[i], corners[(i + 1) % 4]))
                .fold(f32::MAX, f32::min);
            assert!(off < 1.3, "civilian left the pavement link by {off}");
        }
        assert_eq!(visited.len(), 4, "a loop is walked without turning back");
        assert!(s.peds[0].level == 1);
    }
    #[test]
    fn fleeing_civilian_takes_the_link_away_from_the_threat_then_rejoins_the_network() {
        let c = flat();
        let mut s = empty();
        s.pavement = Arc::new(net(
            &[[0., 0.], [10., 0.], [20., 0.], [30., 0.], [40., 0.]],
            &[&[1], &[0, 2], &[1, 3], &[2, 4], &[3]],
        ));
        let mut ped = make_ped([12., 0., 0.], [20., 0., 0.], 1);
        ped.prev = Some(1);
        ped.cur = Some(2);
        ped.panic = 3.;
        s.peds.push(ped);
        let roads = Roads { nodes: vec![] };
        let assets = assets();
        let threat = [24., 0., 0.];
        for _ in 0..30 {
            s.tick(&c, &roads, &assets, threat, 1. / 30., DriveInput::default());
        }
        let ped = &s.peds[0];
        assert_eq!(ped.mode, Mode::Flee { straight: false });
        assert_eq!(ped.cur, Some(0), "ran past node 1 towards node 0, away from the threat");
        assert!(ped.p[0] < 7. && ped.level == 2);
        s.peds[0].panic = 0.;
        s.tick(&c, &roads, &assets, threat, 1. / 30., DriveInput::default());
        assert_eq!(s.peds[0].mode, Mode::Wander);
        assert!(s.peds[0].cur.is_some() && s.peds[0].prev.is_none());
        for _ in 0..150 {
            s.tick(&c, &roads, &assets, threat, 1. / 30., DriveInput::default());
        }
        assert!(s.peds[0].on_pavement(), "calm civilian is walking a link again");
    }
    #[test]
    fn blocked_civilian_turns_round_on_its_link() {
        let mut c = flat();
        let mut s = empty();
        s.pavement = Arc::new(net(&[[0., 0.], [0., 30.]], &[&[1], &[0]]));
        let mut ped = make_ped([0., 2., 0.], [0., 30., 0.], 0);
        ped.prev = Some(0);
        ped.cur = Some(1);
        s.peds.push(ped);
        let (x, y) = Collision::cell_xy(0., 5.).unwrap();
        c.cells[x * 100 + y].boxes.push(BoxShape {
            c: [0., 5., 1.],
            h: [4., 0.2, 1.],
            angle: 0.,
            flags: 0,
        });
        let roads = Roads { nodes: vec![] };
        let assets = assets();
        for _ in 0..90 {
            s.tick(&c, &roads, &assets, [200., 200., 0.], 1. / 30., DriveInput::default());
        }
        let ped = &s.peds[0];
        assert_eq!((ped.prev, ped.cur), (Some(1), Some(0)));
        assert!(ped.p[1] < 4.);
    }
    #[test]
    fn oncoming_car_triggers_evasive_lateral_roll_and_survival() {
        let c = flat();
        let mut s = empty();
        let mut car = make_car([0., 0., 0.], 0., 0, 0, 0);
        car.speed = 16.;
        s.cars.push(car);
        s.driving = Some(0);
        // Civilian standing slightly to the right of the oncoming car's path.
        s.peds.push(make_ped([0.4, 15., 0.], [0.4, 25., 0.], 0));
        let roads = Roads { nodes: vec![] };
        let assets = assets();
        s.tick(&c, &roads, &assets, [0., 0., 0.], 1. / 30., DriveInput::default());
        assert!(s.peds[0].evade_roll.is_some() && s.peds[0].evading);
        assert_eq!(s.peds[0].pose.upper, 336);
        for _ in 0..25 {
            s.tick(
                &c,
                &roads,
                &assets,
                s.cars[0].p,
                1. / 30.,
                DriveInput {
                    throttle: 0.4,
                    ..DriveInput::default()
                },
            );
        }
        assert!(s.peds[0].health > 0., "lateral dive cleared the oncoming car");
        assert!(s.peds[0].p[0] > 3.0, "rolled to the right side of the lane");
        assert!(s.peds[0].evade_roll.is_none());
    }
    #[test]
    fn fallen_civilians_play_death_pose_and_women_flee_instead_of_fighting() {
        let c = flat();
        let mut s = empty();
        // Appearance 10 is female (10 % 12 == 10, false), and even at index 4 or 1
        // female civilians never turn hostile when hit.
        s.peds.push(make_ped([0., 2., 0.], [0., 10., 0.], 1));
        assert!(s.attack(&c, [0., 0., 0.], 0.));
        assert!(!s.peds[0].hostile);
        s.attack_timer = 0.;
        s.peds[0].p = [0., 2., 0.];
        assert!(s.attack(&c, [0., 0., 0.], 0.));
        assert_eq!(s.peds[0].health, 0.);
        assert!(s.peds[0].fall_forward);
        let roads = Roads { nodes: vec![] };
        let assets = assets();
        s.tick(&c, &roads, &assets, [0., 0., 0.], 0.1, DriveInput::default());
        assert_eq!(s.peds[0].pose.upper, 22);
        for _ in 0..5 {
            s.tick(&c, &roads, &assets, [0., 0., 0.], 0.1, DriveInput::default());
        }
        assert_eq!(s.peds[0].pose.upper, 26);
    }
    #[test]
    fn overlapping_civilians_and_player_separate_and_joggers_run_when_calm() {
        let c = flat();
        let mut s = empty();
        // Appearance 2 is a male jogger; appearance 0 is a standard walker.
        s.peds.push(make_ped([5., 5., 0.], [5., 25., 0.], 2));
        s.peds.push(make_ped([5.1, 5., 0.], [5.1, 25., 0.], 0));
        let roads = Roads { nodes: vec![] };
        let assets = assets();
        s.tick(&c, &roads, &assets, [5., 5., 0.], 1. / 30., DriveInput::default());
        let d_peds = planar(s.peds[0].p, s.peds[1].p);
        let d_player = planar(s.peds[0].p, [5., 5., 0.]);
        assert!(d_peds >= 0.9, "pedestrians separated to {d_peds}");
        assert!(d_player >= 0.9, "pedestrian separated from player to {d_player}");
        assert_eq!(s.peds[0].level, 2, "calm jogger runs at level 2");
        assert_eq!(s.peds[1].level, 1, "calm walker stays at level 1");
    }
    #[test]
    fn scene_spawns_cargen_parked_vehicles_and_varied_psp_traffic_models() {
        let mut c = flat();
        let (x, y) = Collision::cell_xy(30., 25.).unwrap();
        c.cells[x * 100 + y].car_gens.push(crate::collision::CarGen {
            p: [30., 25., 0.],
            heading: 16384,
            vehicle: 25, // BOBCAT (model 64)
            flags: 0,
            colour: 4,
        });
        c.cells[x * 100 + y].car_gens.push(crate::collision::CarGen {
            p: [45., 25., 0.],
            heading: 0,
            vehicle: 22, // CHAVOS (model 61)
            flags: 0x80,  // script-disabled: must not spawn
            colour: 2,
        });
        let roads = Roads {
            nodes: vec![
                crate::gps::Node {
                    p: [-60., -15., 0.],
                    flags: 0,
                    links: vec![1],
                },
                crate::gps::Node {
                    p: [60., -15., 0.],
                    flags: 0,
                    links: vec![0],
                },
            ],
        };
        let areas = Arc::new(PedestrianAreas {
            width: 0,
            height: 0,
            allowed: vec![],
        });
        let pavement = Arc::new(Network::default());
        let s = Scene::new(&c, &roads, [0., 0., 0.], areas, pavement);
        assert_eq!(s.cars[0].name, "ADMIRAL");
        assert!(s.cars[0].parked && !s.cars[0].from_cargen);
        let cargen = s
            .cars
            .iter()
            .find(|car| car.from_cargen)
            .expect("CarGen parked Bobcat spawned");
        assert_eq!(cargen.info_id, 25);
        assert_eq!(cargen.model_id, 64);
        assert_eq!(cargen.name, "BOBCAT");
        assert_eq!(cargen.paint, 4);
        assert!(cargen.parked);
        assert_eq!(s.cars.iter().filter(|car| car.from_cargen).count(), 1);
        assert!(
            s.cars.iter().filter(|car| !car.parked).any(|car| car.info_id != 0),
            "moving road traffic uses varied PAK 29 vehicle specs"
        );
        assert!(
            s.cars.iter().any(|car| car.is_bike()),
            "Scene::new places a motorcycle near the player"
        );
    }
    #[test]
    fn motorcycle_leans_into_corners_wheelies_on_launch_and_exposes_rider_position() {
        let c = flat();
        let mut bike = make_vehicle([0., 0., 0.], 0., 0, 0, 89, 0, false, false); // NRG900
        assert!(bike.is_bike());
        assert!(bike.rider_pos().is_some());
        for _ in 0..25 {
            step_car(
                &mut bike,
                &c,
                DriveInput {
                    throttle: 1.,
                    steer: 1.,
                    brake: false,
                },
                1. / 30.,
            );
        }
        assert!(bike.roll > 0.35, "motorcycle leans right into right turn: {}", bike.roll);
        assert!(bike.pitch > 0.08, "motorcycle pitches up (wheelie) under hard throttle: {}", bike.pitch);
        for _ in 0..15 {
            step_car(
                &mut bike,
                &c,
                DriveInput {
                    throttle: 0.,
                    steer: -1.,
                    brake: true,
                },
                1. / 30.,
            );
        }
        assert!(bike.roll < -0.1, "motorcycle transitions to left lean: {}", bike.roll);
        assert!(bike.pitch < -0.05, "motorcycle pitches nose-down (stoppie) under braking: {}", bike.pitch);
    }
    #[test]
    fn pak29_handling_differentiates_f1_muscle_drift_and_heavy_rhino_collision() {
        let c = flat();
        let mut f1 = make_vehicle([0., 0., 0.], 0., 0, 0, 27, 0, false, false); // F1
        let mut rhino = make_vehicle([20., 0., 0.], 0., 0, 0, 7, 0, false, false); // RHINO
        for _ in 0..60 {
            step_car(
                &mut f1,
                &c,
                DriveInput {
                    throttle: 1.,
                    ..DriveInput::default()
                },
                1. / 30.,
            );
            step_car(
                &mut rhino,
                &c,
                DriveInput {
                    throttle: 1.,
                    ..DriveInput::default()
                },
                1. / 30.,
            );
        }
        assert!(
            f1.speed > rhino.speed + 20. && f1.p[1] > rhino.p[1] + 25.,
            "F1 ({:.1} u/s) accelerates much faster and reaches higher top speed than Rhino ({:.1} u/s)",
            f1.speed,
            rhino.speed
        );

        let mut dukes = make_vehicle([0., 0., 0.], 0., 0, 0, 2, 0, false, false); // DUKES RWD
        dukes.speed = 24.;
        for _ in 0..18 {
            step_car(
                &mut dukes,
                &c,
                DriveInput {
                    throttle: 0.8,
                    steer: 1.,
                    brake: true,
                },
                1. / 30.,
            );
        }
        assert!(dukes.wheel_spin && dukes.lat_vel.abs() > 1.0, "handbrake turn induces lateral drift");
        assert!(dukes.roll < -0.02, "car body rolls outward in right turn: {}", dukes.roll);

        let mut s = empty();
        let mut heavy = make_vehicle([0., 0., 0.], 0., 0, 0, 7, 0, false, false); // RHINO (mass 5.0)
        heavy.speed = 20.;
        let light = make_vehicle([0., 8.0, 0.], 0., 0, 0, 1, 0, true, false); // BLISTA (mass 1.0)
        let light_y0 = light.p[1];
        s.cars.push(heavy);
        s.cars.push(light);
        s.driving = Some(0);
        let roads = Roads { nodes: vec![] };
        let assets = assets();
        for _ in 0..10 {
            s.tick(
                &c,
                &roads,
                &assets,
                s.cars[0].p,
                1. / 30.,
                DriveInput {
                    throttle: 1.,
                    ..DriveInput::default()
                },
            );
        }
        assert!(s.collisions > 0);
        assert!(s.cars[1].p[1] > light_y0 + 0.5, "heavy Rhino pushes lighter car forward on impact");
        assert!(!cars_overlap(&s.cars[0], &s.cars[1]));
    }

    #[test]
    fn psp_parity_skidmarks_traffic_following_and_pedestrian_calmness() {
        let c = flat();
        let roads = Roads { nodes: vec![] };
        let assets = assets();

        // 1. Skidmark alpha hold and fade
        let mut strip = SkidStrip {
            points: vec![([0., 0., 0.], [0.2, 0., 0.]), ([0., 1., 0.], [0.2, 1., 0.])],
            car_idx: 0,
            wheel: 0,
            active: false,
            hold_timer: 15.0,
            fade_timer: 4.0,
        };
        assert!((strip.alpha() - (12.0 / 31.0)).abs() < 1e-4, "alpha held at full value during hold_timer");
        strip.hold_timer = 0.0;
        strip.fade_timer = 2.0;
        assert!((strip.alpha() - 0.5 * (12.0 / 31.0)).abs() < 1e-4, "alpha fades linearly during fade_timer");

        // 2. Zone makeup ambient vehicles: no Rhino, Bulldozer, F1 or Gokart
        let mut rng = Rng::new(0x1234_5678);
        for i in 0..150 {
            let p = [((i * 37) % 4000 - 2000) as f32, ((i * 53) % 4000 - 2000) as f32, 0.0];
            let veh_id = crate::vehicle::choose_traffic_vehicle(p, &mut rng);
            assert!(
                veh_id != 7 && veh_id != 30 && veh_id != 27 && veh_id != 51 && veh_id != 45 && veh_id != 3,
                "ambient street traffic must never spawn special vehicle ID {veh_id}"
            );
        }

        // 3. Compact pedestrian evasive roll displacement (~2.2 - 2.5m)
        let mut roll = Roll::with_speed(0.0, 2, 5);
        let mut p = [0.0, 0.0, 0.0];
        let p_start = p;
        while roll.elapsed < ROLL_DURATION {
            roll.elapsed += 1.0 / 30.0;
            roll.travel(&c, &mut p, 1.0 / 30.0);
        }
        let roll_dist = (p[0] - p_start[0]).hypot(p[1] - p_start[1]);
        assert!(
            (2.1..=2.6).contains(&roll_dist),
            "pedestrian evasive roll displacement is compact (~2.2 - 2.5m), got {roll_dist:.2}m"
        );

        // 4. Traffic stopping for obstacle ahead
        let mut s = empty();
        let mut car_ahead = make_vehicle([0.0, 0.0, 0.0], 0.0, 0, 0, 0, 0, false, false);
        car_ahead.on_rail = true;
        car_ahead.speed = 8.0;
        s.cars.push(car_ahead);
        // Player standing directly in path at +6.0m
        let player_p = [0.0, 6.0, 0.0];
        for _ in 0..30 {
            s.tick(&c, &roads, &assets, player_p, 1.0 / 30.0, DriveInput::default());
        }
        assert_eq!(s.cars[0].speed, 0.0, "traffic car stops completely when obstacle is ahead");
        assert!(s.cars[0].start_delay > 0.0, "traffic car sets start_delay when stopped");

        // 5. Motorcycle NPC rider pose updates
        let mut s_bike = empty();
        let bike = make_vehicle([50.0, 50.0, 0.0], 0.0, 0, 0, 12, 0, false, false);
        assert!(bike.driver_appearance.is_some(), "non-parked traffic motorcycle has NPC driver");
        s_bike.cars.push(bike);
        s_bike.tick(&c, &roads, &assets, [0.0, 0.0, 0.0], 1.0 / 30.0, DriveInput::default());
        let rider_pose = &s_bike.cars[0].driver_pose;
        assert!(
            rider_pose.upper >= crate::ped::BIKE_RIDE_OFFSET,
            "NPC rider pose stepped with bike riding animation"
        );
    }

    #[test]
    fn authentic_psp_lighting_particles_motorcycle_crash_and_overtaking() {
        let c = flat();
        let roads = Roads { nodes: vec![] };
        let assets = assets();

        // 1. Vehicle lighting state (headlights and braking)
        let mut car = make_vehicle([0.0, 0.0, 0.0], 0.0, 0, 0, 0, 0, false, false);
        assert!(car.headlights, "headlights are enabled on active vehicles");
        step_car(&mut car, &c, DriveInput { throttle: 1.0, ..DriveInput::default() }, 0.1);
        assert!(!car.braking, "car accelerating forward is not braking");
        step_car(&mut car, &c, DriveInput { brake: true, ..DriveInput::default() }, 0.1);
        assert!(car.braking, "car with brake pressed is braking");

        // 2. Tire smoke particles generation on skidding
        let mut s_drift = empty();
        let mut drift_car = make_vehicle([0.0, 0.0, 0.0], 0.0, 0, 0, 2, 0, false, false);
        drift_car.rear_skidding = true;
        drift_car.speed = 10.0;
        s_drift.cars.push(drift_car);
        s_drift.tick(&c, &roads, &assets, [50.0, 50.0, 0.0], 1.0 / 30.0, DriveInput::default());
        assert!(!s_drift.particles.is_empty(), "tire smoke particles emitted during skidding");
        let p = &s_drift.particles[0];
        assert_eq!(p.kind, 0, "particle kind 0 is tire smoke");
        assert!(p.v[2] > 0.0, "smoke particle drifts upwards");

        // 3. Motorcycle high-speed crash and rider ejection
        let mut s_crash = empty();
        let mut bike = make_vehicle([0.0, 0.0, 0.0], 0.0, 0, 0, 12, 0, false, false);
        bike.speed = 14.0;
        s_crash.cars.push(bike);
        s_crash.driving = Some(0);
        let mut obst_car = make_vehicle([0.0, 1.2, 0.0], 0.0, 0, 0, 0, 0, true, true);
        obst_car.speed = 0.0;
        s_crash.cars.push(obst_car);
        let res = s_crash.tick(&c, &roads, &assets, [0.0, 0.0, 0.0], 1.0 / 30.0, DriveInput::default());
        assert!(res.is_none(), "player driving terminated upon high-speed motorcycle crash");
        assert!(s_crash.driving.is_none(), "player unseated from motorcycle");
        assert!(s_crash.cars[0].roll > 0.5, "crashed motorcycle falls onto its side");
        assert!(s_crash.cars[0].parked, "crashed motorcycle is parked/immobilized");
        assert!(s_crash.health < 127.0 || s_crash.armour < 127, "crash impact inflicts damage");

        // 4. Pedestrian low-speed bumper nudge vs high-speed lethal hit
        let mut s_ped = empty();
        let mut car_slow = make_vehicle([0.0, 0.0, 0.0], 0.0, 0, 0, 0, 0, false, false);
        car_slow.speed = 2.5;
        s_ped.cars.push(car_slow);
        s_ped.peds.push(make_ped([0.0, 1.2, 0.0], [0.0, 5.0, 0.0], 0));
        let ped_health_start = s_ped.peds[0].health;
        s_ped.tick(&c, &roads, &assets, [50.0, 50.0, 0.0], 1.0 / 30.0, DriveInput::default());
        assert!(s_ped.peds[0].health > 0.0, "low-speed nudge does not kill pedestrian");
        assert!(s_ped.peds[0].health < ped_health_start, "low-speed bumper nudge causes minor damage");
        assert!(s_ped.peds[0].panic > 0.0, "nudged pedestrian panics");

        // High speed lethal hit
        let mut s_lethal = empty();
        let mut car_fast = make_vehicle([0.0, 0.0, 0.0], 0.0, 0, 0, 0, 0, false, false);
        car_fast.speed = 12.0;
        s_lethal.cars.push(car_fast);
        s_lethal.peds.push(make_ped([0.0, 1.2, 0.0], [0.0, 5.0, 0.0], 0));
        s_lethal.tick(&c, &roads, &assets, [50.0, 50.0, 0.0], 1.0 / 30.0, DriveInput::default());
        assert_eq!(s_lethal.peds[0].health, 0.0, "high-speed hit is lethal knockdown");
        assert_eq!(s_lethal.kills, 1, "kill counter incremented on lethal hit");
    }

    #[test]
    fn world_interactions_missions_safehouses_and_drug_dealing() {
        let mut s = empty();
        let c = flat();
        let roads = Roads { nodes: vec![] };
        let assets = assets();

        // 1. Mission trigger: Huang meets Uncle Kenny at restaurant
        let ken_pos = crate::missions::Contact::UncleKenny.meeting_pos();
        assert!(s.interact(ken_pos), "interaction with Kenny beacon starts mission");
        assert!(s.missions.current_mission_idx.is_some(), "mission active");
        assert!(matches!(s.missions.state, crate::missions::MissionState::InProgress { .. }));
        assert!(s.banner.is_some(), "mission announcement banner displayed");

        // Complete prologue sinking car escape minigame
        for _ in 0..4 {
            s.interact(ken_pos);
        }
        assert!(s.minigame.is_none(), "sinking car escape completed");

        // 2. Safehouse interaction: sleep in bed restores health and records save
        s.health = 45.0;
        let safehouse_pos = s.safehouses.properties[0].pos; // Cerveza Heights Apartment (owned)
        assert!(s.interact(safehouse_pos), "interaction with owned safehouse");
        assert_eq!(s.health, 127.0, "bed sleep restores health to 127");

        // 3. Safehouse purchase: buy BOABO warehouse
        let unowned_pos = s.safehouses.properties[1].pos;
        s.cash = 10000;
        assert!(s.interact(unowned_pos), "purchasing unowned safehouse");
        assert!(s.safehouses.properties[1].owned, "BOABO safehouse is now owned");
        assert_eq!(s.cash, 5000, "safehouse price deducted from cash");

        // 4. Drug dealer discovery and wholesale trade
        let dealer_pos = s.drugs.dealers[0].pos;
        assert!(!s.drugs.dealers[0].discovered);
        s.tick(&c, &roads, &assets, dealer_pos, 1.0 / 30.0, DriveInput::default());
        assert!(s.drugs.dealers[0].discovered, "dealer discovered upon proximity");
        assert!(s.save_progress.dealer[0], "dealer recorded in save_progress");

        // Trade with dealer
        let prev_cash = s.cash;
        assert!(s.interact(dealer_pos), "trade interaction executed");
        assert!(s.cash != prev_cash || s.drugs.player_bag.total_units() > 0, "transaction took place");
    }

    #[test]
    fn odd_jobs_taxi_emergency_services_and_collectibles() {
        let mut s = empty();
        let c = flat();
        let roads = Roads { nodes: vec![] };
        let assets = assets();

        // 1. Enter Cabbie and start Taxi Driver Job
        let mut taxi = make_vehicle([250.0, 250.0, 5.0], 0.0, 0, 0, 13, 0, false, false);
        taxi.name = "CABBIE";
        s.cars.push(taxi);
        s.driving = Some(0);

        // Interact while driving starts taxi job
        assert!(s.interact([250.0, 250.0, 5.0]), "interaction in cabbie starts taxi service");
        assert!(s.jobs.is_active(), "taxi job active");
        s.tick(&c, &roads, &assets, [250.0, 250.0, 5.0], 1.0 / 30.0, DriveInput::default());
        assert!(s.prompt.is_some(), "shows quit job prompt");

        // Cancel job by pressing interact again
        assert!(s.interact([250.0, 250.0, 5.0]), "interaction toggles/cancels job");
        assert!(!s.jobs.is_active(), "job is cancelled");

        // 2. Dumpster search on foot
        s.driving = None;
        let dumpster_pos = s.collectibles.dumpsters[0].pos;
        let initial_cash = s.cash;
        assert!(s.interact(dumpster_pos), "searching dumpster on foot succeeds");
        assert!(s.collectibles.dumpsters[0].searched, "dumpster marked searched");
        assert!(s.cash >= initial_cash, "dumpster awards cash or weapon");

        // 3. Security camera destruction by explosion
        let cam_pos = s.collectibles.cameras[0].pos;
        assert!(!s.save_progress.package[0], "camera initially intact");
        s.explosions.trigger(cam_pos, crate::explosions::ExplosionType::Rocket, 1.0);
        s.tick(&c, &roads, &assets, [100.0, 100.0, 5.0], 1.0 / 30.0, DriveInput::default());
        assert!(s.collectibles.cameras[0].destroyed, "camera destroyed by blast");
        assert!(s.save_progress.package[0], "camera recorded in save_progress");
    }
}
