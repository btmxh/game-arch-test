use std::{borrow::Cow, ptr::null};

use gl::types::GLuint;
use glutin::prelude::GlConfig;
use winit::dpi::PhysicalSize;

use crate::{
    exec::{dispatch::ReturnMechanism, executor::GameServerExecutor, server::draw},
    utils::enclose::enclose,
};

use super::wrappers::{
    framebuffer::FramebufferHandle, shader::ProgramHandle, texture::TextureHandle,
    vertex_array::VertexArrayHandle,
};

pub fn generate_gaussian_kernel<const N: usize>(sigma: f32) -> [f32; N] {
    let mut arr = [0.0; N];
    if sigma < 1e-3 {
        arr[0] = 1.0;
    } else {
        let inv_sqrt2pi = 0.39894;
        (0..N).for_each(|i| {
            let x = (i as f32) / sigma;
            arr[i] = inv_sqrt2pi * (-0.5 * x * x).exp() / sigma
        });
    }
    tracing::info!("{}", arr.iter().sum::<f32>());
    tracing::info!("{}", arr.iter().skip(1).sum::<f32>());
    arr
}

// https://github.com/ppy/osu/blob/b35e796d75153b53c1e450f407a48032aef1eddf/osu.Game/Graphics/Backgrounds/Background.cs
pub fn calc_blur_framebuffer_scale(sigma: f32) -> f32 {
    if sigma <= 1.0 {
        return 1.0;
    }

    let scale = -0.18 * (0.004 * sigma).ln();
    const STEP: f32 = 0.02;
    (scale / STEP).round() * STEP
}

mod shader {
    pub const VERTEX: &str = r#"
    #version 300 es
    out vec2 tex_coords;
    const vec2 positions[4] = vec2[](
        vec2(-1.0, 1.0), vec2(1.0, 1.0),
        vec2(-1.0, -1.0), vec2(1.0, -1.0)
    );
    void main() {
        vec2 pos = positions[gl_VertexID];
        gl_Position = vec4(pos, 0.0, 1.0);
        tex_coords = (pos + vec2(1.0)) * vec2(0.5);
    }
    "#;

    pub const FRAGMENT: &str = r#"
    #version 300 es
    precision mediump float;
    in vec2 tex_coords;
    out vec4 color;
    uniform sampler2D tex;
    uniform vec2 pixel;
    uniform float sigma;
    const float epsilon = 1e-3;
    float gauss(float x, float sigma) {
        return 0.39894 * exp(-0.5*x*x/(sigma*sigma)) / sigma;
    }
    void main() {
        float factor = gauss(0.0, sigma);
        float total_factor = factor;
        color = texture(tex, tex_coords) * factor;
        for(int i = 1; i < 100; i++) {
            float x = float(i) * 2.0 - 0.5;
            factor = gauss(x, sigma) * 2.0;
            total_factor += factor * 2.0;
            color += texture(tex, tex_coords + x * pixel) * factor;
            color += texture(tex, tex_coords - x * pixel) * factor;
        }
        color /= total_factor;
    }"#;
}

#[derive(Clone)]
pub struct TexturedFramebuffer {
    pub framebuffer: FramebufferHandle,
    pub texture: TextureHandle,
}

#[derive(Clone)]
pub struct BlurRenderer {
    vertex_array: VertexArrayHandle,
    program: ProgramHandle,
    pub framebuffers: [TexturedFramebuffer; 2],
    framebuffer_size: Option<PhysicalSize<f32>>,
}

impl BlurRenderer {
    fn zero_range_two() -> [usize; 2] {
        [0, 1]
    }

    #[allow(unused_mut)]
    pub fn new(
        executor: &mut GameServerExecutor,
        dummy_vao: VertexArrayHandle,
        draw: &mut draw::ServerChannel,
    ) -> anyhow::Result<Self> {
        let program = ProgramHandle::new(draw);
        let framebuffers = Self::zero_range_two().map(|_| FramebufferHandle::new(draw));
        let textures = Self::zero_range_two().map(|_| TextureHandle::new(draw));

        executor.execute_draw(
            draw,
            Some(ReturnMechanism::Sync),
            enclose!((framebuffers, program) move |s| {
                s.handles.create_vf_program(
                    "blur shader program",
                    program,
                    shader::VERTEX,
                    shader::FRAGMENT,
                )?;
                for (i, framebuffer) in framebuffers.iter().enumerate() {
                    s.handles.create_framebuffer(
                        format!("blur framebuffer {}", i),
                        framebuffer.clone(),
                    )?;
                }
                Ok(Box::new(()))
            }),
        )?;

        Ok(Self {
            vertex_array: dummy_vao,
            program,
            framebuffers: Self::zero_range_two().map(|i| TexturedFramebuffer {
                framebuffer: framebuffers[i].clone(),
                texture: textures[i].clone(),
            }),
            framebuffer_size: None,
        })
    }

