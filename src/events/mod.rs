use glutin::surface::SwapInterval;

use crate::exec::dispatch::{DispatchId, DispatchMsg};

pub type GameEvent<'a> = winit::event::Event<'a, GameUserEvent>;

#[derive(Debug)]
pub enum GameUserEvent {
    Exit,
    Dispatch(DispatchMsg),
    VSyncSet(Option<SwapInterval>, Option<DispatchId>),
}
