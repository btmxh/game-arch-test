use std::num::NonZeroU32;

use derivative::Derivative;
use glutin::surface::SwapInterval;
use winit::dpi::PhysicalSize;

use crate::{
    exec::{
        dispatch::{DispatchId, DispatchMsg},
        main_ctx::MainContext,
    },
    scene::main::RootScene,
    ui::utils::geom::UISize,
};

pub type GameEvent<'a> = winit::event::Event<'a, GameUserEvent>;
pub type ExecuteCallback =
    Box<dyn FnOnce(&mut MainContext, &mut RootScene) -> anyhow::Result<()> + Send>;

#[derive(Derivative)]
#[derivative(Debug)]
pub enum GameUserEvent {
    Exit,
    Dispatch(DispatchMsg),
    Execute(#[derivative(Debug = "ignore")] ExecuteCallback),
    VSyncSet(Option<SwapInterval>, Option<DispatchId>),
    ExecuteReturn(ExecuteReturnEvent, Option<DispatchId>),
    Error(anyhow::Error),
    CheckedResize {
        display_size: PhysicalSize<NonZeroU32>,
        ui_size: UISize,
    },
}

#[derive(Debug)]
pub enum ExecuteReturnEvent {}
