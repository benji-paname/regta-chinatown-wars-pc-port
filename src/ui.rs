// SPDX-License-Identifier: GPL-3.0-only
use crate::{
    iso::{Disc, Result},
    pak::Pak,
    texture::{self, Info, Texture},
};
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::{fs::File, io::Write, path::Path};
const W: usize = 1100;
const H: usize = 720;
pub(crate) fn glyph(c: char) -> [u8; 7] {
    match c {
        'a' => [0, 0, 14, 1, 15, 17, 15],
        'b' => [16, 16, 30, 17, 17, 17, 30],
        'e' => [0, 0, 14, 17, 31, 16, 14],
        'i' => [4, 0, 12, 4, 4, 4, 14],
        'j' => [2, 0, 6, 2, 2, 18, 12],
        'm' => [0, 0, 26, 21, 21, 21, 21],
        'n' => [0, 0, 30, 17, 17, 17, 17],
        'y' => [0, 0, 17, 17, 15, 1, 14],
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        ':' => [0, 4, 4, 0, 4, 4, 0],
        '.' => [0, 0, 0, 0, 0, 12, 12],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        _ => [0; 7],
    }
}
fn rect(b: &mut [u32], x: usize, y: usize, w: usize, h: usize, color: u32) {
    for y in y..(y + h).min(H) {
        for x in x..(x + w).min(W) {
            b[y * W + x] = color;
        }
    }
}
fn text(b: &mut [u32], x: usize, y: usize, s: &str, scale: usize, color: u32) {
    for (i, c) in s.to_ascii_uppercase().chars().enumerate() {
        for (r, v) in glyph(c).iter().enumerate() {
            for bit in 0..5 {
                if v & (1 << (4 - bit)) != 0 {
                    rect(
                        b,
                        x + (i * 6 + bit) * scale,
                        y + r * scale,
                        scale,
                        scale,
                        color,
                    );
                }
            }
        }
    }
}
fn place(b: &mut [u32], t: &Texture, area: [usize; 4]) {
    let [ax, ay, aw, ah] = area;
    let (size, pixels) = t
        .high_res
        .as_ref()
        .map(|r| (r.size, r.rgba.as_slice()))
        .unwrap_or(([t.info.width, t.info.height], t.rgba.as_slice()));
    let scale = (aw as f64 / t.info.width as f64).min(ah as f64 / t.info.height as f64);
    let w = (t.info.width as f64 * scale) as usize;
    let h = (t.info.height as f64 * scale) as usize;
    let x0 = ax + (aw - w) / 2;
    let y0 = ay + (ah - h) / 2;
    for y in 0..h {
        for x in 0..w {
            let rgba = pixels[(y * size[1] / h) * size[0] + x * size[0] / w];
            let bg = if (x / 16 + y / 16) % 2 == 0 {
                30u32
            } else {
                39
            };
            let a = rgba[3] as u32;
            let blend = |c: u8| (c as u32 * a + bg * (255 - a)) / 255;
            b[(y0 + y) * W + x0 + x] =
                (blend(rgba[0]) << 16) | (blend(rgba[1]) << 8) | blend(rgba[2]);
        }
    }
}
fn draw(disc: &mut Disc, pak: &Pak, list: &[Info], index: usize) -> Result<Vec<u32>> {
    let mut b = vec![0x10151b; W * H];
    rect(&mut b, 32, 28, 5, 64, 0xe6ae55);
    text(&mut b, 54, 30, "CHINATOWN WARS", 4, 0xf5f2e9);
    text(
        &mut b,
        56,
        72,
        "PSP / RUST / PORT PC EN DEVELOPPEMENT",
        2,
        0xa7b6c5,
    );
    rect(&mut b, 32, 117, W - 64, 1, 0x35404a);
    text(
        &mut b,
        36,
        137,
        "EXPLORATEUR DE RESSOURCES ORIGINALES",
        2,
        0xe6ae55,
    );
    let info = &list[index];
    let (raw, _) = pak.read(disc, info.id)?;
    let tex = texture::decode(info.id, &raw)?;
    place(&mut b, &tex, [36, 176, 740, 424]);
    let x = 816;
    text(&mut b, x, 188, "TEXTURE PSP", 2, 0xa7b6c5);
    text(&mut b, x, 222, &format!("ID {}", info.id), 3, 0xf5f2e9);
    text(
        &mut b,
        x,
        268,
        &format!("{} X {}", info.width, info.height),
        2,
        0xd4dce4,
    );
    text(&mut b, x, 298, &format!("{} BITS", info.bits), 2, 0xd4dce4);
    text(
        &mut b,
        x,
        328,
        &format!("{} COULEURS", info.colors),
        2,
        0xd4dce4,
    );
    text(&mut b, x, 396, "SOURCE", 2, 0xa7b6c5);
    text(&mut b, x, 426, "TON ISO PSP", 2, 0xd4dce4);
    text(&mut b, x, 456, "ULES01347", 2, 0xd4dce4);
    text(
        &mut b,
        x,
        520,
        &format!("{} / {}", index + 1, list.len()),
        2,
        0xe6ae55,
    );
    rect(&mut b, 32, 618, W - 64, 1, 0x35404a);
    text(
        &mut b,
        36,
        638,
        "FLECHES : PARCOURIR    PAGE HAUT/BAS : +/-20    ECHAP : FERMER",
        2,
        0xa7b6c5,
    );
    text(
        &mut b,
        36,
        681,
        "PROTOTYPE TECHNIQUE - VILLE ET GAMEPLAY NON IMPLEMENTES",
        2,
        0xe6ae55,
    );
    Ok(b)
}
pub fn bmp(path: &Path, b: &[u32]) -> Result<()> {
    bmp_sized(path, b, W, H)
}
pub fn bmp_sized(path: &Path, b: &[u32], width: usize, height: usize) -> Result<()> {
    if b.len() != width * height {
        return Err("Invalid capture dimensions".into());
    }
    let mut f = File::create(path)?;
    let size = 54 + width * height * 4;
    let mut h = vec![0u8; 54];
    h[..2].copy_from_slice(b"BM");
    h[2..6].copy_from_slice(&(size as u32).to_le_bytes());
    h[10..14].copy_from_slice(&54u32.to_le_bytes());
    h[14..18].copy_from_slice(&40u32.to_le_bytes());
    h[18..22].copy_from_slice(&(width as i32).to_le_bytes());
    h[22..26].copy_from_slice(&(-(height as i32)).to_le_bytes());
    h[26..28].copy_from_slice(&1u16.to_le_bytes());
    h[28..30].copy_from_slice(&32u16.to_le_bytes());
    f.write_all(&h)?;
    let mut pixels = Vec::with_capacity(width * height * 4);
    for p in b {
        pixels.extend(p.to_le_bytes());
    }
    f.write_all(&pixels)?;
    Ok(())
}

