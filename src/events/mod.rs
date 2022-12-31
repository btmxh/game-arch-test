use crate::exec::dispatch::DispatchMsg;

pub type GameEvent<'a> = winit::event::Event<'a, GameUserEvent>;

#[derive(Debug)]
pub enum GameUserEvent {
    Exit,
    Dispatch(DispatchMsg),
}
