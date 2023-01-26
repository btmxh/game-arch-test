use std::ops::{Deref, DerefMut};

pub struct Mutex<T>(std::sync::Mutex<T>);
pub struct MutexGuard<'a, T>(std::sync::MutexGuard<'a, T>);

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self(std::sync::Mutex::new(value))
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        MutexGuard(self.0.lock().expect("mutex poison error"))
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}
