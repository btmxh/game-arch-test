use std::num::NonZeroU32;

use derivative::Derivative;
use glutin::surface::SwapInterval;
use winit::dpi::PhysicalSize;

use crate::{
    exec::{
        dispatch::{DispatchId, DispatchMsg},
        executor::GameServerExecutor,
        main_ctx::MainContext,
    },
    scene::main::EventRoot,
};

pub type GameEvent<'a> = winit::event::Event<'a, GameUserEvent>;
pub type ExecuteCallback = Box<
    dyn FnOnce(&mut MainContext, &mut GameServerExecutor, &mut EventRoot) -> anyhow::Result<()>
        + Send,
>;

#[derive(Derivative)]
#[derivative(Debug)]
pub enum GameUserEvent {
    Exit,
    Dispatch(DispatchMsg),
    Execute(#[derivative(Debug = "ignore")] ExecuteCallback),
    VSyncSet(Option<SwapInterval>, Option<DispatchId>),
    ExecuteReturn(ExecuteReturnEvent, Option<DispatchId>),
    Error(anyhow::Error),
    CheckedResize(PhysicalSize<NonZeroU32>),
}

#[derive(Debug)]
pub enum ExecuteReturnEvent {}
