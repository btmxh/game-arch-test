use std::num::NonZeroU32;

use anyhow::Context;
use glutin::surface::SwapInterval;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

use crate::{events::GameEvent, exec::main_ctx::MainContext, utils::error::ResultExt};

pub struct VSync {
    current_vsync: bool,
}

impl VSync {
    pub fn new(main_ctx: &mut MainContext) -> anyhow::Result<Self> {
        let mut slf = Self {
            current_vsync: false,
        };
        slf.toggle(main_ctx)?; // current_mode is now true
        Ok(slf)
    }

    pub fn toggle(&mut self, main_ctx: &mut MainContext) -> anyhow::Result<()> {
        self.current_vsync = !self.current_vsync;
        let interval = if self.current_vsync {
            SwapInterval::Wait(NonZeroU32::new(1).unwrap())
        } else {
            SwapInterval::DontWait
        };
        main_ctx.channels.draw.execute_draw_event(move |s, _| {
            s.set_swap_interval(interval)
                .with_context(|| format!("unable to set vsync swap interval to {interval:?}"))
                .log_error();
            tracing::info!(
                "VSync swap interval set to {} ({:?})",
                interval != SwapInterval::DontWait,
                interval
            );
            []
        })?;

        Ok(())
    }

    pub fn handle_event(
        &mut self,

        main_ctx: &mut MainContext,
        event: &GameEvent,
    ) -> anyhow::Result<bool> {
        match event {
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
            } if main_ctx.display.get_window_id() == *window_id => {
                self.toggle(main_ctx)?;
            }

            _ => {}
        }

        Ok(false)
    }
}
