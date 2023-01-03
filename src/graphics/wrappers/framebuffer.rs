use std::mem::swap;

use anyhow::Ok;
use gl::types::{GLenum, GLuint};

use crate::{graphics::GfxHandle, exec::server::draw};

use super::{GLHandle, GLHandleContainer, GLHandleTrait};

pub struct FramebufferTrait;
pub type Framebuffer = GLHandle<FramebufferTrait, ()>;
pub type FramebufferContainer = GLHandleContainer<FramebufferTrait, ()>;
pub type FramebufferHandle = GfxHandle<Framebuffer>;

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
}

impl Framebuffer {
    pub fn recreate(&mut self, name: &str) -> anyhow::Result<Framebuffer> {
        let mut new_fb = Framebuffer::new_default(name)?;
        swap(&mut self.0, &mut new_fb.0);
        Ok(new_fb)
    }
}

impl FramebufferHandle {
    pub fn get(&self, server: &draw::Server) -> Option<GLuint> {
        server.handles.framebuffers.get(self.handle)
    }
}
