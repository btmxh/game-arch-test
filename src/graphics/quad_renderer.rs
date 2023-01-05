use std::ffi::CStr;

use anyhow::Context;
use gl::types::GLuint;
use glsl_layout::vec2;

use crate::exec::{dispatch::ReturnMechanism, executor::GameServerExecutor, server::draw};

use super::wrappers::{shader::ProgramHandle, vertex_array::VertexArrayHandle};

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
        executor: &mut GameServerExecutor,
        dummy_vao: VertexArrayHandle,
        draw: &mut draw::ServerChannel,
    ) -> anyhow::Result<Self> {
        let program = ProgramHandle::new(draw);
        let program_clone = program.clone();
        let slf = Self {
            vertex_array: dummy_vao,
            program,
        };

        executor
            .execute_draw(draw, Some(ReturnMechanism::Sync), move |server| {
                server.handles.create_vf_program(
                    "quad renderer shader program",
                    program_clone,
                    shader::VERTEX,
                    shader::FRAGMENT,
                )?;

                Ok(Box::new(()))
            })
            .context("quad renderer initialization (in draw server) failed")?;

        Ok(slf)
    }

    pub fn draw(&self, server: &draw::Server, texture: GLuint, bounds: &[vec2; 2]) {
        let vao = self
            .vertex_array
            .get(server)
            .expect("quad renderer vertex array not found");
        let program = self
            .program
            .get(server)
            .expect("quad renderer shader program not found");

        unsafe {
            gl::BindVertexArray(vao);
            gl::UseProgram(program);

            gl::Uniform2fv(
                gl::GetUniformLocation(
                    program,
                    CStr::from_bytes_with_nul_unchecked("bounds\0".as_bytes()).as_ptr(),
                ),
                2,
                bounds.as_ptr() as *const _,
            );
            gl::Uniform1i(
                gl::GetUniformLocation(
                    program,
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
