use std::{
    collections::HashMap,
    ffi::CString,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use anyhow::bail;
use gl::types::{GLenum, GLuint};

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
}

pub struct GLHandle<T: GLHandleTrait<A>, A>(GLuint, PhantomData<(T, A, *const ())>);
pub struct GLHandleContainer<T: GLHandleTrait<A>, A>(HashMap<u64, GLuint>, PhantomData<(T, A)>);

impl<T: GLHandleTrait<A>, A> Deref for GLHandle<T, A> {
    type Target = GLuint;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: GLHandleTrait<A>, A> DerefMut for GLHandle<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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
    pub fn new(name: &str, args: A) -> anyhow::Result<Self> {
        let handle = T::create(args);
        if handle == 0 {
            bail!("unable to create GL object for {}", name);
        }

        let c_name = CString::new(name)?;
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

        Ok(Self::wrap(handle))
    }

    pub fn wrap(handle: GLuint) -> Self {
        Self(handle, PhantomData)
    }
}

impl<T: GLHandleTrait<()>> GLHandle<T, ()> {
    pub fn new_default(name: &str) -> anyhow::Result<Self> {
        Self::new(name, ())
    }
}

impl<T: GLHandleTrait<A>, A> GLHandleContainer<T, A> {
    pub fn new() -> Self {
        Self(HashMap::new(), PhantomData)
    }

    pub fn insert(&mut self, id: u64, mut handle: GLHandle<T, A>) -> GLuint {
        let hnd = handle.0;
        let old_value = self.0.insert(id, hnd);
        debug_assert!(old_value.is_none());
        handle.0 = 0;
        hnd
    }

    pub fn remove(&mut self, id: u64) -> Option<GLHandle<T, A>> {
        self.0.remove(&id).map(GLHandle::<T, A>::wrap)
    }

    pub fn get(&self, id: u64) -> Option<GLuint> {
        self.0.get(&id).copied()
    }
}

impl<T: GLHandleTrait<A>, A> Default for GLHandleContainer<T, A> {
    fn default() -> Self {
        Self::new()
    }
}
