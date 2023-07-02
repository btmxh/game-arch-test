use std::{sync::Arc, time::Duration};

use anyhow::Context;
use rand::{thread_rng, Rng};
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

use crate::{
    context::event::EventDispatchContext,
    events::GameEvent,
    utils::{clock::debug_get_time, error::ResultExt, mutex::Mutex},
};

#[derive(Default)]
pub struct AverageDelay {
    num_tests: usize,
    running_avg: f64,
}

pub struct Scene {
    delay: Mutex<AverageDelay>,
}

pub type ArcScene = Arc<Scene>;

impl Scene {}

impl Scene {
    pub fn new() -> ArcScene {
        Arc::new(Self {
            delay: Mutex::new(AverageDelay::default()),
        })
    }

    pub fn test(self: ArcScene, context: &mut EventDispatchContext) -> anyhow::Result<()> {
        let time = debug_get_time();
        let test_duration = thread_rng().gen_range(5.0..10.0);
        tracing::info!("{}", time);
        context.event.update_sender.set_timeout(
            Duration::from_secs_f64(test_duration),
            move |_| {
                let delay = debug_get_time() - time - test_duration;
                let running_avg = self.delay.lock().add_delay(delay);
                tracing::info!("delay: {}s, avg: {}s", delay, running_avg);
            },
        )?;

        Ok(())
    }

    pub fn handle_event<'a>(
        self: ArcScene,
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
                                virtual_keycode: Some(VirtualKeyCode::R),
                                ..
                            },
                        ..
                    },
            } if context.event.display.get_window_id() == *window_id => {
                self.test(context)
                    .context("error while doing update delay test")
                    .log_error();
            }

            _ => {}
        }

        Some(event)
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
