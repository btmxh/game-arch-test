use gl::types::{GLenum, GLuint};

use crate::exec::server::draw;

use super::{GLGfxHandle, GLHandle, GLHandleContainer, GLHandleTrait};

pub struct BufferTrait;
pub type Buffer = GLHandle<BufferTrait>;
pub type BufferContainer = GLHandleContainer<BufferTrait>;
pub type BufferHandle = GLGfxHandle<BufferTrait>;

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

    fn get_container_mut(server: &mut draw::Server) -> Option<&mut GLHandleContainer<Self, ()>> {
        Some(&mut server.handles.buffers)
    }

    fn get_container(server: &draw::Server) -> Option<&GLHandleContainer<Self, ()>> {
        Some(&server.handles.buffers)
    }
}
