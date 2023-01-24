use std::time::Duration;

use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

use crate::{
    events::GameEvent,
    exec::{executor::GameServerExecutor, main_ctx::MainContext}, utils::clock::debug_get_time,
};

pub struct UpdateDelayTest;

impl UpdateDelayTest {
    pub fn new(_: &mut GameServerExecutor, _: &mut MainContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn test(
        &mut self,
        _executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
    ) -> anyhow::Result<()> {
        let time = debug_get_time();
        let test_duration = 5.0;
        tracing::info!("{}", time);
        main_ctx.set_timeout(Duration::from_secs_f64(test_duration), move |_, _, _, _| {
            tracing::info!("delay: {}s", debug_get_time() - time - test_duration);
            Ok(())
        })?;

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
                self.test(executor, main_ctx)?;
            }

            _ => {}
        }

        Ok(false)
    }
}
