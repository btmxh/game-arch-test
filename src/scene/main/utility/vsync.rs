use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Context;
use wgpu::PresentMode;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

use crate::{
    context::{
        common::HasCommonContext,
        event::{EventDispatchContext, Executable},
    },
    events::GameEvent,
    utils::error::ResultExt,
};

pub struct Scene {
    current_vsync: AtomicBool,
}

impl Scene {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            current_vsync: AtomicBool::new(true),
        })
    }

    pub fn handle_event<'a>(
        &self,
        context: &mut EventDispatchContext,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        match &event {
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
            } if context.check_window_id(window_id) => {
                self.toggle(context)
                    .context("unable to toggle VSync mode")
                    .log_warn();
            }

            Event::Resumed => {
                context
                    .execute_draw_sync(|context| {
                        let present_modes = context
                            .graphics
                            .surface_context
                            // TODO: make this more safe
                            .as_ref()
                            .expect("surface should be (re)created now")
                            .surface
                            .get_capabilities(&context.graphics.adapter)
                            .present_modes;
                        for present_mode in present_modes {
                            tracing::info!("Supported present mode: {:?}", present_mode);
                        }
                    })
                    .context("Unable to list present modes")
                    .log_error();
            }

            _ => {}
        };

        Some(event)
    }

    fn toggle(&self, context: &EventDispatchContext) -> anyhow::Result<()> {
        let current_vsync = !self.current_vsync.load(Ordering::Relaxed);
        self.current_vsync.store(current_vsync, Ordering::Relaxed);
        let interval = if current_vsync {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        };
        context.event.draw_sender.execute(move |context| {
            context.graphics.set_swap_interval(interval);
            tracing::info!("VSync swap interval set to {interval:?}");
        })?;

        Ok(())
    }
}
