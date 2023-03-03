use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Context;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

use crate::{
    events::GameEvent,
    exec::{main_ctx::MainContext, server::draw::ServerSendChannelExt},
    scene::{main::RootScene, Scene},
    utils::error::ResultExt,
};

pub struct FreqProfile {
    current_freq_profile: AtomicBool,
}

impl Scene for FreqProfile {
    fn handle_event<'a>(
        self: std::sync::Arc<Self>,
        ctx: &mut MainContext,
        _: &RootScene,
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
            } if ctx.display.get_window_id() == *window_id => {
                self.toggle(ctx)
                    .context("unable to toggle frequency profile mode")
                    .log_error();
            }

            _ => {}
        }

        Some(event)
    }
}

impl FreqProfile {
    pub fn new() -> Self {
        Self {
            current_freq_profile: AtomicBool::new(false),
        }
    }

    pub fn toggle(&self, main_ctx: &mut MainContext) -> anyhow::Result<()> {
        let current_freq_profile = !self.current_freq_profile.load(Ordering::Relaxed);
        self.current_freq_profile
            .store(current_freq_profile, Ordering::Relaxed);
        main_ctx
            .channels
            .update
            .set_frequency_profiling(current_freq_profile)?;
        main_ctx
            .channels
            .draw
            .set_frequency_profiling(current_freq_profile)?;
        main_ctx
            .channels
            .audio
            .set_frequency_profiling(current_freq_profile)?;

        Ok(())
    }
}

impl Default for FreqProfile {
    fn default() -> Self {
        Self::new()
    }
}
