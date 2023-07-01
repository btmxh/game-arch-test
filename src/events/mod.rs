use std::num::NonZeroU32;

use derivative::Derivative;

use wgpu::PresentMode;
use winit::dpi::PhysicalSize;

use crate::exec::dispatch::{DispatchMsg, EventDispatch};

pub type GameEvent<'a> = winit::event::Event<'a, GameUserEvent>;

#[derive(Derivative)]
#[derivative(Debug)]
pub enum GameUserEvent {
    Exit(i32),
    Dispatch(DispatchMsg),
    Execute(#[derivative(Debug = "ignore")] Box<dyn EventDispatch + Send>),
    VSyncSet(Option<PresentMode>),
    ExecuteReturn(ExecuteReturnEvent),
    Error(anyhow::Error),
    CheckedResize {
        display_size: PhysicalSize<NonZeroU32>,
    },
}

#[derive(Debug)]
pub enum ExecuteReturnEvent {}
