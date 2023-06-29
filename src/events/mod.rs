use std::num::NonZeroU32;

use derivative::Derivative;
use trait_set::trait_set;
use wgpu::PresentMode;
use winit::dpi::PhysicalSize;

use crate::{
    exec::{dispatch::DispatchMsg, main_ctx::MainContext},
    scene::main::RootScene,
    ui::utils::geom::UISize,
};

pub type GameEvent<'a> = winit::event::Event<'a, GameUserEvent>;

trait_set! {
    pub trait ExecuteCallback = FnOnce(&mut MainContext, &mut RootScene) -> anyhow::Result<()> + Send;
}

#[derive(Derivative)]
#[derivative(Debug)]
pub enum GameUserEvent {
    Exit(i32),
    Dispatch(DispatchMsg),
    Execute(#[derivative(Debug = "ignore")] Box<dyn ExecuteCallback>),
    VSyncSet(Option<PresentMode>),
    ExecuteReturn(ExecuteReturnEvent),
    Error(anyhow::Error),
    CheckedResize {
        display_size: PhysicalSize<NonZeroU32>,
        ui_size: UISize,
    },
}

#[derive(Debug)]
pub enum ExecuteReturnEvent {}
