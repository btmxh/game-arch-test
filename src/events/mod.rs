use derivative::Derivative;
use glutin::surface::SwapInterval;

use crate::exec::{
    dispatch::{DispatchId, DispatchMsg},
    main_ctx::MainContext,
};

pub type GameEvent<'a> = winit::event::Event<'a, GameUserEvent>;
pub type ExecuteCallback = Box<dyn FnOnce(&mut MainContext) -> anyhow::Result<()> + Send>;

#[derive(Derivative)]
#[derivative(Debug)]
pub enum GameUserEvent {
    Exit,
    Dispatch(DispatchMsg),
    Execute(#[derivative(Debug = "ignore")] ExecuteCallback),
    VSyncSet(Option<SwapInterval>, Option<DispatchId>),
    ExecuteReturn(ExecuteReturnEvent, Option<DispatchId>),
    Error(anyhow::Error),
}

#[derive(Debug)]
pub enum ExecuteReturnEvent {}
