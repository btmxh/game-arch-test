use gl::types::{GLenum, GLuint};

use crate::exec::server::draw;

use super::{texture::TextureHandle, GLGfxHandle, GLHandle, GLHandleContainer, GLHandleTrait};

pub struct FramebufferTrait;
pub type Framebuffer = GLHandle<FramebufferTrait>;
pub type FramebufferContainer = GLHandleContainer<FramebufferTrait>;
pub type FramebufferHandle = GLGfxHandle<FramebufferTrait>;

impl GLHandleTrait for FramebufferTrait {
    fn create(_: ()) -> GLuint {
        let mut handle = 0;
        unsafe { gl::GenFramebuffers(1, &mut handle) };
        handle
    }

    fn delete(handle: GLuint) {
        Self::delete_mul(&[handle])
    }

    fn identifier() -> GLenum {
        gl::FRAMEBUFFER
    }

    fn delete_mul(handles: &[GLuint]) {
        unsafe { gl::DeleteFramebuffers(handles.len().try_into().unwrap(), handles.as_ptr()) }
    }

    fn get_container_mut(server: &mut draw::Server) -> Option<&mut GLHandleContainer<Self, ()>> {
        Some(&mut server.handles.framebuffers)
    }

    fn get_container(server: &draw::Server) -> Option<&GLHandleContainer<Self, ()>> {
        Some(&server.handles.framebuffers)
    }
}

pub struct DefaultTextureFramebuffer {
    pub texture: TextureHandle,
    pub framebuffer: FramebufferHandle,
}
