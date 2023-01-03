use std::{num::NonZeroU32, time::Duration};

use anyhow::Context;
use glutin::surface::SwapInterval;
use image::EncodableLayout;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoopProxy,
};

use crate::{
    display::Display,
    events::{GameEvent, GameUserEvent},
    graphics::{quad_renderer::QuadRenderer, wrappers::texture::TextureHandle},
    utils::{clock::debug_get_time, error::ResultExt},
};

use super::{
    dispatch::{DispatchId, DispatchList, ReturnMechanism},
    executor::GameServerExecutor,
    server::ServerChannels,
};

pub struct MainContext {
    pub display: Display,
    pub event_loop_proxy: EventLoopProxy<GameUserEvent>,
    pub dispatch_list: DispatchList,
    pub channels: ServerChannels,
    pub vsync: bool,
    pub frequency_profiling: bool,
    pub renderer: QuadRenderer,
    pub test_texture: TextureHandle,
}

impl MainContext {
    pub fn new(
        executor: &mut GameServerExecutor,
        display: Display,
        event_loop_proxy: EventLoopProxy<GameUserEvent>,
        dispatch_list: DispatchList,
        mut channels: ServerChannels,
    ) -> anyhow::Result<Self> {
        let renderer = QuadRenderer::new(executor, &mut channels.draw)
            .context("quad renderer initialization failed")?;
        Ok(Self {
            renderer: renderer.clone(),

            test_texture: {
                let tex_handle = channels.draw.generate_id();
                let node_handle = channels.draw.generate_id();
                let img = image::io::Reader::open("BG.jpg")
                    .context("unable to load test texture")?
                    .decode()
                    .context("unable to decode test texture")?
                    .into_rgba8();

                executor
                    .execute_draw(
                        &mut channels.draw,
                        Some(ReturnMechanism::Sync),
                        move |server| {
                            let tex_handle =
                                server.handles.create_texture("test texture", tex_handle)?;
                            unsafe {
                                gl::BindTexture(gl::TEXTURE_2D, tex_handle);
                                gl::TexImage2D(
                                    gl::TEXTURE_2D,
                                    0,
                                    gl::RGBA8.try_into().unwrap(),
                                    img.width().try_into().unwrap(),
                                    img.height().try_into().unwrap(),
                                    0,
                                    gl::RGBA,
                                    gl::UNSIGNED_BYTE,
                                    img.as_bytes().as_ptr() as *const _,
                                );
                                gl::TexParameteri(
                                    gl::TEXTURE_2D,
                                    gl::TEXTURE_MIN_FILTER,
                                    gl::LINEAR.try_into().unwrap(),
                                );
                                gl::TexParameteri(
                                    gl::TEXTURE_2D,
                                    gl::TEXTURE_MAG_FILTER,
                                    gl::LINEAR.try_into().unwrap(),
                                );
                            };

                            server.draw_tree.create_root(node_handle, move |s| {
                                renderer.draw(
                                    s,
                                    tex_handle,
                                    &[[0.0f32, 1.0f32].into(), [1.0f32, 0.0f32].into()],
                                );

                                Ok(())
                            });

                            Ok(Box::new(()))
                        },
                    )
                    .context("unable to initialize test texture (in draw server)")?;

                TextureHandle::from_handle(tex_handle)
            },
            display,
            event_loop_proxy,
            dispatch_list,
            channels,
            vsync: true,
            frequency_profiling: false,
        })
    }

    #[allow(clippy::blocks_in_if_conditions)]
    pub async fn handle_event(
        &mut self,
        executor: &mut GameServerExecutor,
        event: GameEvent<'_>,
    ) -> anyhow::Result<()> {
        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
            } if self.display.get_window_id() == window_id => {
                self.event_loop_proxy.send_event(GameUserEvent::Exit)?;
            }

            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(size),
            } if self.display.get_window_id() == window_id => {
                let width = NonZeroU32::new(size.width);
                let height = NonZeroU32::new(size.height);
                if let Some(width) = width {
                    if let Some(height) = height {
                        self.channels.draw.resize(PhysicalSize { width, height })?;
                    }
                }
            }

            Event::WindowEvent {
                window_id,
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Released,
                                virtual_keycode: Some(VirtualKeyCode::Q),
                                ..
                            },
                        ..
                    },
            } if self.display.get_window_id() == window_id => {
                self.frequency_profiling = !self.frequency_profiling;
                self.channels
                    .update
                    .set_frequency_profiling(self.frequency_profiling)?;
                self.channels
                    .draw
                    .set_frequency_profiling(self.frequency_profiling)?;
                self.channels
                    .audio
                    .set_frequency_profiling(self.frequency_profiling)?;
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
            } if self.display.get_window_id() == window_id => {
                self.vsync = !self.vsync;
                let interval = if self.vsync {
                    SwapInterval::Wait(NonZeroU32::new(1).unwrap())
                } else {
                    SwapInterval::DontWait
                };
                if executor
                    .execute_draw(
                        &mut self.channels.draw,
                        Some(ReturnMechanism::Sync),
                        move |s| {
                            s.set_swap_interval(interval)?;
                            Ok(Box::new(()))
                        },
                    )
                    .with_context(|| format!("unable to set vsync swap interval to {:?}", interval))
                    .log_warn()
                    .is_some()
                {
                    tracing::info!(
                        "VSync swap interval set to {} ({:?})",
                        interval != SwapInterval::DontWait,
                        interval
                    );
                }
            }

            Event::WindowEvent {
                window_id,
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Released,
                                virtual_keycode: Some(VirtualKeyCode::E),
                                ..
                            },
                        ..
                    },
            } if self.display.get_window_id() == window_id => {
                let time = debug_get_time();
                let test_duration = 5.0;
                tracing::info!("{}", time);
                self.set_timeout(Duration::from_secs_f64(test_duration), move |_, _| {
                    tracing::info!("delay: {}s", debug_get_time() - time - test_duration);
                })?;
            }

            Event::UserEvent(GameUserEvent::Dispatch(msg)) => {
                self.dispatch_list
                    .handle_dispatch_msg(msg)
                    .into_iter()
                    .for_each(|(id, d)| {
                        d(self, id);
                    });
            }

            _ => {}
        };
        Ok(())
    }

    pub fn set_timeout<F>(&mut self, timeout: Duration, callback: F) -> anyhow::Result<DispatchId>
    where
        F: FnOnce(&mut MainContext, DispatchId) + 'static,
    {
        let id = self.dispatch_list.push(callback);
        self.channels.update.set_timeout(timeout, id)?;
        Ok(id)
    }
}