    fn create_texture_attachment(
        server: &mut draw::Server,
        name: impl Into<Cow<'static, str>>,
        framebuffer_size: PhysicalSize<u32>,
        handle: TextureHandle,
    ) -> anyhow::Result<()> {
        let texture = server.handles.create_texture(name, handle)?;
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                if server.gl_config.srgb_capable() {
                    gl::SRGB8_ALPHA8.try_into().unwrap()
                } else {
                    gl::RGBA8.try_into().unwrap()
                },
                framebuffer_size.width.try_into().unwrap(),
                framebuffer_size.height.try_into().unwrap(),
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                null(),
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                gl::LINEAR.try_into().unwrap(),
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                gl::LINEAR.try_into().unwrap(),
            );
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                texture,
                0,
            );
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
        Ok(())
    }

    pub fn redraw(
        &mut self,
        server: &mut draw::Server,
        window_size: PhysicalSize<u32>,
        texture: GLuint,
        lod: f32,
        blur_sigma: f32,
    ) -> anyhow::Result<()> {
        let framebuffers = self
            .framebuffers
            .iter()
            .map(|f| f.framebuffer.get(server).unwrap())
            .collect::<Vec<_>>();
        let downscale = calc_blur_framebuffer_scale(blur_sigma);
        let framebuffer_size = PhysicalSize {
            width: window_size.width as f32 * downscale,
            height: window_size.height as f32 * downscale,
        };
        let blur_sigma = blur_sigma * downscale;
        let program = self.program.get(server).unwrap();
        let vertex_array = self.vertex_array.get(server).unwrap();
        if self.framebuffer_size.is_none()
            || ((self.framebuffer_size.unwrap().width - framebuffer_size.width).abs() < 1e-3
                && (self.framebuffer_size.unwrap().height - framebuffer_size.height).abs() < 1e-3)
        {
            self.framebuffer_size = Some(framebuffer_size);
            let textures = self
                .framebuffers
                .iter()
                .map(|f| server.handles.textures.remove(&f.texture.0.handle))
                .collect::<Vec<_>>();
            for (i, f) in self.framebuffers.iter().enumerate() {
                unsafe { gl::BindFramebuffer(gl::FRAMEBUFFER, *framebuffers[i]) };
                Self::create_texture_attachment(
                    server,
                    textures[i]
                        .as_ref()
                        .map(|t| t.name())
                        .unwrap_or_else(|| format!("blur framebuffer {} texture", i).into()),
                    framebuffer_size.cast::<u32>(),
                    f.texture.clone(),
                )?;
                unsafe { gl::BindFramebuffer(gl::FRAMEBUFFER, 0) };
            }
        }
        let textures = self
            .framebuffers
            .iter()
            .map(|f| f.texture.get(server).unwrap())
            .collect::<Vec<_>>();

        unsafe {
            gl::UseProgram(*program);
            gl::BindVertexArray(*vertex_array);
            gl::Uniform1f(
                gl::GetUniformLocation(*program, "sigma\0".as_ptr() as *const _),
                blur_sigma,
            );
            gl::Uniform1i(
                gl::GetUniformLocation(*program, "tex\0".as_ptr() as *const _),
                0,
            );
            let loc_pixel = gl::GetUniformLocation(*program, "pixel\0".as_ptr() as *const _);
            let loc_lod = gl::GetUniformLocation(*program, "lod\0".as_ptr() as *const _);
            gl::Uniform2f(loc_pixel, 1.0 / framebuffer_size.width, 0.0);
            gl::Uniform1f(loc_lod, lod);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::BindFramebuffer(gl::FRAMEBUFFER, *framebuffers[0]);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::Viewport(
                0,
                0,
                framebuffer_size.width as _,
                framebuffer_size.height as _,
            );
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            gl::Uniform2f(loc_pixel, 0.0, 1.0 / framebuffer_size.height);
            gl::Uniform1f(loc_lod, 0.0);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, *textures[0]);
            gl::BindFramebuffer(gl::FRAMEBUFFER, *framebuffers[1]);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::Viewport(
                0,
                0,
                framebuffer_size.width as _,
                framebuffer_size.height as _,
            );
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        };
        Ok(())
    }

    pub fn output_texture_handle(&self) -> TextureHandle {
        self.framebuffers[1].texture.clone()
    }
}
