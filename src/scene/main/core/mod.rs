use crate::{exec::main_ctx::MainContext, scene::SceneContainer};

pub mod clear;
pub mod redraw;

pub fn new(_: &mut MainContext) -> anyhow::Result<SceneContainer> {
    let mut container = SceneContainer::new();
    container.push_event_handler(redraw::handle_event);
    container.push(clear::Clear);
    Ok(container)
}
