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
    utils::{clock::debug_get_time, error::ResultExt},
};

use super::{
    dispatch::{DispatchId, DispatchList, ReturnMechanism},
    server::ServerChannels,
};

pub struct MainContext {
    pub display: Display,
    pub event_loop_proxy: EventLoopProxy<GameUserEvent>,
    pub dispatch_list: DispatchList,
    pub channels: ServerChannels,
    pub vsync: bool,
    pub frequency_profiling: bool,
}

impl MainContext {
    pub fn new(
        display: Display,
        event_loop_proxy: EventLoopProxy<GameUserEvent>,
        dispatch_list: DispatchList,
        channels: ServerChannels,
    ) -> Self {
        Self {
            display,
            event_loop_proxy,
            dispatch_list,
            channels,
            vsync: true,
            frequency_profiling: false,
        }
    }
    pub async fn handle_event(&mut self, event: GameEvent<'_>) -> anyhow::Result<()> {
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
                if let Some(Some(new_interval)) = self
                    .channels
                    .draw
                    .set_vsync(interval, Some(ReturnMechanism::Sync))
                    .with_context(|| format!("unable to set vsync swap interval to {:?}", interval))
                    .log_warn()
                {
                    if interval != new_interval {
                        tracing::warn!(
                            "unable to set vsync swap interval to {:?}, falling back to {:?} instead",
                            interval,
                            new_interval
                        );
                    } else {
                        tracing::info!("vsync swap interval set to {:?}", new_interval);
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
