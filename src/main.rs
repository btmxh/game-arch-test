use anyhow::Context;
use display::Display;
use exec::{
    executor::GameServerExecutor,
    server::{audio, draw, update, ServerChannels}, runner::MAIN_RUNNER_ID,
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
};

pub mod display;
pub mod exec;
pub mod utils;

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new();
    let (display, gl_config) =
        Display::new_display(&event_loop, PhysicalSize::new(1280, 720), "hello")
            .context("unable to create main display")?;
    let (draw, draw_channels) =
        draw::SendServer::new(gl_config, &display).context("unable to initialize draw server")?;
    let (audio, audio_channels) = audio::Server::new();
    let (update, update_channels) = update::Server::new();
    let mut executor = GameServerExecutor::new(audio, draw, update)?;
    let event_loop_proxy = event_loop.create_proxy();
    let _channels = ServerChannels {
        audio: audio_channels,
        draw: draw_channels,
        update: update_channels,
    };
    executor.move_server(MAIN_RUNNER_ID, 1, exec::server::ServerKind::Draw)?;
    executor.run(event_loop, move |e| {
        match e {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
            } if display.get_window_id() == window_id => {
                event_loop_proxy.send_event(())?;
            }

            _ => {}
        };
        Ok(())
    });
}