pub fn model_run(
    mesh: &crate::model::Mesh,
    id: usize,
    capture: Option<&Path>,
    headless: bool,
) -> Result<()> {
    let frame = |angle: f32| {
        let mut b = crate::model::preview(mesh, W, H - 170, angle);
        let mut out = vec![0x10151b; W * H];
        out[100 * W..(H - 70) * W].copy_from_slice(&b);
        b.clear();
        text(&mut out, 32, 24, "CHINATOWN WARS / PSP / RUST", 3, 0xf5f2e9);
        text(
            &mut out,
            32,
            66,
            &format!(
                "MODELE {} - {} SOMMETS - {} NOEUDS",
                id, mesh.vertices, mesh.nodes
            ),
            2,
            0xe6ae55,
        );
        text(
            &mut out,
            32,
            H - 52,
            "GEOMETRIE ORIGINALE - MATERIAUX ET UV NON APPLIQUES",
            2,
            0xe6ae55,
        );
        text(
            &mut out,
            32,
            H - 24,
            "FLECHES : ROTATION    ECHAP : FERMER    PROTOTYPE NON JOUABLE",
            2,
            0xa7b6c5,
        );
        out
    };
    let mut angle = 0.65;
    let mut pixels = frame(angle);
    println!(
        "Modele PSP {id}: {} sommets, {} triangles, {} noeuds",
        mesh.vertices,
        mesh.triangles.len(),
        mesh.nodes
    );
    if let Some(path) = capture {
        bmp(path, &pixels)?;
    }
    if headless {
        return Ok(());
    }
    let mut window = Window::new(
        "CTW-Rust | PSP | Geometrie originale",
        W,
        H,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(60);
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let delta = if window.is_key_down(Key::Right) {
            0.03
        } else if window.is_key_down(Key::Left) {
            -0.03
        } else {
            0.
        };
        if delta != 0. {
            angle += delta;
            pixels = frame(angle);
        }
        window.update_with_buffer(&pixels, W, H)?;
        if capture.is_some() {
            break;
        }
    }
    Ok(())
}
pub fn run(
    disc: &mut Disc,
    pak: &Pak,
    list: &[Info],
    id: usize,
    capture: Option<&Path>,
    headless: bool,
) -> Result<()> {
    if list.is_empty() {
        return Err("No supported textures".into());
    }
    let mut index = list
        .iter()
        .position(|t| t.id == id)
        .ok_or_else(|| format!("Texture {id} not indexed"))?;
    let mut frame = draw(disc, pak, list, index)?;
    if let Some(path) = capture {
        bmp(path, &frame)?;
    }
    if headless {
        return Ok(());
    }
    let mut window = Window::new(
        "CTW-Rust | PSP | Ressources - prototype non jouable",
        W,
        H,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(60);
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let previous = index;
        if window.is_key_pressed(Key::Right, KeyRepeat::Yes) {
            index = (index + 1) % list.len();
        }
        if window.is_key_pressed(Key::Left, KeyRepeat::Yes) {
            index = (index + list.len() - 1) % list.len();
        }
        if window.is_key_pressed(Key::PageDown, KeyRepeat::Yes) {
            index = (index + 20) % list.len();
        }
        if window.is_key_pressed(Key::PageUp, KeyRepeat::Yes) {
            index = (index + list.len() - 20.min(list.len())) % list.len();
        }
        if previous != index {
            frame = draw(disc, pak, list, index)?;
        }
        window.update_with_buffer(&frame, W, H)?;
        if capture.is_some() {
            break;
        }
    }
    Ok(())
}
