use crate::{exec::main_ctx::MainContext, scene::SceneContainer};

pub fn new(_main_ctx: &mut MainContext) -> anyhow::Result<SceneContainer> {
    let container = SceneContainer::new();
    // container.push_arc(Background::new(main_ctx).context("unable to initialize background scene")?);
    Ok(container)
}
