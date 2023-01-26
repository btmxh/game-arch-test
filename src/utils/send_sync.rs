use std::{cell::Cell, marker::PhantomData, sync::MutexGuard};

pub type PhantomUnsync = PhantomData<Cell<()>>;
pub type PhantomUnsend = PhantomData<MutexGuard<'static, ()>>;

#[macro_export]
macro_rules! assert_send {
    ($x: ty) => {
        static_assertions::assert_impl_all!($x: Send)
    };
}

#[macro_export]
macro_rules! assert_sync {
    ($x: ty) => {
        static_assertions::assert_impl_all!($x: Sync)
    };
}

#[macro_export]
macro_rules! assert_not_send {
    ($x: ty) => {
        static_assertions::assert_not_impl_all!($x: Send)
    };
}

#[macro_export]
macro_rules! assert_not_sync {
    ($x: ty) => {
        static_assertions::assert_not_impl_all!($x: Sync)
    };
}

#[test]
fn test_send_sync() {
    assert_send!(PhantomUnsync);
    assert_sync!(PhantomUnsend);

    assert_not_send!(PhantomUnsend);
    assert_not_sync!(PhantomUnsync);
}
