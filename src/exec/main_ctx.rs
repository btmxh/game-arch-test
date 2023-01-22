use std::{num::NonZeroU32, time::Duration};

use anyhow::Context;
use glutin::surface::SwapInterval;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoopProxy,
};

use crate::{
    display::Display,
    events::{GameEvent, GameUserEvent},
    graphics::wrappers::vertex_array::VertexArrayHandle,
    scene::main::EventRoot,
    utils::{args::args, clock::debug_get_time, error::ResultExt},
};

use super::{
    dispatch::{DispatchId, DispatchList},
    executor::GameServerExecutor,
    server::ServerChannels,
    task::CancellationToken,
};

pub struct MainContext {
    pub dummy_vao: VertexArrayHandle,
    pub frequency_profiling: bool,
    pub vsync: bool,
    pub channels: ServerChannels,
    pub dispatch_list: DispatchList,
    pub event_loop_proxy: EventLoopProxy<GameUserEvent>,
    pub display: Display,
    // resize throttling
    // port of https://blog.webdevsimplified.com/2022-03/debounce-vs-throttle/
    resize_should_wait: bool,
    resize_size: Option<PhysicalSize<NonZeroU32>>,
}

impl MainContext {
    pub fn new(
        _executor: &mut GameServerExecutor,
        display: Display,
        event_loop_proxy: EventLoopProxy<GameUserEvent>,
        mut channels: ServerChannels,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            dummy_vao: VertexArrayHandle::new(&mut channels.draw, "dummy vertex array")?,
            display,
            event_loop_proxy,
            dispatch_list: DispatchList::new(),
            channels,
            vsync: true,
            frequency_profiling: false,
            resize_should_wait: false,
            resize_size: None,
        })
    }

    fn resize(
        &mut self,
        executor: &mut GameServerExecutor,
        root_scene: &mut EventRoot,
        size: PhysicalSize<NonZeroU32>,
        block: bool,
    ) -> anyhow::Result<()> {
        if block {
            executor.execute_draw_sync(&mut self.channels.draw, move |context, _| {
                context.resize(size);
                Ok(())
            })?;
        } else {
            GameServerExecutor::execute_draw_event(&mut self.channels.draw, move |context, _| {
                context.resize(size);
                []
            })?;
        }
        root_scene.handle_event(
            executor,
            self,
            GameEvent::UserEvent(GameUserEvent::CheckedResize(size)),
        )?;
        // self.update_blur_texture(32.0)?;
        Ok(())
    }

    #[allow(clippy::blocks_in_if_conditions)]
    pub fn handle_event(
        &mut self,
        executor: &mut GameServerExecutor,
        root_scene: &mut EventRoot,
        event: GameEvent<'_>,
    ) -> anyhow::Result<()> {
        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
            } if self.display.get_window_id() == window_id => {
                self.event_loop_proxy
                    .send_event(GameUserEvent::Exit)
                    .map_err(|e| anyhow::format_err!("{}", e))
                    .context("unable to send event to event loop")?;
            }

            Event::RedrawRequested(window_id) if self.display.get_window_id() == window_id => {
                // somewhat hacky way of waiting a buffer swap
                if args().block_event_loop {
                    if executor.main_runner.base.container.draw.is_some() {
                        executor
                            .main_runner
                            .base
                            .run_single()
                            .expect("error running main runner");
                    } else {
                        executor.execute_draw_sync(&mut self.channels.draw, |_, _| Ok(()))?;
                        executor.execute_draw_sync(&mut self.channels.draw, |_, _| Ok(()))?;
                    }
                }
            }

            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(size),
            } if self.display.get_window_id() == window_id => {
                let width = NonZeroU32::new(size.width);
                let height = NonZeroU32::new(size.height);
                let size =
                    width.and_then(|width| height.map(|height| PhysicalSize::new(width, height)));
                if let Some(size) = size {
                    if args().throttle_resize {
                        // throttle
                        const THROTTLE_DURATION: Duration = Duration::from_millis(100);
                        fn resize_timeout_func(
                            slf: &mut MainContext,
                            executor: &mut GameServerExecutor,
                            root_scene: &mut EventRoot,
                        ) -> anyhow::Result<()> {
                            if let Some(size) = slf.resize_size.take() {
                                slf.resize(executor, root_scene, size, false)?;
                                slf.resize_size = None;
                                slf.set_timeout(
                                    THROTTLE_DURATION,
                                    |slf, executor, root_scene, _| {
                                        resize_timeout_func(slf, executor, root_scene)
                                    },
                                )?;
                            } else {
                                slf.resize_should_wait = false;
                            }

                            Ok(())
                        }

                        if self.resize_should_wait {
                            self.resize_size = Some(size);
                        } else {
                            self.resize(executor, root_scene, size, false)?;
                            self.resize_should_wait = true;
                            self.set_timeout(THROTTLE_DURATION, |slf, executor, root_scene, _| {
                                resize_timeout_func(slf, executor, root_scene)
                            })?;
                        }
                    } else {
                        self.resize(executor, root_scene, size, !args().block_event_loop)?;
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
                    .execute_draw_sync(&mut self.channels.draw, move |s, _| {
                        s.set_swap_interval(interval)?;
                        Ok(Box::new(()))
                    })
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
                self.set_timeout(Duration::from_secs_f64(test_duration), move |_, _, _, _| {
                    tracing::info!("delay: {}s", debug_get_time() - time - test_duration);
                    Ok(())
                })?;
            }

            Event::UserEvent(GameUserEvent::Dispatch(msg)) => {
                for (id, d) in self.dispatch_list.handle_dispatch_msg(msg).into_iter() {
                    d(self, executor, root_scene, id)?;
                }
            }

            Event::UserEvent(GameUserEvent::Execute(callback)) => {
                callback(self, executor, root_scene).log_error();
            }

            Event::UserEvent(GameUserEvent::Error(e)) => {
                tracing::error!("GameUserEvent::Error caught: {}", e);
            }

            _ => {}
        };
        Ok(())
    }

    pub fn set_timeout<F>(
        &mut self,
        timeout: Duration,
        callback: F,
    ) -> anyhow::Result<(DispatchId, CancellationToken)>
    where
        F: FnOnce(
                &mut MainContext,
                &mut GameServerExecutor,
                &mut EventRoot,
                DispatchId,
            ) -> anyhow::Result<()>
            + 'static,
    {
        let cancel_token = CancellationToken::new();
        let id = self.dispatch_list.push(callback, cancel_token.clone());
        self.channels.update.set_timeout(timeout, id)?;
        Ok((id, cancel_token))
    }
}
