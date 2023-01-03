use gl::types::{GLenum, GLuint};

use crate::{exec::server::draw, graphics::GfxHandle};

use super::{GLHandle, GLHandleContainer, GLHandleTrait};

pub struct VertexArrayTrait;
pub type VertexArray = GLHandle<VertexArrayTrait, ()>;
pub type VertexArrayContainer = GLHandleContainer<VertexArrayTrait, ()>;
pub type VertexArrayHandle = GfxHandle<VertexArray>;

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
}

impl VertexArrayHandle {
    pub fn get(&self, server: &draw::Server) -> Option<GLuint> {
        server.handles.vertex_arrays.get(self.handle)
    }
}

#[test]
fn test_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<VertexArrayHandle>();
    assert_sync::<VertexArrayHandle>();
}
