use std::time::Duration;

use rand::{thread_rng, Rng};
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

use crate::{
    events::GameEvent,
    exec::{executor::GameServerExecutor, main_ctx::MainContext}, utils::clock::debug_get_time, scene::main::EventRoot,
};

pub struct UpdateDelayTest {
    num_tests: usize,
    running_avg: f64,
}

impl UpdateDelayTest {
    pub fn new(_: &mut GameServerExecutor, _: &mut MainContext) -> anyhow::Result<Self> {
        Ok(Self {
            num_tests: 0,
            running_avg: 0.0,
        })
    }

    pub fn get(root_scene: &mut EventRoot) -> &mut Self {
        &mut root_scene.utility.update_delay_test
    }

    fn add_delay(&mut self, delay: f64) {
        let num_delay = self.num_tests as f64;
        self.running_avg = (self.running_avg * num_delay + delay) / (num_delay + 1.0);
        self.num_tests += 1;
    }

    pub fn test(
        &mut self,
        _executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
    ) -> anyhow::Result<()> {
        let time = debug_get_time();
        let test_duration = thread_rng().gen_range(5.0..10.0);
        tracing::info!("{}", time);
        main_ctx.set_timeout(Duration::from_secs_f64(test_duration), move |_, _, root_scene, _| {
            let delay = debug_get_time() - time - test_duration;
            let slf = Self::get(root_scene);
            slf.add_delay(delay);
            tracing::info!("delay: {}s, avg: {}s", delay, slf.running_avg);
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
                                virtual_keycode: Some(VirtualKeyCode::R),
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
