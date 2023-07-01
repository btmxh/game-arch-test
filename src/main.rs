#![feature(result_flattening)]

use anyhow::Context;
use context::{draw::GraphicsContext, event::EventContext, init::InitContext};
use display::Display;
use events::GameUserEvent;
use exec::{
    executor::GameServerExecutor,
    runner::MAIN_RUNNER_ID,
    server::{audio, draw, update, ServerKind},
};
use scene::main::RootScene;
use utils::{args::parse_args, log::init_log};
use winit::{dpi::PhysicalSize, event_loop::EventLoopBuilder};

pub mod context;
pub mod display;
pub mod events;
pub mod exec;
pub mod graphics;
pub mod scene;
pub mod test;
pub mod utils;

fn main() -> anyhow::Result<()> {
    parse_args();
    let guard = init_log()?;
    let event_loop = EventLoopBuilder::<GameUserEvent>::with_user_event().build();
    let display = Display::new_display(&event_loop, PhysicalSize::new(1280, 720), "hello")
        .context("unable to create main display")?;

    let (mut event_context, draw_recv, audio_recv, update_recv) =
        EventContext::new(display, event_loop.create_proxy())
            .context("Unable to initialize EventContext")?;
    let mut graphics_context = pollster::block_on(GraphicsContext::new(
        event_loop.create_proxy(),
        &event_context.display,
        draw_recv,
    ))
    .context("Unable to create graphics context")?;

    let (root_scene, executor_args) = {
        let mut init_context = InitContext::new(&mut event_context, &mut graphics_context);
        (
            RootScene::new(&mut init_context).context("Unable to initialize game scenes")?,
            init_context.executor_args,
        )
    };

    let draw = draw::Server::new(graphics_context, root_scene.clone())
        .context("unable to initialize draw server")?;
    let audio = audio::Server::new(event_loop.create_proxy(), audio_recv);
    let update = update::Server::new(event_loop.create_proxy(), update_recv);

    let mut executor = GameServerExecutor::new(executor_args, audio, draw, update)?;
    executor.move_server(MAIN_RUNNER_ID, 0, ServerKind::Audio)?;
    executor.move_server(MAIN_RUNNER_ID, 0, ServerKind::Update)?;
    executor.move_server(MAIN_RUNNER_ID, 1, ServerKind::Draw)?;
    executor.set_frequency(0, 1000.0)?;

    event_context.run(executor, event_loop, root_scene, guard)
}
