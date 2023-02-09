use anyhow::Context;

use crate::{exec::main_ctx::MainContext, scene::SceneContainer};

use self::bg::Background;

pub mod bg;

pub fn new(main_ctx: &mut MainContext) -> anyhow::Result<SceneContainer> {
    let mut container = SceneContainer::new();
    container.push_arc(Background::new(main_ctx).context("unable to initialize background scene")?);
    Ok(container)
}
