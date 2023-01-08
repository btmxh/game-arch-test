use winit::dpi::PhysicalSize;

use crate::exec::{dispatch::ReturnMechanism, executor::GameServerExecutor, server::draw};

use super::wrappers::{
    framebuffer::{DefaultTextureFramebuffer, FramebufferHandle},
    shader::ProgramHandle,
    texture::TextureHandle,
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
    pub framebuffers: [DefaultTextureFramebuffer; 2],
}

impl BlurRenderer {
    #[allow(unused_mut)]
    pub fn new(
        executor: &mut GameServerExecutor,
        dummy_vao: VertexArrayHandle,
        draw: &mut draw::ServerChannel,
    ) -> anyhow::Result<Self> {
        let program = ProgramHandle::new_vf(
            executor,
            draw,
            "blur shader program",
            Some(ReturnMechanism::Sync),
            shader::VERTEX,
            shader::FRAGMENT,
        )?;
        let framebuffer_0 = DefaultTextureFramebuffer::new(executor, draw, "blur framebuffer 0")?;
        let framebuffer_1 = DefaultTextureFramebuffer::new(executor, draw, "blur framebuffer 0")?;
        let framebuffers = [framebuffer_0, framebuffer_1];
        // unstable lol
        // let framebuffers = Self::zero_range_two().try_map(|i| {
        //     DefaultTextureFramebuffer::new(executor, draw, format!("blur framebuffer {i}"))
        // })?;

        Ok(Self {
            vertex_array: dummy_vao,
            program,
            framebuffers,
        })
    }

    pub fn redraw(
        &mut self,
        executor: &mut GameServerExecutor,
        draw: &mut draw::ServerChannel,
        window_size: PhysicalSize<u32>,
        texture: TextureHandle,
        lod: f32,
        blur_sigma: f32,
    ) -> anyhow::Result<()> {
        let downscale = calc_blur_framebuffer_scale(blur_sigma);
        let framebuffer_size = PhysicalSize {
            width: (window_size.width as f32 * downscale) as u32,
            height: (window_size.height as f32 * downscale) as u32,
        };
        let blur_sigma = blur_sigma * downscale;
        for framebuffer in self.framebuffers.iter_mut() {
            framebuffer.resize(executor, draw, framebuffer_size)?;
        }

        let slf = self.clone();
        executor.execute_draw(draw, Some(ReturnMechanism::Sync), move |server| {
            let program = slf.program.get(server);
            let vertex_array = slf.vertex_array.get(server);
            let framebuffers = slf
                .framebuffers
                .iter()
                .map(|f| f.framebuffer.get(server))
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
                gl::Uniform2f(loc_pixel, 1.0 / framebuffer_size.width as f32, 0.0);
                gl::Uniform1f(loc_lod, lod);
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, *texture.get(server));
                gl::BindFramebuffer(gl::FRAMEBUFFER, *framebuffers[0]);
                gl::Clear(gl::COLOR_BUFFER_BIT);
                gl::Viewport(
                    0,
                    0,
                    framebuffer_size.width as _,
                    framebuffer_size.height as _,
                );
                gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
                gl::Uniform2f(loc_pixel, 0.0, 1.0 / framebuffer_size.height as f32);
                gl::Uniform1f(loc_lod, 0.0);
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, *slf.framebuffers[0].texture.get(server));
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
                gl::Viewport(
                    0,
                    0,
                    window_size.width.try_into().unwrap(),
                    window_size.height.try_into().unwrap(),
                );
            };
            Ok(Box::new(()))
        })?;
        Ok(())
    }

    pub fn output_texture_handle(&self) -> TextureHandle {
        self.framebuffers[1].texture.clone()
    }
}
