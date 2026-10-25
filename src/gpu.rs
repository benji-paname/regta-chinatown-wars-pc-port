// SPDX-License-Identifier: GPL-3.0-only
// Windows/OpenGL bootstrap adapted from Benjamin's VCS-Rust.
#![allow(dead_code)]
use crate::camera::Camera;
use crate::ped::{PedAssets, Pose};
use crate::{
    collision::{Collision, Walker},
    iso::Result,
    world::World,
};
use minifb::Window;
use std::{
    collections::{BTreeMap, HashMap},
    ffi::{c_char, c_void, CStr},
    path::Path,
    ptr::null_mut,
};
type Handle = *mut c_void;
#[path = "shaders.rs"]
mod shaders;

#[repr(C)]
#[derive(Default)]
struct PixelFormat {
    size: u16,
    version: u16,
    flags: u32,
    pixel_type: u8,
    color_bits: u8,
    red_bits: u8,
    red_shift: u8,
    green_bits: u8,
    green_shift: u8,
    blue_bits: u8,
    blue_shift: u8,
    alpha_bits: u8,
    alpha_shift: u8,
    accum_bits: u8,
    accum_red_bits: u8,
    accum_green_bits: u8,
    accum_blue_bits: u8,
    accum_alpha_bits: u8,
    depth_bits: u8,
    stencil_bits: u8,
    aux_buffers: u8,
    layer_type: u8,
    reserved: u8,
    layer_mask: u32,
    visible_mask: u32,
    damage_mask: u32,
}

