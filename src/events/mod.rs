use std::num::NonZeroU32;

use derivative::Derivative;

use winit::dpi::PhysicalSize;

use crate::context::update::TimeoutDispatchHandleSet;

pub type GameEvent<'a> = winit::event::Event<'a, GameUserEvent>;

#[derive(Derivative)]
#[derivative(Debug)]
pub enum GameUserEvent {
    Exit(i32),
    ExitError(anyhow::Error),
    UpdateDispatch(TimeoutDispatchHandleSet),
    CheckedResize(PhysicalSize<NonZeroU32>),
}
