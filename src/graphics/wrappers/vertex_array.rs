use gl::types::{GLenum, GLuint};

use crate::graphics::context::DrawContext;

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

    fn bind(handle: GLuint, _: ()) {
        unsafe { gl::BindVertexArray(handle) }
    }

    fn identifier() -> GLenum {
        gl::VERTEX_ARRAY
    }

    fn delete_mul(handles: &[GLuint]) {
        unsafe { gl::DeleteVertexArrays(handles.len().try_into().unwrap(), handles.as_ptr()) }
    }

    fn get_container_mut(context: &mut DrawContext) -> Option<&mut GLHandleContainer<Self, ()>> {
        Some(&mut context.handles.vertex_arrays)
    }

    fn get_container(context: &DrawContext) -> Option<&GLHandleContainer<Self, ()>> {
        Some(&context.handles.vertex_arrays)
    }
}

#[test]
fn test_send_sync() {
    use crate::{assert_not_sync, assert_send, assert_sync};
    assert_send!(VertexArrayHandle);
    assert_sync!(VertexArrayHandle);
    // VertexArrays is stored in a container that must be sendable
    // if `draw::Server` is sendable
    assert_send!(VertexArray);
    assert_not_sync!(VertexArray);
}
