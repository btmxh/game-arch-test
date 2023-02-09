use crate::{exec::main_ctx::MainContext, scene::SceneContainer};

use self::redraw::Redraw;

pub mod redraw;

pub fn new(_: &mut MainContext) -> anyhow::Result<SceneContainer> {
    let mut container = SceneContainer::new();
    container.push(Redraw);
    Ok(container)
}
