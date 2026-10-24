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
            glColor3f(0.12, 0.26, 0.31);
            glBegin(7);
            for (x, y) in [
                (-3500., -2500.),
                (3500., -2500.),
                (3500., 2500.),
                (-3500., 2500.),
            ] {
                glVertex3f(x, y, -7.5);
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
                        };
                        let ox = 24.;
                        let oy = 60. + top;
                        rounded([ox - 6., oy - 4., desc.len() as f32 * 13. + 20., 32.], 3., [0.08, 0.10, 0.14]);
                        self.psp_text(ox, oy, desc, 1.8, [0.98, 0.92, 0.72]);
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
