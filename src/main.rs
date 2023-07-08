#![allow(dead_code)]

use anyhow::Context;
use context::{
    common::CommonContext, draw::GraphicsContext, event::EventContext, init::InitContext,
};
use exec::{
    executor::GameServerExecutor,
    runner::MAIN_RUNNER_ID,
    server::{audio, draw, update, ServerKind},
};
use scene::main::RootScene;
use utils::log::init_log;
use winit::event_loop::EventLoopBuilder;

mod context;
mod display;
mod events;
mod exec;
mod graphics;
mod scene;
mod test;
mod utils;

fn main() -> anyhow::Result<()> {
    let _guard = init_log()?;
    let event_loop = EventLoopBuilder::with_user_event().build();
    let common_context =
        CommonContext::new(&event_loop).context("Unable to initialize CommonContext")?;
    let (mut event_context, draw_recv, audio_recv, update_recv) =
        EventContext::new(common_context, &event_loop)
            .context("Unable to initialize EventContext")?;
    let mut graphics_context: GraphicsContext = event_context
        .create_graphics_context()
        .context("Unable to create graphics context")?;

    let (root_scene, executor_args) = {
        let mut init_context = InitContext::new(&mut event_context, &mut graphics_context);
        (
            RootScene::new(&mut init_context).context("Unable to initialize game scenes")?,
            init_context.executor_args,
        )
    };

    let draw = draw::Server::new(
        event_context.event_sender.clone(),
        draw_recv,
        graphics_context,
        root_scene.clone(),
    )
    .context("unable to initialize draw server")?;
    let audio = audio::Server::new(event_context.event_sender.clone(), audio_recv);
    let update = update::Server::new(event_context.event_sender.clone(), update_recv);

    let mut executor = GameServerExecutor::new(executor_args, audio, draw, update)?;
    executor.move_server(MAIN_RUNNER_ID, 0, ServerKind::Audio)?;
    executor.move_server(MAIN_RUNNER_ID, 0, ServerKind::Update)?;
    executor.move_server(MAIN_RUNNER_ID, 1, ServerKind::Draw)?;
    executor.set_frequency(0, 1000.0)?;

    event_context.run(executor, event_loop, root_scene)
}
