use std::{borrow::Cow, ptr::null};

use gl::types::{GLenum, GLuint};
use glutin::prelude::GlConfig;
use winit::dpi::PhysicalSize;

use crate::{
    events::GameUserEvent,
    exec::server::draw::{self, ServerSendChannelExt},
    graphics::context::DrawContext,
};

use super::{
    texture::{Texture, TextureHandle, TextureType},
    GLGfxHandle, GLHandle, GLHandleContainer, GLHandleTrait, SendGLHandleContainer,
};

pub struct FramebufferTrait;
pub type Framebuffer = GLHandle<FramebufferTrait>;
pub type FramebufferContainer = GLHandleContainer<FramebufferTrait>;
pub type SendFramebufferContainer = SendGLHandleContainer<FramebufferTrait>;
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

    fn bind(handle: GLuint, _: ()) {
        unsafe { gl::BindFramebuffer(gl::FRAMEBUFFER, handle) }
    }

    fn identifier() -> GLenum {
        gl::FRAMEBUFFER
    }

    fn delete_mul(handles: &[GLuint]) {
        unsafe { gl::DeleteFramebuffers(handles.len().try_into().unwrap(), handles.as_ptr()) }
    }

    fn get_container_mut(context: &mut DrawContext) -> Option<&mut GLHandleContainer<Self, ()>> {
        Some(&mut context.handles.framebuffers)
    }

    fn get_container(context: &DrawContext) -> Option<&GLHandleContainer<Self, ()>> {
        Some(&context.handles.framebuffers)
    }
}

#[derive(Clone)]
pub struct DefaultTextureFramebuffer {
    pub texture: TextureHandle,
    pub framebuffer: FramebufferHandle,
    pub size: Option<PhysicalSize<u32>>,
}

impl DefaultTextureFramebuffer {
    pub fn new(
        draw: &mut draw::ServerChannel,
        name: impl Into<Cow<'static, str>>,
    ) -> anyhow::Result<Self> {
        let name = name.into();
        let slf = Self {
            texture: TextureHandle::new_args(
                draw,
                format!("{name} texture attachment"),
                TextureType::E2D,
            )?,
            framebuffer: FramebufferHandle::new(draw, name)?,
            size: None,
        };
        Ok(slf)
    }

    fn resize_in_server(
        &self,
        context: &mut DrawContext,
        size: PhysicalSize<u32>,
    ) -> anyhow::Result<()> {
        let (framebuffer, texture) = match self.size {
            Some(sz) if size == sz => return Ok(()),
            None => (self.framebuffer.get(context), self.texture.get(context)),
            _ => {
                let texture = context
                    .handles
                    .textures
                    .replace(&self.texture, |old_texture| {
                        Texture::new_args(old_texture.name(), TextureType::E2D)
                    })?;

                (self.framebuffer.get(context), texture)
            }
        };
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, *framebuffer);
            gl::BindTexture(gl::TEXTURE_2D, *texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                if context.gl_config.srgb_capable() {
                    gl::SRGB8_ALPHA8.try_into().unwrap()
                } else {
                    gl::RGBA8.try_into().unwrap()
                },
                size.width.try_into().unwrap(),
                size.height.try_into().unwrap(),
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                null(),
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                gl::LINEAR.try_into().unwrap(),
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                gl::LINEAR.try_into().unwrap(),
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_EDGE.try_into().unwrap(),
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_EDGE.try_into().unwrap(),
            );
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                *texture,
                0,
            );

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
        Ok(())
    }

    pub fn resize(
        &mut self,
        draw: &mut draw::ServerChannel,
        new_size: PhysicalSize<u32>,
    ) -> anyhow::Result<()> {
        if self.size.map(|s| s == new_size).unwrap_or(false) {
            return Ok(());
        }

        let slf = self.clone();
        self.size = Some(new_size);
        draw.execute_draw_event(move |context, _| {
            slf.resize_in_server(context, new_size)
                .err()
                .map(GameUserEvent::Error)
        })?;
        Ok(())
    }
}
