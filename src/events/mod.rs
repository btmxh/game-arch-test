pub type GameEvent<'a> = winit::event::Event<'a, GameUserEvent>;

#[derive(Debug)]
pub enum GameUserEvent {
    Exit,
    SetTimeoutDispatch(Vec<u64>),
}
