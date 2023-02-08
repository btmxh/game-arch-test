use std::ops::{Deref, DerefMut};

pub struct Mutex<T>(parking_lot::Mutex<T>);
pub struct MutexGuard<'a, T>(parking_lot::MutexGuard<'a, T>);

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self(parking_lot::Mutex::new(value))
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        MutexGuard(self.0.lock())
    }

    pub fn into_inner(self) -> parking_lot::Mutex<T> {
        self.0
    }
}

impl<'a, T> MutexGuard<'a, T> {
    pub fn into_inner(self) -> parking_lot::MutexGuard<'a, T> {
        self.0
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
