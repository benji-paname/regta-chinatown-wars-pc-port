// SPDX-License-Identifier: GPL-3.0-only
use crate::{
    camera::{Camera, CameraProfile},
    collision::{Collision, Walker},
    gpu::{Renderer, H, W},
    iso::{Disc, Result},
    pak::Pak,
    ped::{PedAssets, Pose},
    wad::Wad,
    world::World,
};
use minifb::{InputCallback, Key, MouseButton, MouseMode, Window, WindowOptions};
use std::{
    cell::RefCell,
    collections::HashSet,
    fs,
    path::Path,
    rc::Rc,
    time::{Duration, Instant},
};
#[derive(Default)]
struct KeyEvents {
    down: HashSet<Key>,
    pressed: HashSet<Key>,
}
struct KeyCallback(Rc<RefCell<KeyEvents>>);
impl InputCallback for KeyCallback {
    fn add_char(&mut self, _: u32) {}
    fn set_key_state(&mut self, key: Key, down: bool) {
        let mut events = self.0.borrow_mut();
        if down {
            if events.down.insert(key) {
                events.pressed.insert(key);
            }
        } else {
            events.down.remove(&key);
        }
    }
}
pub struct Options {
    pub english: Option<bool>,
    pub info_bands: Option<bool>,
    pub fullscreen: Option<bool>,
    pub gps_city: bool,
    pub slice: bool,
    pub demo_drive: bool,
    pub demo_combat: bool,
    pub armour: u8,
    pub health: u8,
    pub enter_at: Option<usize>,
    pub attack_at: Option<usize>,
    pub brake_at: Option<usize>,
    pub pos: [f32; 2],
    pub movement: [f32; 2],
    pub frames: Option<usize>,
    pub headless: bool,
    pub audit: bool,
    pub freecam: bool,
    pub camera_profile: Option<CameraProfile>,
    pub debug: bool,
    pub pace: u8,
    pub heading: f32,
    pub vault_at: Option<usize>,
    pub action_at: Option<usize>,
    pub car: Option<([f32; 2], f32)>,
    pub gps_target: Option<[f32; 2]>,
    pub gps_map: bool,
    pub minimap_rotation: Option<bool>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            english: None,
            info_bands: None,
            fullscreen: None,
            gps_city: false,
            slice: false,
            demo_drive: false,
            demo_combat: false,
            armour: 0,
            health: crate::radar::MAX_VITAL,
            enter_at: None,
            attack_at: None,
            brake_at: None,
            pos: [-1000., -1590.],
            movement: [0.; 2],
            frames: None,
            headless: false,
            audit: false,
            freecam: false,
            camera_profile: None,
            debug: false,
            pace: 2,
            heading: 0.,
            vault_at: None,
            action_at: None,
            car: Some(([-1000., -1582.], 0.)),
            gps_target: None,
            gps_map: false,
            minimap_rotation: None,
        }
    }
}
/// Optional, private reference tables retained for the experimental systems.
/// These are Android-derived in the existing installation, not PSP evidence.
/// Resolve relative to the executable so desktop shortcuts do not change behaviour.
fn reference_table(name: &str) -> Vec<u8> {
    let Ok(exe) = std::env::current_exe() else { return Vec::new(); };
    let Some(base) = exe.parent() else { return Vec::new(); };
    [base.join("data/tables"), base.join("../../data/tables"), base.join("data")]
        .into_iter()
        .find_map(|dir| fs::read(dir.join(name)).ok())
        .unwrap_or_default()
}

