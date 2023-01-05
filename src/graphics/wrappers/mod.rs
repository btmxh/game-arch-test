use std::{
    borrow::Cow, collections::HashMap, ffi::CString, marker::PhantomData, ops::Deref, sync::Arc,
};

use anyhow::{bail, Context};
use gl::types::{GLenum, GLuint};
use sendable::SendRc;

use crate::{
    exec::server::{draw, GameServerChannel},
    utils::{error::ResultExt, mpsc},
};

use super::GfxHandle;

pub mod buffer;
pub mod framebuffer;
pub mod shader;
pub mod texture;
pub mod vertex_array;

pub trait GLHandleTrait<A = ()> {
    fn create(args: A) -> GLuint;
    fn delete(handle: GLuint);
    fn identifier() -> GLenum;
    fn delete_mul(handles: &[GLuint]) {
        handles.iter().for_each(|&handle| Self::delete(handle));
    }

    fn get_container_mut(_server: &mut draw::Server) -> Option<&mut GLHandleContainer<Self, A>>
    where
        Self: Sized,
    {
        None
    }

    fn get_container(_server: &draw::Server) -> Option<&GLHandleContainer<Self, A>>
    where
        Self: Sized,
    {
        None
    }
}

pub struct GLHandleInner<T: GLHandleTrait<A>, A = ()> {
    gl_handle: GLuint,
    name: Cow<'static, str>,
    _phantom: PhantomData<(T, A)>,
}

pub struct GLHandle<T: GLHandleTrait<A>, A = ()>(SendRc<GLHandleInner<T, A>>);

impl<T: GLHandleTrait<A>, A> Clone for GLHandle<T, A> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct GLGfxHandle<T: GLHandleTrait<A> + 'static, A: 'static = ()>(
    pub Arc<GLGfxHandleInner<T, A>>,
);

impl<T: GLHandleTrait<A>, A> Clone for GLGfxHandle<T, A> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct GLGfxHandleInner<T: GLHandleTrait<A> + 'static, A: 'static = ()> {
    pub handle: GfxHandle<GLHandle<T, A>>,
    sender: mpsc::UnboundedSender<draw::RecvMsg>,
    _phantom: PhantomData<fn() -> A>,
}

impl<T: GLHandleTrait<A> + 'static, A: 'static> Drop for GLGfxHandleInner<T, A> {
    fn drop(&mut self) {
        let handle = self.handle;
        self.sender
            .send(draw::RecvMsg::Execute(
                Box::new(move |server| {
                    if let Some(container) = T::get_container_mut(server) {
                        container.remove(&handle);
                    }

                    Ok(Box::new(()))
                }),
                None,
            ))
            .map_err(|e| anyhow::format_err!("{}", e))
            .context("unable to drop GL handle")
            .log_warn();
    }
}

impl<T: GLHandleTrait<A>, A> GLGfxHandle<T, A> {
    pub fn new(draw: &mut draw::ServerChannel) -> Self {
        Self(Arc::new(GLGfxHandleInner {
            handle: GfxHandle::new(draw),
            sender: draw.sender().clone(),
            _phantom: PhantomData,
        }))
    }

    pub fn get(&self, server: &draw::Server) -> Option<GLHandle<T, A>> {
        T::get_container(server).unwrap().get(self)
    }
}

impl<T: GLHandleTrait<A>, A> Deref for GLHandle<T, A> {
    type Target = GLuint;

    fn deref(&self) -> &Self::Target {
        &self.0.gl_handle
    }
}

impl<T: GLHandleTrait<A>, A> Drop for GLHandle<T, A> {
    fn drop(&mut self) {
        let handle = **self;
        if handle != 0 {
            T::delete(**self)
        }
    }
}

impl<T: GLHandleTrait<A>, A> GLHandle<T, A> {
    pub fn new_args(name: impl Into<Cow<'static, str>>, args: A) -> anyhow::Result<Self> {
        let name = name.into();
        let handle = T::create(args);
        if handle == 0 {
            bail!("unable to create GL object for {}", name);
        }

        let c_name = CString::new(name.as_ref())?;
        unsafe {
            if gl::ObjectLabel::is_loaded() {
                gl::ObjectLabel(
                    T::identifier(),
                    handle,
                    name.len().try_into()?,
                    c_name.as_ptr(),
                )
            }
        };

        Ok(Self::wrap(handle, name))
    }

    pub fn wrap(gl_handle: GLuint, name: Cow<'static, str>) -> Self {
        Self(SendRc::new(GLHandleInner {
            gl_handle,
            name,
            _phantom: PhantomData,
        }))
    }

    pub fn name(&self) -> Cow<'static, str> {
        self.0.name.clone()
    }
}

impl<T: GLHandleTrait<()>> GLHandle<T> {
    pub fn new(name: impl Into<Cow<'static, str>>) -> anyhow::Result<Self> {
        Self::new_args(name, ())
    }
}

pub struct GLHandleContainer<T: GLHandleTrait<A>, A = ()>(HashMap<u64, GLHandle<T, A>>);

impl<T: GLHandleTrait<A>, A> Drop for GLHandleContainer<T, A> {
    fn drop(&mut self) {
        T::delete_mul(self.0.values().map(|h| **h).collect::<Vec<_>>().as_slice());
        let mut empty_map = HashMap::new();
        std::mem::swap(&mut self.0, &mut empty_map);
        std::mem::forget(empty_map);
    }
}

impl<T: GLHandleTrait<A>, A> GLHandleContainer<T, A> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    fn handle_to_key(handle: &GLGfxHandle<T, A>) -> u64 {
        handle.0.handle.handle
    }

    pub fn insert(&mut self, gfx_handle: GLGfxHandle<T, A>, handle: GLHandle<T, A>) -> GLuint {
        let gl_handle = *handle;
        let old_value = self.0.insert(Self::handle_to_key(&gfx_handle), handle);
        debug_assert!(old_value.is_none());
        gl_handle
    }

    pub fn remove(&mut self, gfx_handle: &GfxHandle<GLHandle<T, A>>) -> Option<GLHandle<T, A>> {
        self.0.remove(&gfx_handle.handle)
    }

    pub fn get(&self, gfx_handle: &GLGfxHandle<T, A>) -> Option<GLHandle<T, A>> {
        self.0.get(&Self::handle_to_key(gfx_handle)).cloned()
    }
}

impl<T: GLHandleTrait<()>> Default for GLHandleContainer<T> {
    fn default() -> Self {
        Self::new()
    }
}
