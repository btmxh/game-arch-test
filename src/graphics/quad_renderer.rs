use std::ffi::CStr;

use anyhow::Context;
use cgmath::{Matrix, Matrix3};
use gl::types::GLuint;

use crate::exec::server::draw;

use super::{
    context::DrawContext,
    wrappers::{shader::ProgramHandle, vertex_array::VertexArrayHandle},
    Vec2,
};

mod shader {
    pub const VERTEX: &str = r#"
    #version 300 es

    out vec2 vf_orig_pos;
    out vec2 vf_tex_coords;
    out vec2 vf_radius;
    out vec2 vf_pos_bounds[2];

    uniform vec2 pos_bounds[2];
    uniform vec2 radius;
    uniform vec2 tex_bounds[2];
    uniform mat3 transform;

    const vec2 mix_tex_coords[4] = vec2[](
        vec2(0.0, 1.0), vec2(1.0, 1.0),
        vec2(0.0, 0.0), vec2(1.0, 0.0)
    );

    void main() {
        vec2 expanded_pos_bounds[2] = vec2[](
            pos_bounds[0] - radius,
            pos_bounds[1] + radius
        );
        float x = expanded_pos_bounds[int(gl_VertexID % 2)].x;
        float y = expanded_pos_bounds[int(gl_VertexID < 2)].y;
        vf_orig_pos = vec2(x, y);
        vec3 pos = transform * vec3(vf_orig_pos, 1.0);
        gl_Position = vec4(pos.xy, 0.0, pos.z);
        vf_tex_coords = mix(tex_bounds[0], tex_bounds[1], mix_tex_coords[gl_VertexID]);
        vf_radius = radius;
        vf_pos_bounds[0] = pos_bounds[0];
        vf_pos_bounds[1] = pos_bounds[1];
    }
    "#;

    pub const FRAGMENT: &str = r#"
    #version 300 es
    precision mediump float;

    in vec2 vf_orig_pos;
    in vec2 vf_tex_coords;
    in vec2 vf_radius;
    in vec2 vf_pos_bounds[2];

    out vec4 color;

    uniform sampler2D tex;

    void main() {
        const float max_distance = 0.1;
        vec2 offset = clamp(vf_orig_pos, vf_pos_bounds[0], vf_pos_bounds[1]);
        // division could be replaced by some fancy math
        vec2 normalized_offset = offset / vf_radius;
        float distance = dot(normalized_offset, normalized_offset);
        float alpha = 1.0 - smoothstep(1.0, 1.0 + max_distance, distance);

        color = texture(tex, vf_tex_coords);
        color.a *= alpha;
    }
    "#;
}

#[derive(Clone)]
pub struct QuadRenderer {
    vertex_array: VertexArrayHandle,
    program: ProgramHandle,
}

impl QuadRenderer {
    pub const FULL_WINDOW_POS_BOUNDS: [Vec2; 2] = [Vec2::new(-1.0, 1.0), Vec2::new(1.0, -1.0)];

    pub fn new(
        dummy_vao: VertexArrayHandle,
        draw: &mut draw::ServerChannel,
    ) -> anyhow::Result<Self> {
        let program = ProgramHandle::new_vf(
            draw,
            "quad renderer shader program",
            shader::VERTEX,
            shader::FRAGMENT,
        )
        .context("quad renderer initialization (in draw server) failed")?;

        Ok(Self {
            vertex_array: dummy_vao,
            program,
        })
    }

    pub fn draw(
        &self,
        context: &DrawContext,
        texture: GLuint,
        pos_bounds: &[Vec2; 2],
        tex_bounds: &[Vec2; 2],
        radius: &Vec2,
        transform: &Matrix3<f32>,
    ) {
        let vao = self.vertex_array.get(context);
        let program = self.program.get(context);

        unsafe {
            vao.bind();
            gl::UseProgram(*program);

            gl::Uniform2fv(
                gl::GetUniformLocation(
                    *program,
                    CStr::from_bytes_with_nul_unchecked("pos_bounds\0".as_bytes()).as_ptr(),
                ),
                2,
                pos_bounds.as_ptr() as *const _,
            );
            gl::Uniform2fv(
                gl::GetUniformLocation(
                    *program,
                    CStr::from_bytes_with_nul_unchecked("tex_bounds\0".as_bytes()).as_ptr(),
                ),
                2,
                tex_bounds.as_ptr() as *const _,
            );
            gl::Uniform1i(
                gl::GetUniformLocation(
                    *program,
                    CStr::from_bytes_with_nul_unchecked("tex\0".as_bytes()).as_ptr(),
                ),
                0,
            );
            gl::Uniform2f(
                gl::GetUniformLocation(
                    *program,
                    CStr::from_bytes_with_nul_unchecked("radius\0".as_bytes()).as_ptr(),
                ),
                radius.x,
                radius.y,
            );
            gl::UniformMatrix3fv(
                gl::GetUniformLocation(
                    *program,
                    CStr::from_bytes_with_nul_unchecked("transform\0".as_bytes()).as_ptr(),
                ),
                1,
                gl::FALSE,
                transform.as_ptr() as *const _,
            );
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
        }
    }
}

#[test]
fn test_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<QuadRenderer>();
    assert_sync::<QuadRenderer>();
}
