// SPDX-License-Identifier: GPL-3.0-only
//! Optional compatibility-profile GLSL colour pass. Missing shader support leaves
//! the normal renderer intact. The HUD is drawn after this pass for crisp text.
use std::ffi::{c_char, c_void, CStr};

type CreateShader = unsafe extern "system" fn(u32) -> u32;
type ShaderSource = unsafe extern "system" fn(u32, i32, *const *const c_char, *const i32);
type CompileShader = unsafe extern "system" fn(u32);
type GetShaderiv = unsafe extern "system" fn(u32, u32, *mut i32);
type GetShaderInfoLog = unsafe extern "system" fn(u32, i32, *mut i32, *mut c_char);
type DeleteShader = unsafe extern "system" fn(u32);
type CreateProgram = unsafe extern "system" fn() -> u32;
type AttachShader = unsafe extern "system" fn(u32, u32);
type LinkProgram = unsafe extern "system" fn(u32);
type GetProgramiv = unsafe extern "system" fn(u32, u32, *mut i32);
type GetProgramInfoLog = unsafe extern "system" fn(u32, i32, *mut i32, *mut c_char);
type UseProgram = unsafe extern "system" fn(u32);
type DeleteProgram = unsafe extern "system" fn(u32);

pub struct Grade {
    program: u32,
    texture: u32,
    use_program: UseProgram,
    delete_program: DeleteProgram,
    size: [i32; 2],
}
impl Grade {
    pub unsafe fn new() -> Result<Self, String> {
        macro_rules! load {
            ($name:literal, $ty:ty) => {{
                let p = super::wglGetProcAddress(concat!($name, "\0").as_ptr().cast());
                let address = p as usize;
                if address <= 3 || address == usize::MAX {
                    return Err(format!("GLSL unavailable: {}", $name));
                }
                std::mem::transmute::<*const c_void, $ty>(p)
            }};
        }
        let create_shader = load!("glCreateShader", CreateShader);
        let shader_source = load!("glShaderSource", ShaderSource);
        let compile_shader = load!("glCompileShader", CompileShader);
        let get_shader_iv = load!("glGetShaderiv", GetShaderiv);
        let get_shader_log = load!("glGetShaderInfoLog", GetShaderInfoLog);
        let delete_shader = load!("glDeleteShader", DeleteShader);
        let create_program = load!("glCreateProgram", CreateProgram);
        let attach_shader = load!("glAttachShader", AttachShader);
        let link_program = load!("glLinkProgram", LinkProgram);
        let get_program_iv = load!("glGetProgramiv", GetProgramiv);
        let get_program_log = load!("glGetProgramInfoLog", GetProgramInfoLog);
        let use_program = load!("glUseProgram", UseProgram);
        let delete_program = load!("glDeleteProgram", DeleteProgram);
        let mut compiled = Vec::new();
        for (kind, source) in [
            (0x8b31, "#version 120\nvarying vec2 uv;\nvoid main(){gl_Position=ftransform();uv=gl_MultiTexCoord0.xy;}\n\0"),
            (0x8b30, "#version 120\nuniform sampler2D frame;\nvarying vec2 uv;\nvoid main(){\n vec3 c=texture2D(frame,uv).rgb;\n float l=dot(c,vec3(0.2126,0.7152,0.0722));\n c=mix(vec3(l),c,1.10);\n c=(c-0.5)*1.045+0.5;\n c*=vec3(1.035,1.015,0.975);\n vec2 v=(uv-0.5)*vec2(1.0,0.85);\n c*=1.0-0.24*smoothstep(0.12,0.58,dot(v,v));\n gl_FragColor=vec4(clamp(c,0.0,1.0),1.0);\n}\n\0"),
        ] {
            let shader = create_shader(kind);
            if shader == 0 {
                for s in compiled { delete_shader(s); }
                return Err("GLSL shader allocation failed".into());
            }
            let p = source.as_ptr().cast();
            shader_source(shader, 1, &p, std::ptr::null());
            compile_shader(shader);
            let mut status = 0;
            get_shader_iv(shader, 0x8b81, &mut status);
            if status == 0 {
                let mut log = [0i8; 1024];
                get_shader_log(shader, 1023, std::ptr::null_mut(), log.as_mut_ptr());
                delete_shader(shader);
                for s in compiled { delete_shader(s); }
                return Err(format!("GLSL compile: {}", CStr::from_ptr(log.as_ptr()).to_string_lossy()));
            }
            compiled.push(shader);
        }
        let program = create_program();
        if program == 0 {
            for s in compiled {
                delete_shader(s);
            }
            return Err("GLSL program allocation failed".into());
        }
        for &shader in &compiled {
            attach_shader(program, shader);
        }
        link_program(program);
        for shader in compiled {
            delete_shader(shader);
        }
        let mut status = 0;
        get_program_iv(program, 0x8b82, &mut status);
        if status == 0 {
            let mut log = [0i8; 1024];
            get_program_log(program, 1023, std::ptr::null_mut(), log.as_mut_ptr());
            delete_program(program);
            return Err(format!(
                "GLSL link: {}",
                CStr::from_ptr(log.as_ptr()).to_string_lossy()
            ));
        }
        let mut texture = 0;
        super::glGenTextures(1, &mut texture);
        if texture == 0 {
            delete_program(program);
            return Err("GLSL screen texture allocation failed".into());
        }
        super::glBindTexture(0x0de1, texture);
        for param in [0x2800, 0x2801] {
            super::glTexParameteri(0x0de1, param, 0x2601);
        }
        for param in [0x2802, 0x2803] {
            super::glTexParameteri(0x0de1, param, 0x812f);
        }
        Ok(Self {
            program,
            texture,
            use_program,
            delete_program,
            size: [0, 0],
        })
    }
    pub unsafe fn draw(&mut self, width: i32, height: i32) {
        super::glDisable(0x0b71);
        super::glDisable(0x0b44);
        super::glDisable(0x0bc0);
        super::glDisable(0x0be2);
        super::glEnable(0x0de1);
        super::glBindTexture(0x0de1, self.texture);
        super::glReadBuffer(0x0405);
        if self.size != [width, height] {
            super::glCopyTexImage2D(0x0de1, 0, 0x1907, 0, 0, width, height, 0);
            self.size = [width, height];
        } else {
            super::glCopyTexSubImage2D(0x0de1, 0, 0, 0, 0, 0, width, height);
        }
        super::glMatrixMode(0x1701);
        super::glLoadIdentity();
        super::glMatrixMode(0x1700);
        super::glLoadIdentity();
        (self.use_program)(self.program);
        super::glBegin(7);
        for (x, y, u, v) in [
            (-1., -1., 0., 0.),
            (1., -1., 1., 0.),
            (1., 1., 1., 1.),
            (-1., 1., 0., 1.),
        ] {
            super::glTexCoord2f(u, v);
            super::glVertex2f(x, y);
        }
        super::glEnd();
        (self.use_program)(0);
        super::glDisable(0x0de1);
    }
    pub unsafe fn destroy(&mut self) {
        (self.delete_program)(self.program);
        super::glDeleteTextures(1, &self.texture);
        self.program = 0;
        self.texture = 0;
    }
}
