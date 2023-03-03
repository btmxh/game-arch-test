use std::{sync::Arc, time::Duration};

use anyhow::Context;
use rand::{thread_rng, Rng};
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

use crate::{
    events::GameEvent,
    exec::main_ctx::MainContext,
    scene::{main::RootScene, Scene},
    utils::{clock::debug_get_time, error::ResultExt, mutex::Mutex},
};

#[derive(Default)]
pub struct AverageDelay {
    num_tests: usize,
    running_avg: f64,
}

pub struct UpdateDelayTest {
    delay: Mutex<AverageDelay>,
}

impl Scene for UpdateDelayTest {
    fn handle_event<'a>(
        self: Arc<Self>,
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
                                virtual_keycode: Some(VirtualKeyCode::R),
                                ..
                            },
                        ..
                    },
            } if ctx.display.get_window_id() == *window_id => {
                self.test(ctx)
                    .context("error while doing update delay test")
                    .log_error();
            }

            _ => {}
        }

        Some(event)
    }
}

impl UpdateDelayTest {
    pub fn new() -> Self {
        Self {
            delay: Mutex::new(AverageDelay::default()),
        }
    }

    pub fn test(self: Arc<Self>, main_ctx: &mut MainContext) -> anyhow::Result<()> {
        let time = debug_get_time();
        let test_duration = thread_rng().gen_range(5.0..10.0);
        tracing::info!("{}", time);
        main_ctx.set_timeout(Duration::from_secs_f64(test_duration), move |_, _| {
            let delay = debug_get_time() - time - test_duration;
            let running_avg = self.delay.lock().add_delay(delay);
            tracing::info!("delay: {}s, avg: {}s", delay, running_avg);
            Ok(())
        })?;

        Ok(())
    }
}

impl AverageDelay {
    fn add_delay(&mut self, delay: f64) -> f64 {
        let num_delay = self.num_tests as f64;
        self.running_avg = (self.running_avg * num_delay + delay) / (num_delay + 1.0);
        self.num_tests += 1;
        self.running_avg
    }
}

impl Default for UpdateDelayTest {
    fn default() -> Self {
        Self::new()
    }
}
