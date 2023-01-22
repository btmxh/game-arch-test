use gl::types::{GLenum, GLuint};

use crate::graphics::context::DrawContext;

use super::{GLGfxHandle, GLHandle, GLHandleContainer, GLHandleTrait, SendGLHandleContainer};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum BufferTarget {
    ArrayBuffer = gl::ARRAY_BUFFER as _,
    UniformBuffer = gl::UNIFORM_BUFFER as _,
    ShaderStorageBuffer = gl::SHADER_STORAGE_BUFFER as _,
}

pub struct BufferTrait;
pub type Buffer = GLHandle<BufferTrait, BufferTarget>;
pub type BufferContainer = GLHandleContainer<BufferTrait, BufferTarget>;
pub type SendBufferContainer = SendGLHandleContainer<BufferTrait, BufferTarget>;
pub type BufferHandle = GLGfxHandle<BufferTrait, BufferTarget>;

impl GLHandleTrait<BufferTarget> for BufferTrait {
    fn create(_: BufferTarget) -> GLuint {
        let mut handle = 0;
        unsafe { gl::GenBuffers(1, &mut handle) };
        handle
    }

    fn delete(handle: GLuint) {
        Self::delete_mul(&[handle])
    }

    fn bind(handle: GLuint, args: BufferTarget) {
        unsafe { gl::BindBuffer(args as GLenum, handle) }
    }

    fn identifier() -> GLenum {
        gl::BUFFER
    }

    fn delete_mul(handles: &[GLuint]) {
        unsafe { gl::DeleteBuffers(handles.len().try_into().unwrap(), handles.as_ptr()) }
    }

    fn get_container_mut(
        context: &mut DrawContext,
    ) -> Option<&mut GLHandleContainer<Self, BufferTarget>> {
        Some(&mut context.handles.buffers)
    }

    fn get_container(context: &DrawContext) -> Option<&GLHandleContainer<Self, BufferTarget>> {
        Some(&context.handles.buffers)
    }
}
