// SPDX-License-Identifier: GPL-3.0-only
mod actors;
mod audio;
mod camera;
mod collision;
mod drugs;
mod explosions;
mod gamepad;
mod gps;
mod gpu;
mod hud;
mod iso;
mod locale;
mod missions;
mod model;
mod overlays;
mod pak;
mod pavement;
mod ped;
mod ped_paths;
mod play;
mod police;
mod props;
mod psp;
mod radar;
mod radio;
mod roll;
mod safehouse;
mod stats;
#[cfg(test)]
mod test_support;
mod texture;
mod texture_pack;
mod ui;
mod vault;
mod vehicle;
mod wad;
mod weapons;
mod weather;
mod world;
use iso::{Disc, Result};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

const ISO_SHA: &str = "8c62ef8d7263ff0d42db40901927de4e86afd02f3d3450c3a96d0e2bc8d5168e";
const UPSTREAM: &str = "5fa1e36672ee88b0b232c3dc23652be0a33c7f17";
fn digest(d: &[u8]) -> String {
    format!("{:x}", Sha256::digest(d))
}
fn file_hash(path: &Path) -> Result<String> {
    let mut f = fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut b = vec![0; 1024 * 1024];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
fn run() -> Result<()> {
    let base = std::env::current_exe()?
        .parent()
        .ok_or("Executable path absent")?
        .to_path_buf();
    let mut iso = std::env::var_os("CTW_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("../../work/Chinatown-Wars-PSP.iso"));
    let mut output = base.join("data");
    let mut verify = false;
    let mut pavement_audit = false;
    let mut full_hash = false;
    let mut capture = None;
    let mut headless = false;
    let mut model_id = None;
    let mut id = 1000;
    let mut map = true;
    let mut play_options = play::Options::default();
    let default_pack = base.join("data/texture-pack");
    let mut texture_pack_path = default_pack
        .join("manifest.json")
        .is_file()
        .then_some(default_pack);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--iso" => iso = PathBuf::from(args.next().ok_or("Missing --iso path")?),
            "--texture-pack" => {
                texture_pack_path = Some(PathBuf::from(
                    args.next().ok_or("Missing texture pack directory")?,
                ))
            }
            "--original-textures" => texture_pack_path = None,
            "--output" => output = PathBuf::from(args.next().ok_or("Missing --output directory")?),
            "--verify" => verify = true,
            "--pavement-audit" => pavement_audit = true,
            "--hash-iso" => full_hash = true,
            "--headless" => headless = true,
            "--capture" => {
                capture = Some(PathBuf::from(args.next().ok_or("Missing capture path")?))
            }
            "--texture" => {
                id = args.next().ok_or("Missing texture ID")?.parse()?;
                map = false;
            }
            "--model" => {
                model_id = Some(args.next().ok_or("Missing model ID")?.parse::<usize>()?);
                map = false;
            }
            "--map" => map = true,
            "--camera" => {
                let name = args
                    .next()
                    .ok_or("Missing camera profile: psp, mobile or ds")?;
                play_options.camera_profile = Some(
                    camera::CameraProfile::parse(&name)
                        .ok_or("Camera must be psp, mobile or ds")?,
                );
            }
            "--gps" => {
                let x: f32 = args.next().ok_or("Missing GPS X")?.parse()?;
                let y: f32 = args.next().ok_or("Missing GPS Y")?.parse()?;
                if !x.is_finite() || !y.is_finite() {
                    return Err("GPS coordinates must be finite".into());
                }
                play_options.gps_target = Some([x, y]);
            }
            "--minimap-rotation" => {
                play_options.minimap_rotation = Some(match args.next().as_deref() {
                    Some("on") => true,
                    Some("off") => false,
                    _ => return Err("--minimap-rotation expects on or off".into()),
                })
            }
            "--gps-map" => play_options.gps_map = true,
            "--gps-city" => play_options.gps_city = true,
            "--info-bands" => {
                play_options.info_bands = Some(match args.next().as_deref() {
                    Some("on") => true,
                    Some("off") => false,
                    _ => return Err("--info-bands expects on or off".into()),
                })
            }
            "--fullscreen" | "--plein-ecran" => {
                map = true;
                play_options.fullscreen = Some(true);
            }
            "--windowed" | "--fenetre" => {
                map = true;
                play_options.fullscreen = Some(false);
            }
            "--language" => {
                play_options.english = Some(match args.next().as_deref() {
                    Some("en") => true,
                    Some("fr") => false,
                    _ => return Err("--language expects fr or en".into()),
                })
            }
            "--slice" => play_options.slice = true,
            "--armour" | "--health" => {
                let value: u8 = args.next().ok_or("Missing vital value (0..127)")?.parse()?;
                if value > radar::MAX_VITAL {
                    return Err("Vital value must be 0..127".into());
                }
                play_options.slice = true;
                if arg == "--armour" {
                    play_options.armour = value;
                } else {
                    play_options.health = value;
                }
            }
            "--demo-drive" => {
                play_options.slice = true;
                play_options.demo_drive = true;
            }
            "--demo-combat" => {
                play_options.slice = true;
                play_options.demo_combat = true;
            }
            "--enter-at" => {
                play_options.enter_at = Some(args.next().ok_or("Missing enter tick")?.parse()?)
            }
            "--attack-at" => {
                play_options.attack_at = Some(args.next().ok_or("Missing attack tick")?.parse()?)
            }
            "--brake-at" => {
                play_options.brake_at = Some(args.next().ok_or("Missing brake tick")?.parse()?)
            }
            "--no-test-car" => play_options.car = None,
            "--car-pos" => {
                let x = args.next().ok_or("Missing car X")?.parse()?;
                let y = args.next().ok_or("Missing car Y")?.parse()?;
                let angle: f32 = args.next().ok_or("Missing car angle")?.parse()?;
                play_options.car = Some(([x, y], angle.to_radians()));
            }
            "--car-demo" => {
                play_options.pos = [-1000., -1586.6];
                play_options.heading = 0.;
            }
            "--vault-demo" => {
                play_options.pos = [-1023.75, -1561.5];
                play_options.heading = 0.;
            }
            "--heading" => {
                play_options.heading = args.next().ok_or("Missing heading")?.parse()?;
                if !play_options.heading.is_finite() {
                    return Err("Heading must be finite".into());
                }
            }
            "--vault-at" => {
                play_options.vault_at = Some(args.next().ok_or("Missing vault tick")?.parse()?)
            }
            "--action-at" => {
                play_options.action_at = Some(args.next().ok_or("Missing action tick")?.parse()?)
            }
            "--pace" => {
                play_options.pace = match args.next().as_deref() {
                    Some("walk") => 1,
                    Some("run") => 2,
                    Some("sprint") => 3,
                    _ => return Err("--pace expects walk, run or sprint".into()),
                }
            }
            "--debug-collisions" => play_options.debug = true,
            "--freecam" => {
                map = true;
                play_options.freecam = true;
            }
            "--map-audit" => {
                map = true;
                play_options.audit = true;
            }
            "--pos" => {
                play_options.pos = [
                    args.next().ok_or("Missing X")?.parse()?,
                    args.next().ok_or("Missing Y")?.parse()?,
                ];
            }
            "--walk" => {
                play_options.movement = [
                    args.next().ok_or("Missing walk X")?.parse()?,
                    args.next().ok_or("Missing walk Y")?.parse()?,
                ];
            }
            "--frames" => play_options.frames = Some(args.next().ok_or("Missing frames")?.parse()?),
            "--help" | "-h" => {
                println!("ReGTA CTW PSP Recomp - exploration de la carte PSP en Rust.\nPar defaut : carte PSP, Huang et camera de suivi reconstruite.\nZQSD/WASD/fleches : deplacement, Ctrl : marche, Maj : sprint, Espace : roulade/franchir, P : pause, M : GPS, R : retour, F1 : collisions, F2 : camera libre, F3 : PSP/mobile/DS, C maintenu : recentrer, F12 : capture.\nManette XInput : stick gauche deplacement, A sprint, X roulade/franchir, LB recentrer, Start pause, clic stick droit camera.\nCamera libre : clic droit + souris, Espace/Ctrl hauteur, Maj rapide.\n--map --camera psp|mobile|ds --freecam --debug-collisions --pos X Y --heading degres --frames N --walk X Y --pace walk|run|sprint --map-audit\n--gps X Y --gps-map (destination et carte GPS)\n--action-at T (roulade/franchissement au tick T)\n--vault-demo (depart devant une barriere) --vault-at T (action au tick T, base 0)\n--car-demo (depart devant l Admiral) --car-pos X Y angle --no-test-car\n--iso fichier.iso --verify --hash-iso --output dossier\n--texture ID --model ID --capture image.bmp --headless");
                return Ok(());
            }
            _ => return Err(format!("Unknown option: {arg}").into()),
        }
    }
    texture_pack::initialize(texture_pack_path.as_deref())?;
    let mut disc = Disc::open(&iso)?;
    let sfo = psp::sfo(&disc.read("PSP_GAME/PARAM.SFO")?)?;
    if sfo.get("DISC_ID") != Some(&json!("ULES01347"))
        || sfo.get("DISC_VERSION") != Some(&json!("1.01"))
    {
        return Err("Cette version accepte uniquement Chinatown Wars PSP ULES01347 1.01.".into());
    }
    let wad = wad::Wad::open(&mut disc)?;
    if pavement_audit {
        let data = wad.read(&mut disc, wad::name_hash("world.bin"))?;
        let collision = collision::Collision::parse(&data)?;
        let mut report = ped_paths::audit(&collision.cells)?;
        report["network"] = pavement::Network::build(&collision.cells).stats();
        report["nodes_with_cover"] = json!(collision
            .cells
            .iter()
            .flat_map(|c| &c.paths.nodes)
            .filter(|n| n.cover != 0)
            .count());
        report["world_sha256"] = json!(digest(&data));
        fs::create_dir_all(&output)?;
        write_json(&output.join("pavement-validation.json"), &report)?;
        println!("{report}");
        return Ok(());
    }
    let pak = pak::Pak::open(&mut disc, pak::GAME)?;
    let overlay = pak::Pak::open(&mut disc, pak::OVERLAY)?;
    if map && !verify {
        play_options.headless = headless;
        return play::run(
            &mut disc,
            &pak,
            &wad,
            play_options,
            capture.as_deref(),
            &output,
        );
    }
    if !verify {
        if let Some(id) = model_id {
            return ui::model_run(
                &model::parse(&pak.read(&mut disc, id)?.0)?,
                id,
                capture.as_deref(),
                headless,
            );
        }
    }
    println!(
        "Chinatown Wars PSP ULES01347 1.01 | {} ressources ROM.WAD | {} blocs GAME.PAK",
        wad.entries.len(),
        pak.entries.len()
    );
    let mut textures = Vec::new();
    let mut compressed = 0;
    let mut decoded_pixels = 0u64;
    for e in &pak.entries {
        let (raw, z) = pak.read(&mut disc, e.id)?;
        compressed += usize::from(z);
        if let Some(info) = texture::info(e.id, &raw)? {
            if verify {
                let t = texture::decode(e.id, &raw)?;
                decoded_pixels += t.rgba.len() as u64;
            }
            textures.push(info);
        }
    }
    println!(
        "{} textures PSP indexees ; {} blocs decompresses.",
        textures.len(),
        compressed
    );
    if verify {
        fs::create_dir_all(&output)?;
        let mut gxt_files = 0;
        let mut gxt_strings = 0;
        let mut named = Vec::new();
        for e in wad.entries.values() {
            let d = wad.read(&mut disc, e.hash)?;
            if d.starts_with(b"DS_GXT") {
                let strings = wad::gxt(&d)?;
                gxt_files += 1;
                gxt_strings += strings.len();
            }
        }
        for name in ["world.bin", "utility.bin", "SS_Hud.bin", "SS_Portraits.bin"] {
            let h = wad::name_hash(name);
            let data = wad.read(&mut disc, h)?;
            named.push(json!({"name":name,"hash":format!("{h:08x}"),"bytes":data.len(),"sha256":digest(&data)}));
        }
        let mut overlays_compressed = 0;
        for e in &overlay.entries {
            let (_, z) = overlay.read(&mut disc, e.id)?;
            overlays_compressed += usize::from(z);
        }
        let eboot = disc.read("PSP_GAME/SYSDIR/EBOOT.BIN")?;
        let boot = disc.read("PSP_GAME/SYSDIR/BOOT.BIN")?;
        let hash = if full_hash {
            Some(file_hash(&iso)?)
        } else {
            None
        };
        if hash.as_deref().is_some_and(|h| h != ISO_SHA) {
            return Err("ISO fingerprint differs from the inspected source".into());
        }
        let report = json!({"schema":1,"stage":"PSP resource layer; no gameplay or MIPS recompilation yet",
          "reference":{"repository":"https://github.com/official-kryo-to/ctw-native","revision":UPSTREAM,"source_platform":"Android","runtime_language":"Rust","target_data":"PSP"},
          "iso":{"path":iso.canonicalize()?.to_string_lossy(),"bytes":disc.size,"sha256":hash,"expected_sha256":ISO_SHA,"fingerprint_verified":full_hash},
          "sfo":sfo,"iso_files":disc.entries.values().filter(|e|!e.directory).count(),
          "wad_entries":wad.entries.len(),"gxt_files":gxt_files,"gxt_strings":gxt_strings,"named_resources":named,
          "pak_entries":pak.entries.len(),"as01_blocks":compressed,"textures_decoded":textures.len(),"pixels_decoded":decoded_pixels,
          "overlays":overlay.entries.len(),"overlays_compressed":overlays_compressed,
          "eboot":{"sha256":digest(&eboot),"analysis":psp::executable(&eboot)?},"boot":psp::executable(&boot)?});
        write_json(&output.join("validation.json"), &report)?;
        write_json(&output.join("disc-index.json"), &disc.entries)?;
        write_json(&output.join("wad-index.json"), &wad.entries)?;
        write_json(&output.join("pak-index.json"), &pak)?;
        write_json(&output.join("texture-index.json"), &textures)?;
        println!("Verification reussie : {gxt_files} GXT, {gxt_strings} textes, {decoded_pixels} pixels decodes.\nRapport : {}",output.join("validation.json").display());
    }
    if verify && capture.is_none() && !headless {
        return Ok(());
    }
    if let Some(id) = model_id {
        ui::model_run(
            &model::parse(&pak.read(&mut disc, id)?.0)?,
            id,
            capture.as_deref(),
            headless,
        )
    } else {
        ui::run(&mut disc, &pak, &textures, id, capture.as_deref(), headless)
    }
}
fn main() {
    if let Err(e) = run() {
        eprintln!("ReGTA CTW : {e}");
        std::process::exit(1);
    }
}
