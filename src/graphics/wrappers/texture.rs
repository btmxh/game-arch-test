use gl::types::{GLenum, GLuint};

use crate::exec::server::draw;

use super::{GLGfxHandle, GLHandle, GLHandleContainer, GLHandleTrait, SendGLHandleContainer};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextureType {
    E2D = gl::TEXTURE_2D as _,
}

pub struct TextureTrait;
pub type Texture = GLHandle<TextureTrait, TextureType>;
pub type TextureContainer = GLHandleContainer<TextureTrait, TextureType>;
pub type SendTextureContainer = SendGLHandleContainer<TextureTrait, TextureType>;
pub type TextureHandle = GLGfxHandle<TextureTrait, TextureType>;

impl GLHandleTrait<TextureType> for TextureTrait {
    fn create(_: TextureType) -> GLuint {
        let mut handle = 0;
        unsafe { gl::GenTextures(1, &mut handle) };
        handle
    }

    fn delete(handle: GLuint) {
        Self::delete_mul(&[handle])
    }

    fn bind(handle: GLuint, args: TextureType) {
        unsafe { gl::BindTexture(args as _, handle) }
    }

    fn identifier() -> GLenum {
        gl::TEXTURE
    }

    fn delete_mul(handles: &[GLuint]) {
        unsafe { gl::DeleteTextures(handles.len().try_into().unwrap(), handles.as_ptr()) }
    }

    fn get_container_mut(
        server: &mut draw::Server,
    ) -> Option<&mut GLHandleContainer<Self, TextureType>> {
        Some(&mut server.handles.textures)
    }

    fn get_container(server: &draw::Server) -> Option<&GLHandleContainer<Self, TextureType>> {
        Some(&server.handles.textures)
    }
}
