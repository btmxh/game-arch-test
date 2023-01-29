use anyhow::Context;
use display::Display;
use events::GameUserEvent;
use exec::{
    executor::GameServerExecutor,
    main_ctx::MainContext,
    runner::MAIN_RUNNER_ID,
    server::{audio, draw, update, ServerChannels, ServerKind},
};
use scene::main::EventRoot;
use utils::{args::parse_args, log::init_log};
use winit::{dpi::PhysicalSize, event_loop::EventLoopBuilder};

pub mod display;
pub mod events;
pub mod exec;
pub mod graphics;
pub mod scene;
pub mod ui;
pub mod utils;

fn main() -> anyhow::Result<()> {
    parse_args();
    let guard = init_log()?;
    let event_loop = EventLoopBuilder::<GameUserEvent>::with_user_event().build();
    let (display, gl_config) =
        Display::new_display(&event_loop, PhysicalSize::new(1280, 720), "hello")
            .context("unable to create main display")?;
    let (draw, draw_channels) =
        draw::SendServer::new(event_loop.create_proxy(), gl_config, &display)
            .context("unable to initialize draw server")?;
    let (audio, audio_channels) = audio::Server::new(event_loop.create_proxy());
    let (update, update_channels) = update::Server::new(event_loop.create_proxy());
    let mut executor = GameServerExecutor::new(audio, draw, update)?;
    let event_loop_proxy = event_loop.create_proxy();
    let channels = ServerChannels {
        audio: audio_channels,
        draw: draw_channels,
        update: update_channels,
    };
    executor.move_server(MAIN_RUNNER_ID, 0, ServerKind::Audio)?;
    executor.move_server(MAIN_RUNNER_ID, 0, ServerKind::Update)?;
    executor.move_server(MAIN_RUNNER_ID, 1, ServerKind::Draw)?;
    executor.set_frequency(0, 1000.0)?;
    let mut main_ctx = MainContext::new(&mut executor, display, event_loop_proxy, channels)?;
    let root_scene = EventRoot::new(&mut executor, &mut main_ctx)?;
    executor.run(event_loop, main_ctx, root_scene, guard);
}
