use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Context;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

use crate::{context::event::EventHandleContext, events::GameEvent, utils::error::ResultExt};

pub struct Scene {
    current_freq_profile: AtomicBool,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            current_freq_profile: AtomicBool::new(false),
        }
    }

    pub fn handle_event<'a>(
        &self,
        context: &mut EventHandleContext,
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
                                virtual_keycode: Some(VirtualKeyCode::Q),
                                ..
                            },
                        ..
                    },
            } if context.event.display.get_window_id() == *window_id => {
                self.toggle(context)
                    .context("unable to toggle frequency profile mode")
                    .log_error();
            }

            _ => {}
        }

        Some(event)
    }

    fn toggle(&self, context: &mut EventHandleContext) -> anyhow::Result<()> {
        let current_freq_profile = !self.current_freq_profile.load(Ordering::Relaxed);
        self.current_freq_profile
            .store(current_freq_profile, Ordering::Relaxed);
        context
            .event
            .channels
            .update
            .set_frequency_profiling(current_freq_profile)?;
        context
            .event
            .channels
            .draw
            .set_frequency_profiling(current_freq_profile)?;
        context
            .event
            .channels
            .audio
            .set_frequency_profiling(current_freq_profile)?;

        Ok(())
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
