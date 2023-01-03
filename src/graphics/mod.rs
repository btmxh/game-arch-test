use std::marker::PhantomData;

use gl::types::GLuint;

use self::wrappers::{
    buffer::{Buffer, BufferContainer},
    framebuffer::{Framebuffer, FramebufferContainer},
    shader::{Program, ProgramContainer},
    texture::{Texture, TextureContainer},
    vertex_array::{VertexArray, VertexArrayContainer},
};

pub mod quad_renderer;
pub mod tree;
pub mod wrappers;
pub mod debug_callback;

#[derive(Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GfxHandle<T> {
    pub handle: u64,
    data: PhantomData<fn() -> T>,
}

impl<T> GfxHandle<T> {
    pub fn from_handle(handle: u64) -> Self {
        Self {
            handle,
            data: PhantomData,
        }
    }
}

impl<T> Clone for GfxHandle<T> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle,
            data: self.data,
        }
    }
}

#[derive(Default)]
pub struct HandleContainer {
    vertex_arrays: VertexArrayContainer,
    buffers: BufferContainer,
    textures: TextureContainer,
    programs: ProgramContainer,
    framebuffers: FramebufferContainer,
}

impl HandleContainer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_vertex_array(&mut self, name: &str, handle: u64) -> anyhow::Result<GLuint> {
        VertexArray::new_default(name).map(|v| self.vertex_arrays.insert(handle, v))
    }

    pub fn create_buffer(&mut self, name: &str, handle: u64) -> anyhow::Result<GLuint> {
        Buffer::new_default(name).map(|b| self.buffers.insert(handle, b))
    }

    pub fn create_texture(&mut self, name: &str, handle: u64) -> anyhow::Result<GLuint> {
        Texture::new_default(name).map(|t| self.textures.insert(handle, t))
    }

    pub fn create_vf_program(
        &mut self,
        name: &str,
        handle: u64,
        vertex: &str,
        fragment: &str,
    ) -> anyhow::Result<GLuint> {
        Program::new_vf(name, vertex, fragment).map(|p| self.programs.insert(handle, p))
    }

    pub fn create_framebuffer(&mut self, name: &str, handle: u64) -> anyhow::Result<GLuint> {
        Framebuffer::new_default(name).map(|f| self.framebuffers.insert(handle, f))
    }
}