pub fn run(
    disc: &mut Disc,
    pak: &Pak,
    wad: &Wad,
    options: Options,
    capture: Option<&Path>,
    output: &Path,
) -> Result<()> {
    println!("Lecture de la carte PSP et des collisions...");
    let started = Instant::now();
    let mut collision = Collision::parse(&wad.read(disc, crate::wad::name_hash("world.bin"))?)?;
    let mut world = World::load(disc, pak)?;
    let shapes: usize = collision.cells.iter().map(|c| c.boxes.len()).sum();
    let cylinders: usize = collision.cells.iter().map(|c| c.cylinders.len()).sum();
    let collision_tris: usize = collision.cells.iter().map(|c| c.triangles.len()).sum();
    let total_props: usize = collision.cells.iter().map(|c| c.props.len()).sum();
    let total_emitters: usize = collision.cells.iter().map(|c| c.emitters.len()).sum();
    println!(
        "{} blocs, {} placements uniques, {} boites de collision, {} objets de rue, {} emetteurs urbains.",
        world.blocks,
        world.instances.len(),
        shapes,
        total_props,
        total_emitters,
    );
    fs::create_dir_all(output)?;
    if options.audit {
        let ids: std::collections::HashSet<_> = world.instances.iter().map(|i| i.model).collect();
        let mut count = 0;
        let mut triangles = 0;
        let mut failures = Vec::new();
        let mut bound_errors = Vec::new();
        let mut checked_bounds = 0;
        let mut texture_ids = std::collections::HashSet::new();
        for id in ids {
            match crate::model::parse(&pak.read(disc, id as usize)?.0) {
                Ok(m) => {
                    count += 1;
                    texture_ids.extend(m.texture_ids.iter().copied());
                    triangles += m.triangles.len();
                    if m.nodes == 1 {
                        checked_bounds += 1;
                        let mut error = 0f32;
                        for t in &m.triangles {
                            for p in t {
                                for a in 0..3 {
                                    error = error
                                        .max(m.declared_bounds[0][a] - p[a])
                                        .max(p[a] - m.declared_bounds[1][a]);
                                }
                            }
                        }
                        if error > 0.035 {
                            bound_errors.push(serde_json::json!({"id":id,"error":error}));
                        }
                    }
                }
                Err(e) => failures.push(serde_json::json!({"id":id,"error":e.to_string()})),
            }
        }
        let mut animated_textures = 0;
        let mut decoded_textures = std::collections::HashSet::new();
        for id in texture_ids {
            if id == 65535 {
                continue;
            }
            let raw = pak.read(disc, id as usize)?.0;
            if let Some(a) = crate::texture::animation(&raw)? {
                animated_textures += 1;
                for (id, _) in a {
                    if decoded_textures.insert(id) {
                        crate::texture::decode(id as usize, &pak.read(disc, id as usize)?.0)?;
                    }
                }
            } else if decoded_textures.insert(id) {
                crate::texture::decode(id as usize, &raw)?;
            }
        }
        let report = serde_json::json!({"blocks":world.blocks,"instances":world.instances.len(),"models":count,"textures_decoded":decoded_textures.len(),"animated_textures":animated_textures,"triangles":triangles,"model_failures":failures,"bounds_checked":checked_bounds,"bound_errors":bound_errors,"collision_cells":collision.cells.len(),"collision_boxes":shapes,"collision_cylinders":cylinders,"collision_triangles":collision_tris,"seconds":started.elapsed().as_secs_f64()});
        fs::write(
            output.join("map-audit.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        println!("{}", report);
        if !failures.is_empty() || !bound_errors.is_empty() {
            return Err("Certains modeles PSP ne sont pas encore decodes correctement.".into());
        }
        return Ok(());
    }
    let slice_enabled = options.slice || (!options.headless && options.frames.is_none());
    let car = if slice_enabled {
        Some(crate::vehicle::TestCar::load_mesh(disc, pak, [0., 0.], 0.)?)
    } else if let Some((xy, angle)) = options.car {
        Some(crate::vehicle::TestCar::load(
            disc,
            pak,
            &mut collision,
            xy,
            angle,
        )?)
    } else {
        None
    };
    let mut player = Walker::new(&collision, options.pos)?;
    player.set_heading(options.heading);
    let roads = crate::gps::Roads::parse(&wad.read(disc, crate::wad::name_hash("ai.bin"))?)?;
    let mut gps = crate::gps::Gps::new(roads);
    let terrain = crate::gps::original_terrain(disc, pak)?;
    let pedestrian_areas =
        std::sync::Arc::new(crate::actors::PedestrianAreas::from_radar(&terrain));
    // PSP world.bin section 3, merged across cells: the civilians' walking graph.
    let pavement = std::sync::Arc::new(crate::pavement::Network::build(&collision.cells));
    // PAK resources 1866 and 1867: Chinatown Wars prop library and physics kinds.
    let prop_library = std::sync::Arc::new(crate::props::PropLibrary::load(disc, pak)?);
    let weapon_tables_bytes = reference_table("weapon_tables.bin");
    let weapon_tables = crate::weapons::WeaponTables::parse(&weapon_tables_bytes).ok();
    let radio_tables_bytes = reference_table("radio_tables.bin");
    let radio_tables = crate::radio::RadioTables::parse(&radio_tables_bytes).ok();
    let weather_tables_bytes = reference_table("weather_tables.bin");
    let weather_tables = crate::weather::WeatherTables::parse(&weather_tables_bytes).ok();

    let overlays_key = disc.entries.keys().find(|k| k.ends_with("OVERLAYS.PAK")).cloned();
    let overlays_manager = if let Some(ref key) = overlays_key {
        if let Ok(bytes) = disc.read(key) {
            match crate::overlays::OverlayManager::parse(&bytes) {
                Ok(mgr) => {
                    println!("{} code overlays PSP indexes depuis {}.", mgr.overlays.len(), key);
                    Some(mgr)
                }
                Err(e) => {
                    eprintln!("Avertissement parseur overlays : {e}");
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    let mut scene = slice_enabled.then(|| {
        let mut s = crate::actors::Scene::new(
            &collision,
            &gps.roads,
            player.p,
            pedestrian_areas.clone(),
            pavement.clone(),
        );
        s.props = crate::props::PropsManager::with_library(prop_library.clone());
        s.radio = crate::radio::RadioManager::new(radio_tables.clone());
        s.weather = crate::weather::WeatherManager::new(weather_tables.clone());
        if let Some(wt) = &weapon_tables {
            let mut wm = crate::weapons::WeaponManager::new(wt);
            wm.give(1, wt);  // Batte de baseball
            wm.give(2, wt);  // Pistolet 9mm
            wm.give(4, wt);  // Fusil a pompe
            wm.give(6, wt);  // SMG
            wm.give(7, wt);  // Fusil d'assaut
            wm.give(9, wt);  // Fusil sniper
            wm.give(10, wt); // Lance-roquettes
            wm.give(11, wt); // Lance-flammes
            wm.give(13, wt); // Grenades
            wm.give(14, wt); // Cocktail Molotov
            s.weapons = Some(wm);
        }
        s
    });
    if let Some(scene) = scene.as_mut() {
        scene.health = options.health as f32;
        scene.armour = options.armour;
        // Explicit diagnostic modes position the actor for deterministic validation.
        if options.demo_drive {
            if let Some(car) = scene.cars.first() {
                let side = [car.angle.cos(), car.angle.sin()];
                player.p = [car.p[0] + side[0] * 3., car.p[1] + side[1] * 3., car.p[2]];
                player.set_heading(-car.angle.to_degrees());
            }
            if let Some(p) = scene.toggle_vehicle(&collision, player.p) {
                player.p = p;
            }
        } else if options.demo_combat {
            if let Some(npc) = scene.peds.first() {
                let f = [-npc.heading.sin(), npc.heading.cos()];
                let p = [npc.p[0] - f[0] * 2., npc.p[1] - f[1] * 2., npc.p[2]];
                if collision.body_clear(p, 0.875, 2.) {
                    player.p = p;
                    player.set_heading(-npc.heading.to_degrees());
                }
            }
        }
    }
    let hud_assets = crate::hud::Assets::load(disc, pak, wad)?;
    let gps_path = output.join("gps-settings.json");
    let saved_target = if !options.headless && options.frames.is_none() {
        fs::read(&gps_path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Option<[f32; 2]>>(&b).ok())
            .flatten()
    } else {
        None
    };
    if let Some(xy) = options.gps_target.or(saved_target) {
        gps.select(xy, player.p);
    }
    let hud_path = output.join("hud-settings.json");
    let mut hud_settings = if !options.headless && options.frames.is_none() {
        crate::hud::Settings::load(&hud_path)
    } else {
        crate::hud::Settings::default()
    };
    if let Some(rotate) = options.minimap_rotation {
        hud_settings.rotate_minimap = rotate;
    }
    if let Some(english) = options.english {
        hud_settings.english = english;
    }
    if let Some(bands) = options.info_bands {
        hud_settings.info_bands = bands;
    }
    if let Some(fullscreen) = options.fullscreen {
        hud_settings.fullscreen = fullscreen;
    }
    crate::locale::set(hud_settings.english);
    let mut map_ui = crate::gps::MapUi::new(player.p);

    // Optional reference lookup tables; resident sound bank comes from the PSP ISO.
    let sound_tables_bytes = reference_table("sound_tables.bin");
    let parsed_sound_tables = crate::audio::SoundTables::parse(&sound_tables_bytes);
    let sound_reference_loaded = parsed_sound_tables.is_ok();
    let sound_tables = parsed_sound_tables
        .unwrap_or_else(|_| crate::audio::SoundTables {
            events: vec![crate::audio::EventInfo::default(); 156],
            gears: vec![crate::audio::GearSound::default(); 20],
            collision_low: [135, 136, 135],
            collision_med: [137, 138, 137],
            collision_high: [139, 140, 141],
            rev_after_shift: [245760, 204800, 81920, 0],
            prop_sfx: vec![crate::audio::PropSfx::default(); 57],
        });
    let res_bank_data = wad
        .read(disc, crate::wad::name_hash("resbnk.bin"))
        .unwrap_or_default();
    let res_bank = crate::audio::SoundBank::parse(&res_bank_data).unwrap_or_default();
    let mut sound = crate::audio::SoundSystem::new(sound_tables, res_bank);
    map_ui.rotate_minimap = hud_settings.rotate_minimap;
    map_ui.info_bands = hud_settings.info_bands;
    map_ui.open = options.gps_map;
    if map_ui.open && !gps.route.is_empty() {
        map_ui.fit_route(&gps, player.p);
    }
    if options.gps_city {
        map_ui.open = true;
        map_ui.fit_city();
    }
    let mut map_drag: Option<[f32; 2]> = None;
    let ped = PedAssets::load(disc, pak)?;
    let mut pose = Pose::default();
    fs::write(
        output.join("player-audit.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "source":"PSP ULES01347 GAME.PAK", "animation_resource":202, "palette_resource":209,
            "animation_count":ped.animations.len(),"frames":ped.animations.iter().map(|a|a.frames.len()).sum::<usize>(),
            "atlas_variants":ped.textures.len(),"player_body_base":276,"upper_palette":4,"legs_palette":5,
            "locomotion_pairs":[[276,277],[278,279],[280,281],[436,437]],
            "vault_pair":crate::ped::VAULT_PAIR, "roll_pair":crate::ped::ROLL_PAIR,
            "vault_mapping":"416/417 matched to user PSP video 16.4-17.2s and 20.6-21.4s; pair0x8c/0x8d selected at EBOOT0x0883d970",
            "physics_reference":"ctw-native Player, adapted in Rust; static collision simplified",
            "psp_binary_parity_verified":false
        }))?,
    )?;
    let start = player.p;
    let initial_heading = (options.heading * 65536. / 360.) as i32 as u16;
    let settings_path = output.join("camera-settings.json");
    let saved_profile = if options.headless || options.frames.is_some() {
        None
    } else {
        fs::read(&settings_path)
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .and_then(|v| v["camera"].as_str().and_then(CameraProfile::parse))
    };
    let mut camera = Camera::new(
        player.p,
        initial_heading,
        options.camera_profile.or(saved_profile).unwrap_or_default(),
    );
    if options.freecam {
        camera.toggle_free();
    }
    world.stream(
        disc,
        pak,
        camera.eye(),
        if camera.is_free() { 410. } else { 210. },
    )?;
    let events = Rc::new(RefCell::new(KeyEvents::default()));
    let mut window = Window::new(
        "ReGTA CTW PC Recomp - by Benji Paname",
        W,
        H,
        WindowOptions {
            resize: false,
            ..Default::default()
        },
    )?;
    window.set_input_callback(Box::new(KeyCallback(events.clone())));
    if options.headless {
        unsafe {
            #[link(name = "user32")]
            extern "system" {
                fn ShowWindow(h: *mut std::ffi::c_void, n: i32) -> i32;
            }
            ShowWindow(window.get_window_handle() as _, 0);
        }
    }
    let mut renderer = Renderer::new(&window)?;
    if !options.headless && (options.frames.is_none() || options.fullscreen.is_some()) {
        renderer.set_fullscreen(hud_settings.fullscreen);
    }
    renderer.upload(&world, &ped, car.as_ref(), &terrain, &hud_assets)?;
    println!(
        "Rendu : {}. Chargement {:.2} s.",
        renderer.driver,
        started.elapsed().as_secs_f32()
    );
    let mut previous = Instant::now();
    let mut accumulator = 0f32;
    let mut stream_pos = player.p;
    let mut frames = 0;
    let mut ticks = 0;
    let mut debug = options.debug;
    let mut fps = 60.;
    let mut captured = false;
    let mut previous_mouse: Option<(f32, f32)> = None;
    let mut left_was_down = false;
    let mut vault_pending = false;
    let mut enter_pending = false;
    let mut attack_pending = false;
    let mut scene_samples = Vec::new();
    let mut vault_samples = Vec::new();
    let mut roll_samples = Vec::new();
    let mut gamepad = crate::gamepad::Gamepad::new();
    let mut paused = false;
    let mut pad_connected = false;
    let target_frames = options
        .frames
        .or(if options.headless { Some(1) } else { None });
    while window.is_open() {
        let now = Instant::now();
        let elapsed = (now - previous).as_secs_f32();
        previous = now;
        let dt = if target_frames.is_some() {
            1. / 60.
        } else {
            elapsed.min(0.1)
        };
        if elapsed > 0. {
            fps = fps * 0.95 + (1. / elapsed).min(1000.) * 0.05;
        }
        window.update();
        if !window.is_open() {
            break;
        }
        // Preserve taps whose down and up events arrived in one window update.
        let pressed = std::mem::take(&mut events.borrow_mut().pressed);
        let pda_active = scene.as_ref().is_some_and(|s| s.pda.open);
        let safehouse_active = scene.as_ref().is_some_and(|s| s.safehouse_session.is_some());
        let trade_active = scene.as_ref().is_some_and(|s| s.trade_session.is_some());
        let any_overlay_active = map_ui.open || pda_active || safehouse_active || trade_active;
        let escape = pressed.contains(&Key::Escape);
        if escape && !any_overlay_active {
            break;
        }
        let pad = if target_frames.is_none() {
            gamepad.poll(window.is_active())
        } else {
            crate::gamepad::Input::default()
        };
        pad_connected = pad.connected;
        if !any_overlay_active && pad.chord(crate::gamepad::START | crate::gamepad::B) {
            break;
        }
        let capture_pressed = pressed.contains(&Key::F12) || pad.pressed(crate::gamepad::DPAD_LEFT);
        if pressed.contains(&Key::I) || (pad.pressed(crate::gamepad::START) && pad.held(crate::gamepad::LB)) {
            if let Some(s) = scene.as_mut() {
                s.pda.toggle();
            }
        }
        if !any_overlay_active && (pressed.contains(&Key::P) || (pad.pressed(crate::gamepad::START) && !pad.held(crate::gamepad::LB))) {
            paused = !paused;
            accumulator = 0.;
            vault_pending = false;
            enter_pending = false;
            attack_pending = false;
            player.cancel_vault_input();
        }
        let mut reload = false;
        let left_down = window.get_mouse_down(MouseButton::Left);
        let mouse = renderer.mouse_pos().or_else(|| {
            window
                .get_mouse_pos(MouseMode::Discard)
                .map(|(x, y)| [x, y])
        });
        let click = left_down && !left_was_down && window.is_active();
        let bands_changed =
            pressed.contains(&Key::F5) || pad.chord(crate::gamepad::RB | crate::gamepad::BACK);
        if bands_changed {
            hud_settings.info_bands = !hud_settings.info_bands;
            map_ui.info_bands = hud_settings.info_bands;
            if !options.headless && options.frames.is_none() {
                hud_settings.save(&hud_path)?;
            }
        }
        let bands = map_ui.info_bands;
        let alt_enter = pressed.contains(&Key::Enter)
            && (window.is_key_down(Key::LeftAlt) || window.is_key_down(Key::RightAlt));
        let fullscreen_changed = pressed.contains(&Key::F11)
            || alt_enter
            || pad.chord(crate::gamepad::LB | crate::gamepad::BACK)
            || (bands
                && click
                && mouse.is_some_and(|p| crate::hud::contains(crate::hud::FULLSCREEN_BUTTON, p)));
        if fullscreen_changed && !options.headless {
            hud_settings.fullscreen = !renderer.fullscreen;
            renderer.set_fullscreen(hud_settings.fullscreen);
            previous_mouse = None;
            if options.frames.is_none() {
                let _ = hud_settings.save(&hud_path);
            }
        }
        let language_changed = pressed.contains(&Key::F4)
            || pad.chord(crate::gamepad::LB | crate::gamepad::RSTICK)
            || (bands
                && click
                && mouse.is_some_and(|p| crate::hud::contains(crate::hud::LANGUAGE_BUTTON, p)));
        if language_changed {
            hud_settings.english = !hud_settings.english;
            crate::locale::set(hud_settings.english);
            if !options.headless && options.frames.is_none() {
                hud_settings.save(&hud_path)?;
            }
        }
        let resume_click = bands
            && paused
            && !map_ui.open
            && click
            && mouse.is_some_and(|p| crate::hud::contains(crate::hud::ACTION_BUTTON, p));
        if resume_click {
            paused = false;
            accumulator = 0.;
            vault_pending = false;
            player.cancel_vault_input();
        }
        let gps_button = click
            && mouse.is_some_and(|p| {
                (bands && crate::hud::contains(crate::hud::MAP_BUTTON, p))
                    || crate::hud::contains(crate::hud::mini(bands), p)
            });
        let close_map =
            click && mouse.is_some_and(|p| crate::hud::contains(crate::hud::MAP_CLOSE, p));
        let clear_map =
            click && mouse.is_some_and(|p| crate::hud::contains(crate::hud::MAP_CLEAR, p));
        let rotation_button = (bands || map_ui.open)
            && click
            && mouse.is_some_and(|p| {
                crate::hud::contains(
                    if map_ui.open {
                        crate::hud::MAP_ROTATE
                    } else {
                        crate::hud::ROTATE_BUTTON
                    },
                    p,
                )
            });
        if pressed.contains(&Key::G) || pad.pressed(crate::gamepad::DPAD_RIGHT) || rotation_button {
            map_ui.rotate_minimap = !map_ui.rotate_minimap;
            hud_settings.rotate_minimap = map_ui.rotate_minimap;
            if !options.headless && options.frames.is_none() {
                if let Err(e) = hud_settings.save(&hud_path) {
                    eprintln!("Reglage radar non memorise : {e}");
                }
            }
        }
        let was_map_open = map_ui.open;
        if pressed.contains(&Key::M)
            || (pad.pressed(crate::gamepad::BACK)
                && !pad.held(crate::gamepad::RB)
                && !pad.held(crate::gamepad::LB))
            || (!map_ui.open && gps_button)
            || (map_ui.open && (escape || close_map || pad.pressed(crate::gamepad::B)))
        {
            map_ui.toggle(player.p);
            if map_ui.open && !gps.route.is_empty() {
                map_ui.fit_route(&gps, player.p);
            }
            accumulator = 0.;
            vault_pending = false;
            player.cancel_vault_input();
            previous_mouse = None;
            map_drag = None;
        }
        let map_consumes = was_map_open || any_overlay_active;
        let mut gps_changed = false;
        if map_ui.open && window.is_active() {
            let key_axis = |positive: Key, negative: Key| {
                i32::from(window.is_key_down(positive)) as f32
                    - i32::from(window.is_key_down(negative)) as f32
            };
            let axes = [
                key_axis(Key::Right, Key::Left) + pad.movement[0],
                key_axis(Key::Up, Key::Down) + pad.movement[1],
            ];
            map_ui.move_cursor(axes, dt);
            map_ui.view.center[0] += pad.look[0] * map_ui.view.span * dt * 0.65;
            map_ui.view.center[1] += pad.look[1] * map_ui.view.span * dt * 0.65;
            if let Some(pos) = mouse.filter(|&p| map_ui.view.contains(p)) {
                if click {
                    map_ui.cursor = pos;
                    gps_changed = gps.select(map_ui.view.world(pos), player.p);
                }
                if window.get_mouse_down(MouseButton::Right) {
                    if let Some(old) = map_drag {
                        let before = map_ui.view.world(old);
                        let after = map_ui.view.world(pos);
                        map_ui.view.center[0] += before[0] - after[0];
                        map_ui.view.center[1] += before[1] - after[1];
                    }
                    map_drag = Some(pos);
                } else {
                    map_drag = None;
                }
                if let Some((_, scroll)) = window.get_scroll_wheel() {
                    if scroll != 0. {
                        map_ui.view.zoom(1.25f32.powf(-scroll.clamp(-4., 4.)), pos);
                    }
                }
            } else {
                map_drag = None;
            }
            if pad.pressed(crate::gamepad::A) || (pressed.contains(&Key::Enter) && !alt_enter) {
                gps_changed = gps.select(map_ui.view.world(map_ui.cursor), player.p);
            }
            if pressed.contains(&Key::Delete) || clear_map || pad.pressed(crate::gamepad::X) {
                gps.clear();
                gps_changed = true;
            }
            let map_click = |r| click && mouse.is_some_and(|p| crate::hud::contains(r, p));
            if pressed.contains(&Key::Space)
                || pad.pressed(crate::gamepad::Y)
                || map_click(crate::hud::MAP_CENTER)
            {
                map_ui.recenter(player.p);
            }
            if pressed.contains(&Key::V)
                || pad.pressed(crate::gamepad::LSTICK)
                || map_click(crate::hud::MAP_CITY)
            {
                map_ui.fit_city();
            }
            if pressed.contains(&Key::F)
                || map_click(crate::hud::MAP_FIT)
                || (pad.pressed(crate::gamepad::RSTICK) && !pad.held(crate::gamepad::LB))
            {
                map_ui.fit_route(&gps, player.p);
            }
            if pressed.contains(&Key::Equal)
                || (pad.pressed(crate::gamepad::RB) && !pad.held(crate::gamepad::BACK))
                || map_click(crate::hud::MAP_ZOOM_IN)
            {
                map_ui.view.zoom(0.8, map_ui.cursor);
            }
            if pressed.contains(&Key::Minus)
                || map_click(crate::hud::MAP_ZOOM_OUT)
                || (pad.pressed(crate::gamepad::LB) && !pad.held(crate::gamepad::RSTICK))
            {
                map_ui.view.zoom(1.25, map_ui.cursor);
            }
        }
        if pda_active && window.is_active() {
            if let Some(s) = scene.as_mut() {
                if pressed.contains(&Key::Up) || pad.pressed(crate::gamepad::DPAD_UP) {
                    s.pda.navigate(-1);
                }
                if pressed.contains(&Key::Down) || pad.pressed(crate::gamepad::DPAD_DOWN) {
                    s.pda.navigate(1);
                }
                if (pressed.contains(&Key::Enter) && !alt_enter) || pad.pressed(crate::gamepad::A) {
                    match s.pda.current_view {
                        crate::pda::PdaView::Home => {
                            match s.pda.selected_index {
                                0 => { s.pda.current_view = crate::pda::PdaView::EmailInbox; s.pda.selected_index = 0; }
                                1 => { s.pda.current_view = crate::pda::PdaView::AmmuNationStore; s.pda.selected_index = 0; }
                                2 => { s.pda.current_view = crate::pda::PdaView::TradeStats; }
                                _ => { s.pda.current_view = crate::pda::PdaView::SystemSettings; }
                            }
                        }
                        crate::pda::PdaView::EmailInbox => {
                            s.pda.open_selected_email();
                        }
                        crate::pda::PdaView::EmailDetail(idx) => {
                            if let Some(email) = s.pda.emails.get(idx) {
                                if let Some(wp) = email.waypoint {
                                    gps.select([wp[0], wp[1]], player.p);
                                    s.banner = Some(("DESTINATION GPS AJOUTEE DEPUIS L'EMAIL".to_string(), 3.5));
                                }
                            }
                            s.pda.current_view = crate::pda::PdaView::EmailInbox;
                        }
                        crate::pda::PdaView::AmmuNationStore => {
                            let prev_cash = s.cash;
                            if let Ok(name) = s.pda.purchase_store_item(s.pda.selected_index, &mut s.cash, s.weapons.as_mut(), &mut s.armour) {
                                s.stats.add_spent_on_weapons(prev_cash.saturating_sub(s.cash));
                                s.banner = Some((format!("COMMANDE AMMU-NATION: {} RECUE !", name), 4.0));
                            } else {
                                s.banner = Some(("FONDS INSUFFISANTS POUR CET ARTICLE".to_string(), 3.0));
                            }
                        }
                        _ => {}
                    }
                }
                if escape || pressed.contains(&Key::Backspace) || pad.pressed(crate::gamepad::B) {
                    if s.pda.current_view != crate::pda::PdaView::Home {
                        s.pda.current_view = crate::pda::PdaView::Home;
                        s.pda.selected_index = 0;
                    } else {
                        s.pda.open = false;
                    }
                }
            }
        }
        if safehouse_active && window.is_active() {
            if let Some(s) = scene.as_mut() {
                if let Some(session) = &mut s.safehouse_session {
                    if pressed.contains(&Key::Tab) {
                        session.next_tab();
                    }
                    if pressed.contains(&Key::Key1) {
                        session.set_tab(crate::safehouse::SafehouseTab::Bed);
                    }
                    if pressed.contains(&Key::Key2) {
                        session.set_tab(crate::safehouse::SafehouseTab::DrugStash);
                    }
                    if pressed.contains(&Key::Key3) {
                        session.set_tab(crate::safehouse::SafehouseTab::CashSafe);
                    }
                    if pressed.contains(&Key::Key4) {
                        session.set_tab(crate::safehouse::SafehouseTab::Wardrobe);
                    }
                    if pressed.contains(&Key::Up) || pad.pressed(crate::gamepad::DPAD_UP) {
                        session.navigate(-1);
                    }
                    if pressed.contains(&Key::Down) || pad.pressed(crate::gamepad::DPAD_DOWN) {
                        session.navigate(1);
                    }
                    if pressed.contains(&Key::Left) {
                        session.adjust_qty(-5);
                    }
                    if pressed.contains(&Key::Right) {
                        session.adjust_qty(5);
                    }
                    if (pressed.contains(&Key::Enter) && !alt_enter) || pad.pressed(crate::gamepad::A) {
                        match session.active_tab {
                            crate::safehouse::SafehouseTab::Bed => {
                                s.health = 127.0;
                                s.stats.inc_number_of_times_saved();
                                session.message = Some("LIT : SAUVEGARDE COMMANDEE (+6H)".to_string());
                            }
                            crate::safehouse::SafehouseTab::DrugStash => {
                                let drug = match session.selected_item % 6 {
                                    0 => crate::drugs::DrugType::Acid,
                                    1 => crate::drugs::DrugType::Coke,
                                    2 => crate::drugs::DrugType::Downers,
                                    3 => crate::drugs::DrugType::Ecstasy,
                                    4 => crate::drugs::DrugType::Heroin,
                                    _ => crate::drugs::DrugType::Weed,
                                };
                                let qty = session.transfer_qty;
                                if s.drugs.player_bag.get(drug) >= qty {
                                    if s.drugs.stash_deposit(drug, qty) {
                                        session.message = Some(format!("DEPOT {} x{} DANS LA PLANQUE", drug.name(), qty));
                                    }
                                } else if s.drugs.stash_box.get(drug) >= qty {
                                    if s.drugs.stash_withdraw(drug, qty) {
                                        session.message = Some(format!("RETRAIT {} x{} DANS LE SAC", drug.name(), qty));
                                    }
                                }
                            }
                            crate::safehouse::SafehouseTab::CashSafe => {
                                let amount = match session.selected_item {
                                    0 => 100,
                                    1 => 1000,
                                    _ => s.cash,
                                };
                                if s.cash >= amount && amount > 0 {
                                    s.safehouses.deposit_cash(amount, &mut s.cash);
                                    session.message = Some(format!("DEPOT ${} AU COFFRE", amount));
                                } else {
                                    let safe_cash = s.safehouses.properties[s.safehouses.current_safehouse_idx].stash_cash;
                                    let withdraw_amt = amount.min(safe_cash);
                                    if withdraw_amt > 0 {
                                        s.safehouses.withdraw_cash(withdraw_amt, &mut s.cash);
                                        session.message = Some(format!("RETRAIT ${} DU COFFRE", withdraw_amt));
                                    }
                                }
                            }
                            crate::safehouse::SafehouseTab::Wardrobe => {
                                let outfit = match session.selected_item % 4 {
                                    0 => crate::safehouse::PlayerOutfit::CivilianGreen,
                                    1 => crate::safehouse::PlayerOutfit::TriadSuit,
                                    2 => crate::safehouse::PlayerOutfit::Tracksuit,
                                    _ => crate::safehouse::PlayerOutfit::Tourist,
                                };
                                s.safehouses.change_outfit(outfit);
                                session.message = Some(format!("TENUE : {}", outfit.name()));
                            }
                        }
                    }
                    if escape || pressed.contains(&Key::Backspace) || pad.pressed(crate::gamepad::B) {
                        s.safehouse_session = None;
                    }
                }
            }
        }
        if trade_active && window.is_active() {
            if let Some(s) = scene.as_mut() {
                if let Some(session) = &mut s.trade_session {
                    let d_idx = session.dealer_idx;
                    if pressed.contains(&Key::Tab) {
                        session.toggle_mode();
                    }
                    if pressed.contains(&Key::Up) || pad.pressed(crate::gamepad::DPAD_UP) {
                        session.navigate(-1);
                    }
                    if pressed.contains(&Key::Down) || pad.pressed(crate::gamepad::DPAD_DOWN) {
                        session.navigate(1);
                    }
                    if pressed.contains(&Key::Left) {
                        session.adjust_qty(-5);
                    }
                    if pressed.contains(&Key::Right) {
                        session.adjust_qty(5);
                    }
                    if (pressed.contains(&Key::Enter) && !alt_enter) || pad.pressed(crate::gamepad::A) {
                        let drug = session.selected_drug();
                        let qty = session.quantity;
                        match session.mode {
                            crate::drugs::TradeMode::Buy => {
                                match s.drugs.player_buy(d_idx, drug, qty, &mut s.cash) {
                                    Ok(cost) => {
                                        s.stats.inc_trades_done();
                                        s.stats.add_units_of_goods_bought(qty);
                                        s.stats.add_value_of_goods_bought(cost);
                                        session.message = Some(format!("ACHAT : {} x{} (-${})", drug.name(), qty, cost));
                                    }
                                    Err(err) => {
                                        session.message = Some(err.to_string());
                                    }
                                }
                            }
                            crate::drugs::TradeMode::Sell => {
                                match s.drugs.player_sell(d_idx, drug, qty, &mut s.cash) {
                                    Ok(earned) => {
                                        s.stats.inc_trades_done();
                                        s.stats.add_units_of_goods_sold(qty);
                                        s.stats.add_value_of_goods_sold(earned);
                                        session.message = Some(format!("VENTE : {} x{} (+${})", drug.name(), qty, earned));
                                    }
                                    Err(err) => {
                                        session.message = Some(err.to_string());
                                    }
                                }
                            }
                        }
                    }
                    if escape || pressed.contains(&Key::Backspace) || pad.pressed(crate::gamepad::B) {
                        s.trade_session = None;
                    }
                }
            }
        }
        if gps_changed && !options.headless && options.frames.is_none() {
            let target = gps.target.map(|t| [t.p[0], t.p[1]]);
            if let Err(e) = fs::write(&gps_path, serde_json::to_vec(&target)?) {
                eprintln!("GPS non memorise : {e}");
            }
        }
        let camera_button = bands
            && !map_consumes
            && left_down
            && !left_was_down
            && window.is_active()
            && mouse.is_some_and(|p| crate::hud::contains(crate::hud::FREE_BUTTON, p));
        let vault_button = bands
            && !resume_click
            && !map_consumes
            && left_down
            && !left_was_down
            && window.is_active()
            && !camera.is_free()
            && player.grounded
            && player.roll.is_none()
            && player.vault.is_none()
            && !paused
            && mouse.is_some_and(|p| crate::hud::contains(crate::hud::ACTION_BUTTON, p));
        let profile_button = bands
            && !map_consumes
            && left_down
            && !left_was_down
            && window.is_active()
            && mouse.is_some_and(|p| crate::hud::contains(crate::hud::PROFILE_BUTTON, p));
        if !map_consumes
            && (pressed.contains(&Key::F3)
                || profile_button
                || (pad.pressed(crate::gamepad::RSTICK) && !pad.held(crate::gamepad::LB)))
        {
            camera.set_profile(camera.profile.next(), player.p);
            if !options.headless && options.frames.is_none() {
                if let Err(error) = fs::write(
                    &settings_path,
                    serde_json::json!({"camera":camera.profile.name()}).to_string(),
                ) {
                    eprintln!("Impossible de memoriser la camera : {error}");
                }
            }
            previous_mouse = None;
            reload = true;
        }
        left_was_down = left_down;
        if !map_consumes
            && (pressed.contains(&Key::F2) || camera_button || pad.pressed(crate::gamepad::DPAD_UP))
        {
            camera.toggle_free();
            previous_mouse = None;
            accumulator = 0.;
            reload = true;
            vault_pending = false;
        }
        if !map_consumes && (pressed.contains(&Key::F1) || pad.pressed(crate::gamepad::DPAD_DOWN)) {
            debug = !debug;
        }
        if !map_consumes
            && (pressed.contains(&Key::R) || pad.chord(crate::gamepad::LB | crate::gamepad::Y))
        {
            player = Walker::new(&collision, options.pos)?;
            player.set_heading(options.heading);
            if slice_enabled {
                let mut s = crate::actors::Scene::new(
                    &collision,
                    &gps.roads,
                    player.p,
                    pedestrian_areas.clone(),
                    pavement.clone(),
                );
                s.health = options.health as f32;
                s.armour = options.armour;
                s.props = crate::props::PropsManager::with_library(prop_library.clone());
                s.radio = crate::radio::RadioManager::new(radio_tables.clone());
                s.weather = crate::weather::WeatherManager::new(weather_tables.clone());
                if let Some(wt) = &weapon_tables {
                    let mut wm = crate::weapons::WeaponManager::new(wt);
                    wm.give(1, wt);
                    wm.give(2, wt);
                    wm.give(4, wt);
                    wm.give(6, wt);
                    wm.give(7, wt);
                    wm.give(10, wt);
                    wm.give(13, wt);
                    wm.give(14, wt);
                    s.weapons = Some(wm);
                }
                scene = Some(s);
            }
            pose = Pose::default();
            camera = Camera::new(player.p, initial_heading, camera.profile);
            previous_mouse = None;
            accumulator = 0.;
            reload = true;
            vault_pending = false;
        }
        let active = !paused && !map_consumes && (target_frames.is_some() || window.is_active());
        let driving = scene.as_ref().is_some_and(|s| s.driving.is_some());
        let alive = scene.as_ref().is_none_or(|s| s.health > 0.);
        if !active || camera.is_free() || driving || !alive {
            vault_pending = false;
            player.cancel_vault_input();
        } else if pressed.contains(&Key::Space) || vault_button || pad.pressed(crate::gamepad::X) {
            vault_pending = true;
        }
        if active && !camera.is_free() && alive {
            enter_pending |= pressed.contains(&Key::E)
                || (bands
                    && click
                    && mouse.is_some_and(|p| crate::hud::contains(crate::hud::VEHICLE_BUTTON, p)))
                || (pad.pressed(crate::gamepad::Y) && !pad.held(crate::gamepad::LB));
            attack_pending |= pressed.contains(&Key::J)
                || pressed.contains(&Key::F)
                || (click
                    && mouse.is_some_and(|p| {
                        crate::hud::contains([1164., if bands { 109. } else { 49. }, 80., 76.], p)
                    }))
                || (pad.pressed(crate::gamepad::B) && !pad.held(crate::gamepad::START))
                || pad.pressed(crate::gamepad::RB);

            let weapon_wheel_active = scene.as_ref().is_some_and(|s| s.weapon_select.active);
            if pressed.contains(&Key::Q) || (pad.pressed(crate::gamepad::LB) && !pad.held(crate::gamepad::START)) {
                if let Some(s) = scene.as_mut() {
                    if let Some(wm) = s.weapons.as_mut() {
                        s.weapon_select.toggle(wm);
                    }
                }
            }
            if weapon_wheel_active {
                if let Some(s) = scene.as_mut() {
                    if let Some(wm) = s.weapons.as_mut() {
                        if pressed.contains(&Key::Tab) || pressed.contains(&Key::Right) || pad.pressed(crate::gamepad::DPAD_RIGHT) {
                            s.weapon_select.select_next(wm);
                        }
                        if pressed.contains(&Key::Left) || pad.pressed(crate::gamepad::DPAD_LEFT) {
                            s.weapon_select.select_prev(wm);
                        }
                        if let Some((_, scroll)) = window.get_scroll_wheel() {
                            if scroll > 0.5 {
                                s.weapon_select.select_prev(wm);
                            } else if scroll < -0.5 {
                                s.weapon_select.select_next(wm);
                            }
                        }
                        if pressed.contains(&Key::Enter) || pressed.contains(&Key::Space) || pad.pressed(crate::gamepad::A) {
                            s.weapon_select.close(wm);
                        }
                    }
                }
            } else {
                if pressed.contains(&Key::Tab) || pad.pressed(crate::gamepad::DPAD_RIGHT) {
                    if let Some(s) = scene.as_mut() {
                        if let Some(wm) = s.weapons.as_mut() {
                            wm.cycle_next();
                        }
                    }
                }
                if pad.pressed(crate::gamepad::DPAD_LEFT) {
                    if let Some(s) = scene.as_mut() {
                        if let Some(wm) = s.weapons.as_mut() {
                            wm.cycle_prev();
                        }
                    }
                }
                if let Some((_, scroll)) = window.get_scroll_wheel() {
                    if scroll > 0.5 {
                        if let Some(s) = scene.as_mut() {
                            if let Some(wm) = s.weapons.as_mut() {
                                wm.cycle_prev();
                            }
                        }
                    } else if scroll < -0.5 {
                        if let Some(s) = scene.as_mut() {
                            if let Some(wm) = s.weapons.as_mut() {
                                wm.cycle_next();
                            }
                        }
                    }
                }
            }
            if !driving {
                if pressed.contains(&Key::Z) || pad.pressed(crate::gamepad::RSTICK) {
                    if let Some(s) = scene.as_mut() {
                        s.sniper_scope.toggle();
                    }
                }
                if let Some(s) = scene.as_mut() {
                    let hold = window.is_key_down(Key::LeftShift) || pad.held(crate::gamepad::LSTICK);
                    s.sniper_scope.hold_breath(hold);
                }
            }
            if driving {
                if pressed.contains(&Key::N) || pressed.contains(&Key::Comma) || pad.pressed(crate::gamepad::DPAD_UP) {
                    if let Some(s) = scene.as_mut() {
                        s.radio.next_station();
                    }
                }
                if pressed.contains(&Key::B) || pressed.contains(&Key::Period) || pad.pressed(crate::gamepad::DPAD_DOWN) {
                    if let Some(s) = scene.as_mut() {
                        s.radio.prev_station();
                    }
                }
                if pressed.contains(&Key::H) || pad.pressed(crate::gamepad::LSTICK) {
                    if let Some(s) = scene.as_mut() {
                        if let Some(idx) = s.driving {
                            if idx < s.cars.len() {
                                if s.cars[idx].is_emergency() {
                                    s.cars[idx].siren_active = !s.cars[idx].siren_active;
                                } else {
                                    sound.play_car_horn(s.cars[idx].p, player.p, [-player.heading.sin(), player.heading.cos()]);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            enter_pending = false;
            attack_pending = false;
        }
        let held = |keys: &[Key]| {
            if active && keys.iter().any(|&k| window.is_key_down(k)) {
                1.
            } else {
                0.
            }
        };
        let dx = held(&[Key::D, Key::Right]) - held(&[Key::A, Key::Q, Key::Left]);
        let dy = held(&[Key::W, Key::Z, Key::Up]) - held(&[Key::S, Key::Down]);
        let direction = if target_frames.is_some() {
            options.movement
        } else if active && dx == 0. && dy == 0. {
            pad.movement
        } else {
            [dx, dy]
        };
        let run = if target_frames.is_some() {
            options.pace == 3
        } else {
            active && (held(&[Key::LeftShift, Key::RightShift]) > 0. || pad.held(crate::gamepad::A))
        };
        if camera.is_free() {
            let mouse_look = if active && window.get_mouse_down(MouseButton::Right) {
                mouse.map(|[x, y]| (x, y))
            } else {
                None
            };
            let mut look = match (mouse_look, previous_mouse) {
                (Some((x, y)), Some((px, py))) => [x - px, y - py],
                _ => [0., 0.],
            };
            if active {
                look[0] += pad.look[0] * dt * 450.;
                look[1] -= pad.look[1] * dt * 450.;
            }
            previous_mouse = mouse_look;
            camera.fly(
                [
                    direction[0],
                    direction[1],
                    held(&[Key::Space]) - held(&[Key::LeftCtrl, Key::RightCtrl])
                        + if active { pad.lift } else { 0. },
                ],
                look,
                dt,
                run,
            );
        } else {
            previous_mouse = None;
        }
        if active {
            accumulator += dt;
        }
        while accumulator >= 1. / 30. {
            if !camera.is_free() {
                let old = player.p;
                if let Some(s) = scene.as_mut() {
                    if (std::mem::take(&mut enter_pending) || options.enter_at == Some(ticks))
                        && player.roll.is_none()
                        && player.vault.is_none()
                        && s.health > 0.
                    {
                        if !s.interact(player.p) {
                            if let Some(p) = s.toggle_vehicle(&collision, player.p) {
                                let heading =
                                    s.driving.map(|i| s.cars[i].angle).unwrap_or(player.heading);
                                player = Walker::new(&collision, [p[0], p[1]])?;
                                player.p = p;
                                player.set_heading(-heading.to_degrees());
                                pose = Pose::default();
                                vault_pending = false;
                            }
                        }
                    }
                    if (std::mem::take(&mut attack_pending) || options.attack_at == Some(ticks))
                        && player.roll.is_none()
                        && player.vault.is_none()
                    {
                        s.attack(&collision, player.p, player.heading);
                    }
                    let input = crate::actors::DriveInput {
                        throttle: if target_frames.is_some() {
                            direction[1]
                        } else {
                            (dy + pad.lift + pad.movement[1]).clamp(-1., 1.)
                        },
                        steer: direction[0],
                        brake: held(&[Key::Space]) > 0.
                            || vault_button
                            || pad.held(crate::gamepad::X)
                            || options.brake_at.is_some_and(|t| ticks >= t),
                    };
                    let was_driving = s.driving;
                    let bike_info = was_driving.and_then(|idx| {
                        let car = &s.cars[idx];
                        if car.is_bike() {
                            Some((car.angle, car.p, car.speed))
                        } else {
                            None
                        }
                    });
                    if let Some(p) = s.tick(&collision, &gps.roads, &ped, player.p, 1. / 30., input)
                    {
                        if let Some(driving_idx) = s.driving {
                            player.p = p;
                            let car = &s.cars[driving_idx];
                            player.heading = car.angle;
                            player.velocity = [
                                -car.angle.sin() * car.speed,
                                car.angle.cos() * car.speed,
                                0.,
                            ];
                            player.distance += (p[0] - old[0]).hypot(p[1] - old[1]);
                            player.level = 0;
                            if car.is_bike() {
                                pose.ride(
                                    &ped,
                                    car.speed,
                                    car.speed < -0.2,
                                    car.speed.abs() < 1.0,
                                );
                            }
                            sound.update_vehicle_sound(car, true);
                        }
                        let has_fountain = s.props.active.iter().any(|p| p.fountain);
                        sound.update_hydrant_fountain(has_fountain);
                        for exp in &s.explosions.active {
                            if exp.timer > exp.max_timer - 0.04 {
                                sound.play_explosion(exp.pos, player.p, [-player.heading.sin(), player.heading.cos()]);
                            }
                        }
                        s.weather.update_time(ticks);
                        if let Some((thunder_sample, delay)) = s.weather.thunder_pending {
                            if delay <= 0.0 {
                                sound.play_spatial_sfx(
                                    thunder_sample as usize,
                                    [player.p[0], player.p[1], player.p[2] + 45.0],
                                    120.0,
                                    1.0,
                                    player.p,
                                    [-player.heading.sin(), player.heading.cos()],
                                );
                                s.weather.thunder_pending = None;
                            }
                        }
                    } else if let Some((bike_angle, bike_p, _)) = bike_info {
                        if was_driving.is_some() && s.driving.is_none() {
                            let f = [-bike_angle.sin(), bike_angle.cos()];
                            let ejected_p = [bike_p[0] + f[0] * 1.5, bike_p[1] + f[1] * 1.5, bike_p[2]];
                            let ground = collision.ground(ejected_p[0], ejected_p[1], ejected_p[2]);
                            let target_p = [ejected_p[0], ejected_p[1], ground.z];
                            if collision.body_clear(target_p, 0.4, 1.8) {
                                player.p = target_p;
                            } else {
                                player.p = bike_p;
                            }
                            player.set_heading(-bike_angle.to_degrees());
                            player.roll = Some(crate::roll::Roll::with_speed(bike_angle, 2, 14));
                            player.level = 0;
                            player.velocity = [0.; 3];
                            pose = Pose::default();
                        }
                    }
                    if let crate::missions::MissionState::InProgress { objective: crate::missions::MissionObjective::GoToLocation { target, .. }, .. } = &s.missions.state {
                        let t2 = [target[0], target[1]];
                        if gps.target.map(|t| [t.p[0], t.p[1]]) != Some(t2) {
                            gps.select(t2, player.p);
                        }
                    }
                    sound.update_radio(&s.radio);
                    sound.update_sirens(&s.police, &s.cars, player.p, [-player.heading.sin(), player.heading.cos()]);
                    sound.update_helicopter(
                        s.police.heli_manager.heli.as_ref().map(|h| h.p),
                        player.p,
                        [-player.heading.sin(), player.heading.cos()],
                    );
                    if let Some((_, delay)) = s.weather.thunder_pending {
                        if delay <= 0.0 {
                            sound.play_thunder();
                            s.weather.thunder_pending = None;
                        }
                    }
                    for (pos, lethal) in s.drain_pending_impact_sounds() {
                        sound.play_ped_impact(pos, lethal, player.p, [-player.heading.sin(), player.heading.cos()]);
                    }
                }
                let driving = scene.as_ref().is_some_and(|s| s.driving.is_some());
                let alive = scene.as_ref().is_none_or(|s| s.health > 0.);
                if !driving && alive {
                    if std::mem::take(&mut vault_pending) || options.action_at == Some(ticks) {
                        player.queue_action();
                    }
                    if options.vault_at == Some(ticks) {
                        player.queue_vault();
                    }
                    let was_vaulting = player.vault.is_some();
                    let was_rolling = player.roll.is_some();
                    let is_attacking = scene.as_ref().is_some_and(|s| s.attack_timer > 0.);
                    let mut step = |c: &Collision| {
                        player.step(
                            c,
                            if is_attacking {
                                [0.; 2]
                            } else {
                                camera.movement(direction[0], direction[1])
                            },
                            1. / 30.,
                            run,
                            if target_frames.is_some() {
                                options.pace == 1
                            } else {
                                held(&[Key::LeftCtrl, Key::RightCtrl]) > 0.
                                    || (dx == 0. && dy == 0. && pad.walk())
                            },
                        )
                    };
                    if let Some(s) = scene.as_mut() {
                        s.with_car_collision(&mut collision, &mut step);
                        if player.in_water {
                            s.swimming = true;
                        }
                    } else {
                        step(&collision);
                    }
                    if let Some(vault) = &player.vault {
                        pose.vault(&ped, vault.progress());
                    } else if let Some(roll) = &player.roll {
                        pose.roll(&ped, roll.progress());
                    } else if let Some(s) = scene.as_ref().filter(|s| s.attack_timer > 0.) {
                        pose.attack(&ped, 1. - s.attack_timer / 0.48);
                    } else {
                        pose.step(&ped, player.level, player.speed_fixed());
                    }
                    if (was_rolling || player.roll.is_some()) && roll_samples.len() < 128 {
                        roll_samples.push(serde_json::json!({"tick":ticks,"position":player.p,
                        "progress":player.roll.as_ref().map(|r|r.progress()),
                        "velocity":player.velocity,
                        "body_clear":collision.body_clear(player.p,crate::vault::RADIUS,crate::vault::HEIGHT),
                        "upper_animation":pose.upper,"upper_frame_fixed":pose.frame_upper}));
                    }
                    if (was_vaulting || player.vault.is_some()) && vault_samples.len() < 128 {
                        vault_samples.push(serde_json::json!({"tick":ticks,"position":player.p,
                        "progress":player.vault.as_ref().map(|v|v.progress()),
                        "kind":player.vault.as_ref().map(|v|v.plan.kind),
                        "body_clear":collision.body_clear(player.p, crate::vault::RADIUS, crate::vault::HEIGHT),
                        "upper_animation":pose.upper,"upper_frame_fixed":pose.frame_upper}));
                    }
                }
                let velocity = std::array::from_fn(|i| (player.p[i] - old[i]) * 30.);
                let heading = (-player.heading * 65536. / std::f32::consts::TAU) as i32 as u16;
                let walk_angle = if direction[0] != 0. || direction[1] != 0. {
                    Some(
                        (direction[0].atan2(direction[1]) * 65536. / std::f32::consts::TAU) as i32
                            as i16,
                    )
                } else {
                    None
                };
                camera.step_with_input(
                    player.p,
                    heading,
                    velocity,
                    &collision,
                    crate::camera::FollowInput {
                        sprint: run || driving,
                        walk_angle,
                        recenter: (driving && player.velocity[0].hypot(player.velocity[1]) > 3.)
                            || held(&[Key::C]) > 0.
                            || (pad.held(crate::gamepad::LB) && !pad.held(crate::gamepad::Y)),
                    },
                );
            }
            if let Some(s) = scene.as_ref() {
                if ticks % 15 == 0 && scene_samples.len() < 256 {
                    scene_samples.push(serde_json::json!({"tick":ticks,"p":player.p,"driving":s.driving,"health":s.health,"armour":s.armour,"hits":s.hits,"speed":s.driving.map(|i|s.cars[i].speed)}));
                }
            }
            accumulator -= 1. / 30.;
            ticks += 1;
        }
        gps.update(player.p, if active { dt } else { 0. });
        let centre = if camera.is_free() {
            camera.eye()
        } else {
            player.p
        };
        if reload
            || (centre[0] - stream_pos[0]).abs() > 45.
            || (centre[1] - stream_pos[1]).abs() > 45.
        {
            world.stream(
                disc,
                pak,
                centre,
                if camera.is_free() { 410. } else { 210. },
            )?;
            renderer.upload(&world, &ped, car.as_ref(), &terrain, &hud_assets)?;
            stream_pos = centre;
        }
        renderer.draw(
            &world,
            &player,
            &camera,
            &collision,
            &ped,
            &pose,
            car.as_ref(),
            fps,
            debug,
            ticks,
            pad_connected,
            paused,
            &gps,
            &map_ui,
            scene.as_ref(),
        )?;
        frames += 1;
        let finished = target_frames.is_some_and(|n| frames >= n);
        if finished || capture_pressed {
            if let Some(path) = capture {
                renderer.capture(path)?;
                captured = true;
            } else if !finished {
                renderer.capture(&output.join("capture-map.bmp"))?;
            }
        }
        renderer.present();
        if finished {
            break;
        }
        // minifb does not throttle raw OpenGL rendering. Avoid spinning when focus is lost.
        if !active {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    if !captured && window.is_open() {
        if let Some(path) = capture {
            renderer.draw(
                &world,
                &player,
                &camera,
                &collision,
                &ped,
                &pose,
                car.as_ref(),
                fps,
                debug,
                ticks,
                pad_connected,
                paused,
                &gps,
                &map_ui,
                scene.as_ref(),
            )?;
            renderer.capture(path)?;
        }
    }
    let mut report = serde_json::json!({"stage":"PSP native Rust vertical slice","gps":gps.report(player.p),"gps_map_open":map_ui.open,"hud":{"rotate_minimap":map_ui.rotate_minimap,"radar_angle":if map_ui.rotate_minimap {camera.radar_angle()}else{0.},"dual_bindings_always_visible":map_ui.info_bands,"dual_bindings_available":true,"info_bands":map_ui.info_bands,"minimap_rect":crate::hud::mini(map_ui.info_bands),"font_texture":261,"radar_symbols_texture":378,"viewport":[W,H],"minimap_shape":"circle","original_psp_radar_tiles":40,"radar_directory":331},"driver":renderer.driver,"start":start,"end":player.p,"distance":player.distance,"collision_contacts":player.contacts,"frames":frames,"game_ticks":ticks,"player":{"speed_level":player.level,"velocity":player.velocity,"grounded":player.grounded,"upper_animation":pose.upper,"legs_animation":pose.legs,"upper_frame_fixed":pose.frame_upper,"legs_frame_fixed":pose.frame_legs},"test_car":car.as_ref().map(|c| serde_json::json!({"name":"Admiral", "model":30,"info_resource":29,"position":c.p,"angle":c.angle,"collision_center":c.shape.c,"collision_half_extents":c.shape.h,"driveable":false})),"vault":{"psp_binary_parity_verified":false,"completed":player.vaults_completed,"rejected":player.vaults_rejected,"aborted":player.vaults_aborted,"active":player.vault.is_some(),"samples":vault_samples},"roll":{"completed":player.rolls_completed,"blocked":player.rolls_blocked,"active":player.roll.is_some(),"duration":crate::roll::DURATION,"initial_speed":crate::roll::INITIAL_SPEED,"motion_reference":"PSP ULES01347 impulse30, reaction100%, fixed-point grounded drag, integration17/512","psp_physics_parity_verified":false,"samples":roll_samples},"gamepad":{"api":"XInput","connected_at_end":pad_connected,"hardware_gameplay_verified":false},"camera":{"freecam":camera.is_free(),"eye":camera.eye(),"yaw":camera.yaw,"pitch":camera.pitch,"profile":camera.profile.name(),"reference":match camera.profile { CameraProfile::Psp => "ULES01347 EBOOT Update/TryToFaceAngle + cPlayerOnFoot", CameraProfile::Mobile => "supplied APK libGame.so FollowPedCam + cPlayerOnFoot", CameraProfile::Ds => "YGXP ARM9 0x020c6754/0x020c6cb0/0x020c6edc" },"recenter_mode":camera.recenter_mode(),"full_original_parity_verified":false,"height":camera.profile.height(),"fov_y":60,"near":0.39990234,"far":if camera.is_free() {350} else {150},"psp_binary_parity_verified":false},"map_blocks":world.blocks,"map_instances":world.instances.len(),"visible_instances":renderer.instances,"loaded_models":world.meshes.len(),"loaded_textures":world.textures.len(),"model_failures":world.failures,"capture":capture,"seconds":started.elapsed().as_secs_f64()});
    report["hud"]["psp_radar"] = serde_json::json!({
        "canvas":crate::radar::CANVAS,"center":crate::radar::CENTER,
        "inner_radius":crate::radar::MAP_RADIUS,"outer_radius":76. * crate::radar::OUTER_SCALE,
        "world_span":crate::radar::WORLD_SPAN,"map_alpha":191,"vital_max":127,
        "health_color":crate::radar::HEALTH,"armour_color":crate::radar::ARMOUR,
        "health":scene.as_ref().map(|s|s.health as u8).unwrap_or(127),
        "armour":scene.as_ref().map(|s|s.armour).unwrap_or(0),
        "sprite_directory":"SS_Radar.bin","native_tile_atlas":[2050,1794],
        "sprite_count":hud_assets.radar_sprites.len(),
        "sprites":hud_assets.radar_sprites.iter().filter(|s|[37,38,43].contains(&s.id))
            .map(|s|serde_json::json!({"id":s.id,"source":s.source,"offset":s.offset})).collect::<Vec<_>>(),
        "reference":"ULES01347 cRadar 0x089e3ba4/0x089e55b0/0x089e7324; SS_Radar.bin",
        "full_original_parity_verified":false
    });
    report["reference_tables"] = serde_json::json!({
        "weapons": weapon_tables.is_some(), "radio": radio_tables.is_some(),
        "weather": weather_tables.is_some(), "sound": sound_reference_loaded,
        "psp_provenance_verified": false
    });
    report["shader"] = serde_json::json!(renderer.shader_status);
    report["texture_pack"] = crate::texture_pack::report();
    report["hud"]["fullscreen"] = serde_json::json!(renderer.fullscreen);
    report["hud"]["language"] =
        serde_json::json!(if crate::locale::english() { "en" } else { "fr" });
    report["overlays"] = serde_json::json!({
        "loaded": overlays_manager.is_some(),
        "count": overlays_manager.as_ref().map(|o| o.overlays.len()).unwrap_or(0),
        "source": overlays_key,
    });
    report["missions"] = scene.as_ref().map(|s| serde_json::json!({
        "cash": s.cash,
        "missions_passed": s.missions.passed_count,
        "missions_failed": s.missions.failed_count,
        "safehouses_owned": s.safehouses.owned_count(),
        "dealers_discovered": s.drugs.discovered_count(),
        "wanted_stars": s.police.stars,
        "taxi_fares": s.jobs.total_taxi_fares,
        "vigilante_wave": s.jobs.vigilante_highest_wave,
        "firefighter_wave": s.jobs.firefighter_highest_wave,
        "paramedic_wave": s.jobs.paramedic_highest_wave,
        "cameras_destroyed": s.collectibles.destroyed_camera_count(),
        "stunts_completed": s.collectibles.completed_stunt_count(),
    })).unwrap_or(serde_json::Value::Null);
    report["slice"] = scene.as_ref().map(|s| serde_json::json!({
        "cars":s.cars.len(),"cargen_spawned":s.cars.iter().filter(|c|c.from_cargen).count(),"cargen_total":collision.cells.iter().map(|c|c.car_gens.len()).sum::<usize>(),
        "loaded_car_models":car.as_ref().map(|c|c.models.len()).unwrap_or(0),
        "pedestrians":s.peds.len(),"driving":s.driving,
        "health":s.health,"armour":s.armour,"hits":s.hits,"kills":s.kills,
        "vehicle_entries":s.vehicle_entries,"vehicle_exits":s.vehicle_exits,
        "collisions":s.collisions,"samples":scene_samples,
        "cars_end":s.cars.iter().map(|c|serde_json::json!({"p":c.p,"angle":c.angle,"speed":c.speed,"lat_vel":c.lat_vel,"gear":c.gear,"rpm":c.rpm,"roll":c.roll,"pitch":c.pitch,"wheel_spin":c.wheel_spin,"parked":c.parked,"info_id":c.info_id,"kind":c.spec().kind,"model_id":c.model_id,"name":c.name,"paint":c.paint,"from_cargen":c.from_cargen})).collect::<Vec<_>>(),
        "peds_end":s.peds.iter().map(|p|serde_json::json!({"p":p.p,"health":p.health,"appearance":p.appearance,"male":crate::ped::npc_is_male(p.appearance),"jogger":crate::ped::npc_is_jogger(p.appearance),"upper_animation":p.pose.upper,"evading":p.evading,"pedestrian_area":s.pedestrian_areas.contains(p.p),"on_pavement":p.on_pavement()})).collect::<Vec<_>>(),
        "peds_on_pavement":s.peds.iter().filter(|p|p.health>0.&&p.on_pavement()).count(),
        "pavement_nodes":s.pavement.nodes.len(),
        "physics":"Arcade reconstruction for presentation; not original vehicle or population parity"
    })).unwrap_or(serde_json::Value::Null);
    if slice_enabled {
        report["test_car"] = serde_json::Value::Null;
    }
    fs::write(
        output.join("play-validation.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", report);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn short_taps_survive_polling_and_repeat_does_not_toggle_twice() {
        let events = Rc::new(RefCell::new(KeyEvents::default()));
        let mut callback = KeyCallback(events.clone());
        callback.set_key_state(Key::F2, true);
        callback.set_key_state(Key::F2, false);
        assert!(events.borrow_mut().pressed.remove(&Key::F2));
        callback.set_key_state(Key::F2, true);
        assert!(events.borrow_mut().pressed.remove(&Key::F2));
        callback.set_key_state(Key::F2, true);
        assert!(events.borrow().pressed.is_empty());
        callback.set_key_state(Key::Space, true);
        callback.set_key_state(Key::Space, false);
        let mut pending = events.borrow_mut().pressed.remove(&Key::Space);
        // A render frame without a game tick must leave the action queued.
        assert!(pending);
        assert!(std::mem::take(&mut pending));
        assert!(!std::mem::take(&mut pending));
    }
}
