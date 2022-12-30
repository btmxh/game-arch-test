use std::num::NonZeroU32;

use anyhow::Context;
use display::Display;
use exec::{
    executor::GameServerExecutor,
    runner::MAIN_RUNNER_ID,
    server::{audio, draw, update, ServerChannels, ServerKind},
};
use futures::executor::block_on;
use glutin::surface::SwapInterval;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
};

use crate::exec::server::draw::DrawServerChannel;

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
    let mut channels = ServerChannels {
        audio: audio_channels,
        draw: draw_channels,
        update: update_channels,
    };
    executor.move_server(MAIN_RUNNER_ID, 0, ServerKind::Audio)?;
    executor.move_server(MAIN_RUNNER_ID, 0, ServerKind::Update)?;
    executor.move_server(MAIN_RUNNER_ID, 1, exec::server::ServerKind::Draw)?;
    let mut vsync = true;
    executor.run(event_loop, move |e| {
        block_on((|| async {
            match e {
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::CloseRequested,
                } if display.get_window_id() == window_id => {
                    event_loop_proxy.send_event(())?;
                }

                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::Resized(size),
                } if display.get_window_id() == window_id => {
                    // channels.draw.send();
                }

                Event::WindowEvent {
                    window_id,
                    event:
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Released,
                                    virtual_keycode: Some(VirtualKeyCode::R),
                                    ..
                                },
                            ..
                        },
                } => {
                    vsync = !vsync;
                    channels
                        .draw
                        .set_vsync(if vsync {
                            SwapInterval::DontWait
                        } else {
                            SwapInterval::Wait(NonZeroU32::new(1).unwrap())
                        })
                        .await?;
                }

                _ => {}
            };
            Ok(())
        })())
    });
}

async fn handle_event() {

}
