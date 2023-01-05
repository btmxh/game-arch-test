use gl::types::{GLenum, GLuint};

use crate::exec::server::draw;

use super::{GLGfxHandle, GLHandle, GLHandleContainer, GLHandleTrait, SendGLHandleContainer};

pub struct VertexArrayTrait;
pub type VertexArray = GLHandle<VertexArrayTrait>;
pub type VertexArrayContainer = GLHandleContainer<VertexArrayTrait>;
pub type SendVertexArrayContainer = SendGLHandleContainer<VertexArrayTrait>;
pub type VertexArrayHandle = GLGfxHandle<VertexArrayTrait>;

impl GLHandleTrait for VertexArrayTrait {
    fn create(_: ()) -> GLuint {
        let mut handle = 0;
        unsafe { gl::GenVertexArrays(1, &mut handle) };
        handle
    }

    fn delete(handle: GLuint) {
        Self::delete_mul(&[handle])
    }

    fn identifier() -> GLenum {
        gl::VERTEX_ARRAY
    }

    fn delete_mul(handles: &[GLuint]) {
        unsafe { gl::DeleteVertexArrays(handles.len().try_into().unwrap(), handles.as_ptr()) }
    }

    fn get_container_mut(server: &mut draw::Server) -> Option<&mut GLHandleContainer<Self, ()>> {
        Some(&mut server.handles.vertex_arrays)
    }

    fn get_container(server: &draw::Server) -> Option<&GLHandleContainer<Self, ()>> {
        Some(&server.handles.vertex_arrays)
    }
}

#[test]
fn test_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<VertexArrayHandle>();
    assert_sync::<VertexArrayHandle>();
}