pub const W: usize = 1280;
pub const H: usize = 720;
struct Batch {
    list: u32,
    texture: u16,
    flags: u8,
}
pub struct Renderer {
    window: Handle,
    dc: Handle,
    context: Handle,
    models: HashMap<u16, Vec<Batch>>,
    textures: Vec<u32>,
    texture_ids: HashMap<u16, u32>,
    ped_textures: HashMap<(u16, u8), u32>,
    car_textures: HashMap<u16, u32>,
    car_models: HashMap<u16, Vec<Batch>>,
    terrain_texture: u32,
    radar_origin: [f32; 2],
    radar_size: [f32; 2],
    radar_sprites: Vec<crate::radar::Sprite>,
    hud_texture: u32,
    radar_texture: u32,
    font_texture: u32,
    font_size: [f32; 2],
    font_height: f32,
    glyphs: Vec<[u16; 3]>,
    grade: Option<shaders::Grade>,
    pub shader_status: String,
    pub driver: String,
    pub triangles: usize,
    pub instances: usize,
    pub fullscreen: bool,
    windowed_style: isize,
    windowed_rect: Rect,
}
impl Renderer {
    pub fn new(window: &Window) -> Result<Self> {
        let handle = window.get_window_handle() as Handle;
        let mut r = Self {
            window: handle,
            dc: null_mut(),
            context: null_mut(),
            models: HashMap::new(),
            textures: Vec::new(),
            texture_ids: HashMap::new(),
            ped_textures: HashMap::new(),
            car_textures: HashMap::new(),
            car_models: HashMap::new(),
            terrain_texture: 0,
            radar_origin: [0.; 2],
            radar_size: [1.; 2],
            radar_sprites: Vec::new(),
            hud_texture: 0,
            radar_texture: 0,
            font_texture: 0,
            font_size: [512.; 2],
            font_height: 12.,
            glyphs: Vec::new(),
            grade: None,
            shader_status: String::new(),
            driver: String::new(),
            triangles: 0,
            instances: 0,
            fullscreen: false,
            windowed_style: 0,
            windowed_rect: Rect::default(),
        };
        unsafe {
            r.dc = GetDC(handle);
            if r.dc.is_null() {
                return Err("OpenGL: surface Windows absente".into());
            }
            let desc = PixelFormat {
                size: std::mem::size_of::<PixelFormat>() as u16,
                version: 1,
                flags: 1 | 4 | 32,
                color_bits: 32,
                alpha_bits: 8,
                depth_bits: 24,
                ..Default::default()
            };
            if GetPixelFormat(r.dc) == 0 {
                let f = ChoosePixelFormat(r.dc, &desc);
                if f == 0 || SetPixelFormat(r.dc, f, &desc) == 0 {
                    return Err("OpenGL: format indisponible".into());
                }
            }
            r.context = wglCreateContext(r.dc);
            if r.context.is_null() || wglMakeCurrent(r.dc, r.context) == 0 {
                return Err("OpenGL: contexte indisponible".into());
            }
            let s = glGetString(0x1f01);
            if s.is_null() {
                return Err("OpenGL: pilote absent".into());
            }
            r.driver = CStr::from_ptr(s.cast()).to_string_lossy().into_owned();
            let swap = wglGetProcAddress(b"wglSwapIntervalEXT\0".as_ptr().cast());
            let addr = swap as usize;
            if addr > 3 && addr != usize::MAX {
                let f: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(swap);
                f(1);
            }
            glPixelStorei(0x0cf5, 1);
            glPixelStorei(0x0d05, 1);
            glDepthFunc(0x0203);
            glAlphaFunc(0x0204, 0.35);
            glBlendFunc(0x0302, 0x0303);
            glTexEnvi(0x2300, 0x2200, 0x2100);
            glClearColor(0.19, 0.29, 0.34, 1.);
            if std::env::var_os("CTW_DISABLE_SHADERS").as_deref() == Some(std::ffi::OsStr::new("1"))
            {
                r.shader_status = "Standard rendering / shader pass disabled".into();
            } else {
                match shaders::Grade::new() {
                    Ok(grade) => {
                        r.grade = Some(grade);
                        r.shader_status = "GLSL 1.20 / colour grading, contrast, vignette".into();
                    }
                    Err(reason) => {
                        r.shader_status = format!("Standard rendering fallback: {reason}");
                    }
                }
            }
        }
        Ok(r)
    }
    fn clear_assets(&mut self) {
        unsafe {
            for b in self.models.values().chain(self.car_models.values()) {
                for b in b {
                    glDeleteLists(b.list, 1);
                }
            }
            glDeleteTextures(self.textures.len() as i32, self.textures.as_ptr());
        }
        self.models.clear();
        self.car_models.clear();
        self.texture_ids.clear();
        self.ped_textures.clear();
        self.car_textures.clear();
        self.textures.clear();
    }
    pub fn upload(
        &mut self,
        world: &World,
        ped: &PedAssets,
        car: Option<&crate::vehicle::TestCar>,
        _terrain: &crate::texture::Texture,
        hud: &crate::hud::Assets,
    ) -> Result<()> {
        self.clear_assets();
        self.terrain_texture = unsafe { upload_texture(&hud.radar_map.texture, false)? };
        self.radar_origin = hud.radar_map.origin;
        self.radar_size = [
            hud.radar_map.texture.info.width as f32,
            hud.radar_map.texture.info.height as f32,
        ];
        self.radar_sprites = hud.radar_sprites.clone();
        unsafe {
            glTexParameteri(0x0de1, 0x2801, 0x2601);
            glTexParameteri(0x0de1, 0x2800, 0x2601);
        }
        self.textures.push(self.terrain_texture);
        self.hud_texture = unsafe { upload_texture(&hud.weapon, false)? };
        self.textures.push(self.hud_texture);
        self.radar_texture = unsafe { upload_texture(&hud.radar, false)? };
        unsafe {
            glTexParameteri(0x0de1, 0x2801, 0x2601);
            glTexParameteri(0x0de1, 0x2800, 0x2601);
        }
        self.font_texture = unsafe { upload_texture(&hud.font, false)? };
        if hud.font.high_res.is_some() {
            unsafe {
                glTexParameteri(0x0de1, 0x2801, 0x2601);
                glTexParameteri(0x0de1, 0x2800, 0x2601);
            }
        }
        self.textures
            .extend([self.radar_texture, self.font_texture]);
        self.font_size = [hud.font.info.width as f32, hud.font.info.height as f32];
        self.font_height = hud.font_height as f32;
        self.glyphs = hud.glyphs.clone();
        let mut textures = HashMap::new();
        self.triangles = 0;
        unsafe {
            for (&id, t) in &world.textures {
                let tex = upload_texture(t, true)?;
                self.textures.push(tex);
                textures.insert(id, tex);
            }
            for (&key, t) in &ped.textures {
                let tex = upload_texture(t, false)?;
                self.textures.push(tex);
                self.ped_textures.insert(key, tex);
            }
            if let Some(car) = car {
                for (&id, t) in &car.textures {
                    let tex = upload_texture(t, true)?;
                    self.textures.push(tex);
                    self.car_textures.insert(id, tex);
                }
            }
            for (id, m) in world
                .meshes
                .iter()
                .map(|(&id, m)| (id, m))
                .chain(car.map(|c| (65534, &c.mesh)))
            {
                let mut groups = BTreeMap::<(u16, u8, u8), Vec<_>>::new();
                for s in &m.surfaces {
                    groups
                        .entry((s.texture, s.flags, s.alpha))
                        .or_default()
                        .push(s);
                }
                let mut batches = Vec::new();
                for ((tex, flags, alpha), tris) in groups {
                    let list = glGenLists(1);
                    if list == 0 {
                        return Err("OpenGL mesh allocation".into());
                    }
                    glNewList(list, 0x1300);
                    glBegin(4);
                    self.triangles += tris.len();
                    for t in tris {
                        let a = std::array::from_fn::<_, 3, _>(|i| t.v[1].p[i] - t.v[0].p[i]);
                        let b = std::array::from_fn::<_, 3, _>(|i| t.v[2].p[i] - t.v[0].p[i]);
                        let n = [
                            a[1] * b[2] - a[2] * b[1],
                            a[2] * b[0] - a[0] * b[2],
                            a[0] * b[1] - a[1] * b[0],
                        ];
                        let len = (n.iter().map(|v| v * v).sum::<f32>()).sqrt().max(1e-6);
                        let light =
                            0.67 + 0.33 * ((n[0] * 0.3 + n[1] * 0.5 + n[2] * 0.81) / len).abs();
                        glColor4f(light, light, light, alpha as f32 / 255.);
                        for v in t.v {
                            glTexCoord2f(v.uv[0], v.uv[1]);
                            glVertex3f(v.p[0], v.p[1], v.p[2]);
                        }
                    }
                    glEnd();
                    glEndList();
                    batches.push(Batch {
                        list,
                        texture: tex,
                        flags,
                    });
                }
                self.models.insert(id, batches);
            }
            if let Some(car) = car {
                for (&model_id, m) in &car.models {
                    let mut groups = BTreeMap::<(u16, u8, u8), Vec<_>>::new();
                    for s in &m.surfaces {
                        groups
                            .entry((s.texture, s.flags, s.alpha))
                            .or_default()
                            .push(s);
                    }
                    let mut batches = Vec::new();
                    for ((tex, flags, alpha), tris) in groups {
                        let list = glGenLists(1);
                        if list == 0 {
                            return Err("OpenGL car mesh allocation".into());
                        }
                        glNewList(list, 0x1300);
                        glBegin(4);
                        for t in tris {
                            let a = std::array::from_fn::<_, 3, _>(|i| t.v[1].p[i] - t.v[0].p[i]);
                            let b = std::array::from_fn::<_, 3, _>(|i| t.v[2].p[i] - t.v[0].p[i]);
                            let n = [
                                a[1] * b[2] - a[2] * b[1],
                                a[2] * b[0] - a[0] * b[2],
                                a[0] * b[1] - a[1] * b[0],
                            ];
                            let len = (n.iter().map(|v| v * v).sum::<f32>()).sqrt().max(1e-6);
                            let light = 0.67
                                + 0.33 * ((n[0] * 0.3 + n[1] * 0.5 + n[2] * 0.81) / len).abs();
                            glColor4f(light, light, light, alpha as f32 / 255.);
                            for v in t.v {
                                glTexCoord2f(v.uv[0], v.uv[1]);
                                glVertex3f(v.p[0], v.p[1], v.p[2]);
                            }
                        }
                        glEnd();
                        glEndList();
                        batches.push(Batch {
                            list,
                            texture: tex,
                            flags,
                        });
                    }
                    self.car_models.insert(model_id, batches);
                }
            }
            self.texture_ids = textures;
            let e = glGetError();
            if e != 0 {
                return Err(format!("OpenGL chargement: {e:#x}").into());
            }
        }
        Ok(())
    }
    pub fn draw(
        &mut self,
        world: &World,
        walker: &Walker,
        cam: &Camera,
        collision: &Collision,
        ped: &PedAssets,
        pose: &Pose,
        car: Option<&crate::vehicle::TestCar>,
        _fps: f32,
        debug: bool,
        ticks: usize,
        _pad_connected: bool,
        paused: bool,
        gps: &crate::gps::Gps,
        map_ui: &crate::gps::MapUi,
        scene: Option<&crate::actors::Scene>,
    ) -> Result<()> {
        unsafe {
            let mut rect = Rect::default();
            GetClientRect(self.window, &mut rect);
            glViewport(
                0,
                0,
                (rect.right - rect.left).max(1),
                (rect.bottom - rect.top).max(1),
            );
            glClear(0x4000 | 0x0100);
            glEnable(0x0b71);
            glEnable(0x0b44);
            glDisable(0x0b60);
            glMatrixMode(0x1701);
            glLoadIdentity();
            let near = 0x666 as f64 / 4096.;
            let top = near * (30f64.to_radians().tan());
            let aspect = (rect.right - rect.left) as f64 / (rect.bottom - rect.top).max(1) as f64;
            glFrustum(
                -top * aspect,
                top * aspect,
                -top,
                top,
                near,
                if cam.is_free() { 350. } else { 150. },
            );
            glMatrixMode(0x1700);
            glLoadMatrixf(cam.matrix().as_ptr());
            glDisable(0x0de1);
            glDisable(0x0be2);
            glDisable(0x0bc0);

            // 1. Broad distant ocean base
            glColor3f(0.08, 0.18, 0.24);
            glBegin(7);
            for (x, y) in [
                (-3500., -2500.),
                (3500., -2500.),
                (3500., 2500.),
                (-3500., 2500.),
            ] {
                glVertex3f(x, y, -7.6);
            }
            glEnd();

            // 2. Chinatown Wars Dynamic Animated Water Surface Mesh (cWaterRenderBlock)
            let center_x = walker.p[0];
            let center_y = walker.p[1];
            let time = (ticks as f32) * (1.0 / 30.0);
            let wave_fn = |wx: f32, wy: f32| -> f32 {
                -7.44 + 0.11 * (0.055 * wx + 1.8 * time).sin() + 0.08 * (0.065 * wy + 2.3 * time).cos()
            };

            let grid_step = 10.0;
            let grid_half = 14;
            let base_gx = (center_x / grid_step).floor() as i32;
            let base_gy = (center_y / grid_step).floor() as i32;

            glBegin(7);
            for ix in -grid_half..grid_half {
                for iy in -grid_half..grid_half {
                    let x0 = (base_gx + ix) as f32 * grid_step;
                    let y0 = (base_gy + iy) as f32 * grid_step;
                    let x1 = x0 + grid_step;
                    let y1 = y0 + grid_step;

                    let z00 = wave_fn(x0, y0);
                    let z10 = wave_fn(x1, y0);
                    let z11 = wave_fn(x1, y1);
                    let z01 = wave_fn(x0, y1);

                    let avg_h = (z00 + z10 + z11 + z01) * 0.25;
                    let crest_factor = ((avg_h - (-7.44)) / 0.18).clamp(0.0, 1.0);
                    let r = 0.08 + crest_factor * 0.08;
                    let g = 0.20 + crest_factor * 0.24;
                    let b = 0.28 + crest_factor * 0.22;
                    glColor3f(r, g, b);

                    glVertex3f(x0, y0, z00);
                    glVertex3f(x1, y0, z10);
                    glVertex3f(x1, y1, z11);
                    glVertex3f(x0, y1, z01);
                }
            }
            glEnd();

            glEnable(0x0bc0);
            glEnable(0x0be2);
            self.instances = 0;
            let mut visible = world.visible.clone();
            // A surface that does not write depth must follow the opaque world.
            // Otherwise a later road/building can erase it regardless of distance.
            for late in [false, true] {
                if late {
                    let eye = cam.eye();
                    let distance = |idx: usize| {
                        world.instances[idx]
                            .p
                            .iter()
                            .zip(eye)
                            .map(|(p, e)| (p - e) * (p - e))
                            .sum::<f32>()
                    };
                    visible.sort_by(|&a, &b| distance(b).total_cmp(&distance(a)));
                }
                for &idx in &visible {
                    let i = world.instances[idx];
                    if let Some(batches) = self.models.get(&i.model) {
                        if !late {
                            self.instances += 1;
                        }
                        glPushMatrix();
                        glTranslatef(i.p[0], i.p[1], i.p[2]);
                        glRotatef(i.angle.to_degrees(), 0., 0., 1.);
                        for b in batches {
                            if (b.flags & 0x20 != 0) != late {
                                continue;
                            }
                            if b.flags & 0x10 != 0 {
                                glDisable(0x0b44);
                            } else {
                                glEnable(0x0b44);
                            }
                            glDepthMask(if b.flags & 0x20 != 0 { 0 } else { 1 });
                            let id = world
                                .animations
                                .get(&b.texture)
                                .map(|a| crate::texture::frame(a, ticks))
                                .unwrap_or(b.texture);
                            let texture = self.texture_ids.get(&id).copied().unwrap_or(0);
                            if texture != 0 {
                                glEnable(0x0de1);
                                glBindTexture(0x0de1, texture);
                            } else {
                                glDisable(0x0de1);
                            }
                            glCallList(b.list);
                        }
                        glPopMatrix();
                    }
                }
                if let Some(scene) = scene {
                    for prop in &scene.props.active {
                        if prop.state == crate::props::PropState::Broken {
                            continue;
                        }
                        if let Some(batches) = self.models.get(&prop.model_id) {
                            glPushMatrix();
                            glTranslatef(prop.p[0], prop.p[1], prop.p[2]);
                            if prop.state == crate::props::PropState::Tilted {
                                glRotatef(
                                    prop.tilt.to_degrees(),
                                    prop.tilt_axis[0],
                                    prop.tilt_axis[1],
                                    0.0,
                                );
                            }
                            glRotatef(prop.angle.to_degrees(), 0., 0., 1.);
                            for b in batches {
                                if (b.flags & 0x20 != 0) != late {
                                    continue;
                                }
                                if b.flags & 0x10 != 0 {
                                    glDisable(0x0b44);
                                } else {
                                    glEnable(0x0b44);
                                }
                                glDepthMask(if b.flags & 0x20 != 0 { 0 } else { 1 });
                                let id = world
                                    .animations
                                    .get(&b.texture)
                                    .map(|a| crate::texture::frame(a, ticks))
                                    .unwrap_or(b.texture);
                                let texture = self.texture_ids.get(&id).copied().unwrap_or(0);
                                if texture != 0 {
                                    glEnable(0x0de1);
                                    glBindTexture(0x0de1, texture);
                                } else {
                                    glDisable(0x0de1);
                                }
                                glCallList(b.list);
                            }
                            glPopMatrix();
                        }
                    }
                }
                if let Some(car) = car.filter(|_| scene.is_none()) {
                    if let Some(batches) = self.models.get(&65534) {
                        glPushMatrix();
                        glTranslatef(car.p[0], car.p[1], car.p[2]);
                        glRotatef(car.angle.to_degrees(), 0., 0., 1.);
                        for b in batches {
                            if (b.flags & 0x08 != 0) != late {
                                continue;
                            }
                            if b.flags & 0x10 != 0 {
                                glDisable(0x0b44);
                            } else {
                                glEnable(0x0b44);
                            }
                            glDepthMask(if late { 0 } else { 1 });
                            glEnable(0x0de1);
                            glBindTexture(
                                0x0de1,
                                self.car_textures.get(&b.texture).copied().unwrap_or(0),
                            );
                            glCallList(b.list);
                        }
                        glPopMatrix();
                    }
                }
                if let (Some(scene), Some(car)) = (scene, car) {
                    glDepthFunc(0x0201);
                    for actor in &scene.cars {
                        if distance2(actor.p, walker.p) > 200. * 200. && !cam.is_free() {
                            continue;
                        }
                        let Some(batches) = self
                            .car_models
                            .get(&actor.model_id)
                            .or_else(|| self.models.get(&65534))
                        else {
                            continue;
                        };
                        let mesh = car.models.get(&actor.model_id).unwrap_or(&car.mesh);
                        let scale = actor.visual_scale();
                        glPushMatrix();
                        glTranslatef(
                            actor.p[0],
                            actor.p[1],
                            actor.p[2] - mesh.declared_bounds[0][2] * scale,
                        );
                        glRotatef(actor.angle.to_degrees(), 0., 0., 1.);
                        glRotatef(actor.pitch.to_degrees(), 1., 0., 0.);
                        glRotatef(actor.roll.to_degrees(), 0., 1., 0.);
                        if (scale - 1.0).abs() > 1e-4 {
                            glScalef(scale, scale, scale);
                        }
                        for b in batches {
                            if (b.flags & 0x08 != 0) != late {
                                continue;
                            }
                            if b.flags & 0x10 != 0 {
                                glDisable(0x0b44);
                            } else {
                                glEnable(0x0b44);
                            }
                            glDepthMask(if late { 0 } else { 1 });
                            glEnable(0x0de1);
                            let tex_id = if b.texture == 2 {
                                2 + u16::from(actor.paint)
                            } else {
                                b.texture
                            };
                            let tex = self
                                .car_textures
                                .get(&tex_id)
                                .or_else(|| self.car_textures.get(&b.texture))
                                .copied()
                                .unwrap_or(0);
                            glBindTexture(0x0de1, tex);
                            glCallList(b.list);
                        }
                        glPopMatrix();
                    }
                    glDepthFunc(0x0203);
                }
            }
            glDepthMask(1);
            glDisable(0x0de1);
            glDisable(0x0bc0);
            glDisable(0x0be2);
            glDisable(0x0b44);
            glEnable(0x0be2);
            glDepthMask(0);
            if let Some(scene) = scene {
                for strip in &scene.skidmarks {
                    let alpha = strip.alpha();
                    if alpha <= 0.005 || strip.points.len() < 2 {
                        continue;
                    }
                    glColor4f(0.04, 0.04, 0.04, alpha);
                    glBegin(5);
                    for &(left, right) in &strip.points {
                        glVertex3f(left[0], left[1], left[2]);
                        glVertex3f(right[0], right[1], right[2]);
                    }
                    glEnd();
                }
                for p in &scene.particles {
                    let progress = (p.life / p.max_life).clamp(0.0, 1.0);
                    let hs = p.size * (1.0 + p.grow * progress) * 0.5;
                    match p.kind {
                        0 => {
                            // Smoke puff
                            let alpha = (1.0 - progress) * 0.42;
                            if alpha > 0.01 {
                                glColor4f(0.88, 0.88, 0.90, alpha);
                                glBegin(6);
                                glVertex3f(p.p[0], p.p[1], p.p[2] + hs * 0.5);
                                glColor4f(0.88, 0.88, 0.90, 0.0);
                                glVertex3f(p.p[0] - hs, p.p[1] - hs * 0.7, p.p[2]);
                                glVertex3f(p.p[0] + hs, p.p[1] - hs * 0.7, p.p[2]);
                                glVertex3f(p.p[0] + hs, p.p[1] + hs * 0.7, p.p[2] + hs);
                                glVertex3f(p.p[0] - hs, p.p[1] + hs * 0.7, p.p[2] + hs);
                                glVertex3f(p.p[0] - hs, p.p[1] - hs * 0.7, p.p[2]);
                                glEnd();
                            }
                        }
                        1 => {
                            // Water spray geyser (smashed fire hydrant)
                            let alpha = ((1.0 - progress) * 0.70).clamp(0.0, 0.70);
                            if alpha > 0.01 {
                                glColor4f(0.65, 0.85, 0.98, alpha);
                                glBegin(6);
                                glVertex3f(p.p[0], p.p[1], p.p[2] + hs * 0.6);
                                glColor4f(0.80, 0.92, 1.0, 0.0);
                                glVertex3f(p.p[0] - hs * 0.6, p.p[1] - hs * 0.6, p.p[2]);
                                glVertex3f(p.p[0] + hs * 0.6, p.p[1] - hs * 0.6, p.p[2]);
                                glVertex3f(p.p[0] + hs * 0.6, p.p[1] + hs * 0.6, p.p[2] + hs * 0.8);
                                glVertex3f(p.p[0] - hs * 0.6, p.p[1] + hs * 0.6, p.p[2] + hs * 0.8);
                                glVertex3f(p.p[0] - hs * 0.6, p.p[1] - hs * 0.6, p.p[2]);
                                glEnd();
                            }
                        }
                        2 => {
                            // Garbage debris (smashed trash can)
                            let alpha = (1.0 - progress * 0.5).clamp(0.0, 0.95);
                            if alpha > 0.01 {
                                glColor4f(0.38, 0.35, 0.30, alpha);
                                glBegin(6);
                                glVertex3f(p.p[0], p.p[1], p.p[2]);
                                glVertex3f(p.p[0] - hs, p.p[1], p.p[2] + hs);
                                glVertex3f(p.p[0], p.p[1] + hs, p.p[2] + hs);
                                glVertex3f(p.p[0] + hs, p.p[1], p.p[2]);
                                glEnd();
                            }
                        }
                        3 => {
                            // Wood slat splinters (smashed barrier/fence)
                            let alpha = (1.0 - progress * 0.6).clamp(0.0, 0.95);
                            if alpha > 0.01 {
                                glColor4f(0.85, 0.65, 0.40, alpha);
                                glBegin(6);
                                glVertex3f(p.p[0], p.p[1], p.p[2]);
                                glVertex3f(p.p[0] - hs * 0.5, p.p[1] - hs * 1.5, p.p[2] + hs);
                                glVertex3f(p.p[0] + hs * 0.5, p.p[1] + hs * 1.5, p.p[2] + hs);
                                glVertex3f(p.p[0] + hs * 0.5, p.p[1] - hs * 1.5, p.p[2]);
                                glEnd();
                            }
                        }
                        _ => {}
                    }
                }

                // Render active explosion visual effects
                for exp in &scene.explosions.active {
                    for p in &exp.particles {
                        let hs = p.size * 0.5;
                        glColor4f(p.color[0], p.color[1], p.color[2], p.color[3]);
                        match p.kind {
                            0 => {
                                // Additive flash
                                glBegin(6);
                                glVertex3f(p.pos[0], p.pos[1], p.pos[2]);
                                glColor4f(p.color[0], p.color[1], p.color[2], 0.0);
                                for i in 0..=8 {
                                    let a = (i as f32) * (std::f32::consts::TAU / 8.0);
                                    glVertex3f(p.pos[0] + a.cos() * hs, p.pos[1] + a.sin() * hs, p.pos[2] + hs * 0.5);
                                }
                                glEnd();
                            }
                            1 | 2 => {
                                // Fireball cloud & smoke
                                glBegin(6);
                                glVertex3f(p.pos[0], p.pos[1], p.pos[2] + hs * 0.3);
                                glColor4f(p.color[0], p.color[1], p.color[2], 0.0);
                                for i in 0..=6 {
                                    let a = (i as f32) * (std::f32::consts::TAU / 6.0);
                                    glVertex3f(p.pos[0] + a.cos() * hs, p.pos[1] + a.sin() * hs, p.pos[2] + hs * 0.8);
                                }
                                glEnd();
                            }
                            3 => {
                                // Fiery spark line
                                glBegin(1);
                                glVertex3f(p.pos[0], p.pos[1], p.pos[2]);
                                glVertex3f(p.pos[0] - p.vel[0] * 0.04, p.pos[1] - p.vel[1] * 0.04, p.pos[2] - p.vel[2] * 0.04);
                                glEnd();
                            }
                            _ => {}
                        }
                    }
                }

                // Render detached vehicle body fragments (vehicledebris.cpp)
                for d in &scene.explosions.debris {
                    glPushMatrix();
                    glTranslatef(d.pos[0], d.pos[1], d.pos[2]);
                    glRotatef(d.rot[0].to_degrees(), 1.0, 0.0, 0.0);
                    glRotatef(d.rot[1].to_degrees(), 0.0, 1.0, 0.0);
                    glRotatef(d.rot[2].to_degrees(), 0.0, 0.0, 1.0);
                    glColor4f(d.color[0], d.color[1], d.color[2], 1.0);
                    let (hw, hl, hh) = (0.35, 0.55, 0.08);
                    glBegin(7); // GL_QUADS
                    glVertex3f(-hw, -hl, hh);
                    glVertex3f(hw, -hl, hh);
                    glVertex3f(hw, hl, hh);
                    glVertex3f(-hw, hl, hh);
                    glVertex3f(-hw, -hl, -hh);
                    glVertex3f(-hw, hl, -hh);
                    glVertex3f(hw, hl, -hh);
                    glVertex3f(hw, -hl, -hh);
                    glEnd();
                    glPopMatrix();
                }

                // Chinatown Wars authentic Boat Wake Trails (cBoatWakeEmitter)
                for car in &scene.cars {
                    if let Some(wake) = &car.wake_emitter {
                        for seg in &wake.segments {
                            if seg.alpha > 0.02 {
                                glColor4f(0.88, 0.94, 0.98, (seg.alpha * 0.65).clamp(0.0, 1.0));
                                glBegin(7); // GL_QUADS
                                glVertex3f(seg.left_edge[0], seg.left_edge[1], seg.left_edge[2]);
                                glVertex3f(seg.right_edge[0], seg.right_edge[1], seg.right_edge[2]);
                                glVertex3f(seg.right_edge[0] + 0.3, seg.right_edge[1] + 0.3, seg.right_edge[2]);
                                glVertex3f(seg.left_edge[0] - 0.3, seg.left_edge[1] - 0.3, seg.left_edge[2]);
                                glEnd();
                            }
                        }
                    }
                    if let Some(spray) = &car.spray_emitter {
                        for p in &spray.particles {
                            if p.alpha > 0.02 {
                                glColor4f(0.92, 0.96, 1.0, (p.alpha * 0.70).clamp(0.0, 1.0));
                                let sz = p.size;
                                glBegin(7);
                                glVertex3f(p.pos[0] - sz, p.pos[1] - sz, p.pos[2]);
                                glVertex3f(p.pos[0] + sz, p.pos[1] - sz, p.pos[2]);
                                glVertex3f(p.pos[0] + sz, p.pos[1] + sz, p.pos[2] + sz);
                                glVertex3f(p.pos[0] - sz, p.pos[1] + sz, p.pos[2] + sz);
                                glEnd();
                            }
                        }
                    }
                }

                // Chinatown Wars Swimming Water Ripples (cParticleEmitterWaterSplash)
                if scene.swimming {
                    let swim_p = walker.p;
                    for r_idx in 0..3 {
                        let phase = (scene.swim_timer * 2.0 + (r_idx as f32) * 0.8) % 2.0;
                        let r_size = 0.4 + phase * 1.6;
                        let r_alpha = ((1.0 - phase / 2.0) * 0.55).max(0.0);
                        glColor4f(0.85, 0.95, 1.0, r_alpha);
                        glLineWidth(2.0);
                        glBegin(2); // GL_LINE_LOOP
                        for a_step in 0..16 {
                            let angle = (a_step as f32) * std::f32::consts::TAU / 16.0;
                            glVertex3f(swim_p[0] + angle.cos() * r_size, swim_p[1] + angle.sin() * r_size, -7.38);
                        }
                        glEnd();
                        glLineWidth(1.0);
                    }
                }

                for trace in &scene.bullet_traces {
                    let alpha = (trace.life / trace.max_life).clamp(0.0, 1.0);
                    glColor4f(1.0, 0.96, 0.65, alpha);
                    glBegin(1); // GL_LINES
                    glVertex3f(trace.start[0], trace.start[1], trace.start[2]);
                    glVertex3f(trace.end[0], trace.end[1], trace.end[2]);
                    glEnd();
                }
                if scene.targeting.locked_target.is_some() {
                    let tp = scene.targeting.reticle_pos;
                    let h = scene.targeting.reticle_health;
                    let r = 1.0;
                    let g = (1.0 - (h as f32 / 255.0)).clamp(0.0, 0.85);
                    let b = 0.08;
                    glColor4f(r, g, b, 0.90);
                    let rw = 0.65;
                    glBegin(2); // GL_LINE_LOOP
                    glVertex3f(tp[0] - rw, tp[1] - rw, tp[2] + 0.05);
                    glVertex3f(tp[0] + rw, tp[1] - rw, tp[2] + 0.05);
                    glVertex3f(tp[0] + rw, tp[1] + rw, tp[2] + 0.05);
                    glVertex3f(tp[0] - rw, tp[1] + rw, tp[2] + 0.05);
                    glEnd();
                }

                // Chinatown Wars authentic Projectiles Rendering (cBaseProjectile, cGrenade, cMolotov, cRocket)
                for proj in &scene.projectiles.list {
                    if !proj.active {
                        continue;
                    }
                    glPushMatrix();
                    glTranslatef(proj.pos[0], proj.pos[1], proj.pos[2]);
                    match proj.kind {
                        crate::weapons::ProjectileKind::Rocket => {
                            let fwd = [proj.vel[0], proj.vel[1]];
                            let angle = fwd[1].atan2(fwd[0]).to_degrees();
                            glRotatef(angle, 0.0, 0.0, 1.0);
                            glColor4f(0.35, 0.38, 0.36, 1.0);
                            let (rw, rl, rh) = (0.12, 0.55, 0.12);
                            glBegin(7);
                            glVertex3f(-rl, -rw, rh);
                            glVertex3f(rl, -rw, rh);
                            glVertex3f(rl, rw, rh);
                            glVertex3f(-rl, rw, rh);
                            glEnd();
                            // Warhead red tip
                            glColor4f(0.95, 0.22, 0.15, 1.0);
                            glBegin(4);
                            glVertex3f(rl + 0.30, 0.0, 0.0);
                            glVertex3f(rl, -rw * 1.2, -rh * 1.2);
                            glVertex3f(rl, rw * 1.2, rh * 1.2);
                            glEnd();
                            // Thruster fire exhaust plume
                            glColor4f(1.0, 0.82, 0.25, 0.9);
                            glBegin(6);
                            glVertex3f(-rl - 0.45, 0.0, 0.0);
                            for i in 0..=6 {
                                let a = (i as f32) * (std::f32::consts::TAU / 6.0);
                                glVertex3f(-rl, a.cos() * 0.18, a.sin() * 0.18);
                            }
                            glEnd();
                        }
                        crate::weapons::ProjectileKind::Grenade => {
                            let spin = (proj.fuse_timer * 18.0).sin();
                            glRotatef(spin * 45.0, 1.0, 1.0, 0.0);
                            glColor4f(0.28, 0.38, 0.22, 1.0);
                            let s = 0.18;
                            glBegin(7);
                            glVertex3f(-s, -s, s);
                            glVertex3f(s, -s, s);
                            glVertex3f(s, s, s);
                            glVertex3f(-s, s, s);
                            glEnd();
                            glColor4f(0.65, 0.65, 0.68, 1.0);
                            glBegin(1);
                            glVertex3f(0.0, 0.0, s);
                            glVertex3f(0.0, 0.0, s + 0.12);
                            glEnd();
                        }
                        crate::weapons::ProjectileKind::Molotov => {
                            let spin = (proj.fuse_timer * 14.0).cos();
                            glRotatef(spin * 60.0, 1.0, 0.0, 1.0);
                            glColor4f(0.72, 0.52, 0.18, 0.85);
                            let (bw, bh) = (0.12, 0.32);
                            glBegin(7);
                            glVertex3f(-bw, -bw, bh);
                            glVertex3f(bw, -bw, bh);
                            glVertex3f(bw, bw, bh);
                            glVertex3f(-bw, bw, bh);
                            glEnd();
                            let flicker = ((proj.fuse_timer * 40.0).sin() * 0.08).abs();
                            glColor4f(1.0, 0.70 + flicker, 0.15, 0.95);
                            glBegin(4);
                            glVertex3f(0.0, 0.0, bh + 0.25 + flicker);
                            glVertex3f(-0.08, -0.08, bh);
                            glVertex3f(0.08, 0.08, bh);
                            glEnd();
                        }
                        crate::weapons::ProjectileKind::Flashbang => {
                            glColor4f(0.80, 0.82, 0.85, 1.0);
                            let s = 0.16;
                            glBegin(7);
                            glVertex3f(-s, -s, s);
                            glVertex3f(s, -s, s);
                            glVertex3f(s, s, s);
                            glVertex3f(-s, s, s);
                            glEnd();
                        }
                        crate::weapons::ProjectileKind::Mine => {
                            glColor4f(0.20, 0.22, 0.25, 1.0);
                            let r = 0.28;
                            glBegin(6);
                            glVertex3f(0.0, 0.0, 0.06);
                            for i in 0..=8 {
                                let a = (i as f32) * (std::f32::consts::TAU / 8.0);
                                glVertex3f(a.cos() * r, a.sin() * r, 0.02);
                            }
                            glEnd();
                            let blink = (proj.fuse_timer * 8.0).sin() > 0.0;
                            let col = if blink { [1.0, 0.15, 0.1] } else { [0.25, 0.05, 0.05] };
                            glColor4f(col[0], col[1], col[2], 1.0);
                            glBegin(6);
                            glVertex3f(0.0, 0.0, 0.10);
                            for i in 0..=6 {
                                let a = (i as f32) * (std::f32::consts::TAU / 6.0);
                                glVertex3f(a.cos() * 0.06, a.sin() * 0.06, 0.07);
                            }
                            glEnd();
                        }
                    }
                    glPopMatrix();
                }

                // Chinatown Wars authentic Ground Fire Patches (cFire, Fire)
                for patch in &scene.fire_manager.patches {
                    if patch.life <= 0.0 {
                        continue;
                    }
                    let alpha = patch.intensity.clamp(0.0, 1.0);
                    // Ground scorched glow
                    glColor4f(0.95, 0.35, 0.05, alpha * 0.35);
                    glBegin(6);
                    glVertex3f(patch.pos[0], patch.pos[1], patch.pos[2] + 0.04);
                    for i in 0..=8 {
                        let a = (i as f32) * (std::f32::consts::TAU / 8.0);
                        glVertex3f(
                            patch.pos[0] + a.cos() * patch.radius,
                            patch.pos[1] + a.sin() * patch.radius,
                            patch.pos[2] + 0.02,
                        );
                    }
                    glEnd();

                    // Animated rising flame tongues
                    for flame_idx in 0..6 {
                        let offset_a = (flame_idx as f32) * 1.05;
                        let dist = (flame_idx as f32 * 0.28).min(patch.radius * 0.7);
                        let fx = patch.pos[0] + offset_a.cos() * dist;
                        let fy = patch.pos[1] + offset_a.sin() * dist;
                        let flame_h = 0.6 + ((flame_idx as f32 * 3.7 + patch.life * 15.0).sin() * 0.3).abs();
                        glColor4f(1.0, 0.65, 0.12, alpha * 0.85);
                        glBegin(4);
                        glVertex3f(fx, fy, patch.pos[2] + flame_h);
                        glColor4f(0.9, 0.2, 0.05, alpha * 0.5);
                        glVertex3f(fx - 0.25, fy - 0.25, patch.pos[2]);
                        glVertex3f(fx + 0.25, fy + 0.25, patch.pos[2]);
                        glEnd();
                    }
                }

                // Burning Pedestrians flame visualization
                for ped in &scene.peds {
                    if ped.on_fire > 0.0 && ped.health > 0.0 {
                        let flame_h = 1.4;
                        glColor4f(1.0, 0.75, 0.15, 0.9);
                        glBegin(4);
                        glVertex3f(ped.p[0], ped.p[1], ped.p[2] + flame_h);
                        glColor4f(0.95, 0.25, 0.05, 0.6);
                        glVertex3f(ped.p[0] - 0.35, ped.p[1] - 0.35, ped.p[2] + 0.2);
                        glVertex3f(ped.p[0] + 0.35, ped.p[1] + 0.35, ped.p[2] + 0.2);
                        glEnd();
                    }
                }

                // Chinatown Wars authentic Traffic Lights at Road Intersections (cTrafficLight)
                for light in &scene.traffic_lights.lights {
                    let dx = light.pos[0] - walker.p[0];
                    let dy = light.pos[1] - walker.p[1];
                    if dx * dx + dy * dy > 130.0 * 130.0 {
                        continue;
                    }
                    glPushMatrix();
                    glTranslatef(light.pos[0], light.pos[1], light.pos[2]);

                    // Vertical dark steel mast / pole (4.2m height)
                    glColor4f(0.22, 0.24, 0.26, 1.0);
                    let pw = 0.08;
                    glBegin(7);
                    glVertex3f(-pw, -pw, 4.2);
                    glVertex3f(pw, -pw, 4.2);
                    glVertex3f(pw, pw, 4.2);
                    glVertex3f(-pw, pw, 4.2);
                    glVertex3f(-pw, -pw, 0.0);
                    glVertex3f(pw, -pw, 0.0);
                    glVertex3f(pw, -pw, 4.2);
                    glVertex3f(-pw, -pw, 4.2);
                    glEnd();

                    // Horizontal mast arm extending over road
                    glBegin(7);
                    glVertex3f(-pw, -pw, 4.1);
                    glVertex3f(1.8, -pw, 4.1);
                    glVertex3f(1.8, pw, 4.1);
                    glVertex3f(-pw, pw, 4.1);
                    glEnd();

                    // Signal box housing 3 lenses
                    let bx = 1.4;
                    let bz = 3.6;
                    glColor4f(0.12, 0.14, 0.15, 1.0);
                    let (hw, hl, hh) = (0.16, 0.14, 0.45);
                    glBegin(7);
                    glVertex3f(bx - hw, -hl, bz + hh);
                    glVertex3f(bx + hw, -hl, bz + hh);
                    glVertex3f(bx + hw, hl, bz + hh);
                    glVertex3f(bx - hw, hl, bz + hh);
                    glVertex3f(bx - hw, -hl, bz - hh);
                    glVertex3f(bx + hw, -hl, bz - hh);
                    glVertex3f(bx + hw, -hl, bz + hh);
                    glVertex3f(bx - hw, -hl, bz + hh);
                    glEnd();

                    let ns_state = scene.traffic_lights.signal_for_approach(light.cycle_offset, 0.0);
                    let lens_r = 0.09;
                    let red_on = ns_state == crate::vehicle::TrafficLightState::Red;
                    let amber_on = ns_state == crate::vehicle::TrafficLightState::Amber;
                    let green_on = ns_state == crate::vehicle::TrafficLightState::Green;

                    // Top: Red Lamp
                    let red_color = if red_on { [1.0, 0.15, 0.1, 1.0] } else { [0.25, 0.04, 0.04, 0.8] };
                    glColor4f(red_color[0], red_color[1], red_color[2], red_color[3]);
                    glBegin(6);
                    glVertex3f(bx, -hl - 0.02, bz + 0.28);
                    for i in 0..=8 {
                        let a = (i as f32) * (std::f32::consts::TAU / 8.0);
                        glVertex3f(bx + a.cos() * lens_r, -hl - 0.02, bz + 0.28 + a.sin() * lens_r);
                    }
                    glEnd();

                    // Middle: Amber Lamp
                    let amber_color = if amber_on { [1.0, 0.85, 0.15, 1.0] } else { [0.25, 0.18, 0.04, 0.8] };
                    glColor4f(amber_color[0], amber_color[1], amber_color[2], amber_color[3]);
                    glBegin(6);
                    glVertex3f(bx, -hl - 0.02, bz);
                    for i in 0..=8 {
                        let a = (i as f32) * (std::f32::consts::TAU / 8.0);
                        glVertex3f(bx + a.cos() * lens_r, -hl - 0.02, bz + a.sin() * lens_r);
                    }
                    glEnd();

                    // Bottom: Green Lamp
                    let green_color = if green_on { [0.15, 0.98, 0.35, 1.0] } else { [0.04, 0.25, 0.08, 0.8] };
                    glColor4f(green_color[0], green_color[1], green_color[2], green_color[3]);
                    glBegin(6);
                    glVertex3f(bx, -hl - 0.02, bz - 0.28);
                    for i in 0..=8 {
                        let a = (i as f32) * (std::f32::consts::TAU / 8.0);
                        glVertex3f(bx + a.cos() * lens_r, -hl - 0.02, bz - 0.28 + a.sin() * lens_r);
                    }
                    glEnd();

                    glPopMatrix();
                }

                // Chinatown Wars authentic Pay 'n' Spray Garages (cPayNSpray)
                for garage in &scene.pay_n_spray.garages {
                    let dx = garage.p[0] - walker.p[0];
                    let dy = garage.p[1] - walker.p[1];
                    if dx * dx + dy * dy > 160.0 * 160.0 {
                        continue;
                    }
                    glPushMatrix();
                    glTranslatef(garage.p[0], garage.p[1], garage.p[2]);
                    glRotatef(garage.facing_angle.to_degrees(), 0.0, 0.0, 1.0);

                    let gw = garage.bounds[0];
                    let gh = garage.bounds[2];

                    // Left & Right concrete pillar columns
                    glColor4f(0.35, 0.36, 0.38, 1.0);
                    for &side in &[-gw, gw] {
                        glBegin(7);
                        glVertex3f(side - 0.4, -0.4, gh);
                        glVertex3f(side + 0.4, -0.4, gh);
                        glVertex3f(side + 0.4, 0.4, gh);
                        glVertex3f(side - 0.4, 0.4, gh);
                        glVertex3f(side - 0.4, -0.4, 0.0);
                        glVertex3f(side + 0.4, -0.4, 0.0);
                        glVertex3f(side + 0.4, -0.4, gh);
                        glVertex3f(side - 0.4, -0.4, gh);
                        glEnd();
                    }

                    // Overhead lintel beam
                    glBegin(7);
                    glVertex3f(-gw - 0.4, -0.4, gh + 0.6);
                    glVertex3f(gw + 0.4, -0.4, gh + 0.6);
                    glVertex3f(gw + 0.4, 0.4, gh + 0.6);
                    glVertex3f(-gw - 0.4, 0.4, gh + 0.6);
                    glVertex3f(-gw - 0.4, -0.4, gh);
                    glVertex3f(gw + 0.4, -0.4, gh);
                    glVertex3f(gw + 0.4, -0.4, gh + 0.6);
                    glVertex3f(-gw - 0.4, -0.4, gh + 0.6);
                    glEnd();

                    // Yellow and Black Diagonal Hazard Caution Stripes on portal
                    for step in 0..8 {
                        let col = if step % 2 == 0 { [0.95, 0.85, 0.1, 1.0] } else { [0.15, 0.15, 0.15, 1.0] };
                        glColor4f(col[0], col[1], col[2], col[3]);
                        let sx = -gw + (step as f32) * (gw * 2.0 / 8.0);
                        let sw = gw * 2.0 / 8.0;
                        glBegin(7);
                        glVertex3f(sx, -0.41, gh);
                        glVertex3f(sx + sw, -0.41, gh);
                        glVertex3f(sx + sw, -0.41, gh + 0.5);
                        glVertex3f(sx, -0.41, gh + 0.5);
                        glEnd();
                    }

                    // Glowing Cyan / Hot Yellow Neon "PAY 'N' SPRAY" Sign
                    glColor4f(0.1, 0.9, 0.95, 0.95);
                    glBegin(7);
                    glVertex3f(-2.2, -0.45, gh + 0.7);
                    glVertex3f(2.2, -0.45, gh + 0.7);
                    glVertex3f(2.2, -0.45, gh + 1.25);
                    glVertex3f(-2.2, -0.45, gh + 1.25);
                    glEnd();

                    // Animated Corrugated Steel Roll-up Garage Door
                    let door_height = gh * (1.0 - garage.door_progress);
                    if garage.door_progress > 0.02 {
                        let door_top = gh;
                        let door_bottom = door_height;
                        glColor4f(0.24, 0.26, 0.28, 1.0);
                        glBegin(7);
                        glVertex3f(-gw, -0.05, door_top);
                        glVertex3f(gw, -0.05, door_top);
                        glVertex3f(gw, -0.05, door_bottom);
                        glVertex3f(-gw, -0.05, door_bottom);
                        glEnd();

                        // Horizontal Corrugation Segments
                        glColor4f(0.16, 0.18, 0.20, 1.0);
                        glBegin(1); // GL_LINES
                        let num_corrugations = 10;
                        for c in 0..num_corrugations {
                            let cz = door_bottom + (c as f32) * ((door_top - door_bottom) / num_corrugations as f32);
                            glVertex3f(-gw, -0.06, cz);
                            glVertex3f(gw, -0.06, cz);
                        }
                        glEnd();
                    }

                    glPopMatrix();
                }

                // Chinatown Wars authentic Tactical Police Roadblocks & Spike Strips (cRoadBlock, cSpikeStrip)
                for block in &scene.police.roadblocks.blocks {
                    let dx = block.p[0] - walker.p[0];
                    let dy = block.p[1] - walker.p[1];
                    if dx * dx + dy * dy > 160.0 * 160.0 {
                        continue;
                    }
                    glPushMatrix();
                    glTranslatef(block.p[0], block.p[1], block.p[2]);
                    glRotatef(block.angle.to_degrees(), 0.0, 0.0, 1.0);

                    // Wooden Striped Sawhorse Barricades (Left & Right)
                    for &bx in &[-3.2, 3.2] {
                        glColor4f(0.65, 0.45, 0.25, 1.0);
                        glBegin(7);
                        glVertex3f(bx - 0.15, -0.8, 0.9);
                        glVertex3f(bx + 0.15, -0.8, 0.9);
                        glVertex3f(bx + 0.15, 0.8, 0.9);
                        glVertex3f(bx - 0.15, 0.8, 0.9);
                        glEnd();

                        for b_seg in 0..4 {
                            let col = if b_seg % 2 == 0 { [0.95, 0.85, 0.1, 1.0] } else { [0.12, 0.12, 0.12, 1.0] };
                            glColor4f(col[0], col[1], col[2], col[3]);
                            let sy = -0.7 + (b_seg as f32) * 0.35;
                            glBegin(7);
                            glVertex3f(bx - 0.08, sy, 0.5);
                            glVertex3f(bx + 0.08, sy, 0.5);
                            glVertex3f(bx + 0.08, sy + 0.35, 0.75);
                            glVertex3f(bx - 0.08, sy + 0.35, 0.75);
                            glEnd();
                        }

                        let beacon_flash = block.light_timer.sin() > 0.0;
                        let b_col = if beacon_flash { [1.0, 0.80, 0.1, 1.0] } else { [0.35, 0.25, 0.05, 0.8] };
                        glColor4f(b_col[0], b_col[1], b_col[2], b_col[3]);
                        glBegin(6);
                        glVertex3f(bx, 0.0, 1.05);
                        for i in 0..=8 {
                            let a = (i as f32) * (std::f32::consts::TAU / 8.0);
                            glVertex3f(bx + a.cos() * 0.12, a.sin() * 0.12, 1.05);
                        }
                        glEnd();
                    }

                    // 3D Spike Strip (stinger) laid flat on asphalt across the road
                    if let Some(ref spike) = block.spike_strip {
                        let sl = spike.length;
                        glColor4f(0.18, 0.20, 0.22, 1.0);
                        glBegin(7);
                        glVertex3f(-sl * 0.5, -0.25, 0.02);
                        glVertex3f(sl * 0.5, -0.25, 0.02);
                        glVertex3f(sl * 0.5, 0.25, 0.02);
                        glVertex3f(-sl * 0.5, 0.25, 0.02);
                        glEnd();

                        glColor4f(0.85, 0.88, 0.92, 1.0);
                        let num_spikes = 16;
                        for sp in 0..num_spikes {
                            let sp_x = -sl * 0.45 + (sp as f32) * (sl * 0.9 / (num_spikes as f32 - 1.0));
                            for &sp_y in &[-0.12, 0.12] {
                                glBegin(4);
                                glVertex3f(sp_x - 0.03, sp_y, 0.02);
                                glVertex3f(sp_x + 0.03, sp_y, 0.02);
                                glVertex3f(sp_x, sp_y, 0.18);
                                glEnd();
                            }
                        }
                    }

                    glPopMatrix();
                }

                // Chinatown Wars authentic Tactical Police Helicopter & Searchlight (cPoliceHeli, cHeliSearchLight)
                if let Some(ref heli) = scene.police.heli_manager.heli {
                    glPushMatrix();
                    glTranslatef(heli.p[0], heli.p[1], heli.p[2]);
                    glRotatef(heli.angle.to_degrees(), 0.0, 0.0, 1.0);

                    // 1. Aerodynamic Fuselage (LCPD Navy Blue & White)
                    // Cabin body
                    glColor4f(0.12, 0.22, 0.45, 1.0);
                    glBegin(7);
                    glVertex3f(-1.1, -1.8, 0.9);
                    glVertex3f(1.1, -1.8, 0.9);
                    glVertex3f(1.1, 1.6, 0.7);
                    glVertex3f(-1.1, 1.6, 0.7);
                    glVertex3f(-1.1, -1.8, -0.6);
                    glVertex3f(1.1, -1.8, -0.6);
                    glVertex3f(1.1, 1.6, -0.4);
                    glVertex3f(-1.1, 1.6, -0.4);
                    glEnd();

                    // White side stripe with LCPD livery
                    glColor4f(0.95, 0.96, 0.98, 1.0);
                    glBegin(7);
                    glVertex3f(-1.12, -1.4, 0.3);
                    glVertex3f(1.12, -1.4, 0.3);
                    glVertex3f(1.12, 1.2, 0.2);
                    glVertex3f(-1.12, 1.2, 0.2);
                    glVertex3f(-1.12, -1.4, -0.1);
                    glVertex3f(1.12, -1.4, -0.1);
                    glVertex3f(1.12, 1.2, -0.2);
                    glVertex3f(-1.12, 1.2, -0.2);
                    glEnd();

                    // Front Nose & Cockpit Canopy (Dark Tinted Glass)
                    glColor4f(0.08, 0.12, 0.18, 0.85);
                    glBegin(7);
                    glVertex3f(-0.9, 1.6, 0.65);
                    glVertex3f(0.9, 1.6, 0.65);
                    glVertex3f(0.6, 2.8, -0.1);
                    glVertex3f(-0.6, 2.8, -0.1);
                    glEnd();

                    // Tail Boom extending backward
                    glColor4f(0.12, 0.22, 0.45, 1.0);
                    glBegin(7);
                    glVertex3f(-0.35, -1.8, 0.4);
                    glVertex3f(0.35, -1.8, 0.4);
                    glVertex3f(0.15, -6.8, 0.6);
                    glVertex3f(-0.15, -6.8, 0.6);
                    glVertex3f(-0.35, -1.8, -0.2);
                    glVertex3f(0.35, -1.8, -0.2);
                    glVertex3f(0.15, -6.8, 0.2);
                    glVertex3f(-0.15, -6.8, 0.2);
                    glEnd();

                    // Vertical Fin & Horizontal Stabilizers
                    glColor4f(0.95, 0.96, 0.98, 1.0);
                    glBegin(7);
                    glVertex3f(-0.06, -6.8, 0.2);
                    glVertex3f(0.06, -6.8, 0.2);
                    glVertex3f(0.06, -7.4, 1.6);
                    glVertex3f(-0.06, -7.4, 1.6);
                    glEnd();

                    // Landing Skids (Steel tubular skids)
                    glColor4f(0.35, 0.38, 0.42, 1.0);
                    for &side in &[-1.1, 1.1] {
                        glBegin(7);
                        glVertex3f(side - 0.08, -2.2, -0.85);
                        glVertex3f(side + 0.08, -2.2, -0.85);
                        glVertex3f(side + 0.08, 2.2, -0.85);
                        glVertex3f(side - 0.08, 2.2, -0.85);
                        glEnd();
                    }

                    // Spinning Main Rotor Blade Disc (Blur effect)
                    glPushMatrix();
                    glTranslatef(0.0, 0.0, 1.1);
                    glRotatef(heli.rotor_angle.to_degrees(), 0.0, 0.0, 1.0);
                    glColor4f(0.15, 0.16, 0.18, 0.70);
                    let blade_len = 5.2;
                    let blade_w = 0.22;
                    for _ in 0..4 {
                        glRotatef(90.0, 0.0, 0.0, 1.0);
                        glBegin(7);
                        glVertex3f(-blade_w, 0.2, 0.0);
                        glVertex3f(blade_w, 0.2, 0.0);
                        glVertex3f(blade_w, blade_len, 0.0);
                        glVertex3f(-blade_w, blade_len, 0.0);
                        glEnd();
                    }
                    glPopMatrix();

                    // Spinning Tail Rotor
                    glPushMatrix();
                    glTranslatef(0.12, -7.2, 1.2);
                    glRotatef(heli.tail_rotor_angle.to_degrees(), 1.0, 0.0, 0.0);
                    glColor4f(0.2, 0.2, 0.22, 0.75);
                    for _ in 0..2 {
                        glRotatef(90.0, 1.0, 0.0, 0.0);
                        glBegin(7);
                        glVertex3f(0.0, -0.08, -0.8);
                        glVertex3f(0.0, 0.08, -0.8);
                        glVertex3f(0.0, 0.08, 0.8);
                        glVertex3f(0.0, -0.08, 0.8);
                        glEnd();
                    }
                    glPopMatrix();

                    // If helicopter is in death spiral / smoking
                    if heli.is_crashing || heli.health < 200.0 {
                        glColor4f(0.15, 0.15, 0.15, 0.85);
                        glBegin(7);
                        for sm in 0..4 {
                            let sz = 0.4 + sm as f32 * 0.3;
                            glVertex3f(-sz, -2.5 - sm as f32 * 0.8, sz);
                            glVertex3f(sz, -2.5 - sm as f32 * 0.8, sz);
                            glVertex3f(sz, -2.5 - sm as f32 * 0.8, -sz);
                            glVertex3f(-sz, -2.5 - sm as f32 * 0.8, -sz);
                        }
                        glEnd();
                    }

                    glPopMatrix();

                    // 2. Volumetric 3D Tactical Searchlight Cone
                    let apex = [heli.p[0], heli.p[1], heli.p[2] - 0.7];
                    let beam_center = heli.searchlight.beam_pos;
                    let beam_radius = heli.searchlight.radius;
                    let is_alert = heli.searchlight.player_illuminated;

                    let (cr, cg, cb) = if is_alert {
                        (0.95, 0.98, 1.0)
                    } else {
                        (0.75, 0.88, 1.0)
                    };

                    glEnable(0x0be2);

                    // Volumetric Light Cone
                    glBegin(6);
                    glColor4f(cr, cg, cb, 0.40);
                    glVertex3f(apex[0], apex[1], apex[2]);
                    glColor4f(cr, cg, cb, 0.02);
                    for i in 0..=16 {
                        let a = (i as f32) * (std::f32::consts::TAU / 16.0);
                        let gx = beam_center[0] + a.cos() * beam_radius;
                        let gy = beam_center[1] + a.sin() * beam_radius;
                        glVertex3f(gx, gy, beam_center[2] + 0.04);
                    }
                    glEnd();

                    // Bright Ground Spotlight Disc on asphalt
                    glBegin(6);
                    let spot_alpha = if is_alert { 0.55 } else { 0.35 };
                    glColor4f(cr, cg, cb, spot_alpha);
                    glVertex3f(beam_center[0], beam_center[1], beam_center[2] + 0.05);
                    glColor4f(cr, cg, cb, 0.0);
                    for i in 0..=16 {
                        let a = (i as f32) * (std::f32::consts::TAU / 16.0);
                        let gx = beam_center[0] + a.cos() * beam_radius;
                        let gy = beam_center[1] + a.sin() * beam_radius;
                        glVertex3f(gx, gy, beam_center[2] + 0.05);
                    }
                    glEnd();
                }

                for car in &scene.cars {
                    shadow(car.p, [car.h[0] * 1.44, car.h[1] * 0.92], car.angle, 0.28);
                    if distance2(car.p, walker.p) > 180. * 180. && !cam.is_free() {
                        continue;
                    }
                    let cf = [-car.angle.sin(), car.angle.cos()];
                    let cside = [car.angle.cos(), car.angle.sin()];
                    if car.headlights {
                        let fwd_dist = 8.5;
                        let beam_base_w = car.h[0] * 0.85;
                        let beam_end_w = car.h[0] * 2.3;
                        let bx = car.p[0] + cf[0] * car.h[1];
                        let by = car.p[1] + cf[1] * car.h[1];
                        let bz = car.p[2] + 0.025;
                        let ex = bx + cf[0] * fwd_dist;
                        let ey = by + cf[1] * fwd_dist;
                        let ez = bz;

                        glBegin(7);
                        glColor4f(0.98, 0.96, 0.75, 0.16);
                        glVertex3f(bx - cside[0] * beam_base_w, by - cside[1] * beam_base_w, bz);
                        glVertex3f(bx + cside[0] * beam_base_w, by + cside[1] * beam_base_w, bz);
                        glColor4f(0.98, 0.96, 0.75, 0.0);
                        glVertex3f(ex + cside[0] * beam_end_w, ey + cside[1] * beam_end_w, ez);
                        glVertex3f(ex - cside[0] * beam_end_w, ey - cside[1] * beam_end_w, ez);
                        glEnd();
                    }
                    if car.braking || car.gear == -1 {
                        let (r, g, b, a) = if car.gear == -1 {
                            (0.96, 0.96, 0.98, 0.85)
                        } else {
                            (0.98, 0.08, 0.08, 0.85)
                        };
                        let lamp_radius = 0.20;
                        let draw_lamp = |lx: f32, ly: f32, lz: f32| {
                            glBegin(6);
                            glColor4f(r, g, b, a);
                            glVertex3f(lx, ly, lz);
                            glColor4f(r, g, b, 0.0);
                            glVertex3f(lx - lamp_radius, ly, lz);
                            glVertex3f(lx, ly - lamp_radius, lz);
                            glVertex3f(lx + lamp_radius, ly, lz);
                            glVertex3f(lx, ly + lamp_radius, lz);
                            glVertex3f(lx - lamp_radius, ly, lz);
                            glEnd();
                        };
                        let rz = car.p[2] + 0.42;
                        if car.is_bike() {
                            let lx = car.p[0] - cf[0] * car.h[1];
                            let ly = car.p[1] - cf[1] * car.h[1];
                            draw_lamp(lx, ly, rz);
                        } else {
                            let lx1 = car.p[0] - cf[0] * car.h[1] - cside[0] * car.h[0] * 0.78;
                            let ly1 = car.p[1] - cf[1] * car.h[1] - cside[1] * car.h[0] * 0.78;
                            let lx2 = car.p[0] - cf[0] * car.h[1] + cside[0] * car.h[0] * 0.78;
                            let ly2 = car.p[1] - cf[1] * car.h[1] + cside[1] * car.h[0] * 0.78;
                            draw_lamp(lx1, ly1, rz);
                            draw_lamp(lx2, ly2, rz);
                        }
                    }

                    // Chinatown Wars authentic Flat Tire Rim Sparks
                    if car.flat_tires.iter().any(|&f| f) && car.speed.abs() > 1.8 {
                        glColor4f(1.0, 0.72, 0.12, 0.95);
                        glBegin(1); // GL_LINES
                        for &side in &[-car.h[0] * 0.9, car.h[0] * 0.9] {
                            for &axle in &[-car.h[1] * 0.7, car.h[1] * 0.7] {
                                let wx = car.p[0] + cside[0] * side + cf[0] * axle;
                                let wy = car.p[1] + cside[1] * side + cf[1] * axle;
                                let wz = car.p[2] + 0.05;
                                let spark_tail = -car.speed * 0.15;
                                glVertex3f(wx, wy, wz);
                                glVertex3f(wx + cf[0] * spark_tail, wy + cf[1] * spark_tail, wz + 0.12);
                            }
                        }
                        glEnd();
                    }

                    // Celebratory Golden Sparkles when emerging from Pay 'n' Spray
                    if car.respray_timer > 0.0 {
                        glColor4f(0.98, 0.88, 0.35, (car.respray_timer / 3.0).min(1.0));
                        glBegin(1);
                        for sp in 0..6 {
                            let sa = (sp as f32) * (std::f32::consts::TAU / 6.0) + (car.respray_timer * 4.0);
                            let sx = car.p[0] + sa.cos() * (car.h[0] + 0.4);
                            let sy = car.p[1] + sa.sin() * (car.h[1] + 0.4);
                            let sz = car.p[2] + 0.4 + (sa * 0.5).sin() * 0.3;
                            glVertex3f(sx - 0.1, sy, sz);
                            glVertex3f(sx + 0.1, sy, sz);
                            glVertex3f(sx, sy - 0.1, sz);
                            glVertex3f(sx, sy + 0.1, sz);
                        }
                        glEnd();
                    }
                }
                for npc in &scene.peds {
                    shadow(npc.p, [0.65, 0.45], npc.heading, 0.25);
                }
            }
            if !scene.is_some_and(|s| s.driving.is_some()) {
                shadow(walker.p, [0.65, 0.45], walker.heading, 0.30);
            }
            glDepthMask(1);
            // Original PSP layered sprite, with the player's two CLUT palettes.
            glEnable(0x0de1);
            glEnable(0x0bc0);
            glEnable(0x0be2);
            glColor4f(1., 1., 1., 1.);
            if let Some(driving_idx) = scene.and_then(|s| s.driving) {
                let car = &scene.unwrap().cars[driving_idx];
                if car.is_bike() {
                    if let Some((u, l)) = car.rider_render_pos() {
                        self.sprite(ped.riding_quads(pose, u, l, car.angle));
                    }
                } else if let Some(rp) = car.rider_pos() {
                    self.sprite(ped.quads(pose, rp, car.angle));
                }
            } else {
                self.sprite(ped.quads(pose, walker.p, walker.heading));
            }
            if let Some(scene) = scene {
                for (i, car) in scene.cars.iter().enumerate() {
                    if scene.driving == Some(i) {
                        continue;
                    }
                    if let Some(app) = car.driver_appearance {
                        if let Some((u, l)) = car.rider_render_pos() {
                            let quads =
                                ped.npc_riding_quads(&car.driver_pose, u, l, car.angle, app);
                            glColor4f(1., 1., 1., 1.);
                            self.sprite(quads);
                        }
                    }
                }
                for npc in &scene.peds {
                    if distance2(npc.p, walker.p) > 160. * 160. && !cam.is_free() {
                        continue;
                    }
                    let quads = ped.npc_quads(&npc.pose, npc.p, npc.heading, npc.appearance);
                    if npc.health <= 0. {
                        glColor4f(0.60, 0.62, 0.59, 1.);
                    } else if npc.hit_flash > 0. {
                        glColor4f(1., 0.52, 0.28, 1.);
                    } else {
                        glColor4f(1., 1., 1., 1.);
                    }
                    self.sprite(quads);
                }
            }
            glDisable(0x0de1);
            glDisable(0x0bc0);
            glDisable(0x0be2);
            if let Some(scene) = scene {
                for npc in &scene.peds {
                    if npc.hit_flash > 0. {
                        impact(npc.p, npc.hit_flash);
                    }
                }

                // Chinatown Wars authentic Dynamic 3D Rain & Splash Simulation (cRain, cParticleEmitterRainSplashes)
                if scene.weather.rain_intensity > 0.05 {
                    glEnable(0x0be2);
                    glBlendFunc(0x0302, 0x0303);
                    let intensity = scene.weather.rain_intensity;
                    let num_streaks = ((intensity * 240.0) as usize).min(280);
                    let time_s = (ticks as f32) / 30.0;

                    // Slanted silver-blue rain streaks
                    glColor4f(0.80, 0.88, 0.98, 0.40 * intensity);
                    glLineWidth(1.5);
                    glBegin(1);
                    for r_idx in 0..num_streaks {
                        let seed = r_idx as f32 * 17.31 + 4.19;
                        let rx = walker.p[0] + (seed * 11.7).sin() * 26.0;
                        let ry = walker.p[1] + (seed * 23.3).cos() * 26.0;
                        let fall_cycle = (time_s * 18.0 + seed * 3.7) % 16.0;
                        let rz = walker.p[2] + 16.0 - fall_cycle;
                        let slant_x = -0.35;
                        let slant_y = -0.15;
                        let streak_len = 1.2;
                        glVertex3f(rx, ry, rz);
                        glVertex3f(rx + slant_x, ry + slant_y, rz - streak_len);
                    }
                    glEnd();
                    glLineWidth(1.0);

                    // Expanding ground impact splash rings on pavement
                    let num_splashes = ((intensity * 35.0) as usize).min(40);
                    glColor4f(0.85, 0.92, 1.0, 0.28 * intensity);
                    for s_idx in 0..num_splashes {
                        let seed = s_idx as f32 * 31.17 + 8.43;
                        let sx = walker.p[0] + (seed * 13.5).sin() * 22.0;
                        let sy = walker.p[1] + (seed * 29.1).cos() * 22.0;
                        let splash_cycle = (time_s * 7.0 + seed * 2.3) % 1.0;
                        let splash_r = 0.12 + splash_cycle * 0.35;
                        let sz = walker.p[2] + 0.04;
                        glBegin(2);
                        for pt in 0..8 {
                            let a = (pt as f32) * (std::f32::consts::TAU / 8.0);
                            glVertex3f(sx + a.cos() * splash_r, sy + a.sin() * splash_r, sz);
                        }
                        glEnd();
                    }
                    glDisable(0x0be2);
                }
            }
            if debug {
                glDisable(0x0b71);
                glBegin(1);
                glColor3f(1., 1., 0.1);
                let centre = if cam.is_free() { cam.eye() } else { walker.p };
                if let Some((cx, cy)) = Collision::cell_xy(centre[0], centre[1]) {
                    for x in cx.saturating_sub(1)..=(cx + 1).min(139) {
                        for y in cy.saturating_sub(1)..=(cy + 1).min(99) {
                            for b in &collision.cells[x * 100 + y].boxes {
                                let (s, c) = b.angle.sin_cos();
                                let mut corners = [[0.; 3]; 4];
                                for (k, (dx, dy)) in [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)]
                                    .into_iter()
                                    .enumerate()
                                {
                                    corners[k] = [
                                        b.c[0] + dx * b.h[0] * c - dy * b.h[1] * s,
                                        b.c[1] + dx * b.h[0] * s + dy * b.h[1] * c,
                                        b.c[2] + b.h[2],
                                    ];
                                }
                                for k in 0..4 {
                                    let a = corners[k];
                                    let b = corners[(k + 1) % 4];
                                    glVertex3f(a[0], a[1], a[2]);
                                    glVertex3f(b[0], b[1], b[2]);
                                }
                            }
                        }
                    }
                }
                glEnd();
            }
            if let Some(grade) = &mut self.grade {
                grade.draw(
                    (rect.right - rect.left).max(1),
                    (rect.bottom - rect.top).max(1),
                );
            }
            glDisable(0x0b71);
            glMatrixMode(0x1701);
            glLoadIdentity();
            glOrtho(0., W as f64, H as f64, 0., -1., 1.);
            glMatrixMode(0x1700);
            glLoadIdentity();
            let ink = [0.018, 0.018, 0.018];
            let gold = [0.97, 0.79, 0.36];
            let bands = map_ui.info_bands;
            if map_ui.open {
                panel(0., 0., W as f32, H as f32, ink);
            } else if scene.is_some_and(|s| s.hit_flash > 0.) {
                glEnable(0x0be2);
                glColor4f(
                    0.65,
                    0.025,
                    0.015,
                    scene.map(|s| s.hit_flash.min(0.45)).unwrap_or(0.),
                );
                glBegin(7);
                for [x, y] in [
                    [0., 0.],
                    [W as f32, 0.],
                    [W as f32, H as f32],
                    [0., H as f32],
                ] {
                    glVertex2f(x, y);
                }
                glEnd();
                glDisable(0x0be2);
            }
            if bands && !map_ui.open {
                panel(0., 0., W as f32, 58., [0.; 3]);
                panel(
                    0.,
                    crate::hud::FOOTER_Y,
                    W as f32,
                    H as f32 - crate::hud::FOOTER_Y,
                    [0.; 3],
                );
            }
            self.psp_text(20., 10., "by Benji Paname", 2.2, gold);
            if bands {
                label(20., 40., "VERSION DE DEMONSTRATION", 1., [0.7; 3]);
                hud_button(
                    crate::hud::PROFILE_BUTTON,
                    match cam.profile {
                        crate::camera::CameraProfile::Psp => "CAMERA PSP",
                        crate::camera::CameraProfile::Mobile => "CAMERA MOBILE",
                        crate::camera::CameraProfile::Ds => "CAMERA DS",
                    },
                    "F3",
                    "RS CLIC",
                    false,
                );
                hud_button(
                    crate::hud::FREE_BUTTON,
                    if cam.is_free() {
                        "RETOUR JOUEUR"
                    } else {
                        "CAMERA LIBRE"
                    },
                    "F2",
                    "CROIX HAUT",
                    false,
                );
                hud_button(
                    crate::hud::LANGUAGE_BUTTON,
                    if crate::locale::english() {
                        "ENGLISH"
                    } else {
                        "FRANCAIS"
                    },
                    "F4",
                    "LB+RS",
                    false,
                );
                hud_button(
                    crate::hud::FULLSCREEN_BUTTON,
                    if self.fullscreen {
                        "FENETRE"
                    } else {
                        "PLEIN ECRAN"
                    },
                    "F11",
                    "LB+BACK",
                    self.fullscreen,
                );
                label(938., 15., "BANDES D INFORMATIONS", 1., [0.8; 3]);
                pair(938., 29., "F5", "RB+BACK");
            }
            if !map_ui.open {
                let top = if bands { 60. } else { 0. };
                let minutes = (8 * 60 + ticks / 30) % 1440;
                self.psp_text(
                    W as f32 - 180.,
                    6. + top,
                    &format!("MAR {:02}:{:02}", minutes / 60, minutes % 60),
                    2.3,
                    [0.96; 3],
                );
                if let Some(scene) = scene {
                    let cash_str = format!("${}", scene.cash);
                    self.psp_text(
                        W as f32 - 180.,
                        30. + top,
                        &cash_str,
                        2.2,
                        [0.28, 0.88, 0.38],
                    );

                    // Chinatown Wars authentic Wanted Stars & Cop Takedown Badges HUD
                    let num_stars = scene.police.stars;
                    if num_stars > 0 {
                        let (_stars, takedowns_done, takedowns_req) = scene.police.takedown_progress();
                        let cooling = scene.police.is_cooling_down();
                        let flash = scene.police.flash_timer.sin() > 0.0;
                        let star_y = 54.0 + top;
                        let star_spacing = 24.0;
                        let start_x = W as f32 - 180.0;

                        // 1. Draw Golden 5-pointed Wanted Stars
                        for s_idx in 0..num_stars {
                            let sx = start_x + (s_idx as f32) * star_spacing;
                            let star_col = if cooling && flash {
                                [0.45, 0.45, 0.45, 0.85]
                            } else {
                                [1.0, 0.82, 0.08, 1.0]
                            };
                            glColor4f(star_col[0], star_col[1], star_col[2], star_col[3]);
                            glBegin(6);
                            glVertex2f(sx, star_y);
                            for pt in 0..=10 {
                                let a = (pt as f32) * (std::f32::consts::TAU / 10.0) - std::f32::consts::FRAC_PI_2;
                                let r = if pt % 2 == 0 { 8.5 } else { 3.8 };
                                glVertex2f(sx + a.cos() * r, star_y + a.sin() * r);
                            }
                            glEnd();
                        }

                        // 2. Draw Chinatown Wars Takedown Cop Badges below stars
                        let badge_y = star_y + 18.0;
                        for b_idx in 0..takedowns_req {
                            let bx = start_x + (b_idx as f32) * 26.0;
                            let is_crossed = b_idx < takedowns_done;
                            let is_pulsing = is_crossed && (b_idx + 1 == takedowns_done) && scene.police.recent_takedown_flash > 0.0;

                            // Badge background shield
                            glColor4f(0.12, 0.18, 0.28, 0.90);
                            glBegin(7);
                            glVertex2f(bx - 10.0, badge_y - 7.0);
                            glVertex2f(bx + 10.0, badge_y - 7.0);
                            glVertex2f(bx + 10.0, badge_y + 4.0);
                            glVertex2f(bx - 10.0, badge_y + 4.0);
                            glEnd();
                            glBegin(4);
                            glVertex2f(bx - 10.0, badge_y + 4.0);
                            glVertex2f(bx + 10.0, badge_y + 4.0);
                            glVertex2f(bx, badge_y + 11.0);
                            glEnd();

                            // Police cruiser silhouette icon
                            glColor4f(0.85, 0.90, 0.95, 0.85);
                            glBegin(7);
                            glVertex2f(bx - 6.0, badge_y - 3.0);
                            glVertex2f(bx + 6.0, badge_y - 3.0);
                            glVertex2f(bx + 6.0, badge_y + 3.0);
                            glVertex2f(bx - 6.0, badge_y + 3.0);
                            glEnd();

                            // Red / blue emergency lightbar atop badge
                            let bar_color = if b_idx % 2 == 0 { [0.95, 0.15, 0.15, 1.0] } else { [0.2, 0.45, 0.98, 1.0] };
                            glColor4f(bar_color[0], bar_color[1], bar_color[2], bar_color[3]);
                            glBegin(7);
                            glVertex2f(bx - 3.5, badge_y - 5.5);
                            glVertex2f(bx + 3.5, badge_y - 5.5);
                            glVertex2f(bx + 3.5, badge_y - 3.5);
                            glVertex2f(bx - 3.5, badge_y - 3.5);
                            glEnd();

                            // Iconic Chinatown Wars Thick Red/Black "X" Stamp over disabled cop car!
                            if is_crossed {
                                let (xr, xg, xb) = if is_pulsing {
                                    (1.0, 0.95, 0.85) // white-hot flash on recent takedown
                                } else {
                                    (0.92, 0.08, 0.08) // bold crimson red cross
                                };
                                glColor4f(xr, xg, xb, 1.0);
                                glLineWidth(3.0);
                                glBegin(1); // GL_LINES
                                glVertex2f(bx - 11.0, badge_y - 8.0);
                                glVertex2f(bx + 11.0, badge_y + 12.0);
                                glVertex2f(bx + 11.0, badge_y - 8.0);
                                glVertex2f(bx - 11.0, badge_y + 12.0);
                                glEnd();
                                glLineWidth(1.0);
                            }
                        }
                    }
                }
                if let Some((scene, i)) = scene.and_then(|s| s.driving.map(|i| (s, i))) {
                    let vname = scene.cars[i].name;
                    let x = (W as f32 - 30. - vname.len() as f32 * 22.).min(W as f32 - 190.);
                    self.psp_text(
                        x,
                        if bands { 550. } else { 675. },
                        vname,
                        2.3,
                        [0.78, 0.86, 0.77],
                    );
                }

                if let Some(scene) = scene {
                    let w_opt = scene.weapons.as_ref().and_then(|wm| wm.current());
                    let (wname, ammo_str, is_melee) = if let Some(w) = w_opt {
                        let name = match w.type_id {
                            0 => "POINGS",
                            1 => "BATTE DE BASEBALL",
                            2 => "PISTOLET 9MM",
                            3 => "DOUBLE PISTOLET",
                            4 => "FUSIL A POMPE",
                            5 => "DOUBLE CANON",
                            6 => "SMG",
                            7 => "FUSIL D ASSAUT",
                            8 => "CARABINE M4",
                            9 => "FUSIL SNIPER",
                            10 => "LANCE-ROQUETTES",
                            11 => "LANCE-FLAMMES",
                            12 => "MINIGUN",
                            13 => "GRENADES",
                            14 => "COCKTAIL MOLOTOV",
                            15 => "MINES",
                            16 => "TASER",
                            17 => "KATANA",
                            18 => "TRONCONNEUSE",
                            19 => "MATRAQUE",
                            20 => "GRENADE FLASH",
                            21 => "MOLOTOV",
                            _ => "ARME",
                        };
                        let ammo = if w.is_melee() {
                            "--".to_string()
                        } else if w.state == crate::weapons::WeaponState::Reloading {
                            "RECHARGE...".to_string()
                        } else if w.ammo < 0 {
                            format!("{}/INF", w.clip)
                        } else {
                            format!("{}/{}", w.clip, w.ammo)
                        };
                        (name, ammo, w.is_melee())
                    } else {
                        ("POINGS", "--".to_string(), true)
                    };

                    let wx = W as f32 - 195.;
                    let wy = 40. + top;
                    rounded([wx, wy, 180., 44.], 3., [0.06, 0.08, 0.09]);
                    self.psp_text(wx + 8., wy + 4., wname, 1.8, [0.95, 0.90, 0.80]);
                    self.psp_text(
                        wx + 8.,
                        wy + 22.,
                        &ammo_str,
                        2.1,
                        if is_melee { [0.70, 0.75, 0.72] } else { [0.98, 0.85, 0.20] },
                    );
                    label(wx + 105., wy + 24., "TAB / MOLETTE", 0.9, [0.55, 0.60, 0.58]);

                    if scene.police.stars > 0 {
                        let sx = W as f32 - 195.;
                        let sy = wy + 48.;
                        let num_stars = (scene.police.stars as usize).min(6);
                        rounded([sx, sy, 180., 36.], 3., [0.10, 0.04, 0.05]);

                        let flash_on = (ticks / 6) % 2 == 0;
                        let star_color = [1.0, 0.85, 0.12, if flash_on { 1.0 } else { 0.45 }];

                        for s_idx in 0..num_stars {
                            let star_cx = sx + 14. + (s_idx as f32) * 26.;
                            let star_cy = sy + 12.;
                            draw_star(star_cx, star_cy, 7.5, star_color);
                        }

                        let req = (scene.police.takedowns_required as usize).min(6);
                        let done = scene.police.takedowns_done as usize;
                        for k in 0..req {
                            let bx = sx + 8. + (k as f32) * 22.;
                            let by = sy + 21.;
                            draw_takedown_badge(bx, by, k < done, ticks);
                        }
                    }

                    // Chinatown Wars In-Car Radio Station HUD Banner
                    if scene.radio.hud_banner_timer > 0.0 {
                        let rx = W as f32 / 2. - 130.;
                        let ry = 42. + top;
                        rounded([rx, ry, 260., 38.], 3., [0.08, 0.12, 0.18]);
                        self.psp_text(rx + 10., ry + 4., scene.radio.display_name(), 2.0, [0.98, 0.88, 0.22]);
                        label(rx + 10., ry + 24., "RADIO: N / B (DPAD HAUT / BAS)", 0.85, [0.70, 0.78, 0.82]);
                    }

                    // Dynamic Weather Lightning Flash effect overlay
                    if scene.weather.lightning_intensity > 0.0 {
                        glColor4f(1.0, 1.0, 1.0, scene.weather.lightning_intensity * 0.45);
                        glBegin(7);
                        glVertex2f(0.0, 0.0);
                        glVertex2f(W as f32, 0.0);
                        glVertex2f(W as f32, H as f32);
                        glVertex2f(0.0, H as f32);
                        glEnd();
                    }

                    // Chinatown Wars authentic Sniper Rifle Optical Scope Overlay (cSniperCam)
                    if scene.sniper_scope.active {
                        let cx = W as f32 / 2.0;
                        let cy = H as f32 / 2.0;
                        let scope_r = (H as f32 * 0.44).min(330.0);

                        // 1. Dark letterbox & outer circular aperture vignette mask
                        glColor4f(0.04, 0.05, 0.06, 0.96);
                        // Left box
                        glBegin(7);
                        glVertex2f(0.0, 0.0);
                        glVertex2f(cx - scope_r, 0.0);
                        glVertex2f(cx - scope_r, H as f32);
                        glVertex2f(0.0, H as f32);
                        glEnd();
                        // Right box
                        glBegin(7);
                        glVertex2f(cx + scope_r, 0.0);
                        glVertex2f(W as f32, 0.0);
                        glVertex2f(W as f32, H as f32);
                        glVertex2f(cx + scope_r, H as f32);
                        glEnd();
                        // Top box
                        glBegin(7);
                        glVertex2f(cx - scope_r, 0.0);
                        glVertex2f(cx + scope_r, 0.0);
                        glVertex2f(cx + scope_r, cy - scope_r);
                        glVertex2f(cx - scope_r, cy - scope_r);
                        glEnd();
                        // Bottom box
                        glBegin(7);
                        glVertex2f(cx - scope_r, cy + scope_r);
                        glVertex2f(cx + scope_r, cy + scope_r);
                        glVertex2f(cx + scope_r, H as f32);
                        glVertex2f(cx - scope_r, H as f32);
                        glEnd();

                        // 2. Optical Lens Circular Border
                        glColor4f(0.12, 0.16, 0.20, 1.0);
                        glBegin(2);
                        for i in 0..=48 {
                            let a = (i as f32) * (std::f32::consts::TAU / 48.0);
                            glVertex2f(cx + a.cos() * scope_r, cy + a.sin() * scope_r);
                        }
                        glEnd();

                        // 3. Precision Crosshairs with Breath Sway Offset
                        let sx = cx + scene.sniper_scope.sway_offset[0] * 320.0;
                        let sy = cy + scene.sniper_scope.sway_offset[1] * 320.0;

                        glColor4f(0.08, 0.09, 0.10, 0.95);
                        // Horizontal crosshair
                        glBegin(1);
                        glVertex2f(sx - scope_r * 0.92, sy);
                        glVertex2f(sx - 16.0, sy);
                        glVertex2f(sx + 16.0, sy);
                        glVertex2f(sx + scope_r * 0.92, sy);
                        glEnd();
                        // Vertical crosshair
                        glBegin(1);
                        glVertex2f(sx, sy - scope_r * 0.92);
                        glVertex2f(sx, sy - 16.0);
                        glVertex2f(sx, sy + 16.0);
                        glVertex2f(sx, sy + scope_r * 0.92);
                        glEnd();

                        // Center red precision dot
                        glColor4f(0.95, 0.15, 0.15, 0.9);
                        glBegin(6);
                        glVertex2f(sx, sy);
                        for i in 0..=8 {
                            let a = (i as f32) * (std::f32::consts::TAU / 8.0);
                            glVertex2f(sx + a.cos() * 3.0, sy + a.sin() * 3.0);
                        }
                        glEnd();

                        // Mil-dots on crosshair
                        glColor4f(0.15, 0.16, 0.18, 0.85);
                        for dist in [40.0, 80.0, 120.0, 160.0] {
                            glBegin(1);
                            glVertex2f(sx - dist, sy - 6.0);
                            glVertex2f(sx - dist, sy + 6.0);
                            glVertex2f(sx + dist, sy - 6.0);
                            glVertex2f(sx + dist, sy + 6.0);
                            glVertex2f(sx - 6.0, sy - dist);
                            glVertex2f(sx + 6.0, sy - dist);
                            glVertex2f(sx - 6.0, sy + dist);
                            glVertex2f(sx + 6.0, sy + dist);
                            glEnd();
                        }

                        // 4. Digital Readouts & Telemetry
                        let zoom_str = format!("ZOOM : {:.1}X", scene.sniper_scope.zoom);
                        self.psp_text(cx - 130., cy + scope_r * 0.72, &zoom_str, 1.8, [0.35, 0.85, 0.95]);

                        let range = scene.sniper_scope.reticle_range;
                        let range_str = if range > 0.0 {
                            format!("DIST : {:.1} M", range)
                        } else {
                            "DIST : -- M".to_string()
                        };
                        self.psp_text(cx + 40., cy + scope_r * 0.72, &range_str, 1.8, [0.95, 0.85, 0.35]);

                        // Breath stamina bar
                        let breath = scene.sniper_scope.breath_meter;
                        let bar_w = 140.0;
                        let bar_x = cx - bar_w / 2.0;
                        let bar_y = cy - scope_r * 0.78;
                        rounded([bar_x - 4., bar_y - 3., bar_w + 8., 16.], 2., [0.08, 0.10, 0.12]);
                        let breath_color = if scene.sniper_scope.held_breath { [0.3, 0.75, 0.95] } else { [0.4, 0.85, 0.4] };
                        rounded([bar_x, bar_y, bar_w * breath, 10.], 1., breath_color);
                        let breath_label = if scene.sniper_scope.held_breath { "APNEE ACTIVE" } else { "APNEE (MAJ / LSTICK)" };
                        self.psp_text(cx - 65., bar_y - 18., breath_label, 1.4, [0.85, 0.90, 0.92]);
                    }

                    // Contextual World Interaction Prompt
                    if let Some(prompt_text) = &scene.prompt {
                        let px = W as f32 / 2. - (prompt_text.len() as f32 * 6.5);
                        let py = if bands { 540. } else { 660. };
                        rounded([px - 14., py - 6., prompt_text.len() as f32 * 13. + 28., 34.], 3., [0.08, 0.10, 0.12]);
                        self.psp_text(px, py, prompt_text, 1.9, [1.0, 0.92, 0.35]);
                    }

                    // Authentic Chinatown Wars Announcement Banner
                    if let Some((banner_text, _)) = &scene.banner {
                        let bx = W as f32 / 2. - (banner_text.len() as f32 * 7.5);
                        let by = 90. + top;
                        rounded([bx - 16., by - 6., banner_text.len() as f32 * 15. + 32., 38.], 3., [0.12, 0.08, 0.06]);
                        self.psp_text(bx, by, banner_text, 2.1, [1.0, 0.85, 0.20]);
                    }

                    // Active Mission Objective Banner
                    if let crate::missions::MissionState::InProgress { objective, .. } = &scene.missions.state {
                        let desc = match objective {
                            crate::missions::MissionObjective::GoToLocation { description, .. } => *description,
                            crate::missions::MissionObjective::KillTargets { description, .. } => *description,
                            crate::missions::MissionObjective::LoseWantedLevel { .. } => "Semez la police !",
                            crate::missions::MissionObjective::SurviveFrames { .. } => "Survivez aux assaillants !",
                            crate::missions::MissionObjective::CompleteMinigame { .. } => "Terminez le mini-jeu !",
                            crate::missions::MissionObjective::TailVehicle { description, .. } => *description,
                        };
                        let ox = 24.;
                        let oy = 60. + top;
                        rounded([ox - 6., oy - 4., desc.len() as f32 * 13. + 20., 32.], 3., [0.08, 0.10, 0.14]);
                        self.psp_text(ox, oy, desc, 1.8, [0.98, 0.92, 0.72]);
                    }

                    // Active Street Race HUD Overlay
                    if let Some(race) = &scene.active_race {
                        let rx = 24.;
                        let ry = 60. + top;
                        let text = format!("COURSE : {}  |  {}  |  POS : {}  |  {}", race.circuit.name, race.lap_display(), race.rank_display(), race.timer_display());
                        rounded([rx - 6., ry - 4., text.len() as f32 * 12. + 24., 32.], 3., [0.12, 0.08, 0.04]);
                        self.psp_text(rx, ry, &text, 1.8, [1.0, 0.85, 0.20]);
                    }

                    // Chinatown Wars authentic Interactive Minigames Overlay (cHotWiringApp, cSinkingCarApp, etc.)
                    if let Some(minigame) = &scene.minigame {
                        let cx = W as f32 / 2.0;
                        let cy = H as f32 / 2.0;

                        // Darkened background backdrop
                        glEnable(0x0be2);
                        glBlendFunc(0x0302, 0x0303);
                        glColor4f(0.02, 0.03, 0.05, 0.75);
                        glBegin(7);
                        glVertex2f(0.0, 0.0);
                        glVertex2f(W as f32, 0.0);
                        glVertex2f(W as f32, H as f32);
                        glVertex2f(0.0, H as f32);
                        glEnd();

                        // Bezel Chassis (PSP dual-screen / PDA hardware style)
                        rounded([cx - 250.0, cy - 180.0, 500.0, 360.0], 8.0, [0.12, 0.14, 0.18]);
                        rounded([cx - 240.0, cy - 170.0, 480.0, 40.0], 4.0, [0.18, 0.20, 0.26]);

                        match &minigame.kind {
                            crate::minigames::MinigameKind::SinkingCar { hits_done, hits_required, water_level } => {
                                self.psp_text(cx - 160.0, cy - 162.0, "PROLOGUE : FUITE DU VEHICULE IMMERGE", 1.8, [1.0, 0.88, 0.20]);
                                // Windshield glass viewport
                                rounded([cx - 210.0, cy - 110.0, 420.0, 220.0], 6.0, [0.08, 0.16, 0.24]);
                                // Rising water tide
                                let water_h = (*water_level * 220.0).clamp(0.0, 220.0);
                                glColor4f(0.05, 0.35, 0.55, 0.65);
                                glBegin(7);
                                glVertex2f(cx - 210.0, cy + 110.0 - water_h);
                                glVertex2f(cx + 210.0, cy + 110.0 - water_h);
                                glVertex2f(cx + 210.0, cy + 110.0);
                                glVertex2f(cx - 210.0, cy + 110.0);
                                glEnd();

                                // Radiating glass cracks
                                glLineWidth(2.0);
                                glColor4f(0.92, 0.95, 1.0, 0.90);
                                glBegin(1);
                                for h in 0..*hits_done {
                                    let a1 = (h as f32) * 1.57;
                                    let a2 = a1 + 0.45;
                                    let r = (h as f32 + 1.0) * 35.0;
                                    glVertex2f(cx, cy);
                                    glVertex2f(cx + a1.cos() * r, cy + a1.sin() * r);
                                    glVertex2f(cx + a1.cos() * (r * 0.6), cy + a1.sin() * (r * 0.6));
                                    glVertex2f(cx + a2.cos() * r, cy + a2.sin() * r);
                                }
                                glEnd();
                                glLineWidth(1.0);

                                let prompt_str = format!("[ ESPACE / TOUCHE E ] FRAPPER LE PARE-BRISE ({}/{})", hits_done, hits_required);
                                self.psp_text(cx - 190.0, cy + 130.0, &prompt_str, 1.8, [1.0, 0.95, 0.85]);
                            }
                            crate::minigames::MinigameKind::Hotwire { method, progress } => {
                                let method_name = match method {
                                    crate::minigames::HotwireMethod::ScrewdriverTurn => "CONTACTEUR DE CLEF",
                                    crate::minigames::HotwireMethod::WiresTwist => "FILS D'ALLUMAGE",
                                    crate::minigames::HotwireMethod::TransponderKeypad => "PIRATAGE DU CALCULATEUR",
                                };
                                let header = format!("DEMARRAGE DU VEHICULE : {}", method_name);
                                self.psp_text(cx - 180.0, cy - 162.0, &header, 1.8, [1.0, 0.88, 0.20]);
                                rounded([cx - 200.0, cy - 110.0, 400.0, 200.0], 6.0, [0.07, 0.08, 0.10]);

                                // 3 Colored ignition wires
                                let wire_colors = [[0.95, 0.15, 0.15], [0.15, 0.45, 0.95], [0.95, 0.85, 0.15]];
                                for (w_idx, col) in wire_colors.iter().enumerate() {
                                    let wy = cy - 70.0 + (w_idx as f32) * 45.0;
                                    glColor3f(col[0], col[1], col[2]);
                                    glLineWidth(4.0);
                                    glBegin(1);
                                    glVertex2f(cx - 150.0, wy);
                                    glVertex2f(cx - 30.0 + (*progress * 60.0), wy);
                                    glVertex2f(cx + 150.0, wy);
                                    glVertex2f(cx + 30.0 - (*progress * 60.0), wy);
                                    glEnd();
                                    glLineWidth(1.0);
                                }

                                // Progress bar
                                rounded([cx - 180.0, cy + 115.0, 360.0, 22.0], 3.0, [0.04, 0.05, 0.06]);
                                rounded([cx - 178.0, cy + 117.0, 356.0 * progress.clamp(0.0, 1.0), 18.0], 2.0, [0.20, 0.85, 0.35]);
                                self.psp_text(cx - 170.0, cy + 145.0, "[ MAINTENIR E ] CONNECTER LES FILS ET DEMARRER", 1.7, [0.90, 0.92, 0.95]);
                            }
                            crate::minigames::MinigameKind::MolotovFilling { bottles_filled, current_fill, is_pumping } => {
                                self.psp_text(cx - 180.0, cy - 162.0, "STATION-SERVICE : COCKTAILS MOLOTOV", 1.8, [1.0, 0.88, 0.20]);
                                rounded([cx - 210.0, cy - 110.0, 420.0, 210.0], 6.0, [0.08, 0.09, 0.11]);

                                // 3 Beer bottles in crate
                                for b in 0..3 {
                                    let bx = cx - 120.0 + (b as f32) * 120.0;
                                    let is_done = b < *bottles_filled;
                                    let is_cur = b == *bottles_filled;
                                    rounded([bx - 22.0, cy - 60.0, 44.0, 110.0], 4.0, [0.18, 0.14, 0.08]);
                                    let fill_h = if is_done { 95.0 } else if is_cur { (*current_fill * 95.0).clamp(0.0, 95.0) } else { 0.0 };
                                    if fill_h > 0.0 {
                                        rounded([bx - 18.0, cy + 45.0 - fill_h, 36.0, fill_h], 2.0, [0.95, 0.65, 0.12]);
                                    }
                                    if is_done || is_cur {
                                        glColor3f(0.95, 0.92, 0.88);
                                        glBegin(7);
                                        glVertex2f(bx - 6.0, cy - 75.0);
                                        glVertex2f(bx + 6.0, cy - 75.0);
                                        glVertex2f(bx + 6.0, cy - 55.0);
                                        glVertex2f(bx - 6.0, cy - 55.0);
                                        glEnd();
                                    }
                                }

                                let prompt_str = if *is_pumping {
                                    "[ MAINTENIR E ] REMPLISSAGE EN COURS..."
                                } else {
                                    "[ E ] ACTIONNER LE PISTOLET DE POMPE"
                                };
                                self.psp_text(cx - 170.0, cy + 125.0, prompt_str, 1.8, [1.0, 0.90, 0.20]);
                            }
                            crate::minigames::MinigameKind::SafeCracking { combination: _, current_dial, unlocked_tumblers } => {
                                self.psp_text(cx - 160.0, cy - 162.0, "COFFRE-FORT : CROCHETAGE DE COMBINAISON", 1.8, [1.0, 0.88, 0.20]);
                                rounded([cx - 200.0, cy - 110.0, 400.0, 210.0], 6.0, [0.06, 0.07, 0.09]);

                                for p in 0..3 {
                                    let px = cx - 70.0 + (p as f32) * 70.0;
                                    let col = if p < *unlocked_tumblers { [0.20, 0.85, 0.35] } else { [0.85, 0.20, 0.15] };
                                    rounded([px - 18.0, cy - 90.0, 36.0, 20.0], 3.0, col);
                                }

                                let dial_r = 65.0;
                                glColor3f(0.72, 0.62, 0.32);
                                glBegin(6);
                                glVertex2f(cx, cy + 10.0);
                                for i in 0..=24 {
                                    let a = (i as f32) * (std::f32::consts::TAU / 24.0);
                                    glVertex2f(cx + a.cos() * dial_r, cy + 10.0 + a.sin() * dial_r);
                                }
                                glEnd();

                                glColor3f(0.25, 0.22, 0.16);
                                glBegin(6);
                                glVertex2f(cx, cy + 10.0);
                                for i in 0..=16 {
                                    let a = (i as f32) * (std::f32::consts::TAU / 16.0);
                                    glVertex2f(cx + a.cos() * 24.0, cy + 10.0 + a.sin() * 24.0);
                                }
                                glEnd();

                                let dial_text = format!("{:02}", current_dial);
                                self.psp_text(cx - 14.0, cy + 2.0, &dial_text, 2.2, [1.0, 0.95, 0.85]);
                                self.psp_text(cx - 180.0, cy + 125.0, "[ A / D / GAUCHE / DROITE ] TOURNER  |  [ E ] CRAN", 1.7, [0.90, 0.92, 0.95]);
                            }
                            crate::minigames::MinigameKind::ScratchCard { card_name, scratched_cells, symbols: _, prize_cash } => {
                                let hdr = format!("LOTERIE : {} (JACKPOT ${})", card_name, prize_cash);
                                self.psp_text(cx - 180.0, cy - 162.0, &hdr, 1.8, [1.0, 0.88, 0.20]);
                                rounded([cx - 210.0, cy - 110.0, 420.0, 210.0], 6.0, [0.16, 0.12, 0.18]);

                                for c in 0..6 {
                                    let gx = cx - 130.0 + (c % 3) as f32 * 130.0;
                                    let gy = cy - 60.0 + (c / 3) as f32 * 80.0;
                                    let scratched = scratched_cells[c];
                                    let bg = if scratched { [0.22, 0.35, 0.25] } else { [0.72, 0.74, 0.78] };
                                    rounded([gx - 45.0, gy - 25.0, 90.0, 50.0], 4.0, bg);
                                    if !scratched {
                                        self.psp_text(gx - 8.0, gy - 10.0, "?", 2.2, [0.35, 0.38, 0.42]);
                                    } else {
                                        self.psp_text(gx - 18.0, gy - 8.0, "$$$", 2.0, [1.0, 0.92, 0.30]);
                                    }
                                }
                                self.psp_text(cx - 160.0, cy + 125.0, "[ 1..=6 ] GRATTER LA CASE  |  [ ENTREE ] ENCAISSER", 1.7, [0.95, 0.95, 0.95]);
                            }
                            crate::minigames::MinigameKind::Tattooing { motif, stencil_progress, needle_accuracy: _, pain_level, ink_level, reloads_done: _ } => {
                                let motif_str = match motif {
                                    crate::minigames::TattooMotif::DragonOfBroker => "DRAGON DE BROKER",
                                    crate::minigames::TattooMotif::TigerOfBohan => "TIGRE DE BOHAN",
                                    crate::minigames::TattooMotif::KoiOfDukes => "CARPE KOI DE DUKES",
                                    crate::minigames::TattooMotif::ViperOfAlgonquin => "VIPERE D'ALGONQUIN",
                                };
                                let hdr = format!("TATOUAGE TRIADE : {}", motif_str);
                                self.psp_text(cx - 170.0, cy - 162.0, &hdr, 1.8, [1.0, 0.88, 0.20]);
                                rounded([cx - 210.0, cy - 110.0, 420.0, 210.0], 6.0, [0.28, 0.24, 0.20]);

                                rounded([cx - 180.0, cy + 65.0, 360.0, 16.0], 2.0, [0.10, 0.08, 0.06]);
                                rounded([cx - 178.0, cy + 67.0, 356.0 * stencil_progress.clamp(0.0, 1.0), 12.0], 2.0, [0.20, 0.85, 0.35]);

                                self.psp_text(cx - 180.0, cy + 90.0, "DOULEUR DU CLIENT :", 1.6, [0.90, 0.80, 0.70]);
                                rounded([cx + 10.0, cy + 88.0, 170.0, 14.0], 2.0, [0.10, 0.06, 0.06]);
                                rounded([cx + 12.0, cy + 90.0, 166.0 * pain_level.clamp(0.0, 1.0), 10.0], 2.0, [0.95, 0.25, 0.20]);

                                self.psp_text(cx - 180.0, cy + 110.0, "NIVEAU D'ENCRE :", 1.6, [0.70, 0.80, 0.90]);
                                rounded([cx + 10.0, cy + 108.0, 170.0, 14.0], 2.0, [0.06, 0.08, 0.12]);
                                rounded([cx + 12.0, cy + 110.0, 166.0 * ink_level.clamp(0.0, 1.0), 10.0], 2.0, [0.15, 0.55, 0.95]);

                                self.psp_text(cx - 180.0, cy + 135.0, "[ MAINTENIR E ] TATOUER  |  [ R ] RECHARGER L'ENCRE", 1.7, [1.0, 0.95, 0.85]);
                            }
                            crate::minigames::MinigameKind::CarBombFitting { wires_connected, armed } => {
                                self.psp_text(cx - 180.0, cy - 162.0, "EXPLOSIFS : AMORCAGE DE BOMBE DE VOITURE", 1.8, [1.0, 0.88, 0.20]);
                                rounded([cx - 200.0, cy - 110.0, 400.0, 210.0], 6.0, [0.08, 0.08, 0.08]);

                                rounded([cx - 130.0, cy - 80.0, 260.0, 90.0], 4.0, [0.55, 0.48, 0.35]);
                                self.psp_text(cx - 70.0, cy - 50.0, "C-4 EXPLOSIF", 2.2, [0.15, 0.12, 0.10]);

                                rounded([cx - 70.0, cy + 25.0, 140.0, 36.0], 3.0, [0.04, 0.04, 0.04]);
                                let led_col = if *armed { [0.95, 0.10, 0.10] } else { [0.20, 0.85, 0.30] };
                                let led_text = if *armed { "00:05 ARME" } else { "--:-- PRET" };
                                self.psp_text(cx - 55.0, cy + 34.0, led_text, 1.9, led_col);

                                let prompt_str = format!("[ 1..=3 ] CONNECTER LES FILS ({}/3)", wires_connected);
                                self.psp_text(cx - 160.0, cy + 125.0, &prompt_str, 1.7, [1.0, 0.90, 0.20]);
                            }
                            crate::minigames::MinigameKind::DumpsterSearch { searched, found_weapon, found_cash } => {
                                self.psp_text(cx - 160.0, cy - 162.0, "FOUILLE DE BENNE A ORDURES", 1.8, [1.0, 0.88, 0.20]);
                                rounded([cx - 200.0, cy - 110.0, 400.0, 210.0], 6.0, [0.12, 0.16, 0.12]);
                                if *searched {
                                    let loot_str = if let Some(w) = found_weapon {
                                        format!("ARME TROUVEE : TYPE {}", w)
                                    } else {
                                        format!("ARGENT TROUVE : ${}", found_cash)
                                    };
                                    self.psp_text(cx - 110.0, cy - 10.0, &loot_str, 2.2, [1.0, 0.92, 0.25]);
                                } else {
                                    self.psp_text(cx - 110.0, cy - 10.0, "BENNE PLEINE...", 2.2, [0.75, 0.78, 0.82]);
                                }
                                self.psp_text(cx - 160.0, cy + 125.0, "[ E ] FOUILLER JUSQU'AU FOND DE LA BENNE", 1.7, [0.95, 0.95, 0.95]);
                            }
                            crate::minigames::MinigameKind::SniperAssembly { current_step, step_progress, parts_fitted, alignment_error: _ } => {
                                self.psp_text(cx - 180.0, cy - 162.0, "MALLETTE : MONTAGE DU FUSIL DE PRECISION", 1.8, [1.0, 0.88, 0.20]);
                                rounded([cx - 220.0, cy - 110.0, 440.0, 210.0], 6.0, [0.08, 0.09, 0.10]);
                                let steps = ["1. BOITIER", "2. CANON", "3. CULASSE", "4. LUNETTE", "5. SILENCIEUX"];
                                for (s, name) in steps.iter().enumerate() {
                                    let sx = cx - 180.0 + (s as f32) * 75.0;
                                    let fitted = parts_fitted[s];
                                    let bg = if fitted { [0.20, 0.85, 0.35] } else if s == *current_step as usize { [0.85, 0.75, 0.20] } else { [0.25, 0.28, 0.32] };
                                    rounded([sx - 32.0, cy - 70.0, 64.0, 40.0], 3.0, bg);
                                    self.psp_text(sx - 28.0, cy - 55.0, name, 1.4, [0.95, 0.95, 0.95]);
                                }
                                rounded([cx - 180.0, cy + 50.0, 360.0, 18.0], 2.0, [0.06, 0.07, 0.08]);
                                rounded([cx - 178.0, cy + 52.0, 356.0 * step_progress.clamp(0.0, 1.0), 14.0], 2.0, [0.25, 0.75, 0.95]);
                                self.psp_text(cx - 170.0, cy + 125.0, "[ E ] ALIGNER ET VISSER L'ELEMENT EN COURS", 1.7, [1.0, 0.95, 0.85]);
                            }
                        }
                        glDisable(0x0be2);
                    }
                }
                let mini = crate::gps::MapView {
                    rect: crate::hud::mini(bands),
                    center: [walker.p[0], walker.p[1]],
                    span: crate::radar::WORLD_SPAN,
                    rotation: if map_ui.rotate_minimap {
                        cam.radar_angle()
                    } else {
                        0.
                    },
                    circular: true,
                };
                let [mx, my, mw, mh] = mini.rect;
                let center = crate::radar::center(bands);
                let scale = crate::radar::scale();
                radar_arc(
                    center,
                    scale,
                    256,
                    false,
                    [0., 0., 0., crate::radar::MAP_ALPHA],
                );
                let health = scene.map(|s| s.health.clamp(0., 127.) as u8).unwrap_or(127);
                let armour = scene.map(|s| s.armour).unwrap_or(0);
                if let Some(rgb) = crate::radar::health_color(health, ticks) {
                    radar_arc(
                        center,
                        scale,
                        crate::radar::vital_steps(health),
                        false,
                        [
                            rgb[0] as f32 / 255.,
                            rgb[1] as f32 / 255.,
                            rgb[2] as f32 / 255.,
                            1.,
                        ],
                    );
                }
                let rgb = crate::radar::ARMOUR;
                radar_arc(
                    center,
                    scale,
                    crate::radar::vital_steps(armour),
                    true,
                    [
                        rgb[0] as f32 / 255.,
                        rgb[1] as f32 / 255.,
                        rgb[2] as f32 / 255.,
                        1.,
                    ],
                );
                self.navigation_map(mini, gps, walker, false, scene);
                self.radar_sprite(43, center, scale);
                // PSP GetRadarSpace clamps the north badge to 0x26000 Q12 = 38px.
                let north = mini.edge_marker(
                    mini.screen([walker.p[0], walker.p[1] + 1000., walker.p[2]]),
                    0.,
                );
                let north = [
                    ((north[0] - center[0]) / scale[0]).floor() * scale[0] + center[0],
                    ((north[1] - center[1]) / scale[1]).floor() * scale[1] + center[1],
                ];
                self.radar_sprite(37, north, scale);
                if gps.target.is_some() {
                    self.psp_text(
                        mx + mw + 22.,
                        my + mh - 46.,
                        &gps.guidance(walker.p),
                        1.8,
                        [0.98, 0.92, 0.72],
                    );
                    if !gps.route.is_empty() {
                        self.psp_text(
                            mx + mw + 22.,
                            my + mh - 21.,
                            &distance_label(gps.remaining(walker.p)),
                            1.8,
                            [0.95; 3],
                        );
                    }
                }
                if bands {
                    hud_button(crate::hud::MAP_BUTTON, "GPS", "M", "BACK", false);
                    hud_button(
                        crate::hud::ROTATE_BUTTON,
                        if map_ui.rotate_minimap {
                            "RADAR : ROTATION"
                        } else {
                            "RADAR : NORD FIXE"
                        },
                        "G",
                        "CROIX DROITE",
                        map_ui.rotate_minimap,
                    );
                    let state = if paused {
                        "PAUSE"
                    } else if scene.is_some_and(|s| s.driving.is_some()) {
                        "FREIN A MAIN"
                    } else if walker.roll.is_some() {
                        "ROULADE EN COURS"
                    } else if walker.vault.is_some() {
                        "FRANCHISSEMENT"
                    } else if walker.vault_ready {
                        "FRANCHIR"
                    } else {
                        "ROULADE"
                    };
                    if !cam.is_free() || paused {
                        hud_button(
                            crate::hud::ACTION_BUTTON,
                            state,
                            if paused { "P" } else { "ESPACE" },
                            if paused { "START" } else { "X" },
                            false,
                        );
                    }
                    if let Some(scene) = scene.filter(|_| !cam.is_free() && !paused) {
                        if scene.driving.is_some() || scene.nearest_car(walker.p).is_some() {
                            hud_button(
                                crate::hud::VEHICLE_BUTTON,
                                if let Some(i) = scene.driving {
                                    if scene.cars[i].speed.abs() > 4. {
                                        "FREINE POUR DESCENDRE"
                                    } else {
                                        "DESCENDRE DU VEHICULE"
                                    }
                                } else {
                                    "MONTER DANS LE VEHICULE"
                                },
                                "E",
                                "Y",
                                false,
                            );
                        }
                    }
                }
                if paused {
                    rounded([W as f32 / 2. - 110., 290., 220., 72.], 4., ink);
                    self.psp_text(W as f32 / 2. - 45., 300., "PAUSE", 2.5, [0.95; 3]);
                    pair(W as f32 / 2. - 48., 345., "P", "START");
                }
                if let Some(scene) = scene.filter(|s| s.pda.open && !paused) {
                    self.draw_pda(&scene.pda, scene.cash, ticks);
                }
                if let Some(s) = scene.filter(|s| s.safehouse_session.is_some() && !paused) {
                    self.draw_safehouse_ui(s, ticks);
                }
                if let Some(s) = scene.filter(|s| s.trade_session.is_some() && !paused) {
                    self.draw_dealer_trade_ui(s, ticks);
                }
                if scene.is_some_and(|s| s.health <= 0.) {
                    rounded([W as f32 / 2. - 200., 245., 400., 96.], 4., ink);
                    self.psp_text(
                        W as f32 / 2. - 105.,
                        259.,
                        "HORS COMBAT",
                        3.,
                        [0.88, 0.22, 0.16],
                    );
                    label(W as f32 / 2. - 105., 309., "RECOMMENCER", 1., [0.85; 3]);
                    pair(W as f32 / 2. + 34., 307., "R", "LB+Y");
                }
            } else {
                self.psp_text(24., 59., "CARTE DE LIBERTY CITY", 2.4, [0.9, 0.87, 0.77]);
                self.navigation_map(map_ui.view, gps, walker, true, scene);
                hud_button(
                    crate::hud::MAP_ROTATE,
                    if map_ui.rotate_minimap {
                        "ROTATION RADAR : ON"
                    } else {
                        "ROTATION RADAR : OFF"
                    },
                    "G",
                    "CROIX DROITE",
                    map_ui.rotate_minimap,
                );
                hud_button(crate::hud::MAP_CLOSE, "RETOUR AU JEU", "ECHAP", "B", false);
                hud_button(
                    crate::hud::MAP_CENTER,
                    "RECENTRER JOUEUR",
                    "ESPACE",
                    "Y",
                    false,
                );
                hud_button(crate::hud::MAP_FIT, "VOIR LE TRAJET", "F", "RS CLIC", false);
                hud_button(
                    crate::hud::MAP_CITY,
                    "TOUTE LA VILLE",
                    "V",
                    "LS CLIC",
                    false,
                );
                hud_button(crate::hud::MAP_ZOOM_IN, "ZOOM +", "+", "RB", false);
                hud_button(crate::hud::MAP_ZOOM_OUT, "ZOOM -", "-", "LB", false);
                hud_button(crate::hud::MAP_CLEAR, "EFFACER LE GPS", "SUPPR", "X", false);
                let [cx, cy] = map_ui.cursor;
                for (a, b) in [
                    ([cx - 10., cy], [cx + 10., cy]),
                    ([cx, cy - 10.], [cx, cy + 10.]),
                ] {
                    map_line(map_ui.view, a, b, 4., ink);
                    map_line(map_ui.view, a, b, 1.5, [1.; 3]);
                }
                if !bands {
                    self.psp_text(24., 642., &gps.guidance(walker.p), 2.1, gold);
                    for (i, &(action, pc, xbox)) in [
                        ("DESTINATION", "CLIC / ENTREE", "A"),
                        ("DEPLACER CARTE", "CLIC DROIT", "RS"),
                        ("CURSEUR", "FLECHES", "LS"),
                        ("FERMER CARTE", "M", "BACK"),
                    ]
                    .iter()
                    .enumerate()
                    {
                        let x = 24. + i as f32 * 310.;
                        label(x, 685., action, 1., [0.75; 3]);
                        pair(x, 697., pc, xbox);
                    }
                }
            }
            if bands {
                let hints = if map_ui.open {
                    crate::hud::MAP
                } else if cam.is_free() {
                    crate::hud::FREE
                } else if scene.is_some_and(|s| s.driving.is_some()) {
                    crate::hud::DRIVE
                } else {
                    crate::hud::FOOT
                };
                panel(0., 632., W as f32, 88., [0.; 3]);
                for (i, &(action, pc, xbox)) in hints.iter().enumerate() {
                    let x = 18. + (i % 5) as f32 * 252.;
                    let y = 636. + (i / 5) as f32 * 21.;
                    label(x, y, action, 1., [0.65, 0.70, 0.69]);
                    pair(x, y + 9., pc, xbox);
                }
            }
            let e = glGetError();
            if e != 0 {
                return Err(format!("OpenGL rendu: {e:#x}").into());
            }
        }
        Ok(())
    }
    /// PSP_Hel12x12.bin + PSP PAK261. Original glyph widths, ASCII mapping and
    /// advance of width-1, with the dark displaced shadow visible in PSP footage.
    unsafe fn psp_text(&self, x: f32, y: f32, text: &str, scale: f32, color: [f32; 3]) {
        let text = crate::locale::text(text);
        for (offset, c) in [
            (1.5, [0.02, 0.02, 0.02, 0.9]),
            (0., [color[0], color[1], color[2], 1.]),
        ] {
            let mut px = x + offset;
            for ch in text.chars() {
                let index = (ch as usize).checked_sub(32).unwrap_or(13);
                let Some(&[w, gx, gy]) = self.glyphs.get(index) else {
                    continue;
                };
                ui_sprite(
                    self.font_texture,
                    self.font_size,
                    [gx as f32, gy as f32, w as f32, self.font_height],
                    [px, y + offset, w as f32 * scale, self.font_height * scale],
                    c,
                );
                px += w.saturating_sub(1) as f32 * scale;
            }
        }
    }
    unsafe fn sprite(&self, quads: Vec<crate::ped::Quad>) {
        for q in quads {
            if let Some(&tex) = self.ped_textures.get(&q.texture) {
                glBindTexture(0x0de1, tex);
                glBegin(4);
                for k in [0, 1, 2, 0, 2, 3] {
                    glTexCoord2f(q.uv[k][0], q.uv[k][1]);
                    glVertex3f(q.p[k][0], q.p[k][1], q.p[k][2]);
                }
                glEnd();
            }
        }
    }
    unsafe fn navigation_map(
        &self,
        view: crate::gps::MapView,
        gps: &crate::gps::Gps,
        walker: &Walker,
        large: bool,
        _scene: Option<&crate::actors::Scene>,
    ) {
        let [x, y, w, h] = view.rect;
        glEnable(0x0de1);
        glEnable(0x0be2);
        glBindTexture(0x0de1, self.terrain_texture);
        glColor4f(1., 1., 1., if large { 1. } else { crate::radar::MAP_ALPHA });
        let mut terrain_view = view;
        if !large {
            terrain_view.center = crate::radar::snapped_focus(view.center);
        }
        let tex_coord = |p| {
            let uv =
                crate::radar::map_uv(terrain_view.world(p), self.radar_origin, self.radar_size);
            glTexCoord2f(uv[0], uv[1]);
        };
        if view.circular {
            glBegin(6);
            let center = [x + w / 2., y + h / 2.];
            tex_coord(center);
            glVertex2f(center[0], center[1]);
            for k in 0..=256 {
                let v = crate::radar::perimeter(k);
                let p = [center[0] + v[0] * w / 152., center[1] + v[1] * h / 152.];
                tex_coord(p);
                glVertex2f(p[0], p[1]);
            }
            glEnd();
        } else {
            glBegin(7);
            for p in [[x, y], [x + w, y], [x + w, y + h], [x, y + h]] {
                tex_coord(p);
                glVertex2f(p[0], p[1]);
            }
            glEnd();
        }
        glDisable(0x0de1);
        glDisable(0x0be2);
        for segment in gps.route.windows(2) {
            let a = view.screen(segment[0]);
            let b = view.screen(segment[1]);
            map_line(view, a, b, 7., [0.08, 0.09, 0.07]);
            map_line(view, a, b, 4., [1., 0.77, 0.12]);
        }
        if let Some(first) = gps.route.first() {
            // Dashed approach is guidance to the road, not a collision-checked walking path.
            let a = view.screen(walker.p);
            let b = view.screen(*first);
            let n = (((a[0] - b[0]).hypot(a[1] - b[1]) / 7.).ceil() as usize)
                .min(1000)
                .max(1);
            for i in (0..n).step_by(2) {
                let p = std::array::from_fn(|k| a[k] + (b[k] - a[k]) * i as f32 / n as f32);
                let q = std::array::from_fn(|k| a[k] + (b[k] - a[k]) * (i + 1) as f32 / n as f32);
                map_line(view, p, q, 2., [1., 0.87, 0.5]);
            }
        }
        if let Some(t) = gps.target {
            let mut p = view.screen(t.p);
            if !large {
                p = view.edge_marker(p, 10.);
            }
            if view.contains(p) {
                let c = if gps.arrived {
                    [0.2, 1., 0.55]
                } else {
                    [1., 0.28, 0.45]
                };
                for k in 0..2 {
                    let r = if k == 0 { 8. } else { 5. };
                    glColor3f(
                        if k == 0 { 0.05 } else { c[0] },
                        if k == 0 { 0.05 } else { c[1] },
                        if k == 0 { 0.05 } else { c[2] },
                    );
                    glBegin(7);
                    for v in [[0., -r], [r, 0.], [0., r], [-r, 0.]] {
                        glVertex2f(p[0] + v[0], p[1] + v[1]);
                    }
                    glEnd();
                }
            }
        }
        if let Some(s) = _scene {
            // Safehouse markers
            for sh in &s.safehouses.properties {
                if sh.owned || large {
                    let mut sp = view.screen(sh.pos);
                    if !large {
                        sp = view.edge_marker(sp, 8.);
                    }
                    if view.contains(sp) {
                        let c = if sh.owned { [0.22, 0.85, 0.35] } else { [0.85, 0.85, 0.25] };
                        for k in 0..2 {
                            let r = if k == 0 { 5. } else { 3.5 };
                            glColor3f(if k == 0 { 0.05 } else { c[0] }, if k == 0 { 0.05 } else { c[1] }, if k == 0 { 0.05 } else { c[2] });
                            glBegin(7);
                            for v in [[-r, -r], [r, -r], [r, r], [-r, r]] {
                                glVertex2f(sp[0] + v[0], sp[1] + v[1]);
                            }
                            glEnd();
                        }
                    }
                }
            }

            // Story Contact Golden Blips
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
                let mut cp = view.screen(contact.meeting_pos());
                if !large {
                    cp = view.edge_marker(cp, 9.);
                }
                if view.contains(cp) {
                    for k in 0..2 {
                        let r = if k == 0 { 6.5 } else { 4.5 };
                        glColor3f(if k == 0 { 0.05 } else { 1.0 }, if k == 0 { 0.05 } else { 0.85 }, if k == 0 { 0.05 } else { 0.15 });
                        glBegin(7);
                        for v in [[0., -r], [r, 0.], [0., r], [-r, 0.]] {
                            glVertex2f(cp[0] + v[0], cp[1] + v[1]);
                        }
                        glEnd();
                    }
                }
            }

            // Discovered Drug Dealers (Cyan Blips)
            for dealer in &s.drugs.dealers {
                if dealer.discovered || large {
                    let mut dp = view.screen(dealer.pos);
                    if !large {
                        dp = view.edge_marker(dp, 7.);
                    }
                    if view.contains(dp) {
                        for k in 0..2 {
                            let r = if k == 0 { 4.5 } else { 3. };
                            glColor3f(if k == 0 { 0.05 } else { 0.2 }, if k == 0 { 0.05 } else { 0.65 }, if k == 0 { 0.05 } else { 0.95 });
                            glBegin(7);
                            for v in [[-r, -r], [r, -r], [r, r], [-r, r]] {
                                glVertex2f(dp[0] + v[0], dp[1] + v[1]);
                            }
                            glEnd();
                        }
                    }
                }
            }

            // Active Mission Objective Waypoint
            if let crate::missions::MissionState::InProgress { objective: crate::missions::MissionObjective::GoToLocation { target, .. }, .. } = &s.missions.state {
                let mut mp = view.screen(*target);
                if !large {
                    mp = view.edge_marker(mp, 10.);
                }
                if view.contains(mp) {
                    for k in 0..2 {
                        let r = if k == 0 { 8. } else { 5.5 };
                        glColor3f(if k == 0 { 0.05 } else { 0.95 }, if k == 0 { 0.05 } else { 0.90 }, if k == 0 { 0.05 } else { 0.15 });
                        glBegin(7);
                        for v in [[0., -r], [r, 0.], [0., r], [-r, 0.]] {
                            glVertex2f(mp[0] + v[0], mp[1] + v[1]);
                        }
                        glEnd();
                    }
                }
            }

            // Active Odd Job Objective Waypoint (Taxi, Vigilante, Firefighter, Paramedic, Delivery)
            if let Some((blip_pos, _sym, blip_color)) = s.jobs.current_radar_blip() {
                let mut jp = view.screen(blip_pos);
                if !large {
                    jp = view.edge_marker(jp, 9.);
                }
                if view.contains(jp) {
                    let cr = blip_color[0] as f32 / 255.0;
                    let cg = blip_color[1] as f32 / 255.0;
                    let cb = blip_color[2] as f32 / 255.0;
                    for k in 0..2 {
                        let r = if k == 0 { 7. } else { 4.5 };
                        glColor3f(if k == 0 { 0.05 } else { cr }, if k == 0 { 0.05 } else { cg }, if k == 0 { 0.05 } else { cb });
                        glBegin(7);
                        for v in [[0., -r], [r, 0.], [0., r], [-r, 0.]] {
                            glVertex2f(jp[0] + v[0], jp[1] + v[1]);
                        }
                        glEnd();
                    }
                }
            }

            // Active Street Race Checkpoint Waypoint
            if let Some(race) = &s.active_race {
                if let Some((blip_pos, blip_color)) = race.current_radar_blip() {
                    let mut jp = view.screen(blip_pos);
                    if !large {
                        jp = view.edge_marker(jp, 9.);
                    }
                    if view.contains(jp) {
                        let cr = blip_color[0] as f32 / 255.0;
                        let cg = blip_color[1] as f32 / 255.0;
                        let cb = blip_color[2] as f32 / 255.0;
                        for k in 0..2 {
                            let r = if k == 0 { 8. } else { 5.0 };
                            glColor3f(if k == 0 { 0.05 } else { cr }, if k == 0 { 0.05 } else { cg }, if k == 0 { 0.05 } else { cb });
                            glBegin(7);
                            for v in [[0., -r], [r, 0.], [0., r], [-r, 0.]] {
                                glVertex2f(jp[0] + v[0], jp[1] + v[1]);
                            }
                            glEnd();
                        }
                    }
                }
            }

            // Nearest Pay 'n' Spray Garage Waypoint
            if let Some((pns_pos, pns_col)) = s.pay_n_spray.nearest_blip(walker.p) {
                let mut jp = view.screen([pns_pos[0], pns_pos[1], 0.0]);
                if !large {
                    jp = view.edge_marker(jp, 9.);
                }
                if view.contains(jp) {
                    for k in 0..2 {
                        let r = if k == 0 { 7.5 } else { 4.5 };
                        glColor3f(if k == 0 { 0.05 } else { pns_col[0] }, if k == 0 { 0.05 } else { pns_col[1] }, if k == 0 { 0.05 } else { pns_col[2] });
                        glBegin(7);
                        for v in [[0., -r], [r, 0.], [0., r], [-r, 0.]] {
                            glVertex2f(jp[0] + v[0], jp[1] + v[1]);
                        }
                        glEnd();
                    }
                }
            }
        }
        let p = view.screen(walker.p);
        if view.contains(p) {
            let forward = view.screen([
                walker.p[0] - walker.heading.sin(),
                walker.p[1] + walker.heading.cos(),
                walker.p[2],
            ]);
            let d = [forward[0] - p[0], forward[1] - p[1]];
            let length = d[0].hypot(d[1]).max(0.001);
            let u = [d[0] / length, d[1] / length];

            let scale = if large {
                [1.5; 2]
            } else {
                crate::radar::scale()
            };
            glPushMatrix();
            glTranslatef(p[0], p[1], 0.);
            glScalef(scale[0], scale[1], 1.);
            glRotatef(
                (u[0] / scale[0]).atan2(-u[1] / scale[1]).to_degrees(),
                0.,
                0.,
                1.,
            );
            self.radar_sprite(38, [0.; 2], [1.; 2]);
            glPopMatrix();
        }
        if large {
            label(x + 7., y + 7., "N", 1., [0.9, 0.95, 1.]);
        }
        glLineWidth(1.);
    }
    unsafe fn radar_sprite(&self, id: u16, anchor: [f32; 2], scale: [f32; 2]) {
        if let Some(sprite) = self.radar_sprites.iter().find(|s| s.id == id) {
            ui_sprite(
                self.radar_texture,
                [256., 128.],
                sprite.source,
                sprite.target(anchor, scale),
                [1.; 4],
            );
        }
    }
    unsafe fn draw_pda(&self, pda: &crate::pda::PdaEngine, cash: u32, ticks: usize) {
        let px = W as f32 / 2. - 250.;
        let py = H as f32 / 2. - 240.;
        let pw = 500.;
        let ph = 480.;

        // 1. Outer PDA Shell and Inner Glass Screen
        rounded([px - 10., py - 10., pw + 20., ph + 20.], 8., [0.08, 0.09, 0.11]);
        rounded([px, py, pw, ph], 6., [0.03, 0.04, 0.05]);

        // 2. Status Bar Header
        rounded([px + 8., py + 8., pw - 16., 34.], 4., [0.12, 0.15, 0.20]);
        self.psp_text(px + 16., py + 14., "PDA - LIBERTY CITY OS", 2.0, [0.95, 0.90, 0.70]);
        let minutes = (8 * 60 + ticks / 30) % 1440;
        let time_str = format!("MAR {:02}:{:02}", minutes / 60, minutes % 60);
        self.psp_text(px + pw - 130., py + 14., &time_str, 2.0, [0.85, 0.92, 0.88]);

        // 3. App Views
        match pda.current_view {
            crate::pda::PdaView::Home => {
                self.psp_text(px + 20., py + 55., "MENU PRINCIPAL", 2.2, [1.0, 0.85, 0.20]);

                let apps = [
                    ("1. MESSAGERIE ELECTRONIQUE", format!("{} message(s) non lu(s)", pda.unread_count)),
                    ("2. CARNET DE CONTACTS & GPS", format!("{} contacts enregistres", pda.contacts.len())),
                    ("3. BOUTIQUE AMMU-NATION", "Commandez vos armes en ligne".to_string()),
                    ("4. COURS DU MARCHE & STATS", "Alertes penuries et surplus".to_string()),
                    ("5. SYSTEME ET PARAMETRES", "Options d'affichage et son".to_string()),
                ];

                for (idx, (title, sub)) in apps.iter().enumerate() {
                    let ay = py + 90. + idx as f32 * 62.;
                    let selected = pda.selected_index == idx;
                    let bg = if selected { [0.22, 0.26, 0.35] } else { [0.07, 0.09, 0.12] };
                    rounded([px + 20., ay, pw - 40., 54.], 4., bg);
                    let title_col = if selected { [1.0, 0.92, 0.35] } else { [0.90, 0.92, 0.94] };
                    self.psp_text(px + 32., ay + 8., title, 1.9, title_col);
                    self.psp_text(px + 32., ay + 30., sub, 1.6, [0.65, 0.70, 0.75]);
                }
            }
            crate::pda::PdaView::ContactsList => {
                self.psp_text(px + 20., py + 55., "CARNET DE CONTACTS & GPS", 2.2, [1.0, 0.85, 0.20]);
                for (idx, contact) in pda.contacts.iter().enumerate().take(5) {
                    let cy = py + 95. + idx as f32 * 68.;
                    let selected = pda.selected_index == idx;
                    let bg = if selected { [0.22, 0.26, 0.35] } else { [0.07, 0.09, 0.12] };
                    rounded([px + 20., cy, pw - 40., 60.], 4., bg);
                    let title = format!("{}: {}", contact.name, contact.title);
                    let col = if selected { [1.0, 0.92, 0.35] } else { [0.85, 0.88, 0.90] };
                    self.psp_text(px + 28., cy + 10., &title, 1.8, col);
                    let sub = format!("{} | Tel: {}", contact.district, contact.phone);
                    self.psp_text(px + 28., cy + 32., &sub, 1.6, [0.60, 0.65, 0.70]);
                }
            }
            crate::pda::PdaView::ContactDetail(idx) => {
                if let Some(contact) = pda.contacts.get(idx) {
                    self.psp_text(px + 20., py + 55., contact.name, 2.2, [1.0, 0.85, 0.20]);
                    self.psp_text(px + 20., py + 85., contact.title, 1.9, [0.85, 0.90, 0.95]);

                    rounded([px + 20., py + 115., pw - 40., 270.], 4., [0.06, 0.08, 0.10]);
                    self.psp_text(px + 32., py + 135., &format!("Organisation : {}", contact.organization), 1.8, [0.92, 0.92, 0.94]);
                    self.psp_text(px + 32., py + 165., &format!("Quartier : {}", contact.district), 1.8, [0.92, 0.92, 0.94]);
                    self.psp_text(px + 32., py + 195., &format!("Numero de telephone : {}", contact.phone), 1.8, [0.92, 0.92, 0.94]);
                    self.psp_text(px + 32., py + 235., "[ ENTREE / A ] : ROUTER LE GPS VERS CE CONTACT", 1.9, [0.28, 0.88, 0.38]);
                }
            }
            crate::pda::PdaView::EmailInbox => {
                self.psp_text(px + 20., py + 55., "BOITE DE RECEPTION", 2.2, [1.0, 0.85, 0.20]);
                for (idx, email) in pda.emails.iter().enumerate().take(5) {
                    let ey = py + 95. + idx as f32 * 68.;
                    let selected = pda.selected_index == idx;
                    let bg = if selected { [0.22, 0.26, 0.35] } else { [0.07, 0.09, 0.12] };
                    rounded([px + 20., ey, pw - 40., 60.], 4., bg);
                    let prefix = if !email.read { "[NOUVEAU] " } else { "" };
                    let title = format!("{}{}: {}", prefix, email.sender, email.subject);
                    let col = if !email.read { [0.98, 0.85, 0.25] } else { [0.85, 0.88, 0.90] };
                    self.psp_text(px + 28., ey + 10., &title, 1.8, col);
                    self.psp_text(px + 28., ey + 32., email.timestamp_str, 1.6, [0.60, 0.65, 0.70]);
                }
            }
            crate::pda::PdaView::EmailDetail(idx) => {
                if let Some(email) = pda.emails.get(idx) {
                    self.psp_text(px + 20., py + 55., email.subject, 2.1, [1.0, 0.85, 0.20]);
                    self.psp_text(px + 20., py + 85., &format!("De: {}", email.sender), 1.9, [0.85, 0.90, 0.95]);

                    rounded([px + 20., py + 115., pw - 40., 270.], 4., [0.06, 0.08, 0.10]);
                    let words = email.body.split_whitespace().collect::<Vec<_>>();
                    let mut line = String::new();
                    let mut ly = py + 130.;
                    for word in words {
                        if line.len() + word.len() > 38 {
                            self.psp_text(px + 32., ly, &line, 1.8, [0.92, 0.92, 0.94]);
                            ly += 24.;
                            line.clear();
                        }
                        if !line.is_empty() { line.push(' '); }
                        line.push_str(word);
                    }
                    if !line.is_empty() {
                        self.psp_text(px + 32., ly, &line, 1.8, [0.92, 0.92, 0.94]);
                    }
                }
            }
            crate::pda::PdaView::AmmuNationStore => {
                self.psp_text(px + 20., py + 55., "AMMU-NATION ONLINE", 2.2, [0.95, 0.25, 0.20]);
                let cash_msg = format!("FONDS: ${}", cash);
                self.psp_text(px + pw - 200., py + 58., &cash_msg, 1.8, [0.28, 0.88, 0.38]);

                for (idx, item) in crate::pda::AMMUNATION_CATALOG.iter().enumerate().take(5) {
                    let iy = py + 95. + idx as f32 * 68.;
                    let selected = pda.selected_index == idx;
                    let bg = if selected { [0.22, 0.26, 0.35] } else { [0.07, 0.09, 0.12] };
                    rounded([px + 20., iy, pw - 40., 60.], 4., bg);
                    let title = format!("{} (${})", item.name, item.price);
                    let title_col = if selected { [1.0, 0.92, 0.35] } else { [0.90, 0.92, 0.94] };
                    self.psp_text(px + 28., iy + 10., &title, 1.9, title_col);
                    let sub = format!("Munitions: +{} | {}", item.rounds_per_purchase, item.category);
                    self.psp_text(px + 28., iy + 32., &sub, 1.6, [0.65, 0.70, 0.75]);
                }
            }
            crate::pda::PdaView::TradeStats => {
                self.psp_text(px + 20., py + 55., "COURS DU MARCHE DES DROGUES", 2.2, [0.30, 0.85, 0.40]);
                let drugs = [
                    (crate::drugs::DrugType::Acid, "Acide (LSD)", "Triads (Vente) / Yardies (Achat)"),
                    (crate::drugs::DrugType::Coke, "Coke", "Spanish Lords (Vente) / Mobsters (Achat)"),
                    (crate::drugs::DrugType::Downers, "Downers", "Angels of Death (Vente) / Midtown (Achat)"),
                    (crate::drugs::DrugType::Ecstasy, "Extasy", "Mobsters (Vente) / Spanish Lords (Achat)"),
                    (crate::drugs::DrugType::Heroin, "Heroine", "Midtown (Vente) / Triads (Achat)"),
                    (crate::drugs::DrugType::Weed, "Weed", "Hustlers (Vente) / Angels of Death (Achat)"),
                ];
                for (idx, (drug, name, spec)) in drugs.iter().enumerate() {
                    let dy = py + 95. + idx as f32 * 54.;
                    rounded([px + 20., dy, pw - 40., 48.], 3., [0.07, 0.09, 0.12]);
                    let (min_p, max_p) = drug.price_range();
                    let info = format!("{}: Base ${} (${}-${})", name, drug.base_price(), min_p, max_p);
                    self.psp_text(px + 28., dy + 8., &info, 1.8, [0.95, 0.92, 0.80]);
                    self.psp_text(px + 28., dy + 26., spec, 1.5, [0.60, 0.75, 0.65]);
                }
            }
            crate::pda::PdaView::SystemSettings => {
                self.psp_text(px + 20., py + 55., "PARAMETRES SYSTEME", 2.2, [1.0, 0.85, 0.20]);
                rounded([px + 20., py + 95., pw - 40., 280.], 4., [0.06, 0.08, 0.10]);
                self.psp_text(px + 32., py + 120., "ReGTA CTW PC Recomp - v0.1.4-alpha", 2.0, [0.95, 0.90, 0.70]);
                self.psp_text(px + 32., py + 155., "Moteur : Rust pur natif (Allegrex MIPS recomp)", 1.8, [0.75, 0.80, 0.85]);
                self.psp_text(px + 32., py + 185., "Rendu : OpenGL direct (GeForce RTX 5070 Ti)", 1.8, [0.75, 0.80, 0.85]);
                self.psp_text(px + 32., py + 215., "Resolution : Dynamique (FPS Illimités / Uncapped)", 1.8, [0.75, 0.80, 0.85]);
                self.psp_text(px + 32., py + 245., "Par Benji Paname (Paris, France)", 1.9, [1.0, 0.85, 0.20]);
            }
        }

        // 4. Footer navigation hint
        rounded([px + 8., py + ph - 42., pw - 16., 34.], 4., [0.10, 0.12, 0.15]);
        self.psp_text(px + 16., py + ph - 34., "HAUT/BAS: CHOISIR | ENTREE: OUVRIR | RETOUR/I: RETOUR", 1.6, [0.80, 0.85, 0.88]);
    }

    unsafe fn draw_safehouse_ui(&self, s: &crate::actors::Scene, _ticks: usize) {
        let Some(session) = &s.safehouse_session else { return };
        let sh_idx = s.safehouses.properties.iter().position(|sh| sh.id == session.safehouse_id).unwrap_or(0);
        let safehouse = &s.safehouses.properties[sh_idx];

        let pw = 720.0;
        let ph = 460.0;
        let px = (W as f32 - pw) * 0.5;
        let py = (H as f32 - ph) * 0.5;

        // Dark chassis
        rounded([px - 4., py - 4., pw + 8., ph + 8.], 10.0, [0.08, 0.12, 0.16]);
        rounded([px, py, pw, ph], 8.0, [0.05, 0.07, 0.09]);

        // Header
        let header = format!("PLANQUE : {}  |  {}, {}", safehouse.name, safehouse.district, safehouse.borough);
        self.psp_text(px + 20., py + 26., &header, 2.0, [1.0, 0.88, 0.20]);
        let cash_str = format!("PORTEFEUILLE : ${}", s.cash);
        self.psp_text(px + pw - 200., py + 26., &cash_str, 1.8, [0.35, 0.90, 0.45]);

        // 4 Tabs Bar
        let tabs = [
            (crate::safehouse::SafehouseTab::Bed, "[ 1 ] LIT & SAUVEGARDE"),
            (crate::safehouse::SafehouseTab::DrugStash, "[ 2 ] PLANQUE DROGUES"),
            (crate::safehouse::SafehouseTab::CashSafe, "[ 3 ] COFFRE-FORT"),
            (crate::safehouse::SafehouseTab::Wardrobe, "[ 4 ] PENDERIE"),
        ];
        let tab_w = (pw - 40.) / 4.;
        for (i, (tab_kind, tab_label)) in tabs.iter().enumerate() {
            let tx = px + 20. + (i as f32) * tab_w;
            let active = session.active_tab == *tab_kind;
            let col = if active { [0.14, 0.28, 0.38] } else { [0.08, 0.10, 0.13] };
            let text_col = if active { [1.0, 0.95, 0.30] } else { [0.65, 0.70, 0.75] };
            rounded([tx, py + 45., tab_w - 4., 28.], 4.0, col);
            self.psp_text(tx + 8., py + 64., tab_label, 1.5, text_col);
        }

        // Active Tab Body
        match session.active_tab {
            crate::safehouse::SafehouseTab::Bed => {
                rounded([px + 20., py + 85., pw - 40., 315.], 6.0, [0.07, 0.09, 0.12]);
                self.psp_text(px + 40., py + 125., "CHAMBRE PRINCIPALE - LIT DE SAUVEGARDE", 2.2, [0.30, 0.85, 0.95]);
                self.psp_text(px + 40., py + 165., "Dormir avance le temps de 6 heures (+360 min), restaure la sante", 1.8, [0.85, 0.88, 0.90]);
                self.psp_text(px + 40., py + 195., "de Huang Lee a 127 HP (100%) et sauvegarde la progression.", 1.8, [0.85, 0.88, 0.90]);

                let times_saved = format!("Sauvegardes effectuees au total : {}", s.stats.number_of_times_saved());
                self.psp_text(px + 40., py + 245., &times_saved, 1.7, [0.95, 0.85, 0.20]);
                let cur_hp = format!("Sante actuelle : {:.0} / 127 HP", s.health);
                self.psp_text(px + 40., py + 275., &cur_hp, 1.7, [0.95, 0.35, 0.35]);

                rounded([px + 40., py + 325., 320., 45.], 4.0, [0.12, 0.35, 0.22]);
                self.psp_text(px + 55., py + 355., "[ ENTREE ] DORMIR & COMMETTRE SAUVEGARDE", 1.7, [1.0, 1.0, 1.0]);
            }
            crate::safehouse::SafehouseTab::DrugStash => {
                rounded([px + 20., py + 85., pw - 40., 315.], 6.0, [0.07, 0.09, 0.12]);
                let bag_total = s.drugs.player_bag.total_units();
                let stash_total = safehouse.stash_drugs.total_units();
                let bag_hdr = format!("SAC A DOS HUANG ({} / 100 U)", bag_total);
                let stash_hdr = format!("PLANQUE SECURISEE ({} / 500 U)", stash_total);
                self.psp_text(px + 35., py + 115., &bag_hdr, 1.8, [0.30, 0.85, 0.40]);
                self.psp_text(px + 390., py + 115., &stash_hdr, 1.8, [0.35, 0.75, 0.95]);

                let drugs = [
                    crate::drugs::DrugType::Acid,
                    crate::drugs::DrugType::Coke,
                    crate::drugs::DrugType::Downers,
                    crate::drugs::DrugType::Ecstasy,
                    crate::drugs::DrugType::Heroin,
                    crate::drugs::DrugType::Weed,
                ];
                for (i, drug) in drugs.iter().enumerate() {
                    let ry = py + 135. + (i as f32) * 36.;
                    let selected = session.selected_item == i;
                    let row_col = if selected { [0.16, 0.22, 0.28] } else { [0.09, 0.11, 0.14] };
                    rounded([px + 30., ry, pw - 60., 32.], 3.0, row_col);

                    let bag_qty = s.drugs.player_bag.get(*drug);
                    let stash_qty = safehouse.stash_drugs.get(*drug);
                    let cursor = if selected { "> " } else { "  " };
                    let drug_lbl = format!("{}{:<12} | Sac : {:>3} u", cursor, drug.name(), bag_qty);
                    let stash_lbl = format!("| Planque : {:>3} u (Valeur ~${})", stash_qty, stash_qty * drug.base_price());
                    self.psp_text(px + 38., ry + 22., &drug_lbl, 1.6, if selected { [1.0, 0.92, 0.30] } else { [0.88, 0.90, 0.92] });
                    self.psp_text(px + 390., ry + 22., &stash_lbl, 1.6, [0.70, 0.85, 0.95]);
                }
                let qty_hint = format!("Transfert : x{} u  ([ GAUCHE/DROITE ] ajuster)", session.transfer_qty);
                self.psp_text(px + 40., py + 380., &qty_hint, 1.6, [1.0, 0.85, 0.20]);
                self.psp_text(px + 390., py + 380., "[ ENTREE ] DEPOSER (SAC->PLANQUE) / RETIRER", 1.5, [0.85, 0.95, 0.85]);
            }
            crate::safehouse::SafehouseTab::CashSafe => {
                rounded([px + 20., py + 85., pw - 40., 315.], 6.0, [0.07, 0.09, 0.12]);
                self.psp_text(px + 40., py + 125., "COFFRE-FORT DE LA PLANQUE", 2.2, [1.0, 0.85, 0.20]);
                let w_cash = format!("Liquide sur vous : ${}", s.cash);
                let s_cash = format!("Tresor dans le coffre : ${}", safehouse.stash_cash);
                self.psp_text(px + 40., py + 165., &w_cash, 1.8, [0.40, 0.90, 0.45]);
                self.psp_text(px + 40., py + 195., &s_cash, 1.8, [0.35, 0.75, 0.95]);

                let options = ["1. Transferer $100", "2. Transferer $1,000", "3. Transferer la Totalite"];
                for (i, opt) in options.iter().enumerate() {
                    let oy = py + 235. + (i as f32) * 40.;
                    let selected = session.selected_item == i;
                    rounded([px + 40., oy, 320., 34.], 3.0, if selected { [0.18, 0.26, 0.32] } else { [0.10, 0.12, 0.15] });
                    self.psp_text(px + 55., oy + 24., opt, 1.6, if selected { [1.0, 0.95, 0.30] } else { [0.85, 0.88, 0.90] });
                }
                self.psp_text(px + 40., py + 380., "[ ENTREE ] DEPOSER DANS LE COFFRE  (ou RETIRER si portefeuille vide)", 1.5, [0.85, 0.95, 0.85]);
            }
            crate::safehouse::SafehouseTab::Wardrobe => {
                rounded([px + 20., py + 85., pw - 40., 315.], 6.0, [0.07, 0.09, 0.12]);
                self.psp_text(px + 40., py + 125., "PENDERIE & GARDEROBE", 2.2, [0.95, 0.45, 0.85]);
                let outfits = [
                    (crate::safehouse::PlayerOutfit::CivilianGreen, "Veste Civile Huang (Defaut) - Veste kaki et jean urbain"),
                    (crate::safehouse::PlayerOutfit::TriadSuit, "Costume Enforceur de la Triade - Costume cravate noir"),
                    (crate::safehouse::PlayerOutfit::Tracksuit, "Survetement Street Racer - Tenue de course jaune et noire"),
                    (crate::safehouse::PlayerOutfit::Tourist, "Chemise Hawaiienne de Touriste - Chemise a fleurs sous couverture"),
                ];
                for (i, (outfit, desc)) in outfits.iter().enumerate() {
                    let oy = py + 155. + (i as f32) * 48.;
                    let selected = session.selected_item == i;
                    let is_active = s.safehouses.current_outfit == *outfit;
                    rounded([px + 40., oy, pw - 80., 40.], 3.0, if selected { [0.20, 0.18, 0.26] } else { [0.09, 0.11, 0.14] });
                    let tag = if is_active { " [ EQUIPE ]" } else { "" };
                    let lbl = format!("{}{}", desc, tag);
                    let col = if is_active { [0.35, 0.95, 0.45] } else if selected { [1.0, 0.90, 0.30] } else { [0.85, 0.88, 0.90] };
                    self.psp_text(px + 55., oy + 26., &lbl, 1.6, col);
                }
                self.psp_text(px + 40., py + 380., "[ ENTREE ] ENFILER LA TENUE SELECTIONNEE", 1.6, [1.0, 0.85, 0.20]);
            }
        }

        // Notification / feedback banner
        if let Some(msg) = &session.message {
            rounded([px + 20., py + ph - 75., pw - 40., 28.], 3.0, [0.15, 0.35, 0.20]);
            self.psp_text(px + 32., py + ph - 56., msg, 1.6, [1.0, 1.0, 0.90]);
        }

        // Footer hint
        rounded([px + 8., py + ph - 42., pw - 16., 34.], 4.0, [0.10, 0.12, 0.15]);
        self.psp_text(px + 16., py + ph - 34., "1..4/TAB: ONGLETS | HAUT/BAS: CHOISIR | GAUCHE/DROITE: QUANTITE | ENTREE: VALIDER | ECHAP: QUITTER", 1.5, [0.80, 0.85, 0.88]);
    }

    unsafe fn draw_dealer_trade_ui(&self, s: &crate::actors::Scene, _ticks: usize) {
        let Some(session) = &s.trade_session else { return };
        let dealer = &s.drugs.dealers[session.dealer_idx.min(s.drugs.dealers.len() - 1)];

        let pw = 740.0;
        let ph = 470.0;
        let px = (W as f32 - pw) * 0.5;
        let py = (H as f32 - ph) * 0.5;

        // Dark chassis
        rounded([px - 4., py - 4., pw + 8., ph + 8.], 10.0, [0.14, 0.10, 0.08]);
        rounded([px, py, pw, ph], 8.0, [0.07, 0.06, 0.08]);

        // Header
        let d_hdr = format!("DEALER : {}  |  GANG : {}  |  QUARTIER : {}", dealer.name, dealer.gang.name(), dealer.district);
        self.psp_text(px + 20., py + 26., &d_hdr, 1.9, [1.0, 0.85, 0.20]);

        // Mode switch tabs
        let buy_active = session.mode == crate::drugs::TradeMode::Buy;
        let buy_col = if buy_active { [0.15, 0.38, 0.22] } else { [0.09, 0.12, 0.14] };
        let sell_col = if !buy_active { [0.38, 0.18, 0.15] } else { [0.09, 0.12, 0.14] };
        rounded([px + 20., py + 45., 220., 28.], 4.0, buy_col);
        rounded([px + 250., py + 45., 220., 28.], 4.0, sell_col);
        self.psp_text(px + 32., py + 64., "[ TAB ] ACHAT (DEALER->SAC)", 1.5, if buy_active { [1.0, 1.0, 1.0] } else { [0.65, 0.70, 0.75] });
        self.psp_text(px + 262., py + 64., "[ TAB ] VENTE (SAC->DEALER)", 1.5, if !buy_active { [1.0, 1.0, 1.0] } else { [0.65, 0.70, 0.75] });

        let bag_total = s.drugs.player_bag.total_units();
        let bag_str = format!("SAC : {}/100 U  |  ${}", bag_total, s.cash);
        self.psp_text(px + pw - 240., py + 64., &bag_str, 1.7, [0.35, 0.90, 0.45]);

        // 6 Drug rows
        let drugs = [
            (crate::drugs::DrugType::Acid, "Acide (LSD)", [0.45, 0.85, 0.95]),
            (crate::drugs::DrugType::Coke, "Cocaine", [0.95, 0.95, 0.98]),
            (crate::drugs::DrugType::Downers, "Downers", [0.85, 0.45, 0.35]),
            (crate::drugs::DrugType::Ecstasy, "Extasy", [0.95, 0.40, 0.85]),
            (crate::drugs::DrugType::Heroin, "Heroine", [0.75, 0.65, 0.30]),
            (crate::drugs::DrugType::Weed, "Weed", [0.30, 0.85, 0.40]),
        ];

        let tip = s.drugs.active_tips.iter().find(|t| match t {
            crate::drugs::MarketTip::Shortage { district, .. } => *district == dealer.district,
            crate::drugs::MarketTip::Surplus { district, .. } => *district == dealer.district,
        });

        rounded([px + 20., py + 82., pw - 40., 240.], 4.0, [0.06, 0.07, 0.09]);
        for (idx, (drug, name, color)) in drugs.iter().enumerate() {
            let ry = py + 88. + (idx as f32) * 38.;
            let selected = session.selected_drug_idx % 6 == idx;
            let row_col = if selected { [0.18, 0.22, 0.28] } else { [0.08, 0.09, 0.11] };
            rounded([px + 25., ry, pw - 50., 34.], 3.0, row_col);

            let unit_price = if buy_active {
                dealer.sell_price(*drug, tip)
            } else {
                dealer.buy_price(*drug, tip)
            };

            let in_bag = s.drugs.player_bag.get(*drug);
            let in_stock = dealer.stock.get(*drug);

            let is_discount = dealer.gang.specialty_sale() == *drug;
            let is_demand = dealer.gang.specialty_buy() == *drug;

            let cursor = if selected { "> " } else { "  " };
            let lbl = format!("{}{:<10}  |  Prix/u : ${}", cursor, name, unit_price);
            self.psp_text(px + 32., ry + 24., &lbl, 1.7, if selected { [1.0, 0.95, 0.30] } else { *color });

            if is_discount {
                rounded([px + 280., ry + 6., 120., 22.], 2.0, [0.15, 0.45, 0.22]);
                self.psp_text(px + 286., ry + 22., "AU RABAIS !", 1.4, [0.40, 1.0, 0.50]);
            } else if is_demand {
                rounded([px + 280., ry + 6., 140., 22.], 2.0, [0.45, 0.18, 0.18]);
                self.psp_text(px + 286., ry + 22., "FORTE DEMANDE !", 1.4, [1.0, 0.40, 0.40]);
            }

            let quant_str = format!("Sac : {:>2} u  |  Stock : {:>2} u", in_bag, in_stock);
            self.psp_text(px + pw - 240., ry + 24., &quant_str, 1.6, [0.80, 0.85, 0.90]);
        }

        // Transaction Summary Box
        rounded([px + 20., py + 330., pw - 40., 70.], 4.0, [0.09, 0.11, 0.14]);
        let sel_drug = session.selected_drug();
        let sel_price = if buy_active {
            dealer.sell_price(sel_drug, tip)
        } else {
            dealer.buy_price(sel_drug, tip)
        };
        let total_val = sel_price * session.quantity;
        let action_name = if buy_active { "COUT TOTAL" } else { "REVENU TOTAL" };
        let act_str = format!("QUANTITE : {} u  ([ GAUCHE/DROITE ])    |    {} : ${}", session.quantity, action_name, total_val);
        self.psp_text(px + 35., py + 360., &act_str, 1.8, [1.0, 0.92, 0.30]);

        if let Some(msg) = &session.message {
            self.psp_text(px + 35., py + 388., msg, 1.6, [0.35, 0.95, 0.45]);
        } else {
            let prompt = if buy_active { "[ ENTREE ] ACHETER AU DEALER" } else { "[ ENTREE ] VENDRE AU DEALER" };
            self.psp_text(px + 35., py + 388., prompt, 1.6, [0.85, 0.90, 0.95]);
        }

        // Footer hint
        rounded([px + 8., py + ph - 42., pw - 16., 34.], 4.0, [0.10, 0.12, 0.15]);
        self.psp_text(px + 16., py + ph - 34., "TAB: ACHAT/VENTE | HAUT/BAS: DROGUE | GAUCHE/DROITE: QUANTITE | ENTREE: VALIDER | ECHAP: QUITTER", 1.5, [0.80, 0.85, 0.88]);
    }
    pub fn set_fullscreen(&mut self, enable: bool) {
        if enable == self.fullscreen || self.window.is_null() {
            return;
        }
        const GWL_STYLE: i32 = -16;
        const WS_OVERLAPPEDWINDOW: isize = 0x00CF_0000;
        const WS_POPUP: isize = 0x8000_0000u32 as i32 as isize;
        const WS_VISIBLE: isize = 0x1000_0000;
        const SWP_NOZORDER: u32 = 0x0004;
        const SWP_FRAMECHANGED: u32 = 0x0020;
        const SWP_NOOWNERZORDER: u32 = 0x0200;
        unsafe {
            if enable {
                self.windowed_style = GetWindowLongPtrW(self.window, GWL_STYLE);
                GetWindowRect(self.window, &mut self.windowed_rect);
                let mut info = MonitorInfo {
                    size: std::mem::size_of::<MonitorInfo>() as u32,
                    ..MonitorInfo::default()
                };
                let monitor = MonitorFromWindow(self.window, 2);
                let bounds = if !monitor.is_null() && GetMonitorInfoW(monitor, &mut info) != 0 {
                    info.monitor
                } else {
                    Rect {
                        left: 0,
                        top: 0,
                        right: GetSystemMetrics(0).max(W as i32),
                        bottom: GetSystemMetrics(1).max(H as i32),
                    }
                };
                SetWindowLongPtrW(
                    self.window,
                    GWL_STYLE,
                    (self.windowed_style & !WS_OVERLAPPEDWINDOW) | WS_POPUP | WS_VISIBLE,
                );
                SetWindowPos(
                    self.window,
                    null_mut(),
                    bounds.left,
                    bounds.top,
                    (bounds.right - bounds.left).max(1),
                    (bounds.bottom - bounds.top).max(1),
                    SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
                );
                self.fullscreen = true;
            } else {
                let style = if self.windowed_style != 0 {
                    self.windowed_style
                } else {
                    WS_OVERLAPPEDWINDOW
                };
                SetWindowLongPtrW(self.window, GWL_STYLE, style | WS_VISIBLE);
                let r = self.windowed_rect;
                let w = (r.right - r.left).max(W as i32);
                let h = (r.bottom - r.top).max(H as i32);
                SetWindowPos(
                    self.window,
                    null_mut(),
                    r.left,
                    r.top,
                    w,
                    h,
                    SWP_NOOWNERZORDER | SWP_NOZORDER | SWP_FRAMECHANGED,
                );
                self.fullscreen = false;
            }
        }
    }
    pub fn mouse_pos(&self) -> Option<[f32; 2]> {
        if self.window.is_null() {
            return None;
        }
        unsafe {
            let mut pt = Point::default();
            let mut rect = Rect::default();
            if GetCursorPos(&mut pt) == 0
                || ScreenToClient(self.window, &mut pt) == 0
                || GetClientRect(self.window, &mut rect) == 0
            {
                return None;
            }
            let cw = (rect.right - rect.left).max(1);
            let ch = (rect.bottom - rect.top).max(1);
            if pt.x < 0 || pt.y < 0 || pt.x >= cw || pt.y >= ch {
                return None;
            }
            Some([
                pt.x as f32 * W as f32 / cw as f32,
                pt.y as f32 * H as f32 / ch as f32,
            ])
        }
    }
    pub fn present(&self) {
        unsafe {
            SwapBuffers(self.dc);
        }
    }
    pub fn capture(&self, path: &Path) -> Result<()> {
        let mut rect = Rect::default();
        unsafe {
            GetClientRect(self.window, &mut rect);
        }
        let cw = (rect.right - rect.left).max(1) as usize;
        let ch = (rect.bottom - rect.top).max(1) as usize;
        let mut rgb = vec![0u8; cw * ch * 3];
        unsafe {
            glReadBuffer(0x0405);
            glReadPixels(
                0,
                0,
                cw as i32,
                ch as i32,
                0x1907,
                0x1401,
                rgb.as_mut_ptr().cast(),
            );
        }
        let mut pixels = vec![0; cw * ch];
        for y in 0..ch {
            for x in 0..cw {
                let p = ((ch - 1 - y) * cw + x) * 3;
                pixels[y * cw + x] =
                    (rgb[p] as u32) << 16 | (rgb[p + 1] as u32) << 8 | rgb[p + 2] as u32;
            }
        }
        crate::ui::bmp_sized(path, &pixels, cw, ch)
    }
}
fn distance2(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}
unsafe fn shadow(p: [f32; 3], radii: [f32; 2], angle: f32, opacity: f32) {
    glColor4f(0.015, 0.02, 0.025, opacity);
    let (s, c) = angle.sin_cos();
    glBegin(6);
    glVertex3f(p[0], p[1], p[2] + 0.035);
    for k in 0..=24 {
        let a = k as f32 * std::f32::consts::TAU / 24.;
        let (x, y) = (a.cos() * radii[0], a.sin() * radii[1]);
        glVertex3f(p[0] + x * c - y * s, p[1] + x * s + y * c, p[2] + 0.035);
    }
    glEnd();
}
unsafe fn impact(p: [f32; 3], flash: f32) {
    let r = 0.7 + flash.min(0.5);
    glLineWidth(3.);
    glColor3f(1., 0.81, 0.34);
    glBegin(1);
    for k in 0..8 {
        let a = k as f32 * std::f32::consts::TAU / 8.;
        for t in [0.35, 1.] {
            glVertex3f(p[0] + a.cos() * r * t, p[1] + a.sin() * r * t, p[2] + 1.5);
        }
    }
    glEnd();
    glLineWidth(1.);
}
fn distance_label(m: f32) -> String {
    if m >= 1000. {
        format!("{:.2} KM", m / 1000.)
    } else {
        format!("{:.0} M", m)
    }
}
unsafe fn ui_sprite(
    texture: u32,
    size: [f32; 2],
    source: [f32; 4],
    target: [f32; 4],
    color: [f32; 4],
) {
    glEnable(0x0de1);
    glEnable(0x0be2);
    glDisable(0x0bc0);
    glBindTexture(0x0de1, texture);
    glColor4f(color[0], color[1], color[2], color[3]);
    let [x, y, w, h] = target;
    let [u, v, uw, vh] = source;
    glBegin(7);
    for (px, py, tx, ty) in [
        (x, y, u, v),
        (x + w, y, u + uw, v),
        (x + w, y + h, u + uw, v + vh),
        (x, y + h, u, v + vh),
    ] {
        glTexCoord2f(tx / size[0], ty / size[1]);
        glVertex2f(px, py);
    }
    glEnd();
    glDisable(0x0de1);
    glDisable(0x0be2);
}
/// Draw the annulus from the same quantized perimeter as the PSP map mask.
unsafe fn radar_arc(
    center: [f32; 2],
    scale: [f32; 2],
    steps: usize,
    mirror: bool,
    color: [f32; 4],
) {
    glEnable(0x0be2);
    glColor4f(color[0], color[1], color[2], color[3]);
    glBegin(5);
    for k in 0..=steps {
        let v = crate::radar::perimeter(k);
        for radius in [0.5, crate::radar::OUTER_SCALE] {
            glVertex2f(
                center[0] + v[0] * radius * scale[0] * if mirror { -1. } else { 1. },
                center[1] + v[1] * radius * scale[1],
            );
        }
    }
    glEnd();
    glDisable(0x0be2);
}
unsafe fn disc(x: f32, y: f32, r: f32, c: [f32; 3]) {
    glColor3f(c[0], c[1], c[2]);
    glBegin(6);
    glVertex2f(x, y);
    for k in 0..=96 {
        let a = k as f32 * std::f32::consts::TAU / 96.;
        glVertex2f(x + a.cos() * r, y + a.sin() * r);
    }
    glEnd();
}
unsafe fn ring(x: f32, y: f32, inner: f32, outer: f32, c: [f32; 3]) {
    glColor3f(c[0], c[1], c[2]);
    glBegin(5);
    for k in 0..=96 {
        let a = k as f32 * std::f32::consts::TAU / 96.;
        for r in [inner, outer] {
            glVertex2f(x + a.cos() * r, y + a.sin() * r);
        }
    }
    glEnd();
}
unsafe fn arc(x: f32, y: f32, inner: f32, outer: f32, start: f32, sweep: f32, c: [f32; 3]) {
    glColor3f(c[0], c[1], c[2]);
    glBegin(5);
    for k in 0..=96 {
        let a = start + sweep * k as f32 / 96.;
        for r in [inner, outer] {
            glVertex2f(x + a.cos() * r, y + a.sin() * r);
        }
    }
    glEnd();
}
unsafe fn outlined(x: f32, y: f32, text: &str, size: f32, color: [f32; 3]) {
    for (dx, dy) in [(-1., 0.), (1., 0.), (0., -1.), (0., 1.), (1., 2.)] {
        label(x + dx, y + dy, text, size, [0.025, 0.025, 0.02]);
    }
    label(x, y, text, size, color);
}
unsafe fn stroke(a: [f32; 2], b: [f32; 2], width: f32, c: [f32; 3]) {
    glLineWidth(width);
    glColor3f(c[0], c[1], c[2]);
    glBegin(1);
    glVertex2f(a[0], a[1]);
    glVertex2f(b[0], b[1]);
    glEnd();
}
unsafe fn rounded(r: [f32; 4], radius: f32, c: [f32; 3]) {
    let [x, y, w, h] = r;
    panel(x + radius, y, w - 2. * radius, h, c);
    panel(x, y + radius, w, h - 2. * radius, c);
    for (cx, cy) in [
        (x + radius, y + radius),
        (x + w - radius, y + radius),
        (x + radius, y + h - radius),
        (x + w - radius, y + h - radius),
    ] {
        disc(cx, cy, radius, c);
    }
}
unsafe fn draw_star(cx: f32, cy: f32, r_outer: f32, color: [f32; 4]) {
    let r_inner = r_outer * 0.382;
    glColor4f(color[0], color[1], color[2], color[3]);
    glBegin(6); // GL_TRIANGLE_FAN
    glVertex2f(cx, cy);
    for i in 0..=10 {
        let angle = -std::f32::consts::FRAC_PI_2 + (i as f32) * (std::f32::consts::TAU / 10.0);
        let r = if i % 2 == 0 { r_outer } else { r_inner };
        glVertex2f(cx + angle.cos() * r, cy + angle.sin() * r);
    }
    glEnd();
}
unsafe fn draw_takedown_badge(x: f32, y: f32, done: bool, ticks: usize) {
    if done {
        rounded([x, y, 16., 10.], 2., [0.2, 0.2, 0.25]);
        glColor4f(0.95, 0.15, 0.15, 1.0);
        glLineWidth(2.0);
        glBegin(1); // GL_LINES
        glVertex2f(x + 2., y + 2.);
        glVertex2f(x + 14., y + 8.);
        glVertex2f(x + 14., y + 2.);
        glVertex2f(x + 2., y + 8.);
        glEnd();
        glLineWidth(1.0);
    } else {
        let strobe_red = (ticks / 8) % 2 == 0;
        rounded([x, y, 16., 10.], 2., [0.15, 0.25, 0.45]);
        glColor4f(
            if strobe_red { 1.0 } else { 0.1 },
            0.1,
            if strobe_red { 0.1 } else { 1.0 },
            1.0,
        );
        glBegin(7); // GL_QUADS
        glVertex2f(x + 6., y + 1.);
        glVertex2f(x + 10., y + 1.);
        glVertex2f(x + 10., y + 3.);
        glVertex2f(x + 6., y + 3.);
        glEnd();
    }
}
unsafe fn pair(x: f32, y: f32, pc: &str, xbox: &str) {
    let (pc, xbox) = (crate::locale::text(pc), crate::locale::text(xbox));
    let pw = pc.len() as f32 * 6. + 10.;
    let xw = xbox.len() as f32 * 6. + 10.;
    rounded([x, y, pw, 11.], 2., [0.10, 0.17, 0.23]);
    label(x + 5., y + 2., &pc, 1., [0.70, 0.85, 0.98]);
    rounded([x + pw + 4., y, xw, 11.], 2., [0.12, 0.21, 0.14]);
    label(x + pw + 9., y + 2., &xbox, 1., [0.65, 0.94, 0.65]);
}
unsafe fn hud_button(r: [f32; 4], title: &str, pc: &str, xbox: &str, on: bool) {
    rounded(
        r,
        5.,
        if on {
            [0.09, 0.18, 0.17]
        } else {
            [0.07, 0.11, 0.13]
        },
    );
    label(r[0] + 10., r[1] + 6., title, 1., [0.88, 0.92, 0.91]);
    pair(
        r[0] + 10.,
        r[1] + if r[3] < 32. { 16. } else { 20. },
        pc,
        xbox,
    );
    if on {
        disc(r[0] + r[2] - 13., r[1] + 11., 3., [0.37, 0.89, 0.64]);
    }
}
unsafe fn map_line(
    view: crate::gps::MapView,
    a: [f32; 2],
    b: [f32; 2],
    width: f32,
    color: [f32; 3],
) {
    if let Some((a, b)) = view.clip(a, b) {
        glLineWidth(width);
        glColor3f(color[0], color[1], color[2]);
        glBegin(1);
        glVertex2f(a[0], a[1]);
        glVertex2f(b[0], b[1]);
        glEnd();
    }
}
unsafe fn panel(x: f32, y: f32, w: f32, h: f32, c: [f32; 3]) {
    glColor3f(c[0], c[1], c[2]);
    glBegin(7);
    for (a, b) in [(x, y), (x + w, y), (x + w, y + h), (x, y + h)] {
        glVertex2f(a, b);
    }
    glEnd();
}
unsafe fn label(x: f32, y: f32, s: &str, size: f32, c: [f32; 3]) {
    let translated = crate::locale::text(s);
    let s = translated.as_str();
    let (x, y) = (x.round(), y.round());
    let text = if s == "by Benji Paname" {
        s.to_owned()
    } else {
        s.to_ascii_uppercase()
    };
    for (i, ch) in text.chars().enumerate() {
        for (row, bits) in crate::ui::glyph(ch).iter().enumerate() {
            for bit in 0..5 {
                if bits & (1 << (4 - bit)) != 0 {
                    panel(
                        x + (i * 6 + bit) as f32 * size,
                        y + row as f32 * size,
                        size,
                        size,
                        c,
                    );
                }
            }
        }
    }
}
unsafe fn upload_texture(t: &crate::texture::Texture, repeat: bool) -> Result<u32> {
    let (size, pixels) = t
        .high_res
        .as_ref()
        .map(|r| (r.size, r.rgba.as_slice()))
        .unwrap_or(([t.info.width, t.info.height], t.rgba.as_slice()));
    let mut tex = 0;
    glGenTextures(1, &mut tex);
    if tex == 0 {
        return Err("OpenGL texture allocation".into());
    }
    glBindTexture(0x0de1, tex);
    // Point sampling for the tightly packed low-resolution pedestrian atlases:
    // linear filtering blends colour-key pixels and neighbouring sprite frames.
    let filter = if repeat { 0x2601 } else { 0x2600 };
    glTexParameteri(0x0de1, 0x2801, filter);
    glTexParameteri(0x0de1, 0x2800, filter);
    glTexParameteri(0x0de1, 0x2802, if repeat { 0x2901 } else { 0x812f });
    glTexParameteri(0x0de1, 0x2803, if repeat { 0x2901 } else { 0x812f });
    glTexImage2D(
        0x0de1,
        0,
        0x1908,
        size[0] as i32,
        size[1] as i32,
        0,
        0x1908,
        0x1401,
        pixels.as_ptr().cast(),
    );
    Ok(tex)
}
unsafe fn cube(p: [f32; 3], h: [f32; 3], color: [f32; 3]) {
    let corners: [[f32; 3]; 8] = std::array::from_fn(|k| {
        std::array::from_fn(|a| p[a] + h[a] * if k & (1 << a) == 0 { -1. } else { 1. })
    });
    glBegin(7);
    for (fi, face) in [
        [0, 1, 3, 2],
        [4, 6, 7, 5],
        [0, 4, 5, 1],
        [2, 3, 7, 6],
        [0, 2, 6, 4],
        [1, 5, 7, 3],
    ]
    .into_iter()
    .enumerate()
    {
        let light = 0.7 + fi as f32 * 0.05;
        glColor3f(color[0] * light, color[1] * light, color[2] * light);
        for i in face {
            let v = corners[i];
            glVertex3f(v[0], v[1], v[2]);
        }
    }
    glEnd();
}
impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            if !self.context.is_null() {
                wglMakeCurrent(self.dc, self.context);
                if let Some(grade) = &mut self.grade {
                    grade.destroy();
                }
                self.clear_assets();
                wglMakeCurrent(null_mut(), null_mut());
                wglDeleteContext(self.context);
            }
            if !self.dc.is_null() {
                ReleaseDC(self.window, self.dc);
            }
        }
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Default)]
struct MonitorInfo {
    size: u32,
    monitor: Rect,
    work: Rect,
    flags: u32,
}

