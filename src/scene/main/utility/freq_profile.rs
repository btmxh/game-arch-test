use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

use crate::{
    events::GameEvent,
    exec::{executor::GameServerExecutor, main_ctx::MainContext},
};

pub struct FreqProfile {
    current_freq_profile: bool,
}

impl FreqProfile {
    pub fn new(_: &mut GameServerExecutor, _: &mut MainContext) -> anyhow::Result<Self> {
        Ok(Self {
            current_freq_profile: false,
        })
    }

    pub fn toggle(
        &mut self,
        _executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
    ) -> anyhow::Result<()> {
        self.current_freq_profile = !self.current_freq_profile;
        main_ctx
            .channels
            .update
            .set_frequency_profiling(self.current_freq_profile)?;
        main_ctx
            .channels
            .draw
            .set_frequency_profiling(self.current_freq_profile)?;
        main_ctx
            .channels
            .audio
            .set_frequency_profiling(self.current_freq_profile)?;

        Ok(())
    }

    pub fn handle_event(
        &mut self,
        executor: &mut GameServerExecutor,
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
                                virtual_keycode: Some(VirtualKeyCode::Q),
                                ..
                            },
                        ..
                    },
            } if main_ctx.display.get_window_id() == *window_id => {
                self.toggle(executor, main_ctx)?;
            }

            _ => {}
        }

        Ok(false)
    }
}
