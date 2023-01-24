use std::ffi::CStr;

use anyhow::Context;
use gl::types::GLuint;

use crate::exec::server::draw;

use super::{
    context::DrawContext,
    wrappers::{shader::ProgramHandle, vertex_array::VertexArrayHandle}, Vec2,
};

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
    uniform vec2 bounds[2];
    void main() {
        color = texture(tex, mix(bounds[0], bounds[1], tex_coords));
    }
    "#;
}

#[derive(Clone)]
pub struct QuadRenderer {
    vertex_array: VertexArrayHandle,
    program: ProgramHandle,
}

impl QuadRenderer {
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

    pub fn draw(&self, context: &DrawContext, texture: GLuint, bounds: &[Vec2; 2]) {
        let vao = self.vertex_array.get(context);
        let program = self.program.get(context);

        unsafe {
            vao.bind();
            gl::UseProgram(*program);

            gl::Uniform2fv(
                gl::GetUniformLocation(
                    *program,
                    CStr::from_bytes_with_nul_unchecked("bounds\0".as_bytes()).as_ptr(),
                ),
                2,
                bounds.as_ptr() as *const _,
            );
            gl::Uniform1i(
                gl::GetUniformLocation(
                    *program,
                    CStr::from_bytes_with_nul_unchecked("tex\0".as_bytes()).as_ptr(),
                ),
                0,
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