#[link(name = "user32")]
extern "system" {
    fn GetDC(window: Handle) -> Handle;
    fn ReleaseDC(window: Handle, dc: Handle) -> i32;
    fn GetClientRect(window: Handle, rect: *mut Rect) -> i32;
    fn GetWindowRect(window: Handle, rect: *mut Rect) -> i32;
    fn GetWindowLongPtrW(window: Handle, index: i32) -> isize;
    fn SetWindowLongPtrW(window: Handle, index: i32, new_long: isize) -> isize;
    fn SetWindowPos(
        window: Handle,
        insert_after: Handle,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> i32;
    fn MonitorFromWindow(window: Handle, flags: u32) -> Handle;
    fn GetMonitorInfoW(monitor: Handle, info: *mut MonitorInfo) -> i32;
    fn GetSystemMetrics(index: i32) -> i32;
    fn GetCursorPos(point: *mut Point) -> i32;
    fn ScreenToClient(window: Handle, point: *mut Point) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn ChoosePixelFormat(dc: Handle, descriptor: *const PixelFormat) -> i32;
    fn GetPixelFormat(dc: Handle) -> i32;
    fn SetPixelFormat(dc: Handle, format: i32, descriptor: *const PixelFormat) -> i32;
    fn SwapBuffers(dc: Handle) -> i32;
}

#[link(name = "opengl32")]
extern "system" {
    fn wglCreateContext(dc: Handle) -> Handle;
    fn wglDeleteContext(context: Handle) -> i32;
    fn wglMakeCurrent(dc: Handle, context: Handle) -> i32;
    fn wglGetProcAddress(name: *const c_char) -> *const c_void;
    fn glGetString(name: u32) -> *const u8;
    fn glGetError() -> u32;
    fn glGetFloatv(name: u32, values: *mut f32);
    fn glViewport(x: i32, y: i32, width: i32, height: i32);
    fn glClearColor(r: f32, g: f32, b: f32, a: f32);
    fn glClear(mask: u32);
    fn glEnable(capability: u32);
    fn glDisable(capability: u32);
    fn glDepthFunc(function: u32);
    fn glDepthMask(enabled: u8);
    fn glAlphaFunc(function: u32, reference: f32);
    fn glBlendFunc(source: u32, destination: u32);
    fn glShadeModel(mode: u32);
    fn glHint(target: u32, mode: u32);
    fn glFogf(name: u32, value: f32);
    fn glFogfv(name: u32, value: *const f32);
    fn glMatrixMode(mode: u32);
    fn glLoadIdentity();
    fn glLoadMatrixf(matrix: *const f32);
    fn glMultMatrixf(matrix: *const f32);
    fn glPushMatrix();
    fn glPopMatrix();
    fn glFrustum(left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64);
    fn glOrtho(left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64);
    fn glLineWidth(width: f32);
    fn glBegin(mode: u32);
    fn glEnd();
    fn glColor3f(r: f32, g: f32, b: f32);
    fn glColor4f(r: f32, g: f32, b: f32, a: f32);
    fn glTexCoord2f(u: f32, v: f32);
    fn glTranslatef(x: f32, y: f32, z: f32);
    fn glScalef(x: f32, y: f32, z: f32);
    fn glRotatef(angle: f32, x: f32, y: f32, z: f32);
    fn glVertex2f(x: f32, y: f32);
    fn glVertex3f(x: f32, y: f32, z: f32);
    fn glGenTextures(count: i32, textures: *mut u32);
    fn glDeleteTextures(count: i32, textures: *const u32);
    fn glBindTexture(target: u32, texture: u32);
    fn glTexParameteri(target: u32, name: u32, value: i32);
    fn glTexParameterf(target: u32, name: u32, value: f32);
    fn glTexEnvi(target: u32, name: u32, value: i32);
    fn glPixelStorei(name: u32, value: i32);
    fn glTexImage2D(
        target: u32,
        level: i32,
        internal_format: i32,
        width: i32,
        height: i32,
        border: i32,
        format: u32,
        data_type: u32,
        pixels: *const c_void,
    );
    fn glGenLists(count: i32) -> u32;
    fn glDeleteLists(first: u32, count: i32);
    fn glNewList(list: u32, mode: u32);
    fn glEndList();
    fn glCallList(list: u32);
    fn glReadBuffer(buffer: u32);
    fn glCopyTexImage2D(
        target: u32,
        level: i32,
        internal_format: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        border: i32,
    );
    fn glCopyTexSubImage2D(
        target: u32,
        level: i32,
        xoffset: i32,
        yoffset: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    );
    fn glReadPixels(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        format: u32,
        data_type: u32,
        pixels: *mut c_void,
    );
}
