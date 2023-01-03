use gl::types::{GLenum, GLuint};

use crate::{graphics::GfxHandle, exec::server::draw};

use super::{GLHandle, GLHandleContainer, GLHandleTrait};

pub struct TextureTrait;
pub type Texture = GLHandle<TextureTrait, ()>;
pub type TextureContainer = GLHandleContainer<TextureTrait, ()>;
pub type TextureHandle = GfxHandle<Texture>;

impl GLHandleTrait for TextureTrait {
    fn create(_: ()) -> GLuint {
        let mut handle = 0;
        unsafe { gl::GenTextures(1, &mut handle) };
        handle
    }

    fn delete(handle: GLuint) {
        Self::delete_mul(&[handle])
    }

    fn identifier() -> GLenum {
        gl::TEXTURE
    }

    fn delete_mul(handles: &[GLuint]) {
        unsafe { gl::DeleteTextures(handles.len().try_into().unwrap(), handles.as_ptr()) }
    }
}

impl TextureHandle {
    pub fn get(&self, server: &draw::Server) -> Option<GLuint> {
        server.handles.textures.get(self.handle)
    }
}
