use std::{num::NonZeroU32, time::Duration};

use glutin::surface::SwapInterval;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoopProxy,
};

use crate::{
    display::Display,
    events::{GameEvent, GameUserEvent},
    utils::clock::debug_get_time,
};

use super::{
    dispatch::{DispatchId, DispatchList},
    server::ServerChannels,
};

pub struct MainContext {
    pub display: Display,
    pub event_loop_proxy: EventLoopProxy<GameUserEvent>,
    pub dispatch_list: DispatchList,
    pub channels: ServerChannels,
    pub vsync: bool,
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
                                virtual_keycode: Some(VirtualKeyCode::R),
                                ..
                            },
                        ..
                    },
            } if self.display.get_window_id() == window_id => {
                self.vsync = !self.vsync;
                self.channels
                    .draw
                    .set_vsync(if self.vsync {
                        SwapInterval::Wait(NonZeroU32::new(1).unwrap())
                    } else {
                        SwapInterval::DontWait
                    })
                    .await?;
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
                println!("{}", time);
                self.set_timeout(Duration::from_secs(5), move |_| {
                    println!("hello {}", debug_get_time() - time - 5.0)
                })?;
            }

            Event::UserEvent(GameUserEvent::Dispatch(msg)) => {
                self.dispatch_list
                    .handle_dispatch_msg(msg)
                    .into_iter()
                    .for_each(|d| d(self));
            }

            _ => {}
        };
        Ok(())
    }

    pub fn set_timeout<F>(&mut self, timeout: Duration, callback: F) -> anyhow::Result<DispatchId>
    where
        F: FnOnce(&mut MainContext) + 'static,
    {
        let id = self.dispatch_list.push(callback);
        self.channels.update.set_timeout(timeout, id)?;
        Ok(id)
    }
}
