use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Context;
use wgpu::PresentMode;
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    keyboard::KeyCode,
};

use crate::{
    context::{
        common::HasCommonContext,
        event::{EventDispatchContext, Executable},
    },
    events::GameEvent,
    graphics::SurfaceContext,
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
                        event:
                            KeyEvent {
                                state: ElementState::Released,
                                physical_key: KeyCode::KeyE,
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
                    .execute_draw(|context| {
                        if let Some(SurfaceContext { surface, .. }) =
                            context.graphics.surface_context.as_ref()
                        {
                            for present_mode in surface
                                .get_capabilities(&context.graphics.adapter)
                                .present_modes
                            {
                                tracing::info!("Supported present mode: {:?}", present_mode);
                            }
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
        let vsync = self.current_vsync.fetch_xor(true, Ordering::Relaxed);
        context.event.draw_sender.execute(move |context| {
            if let Some(surface_context) = context.graphics.surface_context.as_mut() {
                let supported_present_modes = surface_context.surface.get_capabilities(&context.graphics.adapter).present_modes;
                surface_context.config.present_mode = match vsync {
                    true => {
                        if supported_present_modes.contains(&PresentMode::FifoRelaxed) {
                            PresentMode::FifoRelaxed
                        } else {
                            PresentMode::Fifo
                        }
                    }

                    false => {
                        if supported_present_modes.contains(&PresentMode::Immediate) {
                            PresentMode::Immediate
                        } else {
                            tracing::warn!("Immediate present mode not supported. Considering running the draw server on a separate runner thread ");
                            if supported_present_modes.contains(&PresentMode::Mailbox) {
                                PresentMode::Mailbox
                            } else {
                                tracing::error!("Immediate/Mailbox present modes not supported. Falling back to Fifo (VSync on)");
                                PresentMode::Fifo
                            }
                        }
                    }
                };
                tracing::info!("VSync set to {}, present mode {:?}", vsync, surface_context.config.present_mode);
                surface_context.configure(&context.graphics.device);
            } else {
                tracing::warn!("Attempting to set vsync to {vsync} while surface is not present");
            }
        })?;

        Ok(())
    }
}
