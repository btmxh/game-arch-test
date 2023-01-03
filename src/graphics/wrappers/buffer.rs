use gl::types::{GLenum, GLuint};

use crate::{graphics::GfxHandle, exec::server::draw};

use super::{GLHandle, GLHandleContainer, GLHandleTrait};

pub struct BufferTrait;
pub type Buffer = GLHandle<BufferTrait, ()>;
pub type BufferContainer = GLHandleContainer<BufferTrait, ()>;
pub type BufferHandle = GfxHandle<Buffer>;

impl GLHandleTrait for BufferTrait {
    fn create(_: ()) -> GLuint {
        let mut handle = 0;
        unsafe { gl::GenBuffers(1, &mut handle) };
        handle
    }

    fn delete(handle: GLuint) {
        Self::delete_mul(&[handle])
    }

    fn identifier() -> GLenum {
        gl::BUFFER
    }

    fn delete_mul(handles: &[GLuint]) {
        unsafe { gl::DeleteBuffers(handles.len().try_into().unwrap(), handles.as_ptr()) }
    }
}

impl BufferHandle {
    pub fn get(&self, server: &draw::Server) -> Option<GLuint> {
        server.handles.buffers.get(self.handle)
    }
}
