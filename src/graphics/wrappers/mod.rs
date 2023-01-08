use std::{
    borrow::Cow,
    collections::HashMap,
    ffi::CString,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, MutexGuard},
};

use anyhow::{bail, Context};
use gl::types::{GLenum, GLuint};
use sendable::{send_rc::PostSend, SendRc};

use crate::{
    enclose,
    exec::{
        dispatch::ReturnMechanism,
        executor::GameServerExecutor,
        server::{draw, GameServerChannel},
    },
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
                        unsafe { container.remove(&handle) };
                    }

                    Ok(Box::new(()))
                }),
                None,
            ))
            .map_err(|e| anyhow::format_err!("{}", e))
            .context("unable to send GL handle drop execute message to draw server, the connection was closed (the handles were probably dropped with the server earlier, if so this is not a leak)")
            .log_info();
    }
}

impl<T: GLHandleTrait<A> + 'static, A: 'static> GLGfxHandle<T, A> {
    /// # Safety
    /// 
    /// Use this only if you are going to initialize the handle later
    pub unsafe fn new_uninit(draw: &mut draw::ServerChannel) -> Self {
        Self(Arc::new(GLGfxHandleInner {
            handle: GfxHandle::new(draw),
            sender: draw.sender().clone(),
            _phantom: PhantomData,
        }))
    }

    #[allow(unused_mut)]
    pub async fn new_args(
        executor: &mut GameServerExecutor,
        draw: &mut draw::ServerChannel,
        return_mechanism: Option<ReturnMechanism>,
        name: impl Into<Cow<'static, str>> + Send + 'static,
        args: A,
    ) -> anyhow::Result<Self>
    where
        A: Send,
    {
        let slf = unsafe { Self::new_uninit(draw) };
        executor.execute_draw(
            draw,
            return_mechanism,
            enclose!((slf) move |server| {
                if let Some(container) = T::get_container_mut(server) {
                    let handle = GLHandle::<T, A>::new_args(name, args)?;
                    container.insert(&slf, handle);
                }
                Ok(Box::new(()))
            }),
        ).await?;
        Ok(slf)
    }

    pub fn try_get(&self, server: &draw::Server) -> Option<GLHandle<T, A>> {
        T::get_container(server).and_then(|c| c.get(self))
    }

    pub fn get(&self, server: &draw::Server) -> GLHandle<T, A> {
        self.try_get(server)
            .expect("get() called on a null GLHandle")
    }
}

impl<T: GLHandleTrait<()> + 'static> GLGfxHandle<T> {
    pub async fn new(
        executor: &mut GameServerExecutor,
        draw: &mut draw::ServerChannel,
        return_mechanism: Option<ReturnMechanism>,
        name: impl Into<Cow<'static, str>> + Send + 'static,
    ) -> anyhow::Result<Self> {
        Self::new_args(executor, draw, return_mechanism, name, ()).await
    }
}

impl<T: GLHandleTrait<A>, A> Deref for GLHandle<T, A> {
    type Target = GLuint;

    fn deref(&self) -> &Self::Target {
        &self.0.gl_handle
    }
}

impl<T: GLHandleTrait<A>, A> Drop for GLHandleInner<T, A> {
    fn drop(&mut self) {
        let handle = self.gl_handle;
        if handle != 0 {
            T::delete(handle)
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

pub struct GLHandleContainer<T: GLHandleTrait<A>, A = ()>(
    HashMap<u64, GLHandle<T, A>>,
    PhantomData<MutexGuard<'static, ()>>, // impl !Send
);

pub struct SendGLHandleContainer<T: GLHandleTrait<A>, A = ()>(
    HashMap<u64, GLHandle<T, A>>,
    PostSend<GLHandleInner<T, A>>,
);

impl<T: GLHandleTrait<A>, A> Default for SendGLHandleContainer<T, A> {
    fn default() -> Self {
        Self(HashMap::new(), SendRc::pre_send().ready())
    }
}

impl<T: GLHandleTrait<A>, A> Drop for GLHandleContainer<T, A> {
    fn drop(&mut self) {
        T::delete_mul(self.0.values().map(|h| **h).collect::<Vec<_>>().as_slice());
        let mut empty_map = HashMap::new();
        std::mem::swap(&mut self.0, &mut empty_map);
        std::mem::forget(empty_map);
    }
}

impl<T: GLHandleTrait<A>, A> Drop for SendGLHandleContainer<T, A> {
    fn drop(&mut self) {
        T::delete_mul(self.0.values().map(|h| **h).collect::<Vec<_>>().as_slice());
        let mut empty_map = HashMap::new();
        std::mem::swap(&mut self.0, &mut empty_map);
        std::mem::forget(empty_map);
    }
}

impl<T: GLHandleTrait<A>, A> GLHandleContainer<T, A> {
    pub fn new() -> Self {
        Self(HashMap::new(), PhantomData)
    }

    fn handle_to_key(handle: &GLGfxHandle<T, A>) -> u64 {
        handle.0.handle.handle
    }

    pub fn insert(
        &mut self,
        gfx_handle: &GLGfxHandle<T, A>,
        handle: GLHandle<T, A>,
    ) -> GLHandle<T, A> {
        let old_value = self
            .0
            .insert(Self::handle_to_key(gfx_handle), handle.clone());
        debug_assert!(old_value.is_none());
        handle
    }

    /// # Safety
    ///
    /// use this only if you put in a replacement immediately (in the implementation of
    /// the replace fn) or to drop the GLHandle
    pub unsafe fn remove(
        &mut self,
        gfx_handle: &GfxHandle<GLHandle<T, A>>,
    ) -> Option<GLHandle<T, A>> {
        self.0.remove(&gfx_handle.handle)
    }

    pub fn replace<F>(
        &mut self,
        gfx_handle: &GLGfxHandle<T, A>,
        transform: F,
    ) -> anyhow::Result<GLHandle<T, A>>
    where
        F: FnOnce(GLHandle<T, A>) -> anyhow::Result<GLHandle<T, A>>,
    {
        let old_handle = unsafe { self.remove(&gfx_handle.0.handle) }
            .expect("replace() called on a null GLHandle");
        let new_handle = transform(old_handle)?;
        self.insert(gfx_handle, new_handle.clone());
        Ok(new_handle)
    }

    pub fn get(&self, gfx_handle: &GLGfxHandle<T, A>) -> Option<GLHandle<T, A>> {
        self.0.get(&Self::handle_to_key(gfx_handle)).cloned()
    }

    pub fn to_send(mut self) -> SendGLHandleContainer<T, A> {
        let presend = SendRc::pre_send();
        for value in self.0.values_mut() {
            presend.park(&mut value.0);
        }
        let token = presend.ready();
        SendGLHandleContainer(std::mem::take(&mut self.0), token)
    }
}

impl<T: GLHandleTrait<A>, A> SendGLHandleContainer<T, A> {
    pub fn to_unsend(mut self) -> GLHandleContainer<T, A> {
        let map = std::mem::take(&mut self.0);
        let token = std::mem::replace(&mut self.1, SendRc::pre_send().ready());
        token.unpark();
        GLHandleContainer(map, PhantomData)
    }
}

impl<T: GLHandleTrait<()>> Default for GLHandleContainer<T> {
    fn default() -> Self {
        Self::new()
    }
}
