use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{
    exec::main_ctx::MainContext,
    graphics::{context::DrawContext, quad_renderer::QuadRenderer},
    utils::mutex::Mutex,
};

use super::utils::geom::{UIPos, UISize};

use delegate::delegate;
use derive_more::{Deref, DerefMut};
use glam::Mat3;

pub struct UIEventContext<'a> {
    pub main_ctx: &'a MainContext,
}

#[derive(Clone, Copy)]
pub struct UIDrawContext<'a> {
    pub context: &'a DrawContext,
    pub quad_renderer: &'a QuadRenderer,
}

pub trait UISizeConstraint {
    fn min_size(&self) -> Option<UISize>;
    fn max_size(&self) -> Option<UISize>;
    fn pref_size(&self) -> Option<UISize>;
}

pub trait UIStateSizeTrait: Deref<Target = UISize> + DerefMut + UISizeConstraint {}

impl<C: UISizeConstraint> UISizeConstraint for UIStateSize<C> {
    delegate! {
        to self.1 {
            fn min_size(&self) -> Option<UISize>;
            fn max_size(&self) -> Option<UISize>;
            fn pref_size(&self) -> Option<UISize>;
        }
    }
}

pub type UIStatePos = UIPos;

#[derive(Deref, DerefMut)]
pub struct UIStateSize<C: UISizeConstraint>(
    #[deref]
    #[deref_mut]
    pub UISize,
    pub C,
);

#[derive(Deref, DerefMut)]
pub struct UIStateOpacity(pub f32);

impl UIStateOpacity {
    pub fn is_visible(&self) -> bool {
        self.0 > 0.0
    }
}

#[derive(Deref, DerefMut)]
pub struct UIStateTransform(pub Mat3);

impl<C: UISizeConstraint> UIStateSizeTrait for UIStateSize<C> {}

pub type SharedState<S> = Arc<Mutex<S>>;

pub fn new_shared_state<S>(state: S) -> SharedState<S> {
    Arc::new(Mutex::new(state))
}
