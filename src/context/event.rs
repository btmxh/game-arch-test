use crate::{
    display::EventSender,
    exec::{
        executor::GameServerExecutor,
        server::{audio, draw, update},
    },
    scene::main::RootScene,
    test::manager::{OptionTestManager, TestManager},
    utils::{error::ResultExt, mpsc::Sender},
};

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use winit::{event::Event, event_loop::EventLoop};

use crate::{
    context::draw::DrawDispatch,
    events::{GameEvent, GameUserEvent},
    utils::mpsc,
};

use super::{
    common::SharedCommonContext,
    draw::{DrawDispatchContext, GraphicsContext},
    update::UpdateSender,
};

pub struct EventContext {
    pub common: SharedCommonContext,
    pub test_manager: OptionTestManager,
    pub event_sender: EventSender,
    pub audio_sender: Sender<audio::Message>,
    pub draw_sender: Sender<draw::Message>,
    pub update_sender: UpdateSender,
}

impl EventContext {
    #[allow(clippy::type_complexity)]
    pub fn new(
        common: SharedCommonContext,
        event_loop: &EventLoop<GameUserEvent>,
    ) -> anyhow::Result<(
        Self,
        mpsc::Receiver<draw::Message>,
        mpsc::Receiver<audio::Message>,
        mpsc::Receiver<update::Message>,
    )> {
        let event_sender = EventSender::new(event_loop);
        let (draw_sender, draw_receiver) = mpsc::channels();
        let (audio_sender, audio_receiver) = mpsc::channels();
        let (mut update_sender, update_receiver) = UpdateSender::new();

        Ok((
            Self {
                common,
                test_manager: TestManager::new_if_enabled(&mut update_sender, &event_sender)
                    .context("unable to create test manager")?,
                event_sender,
                audio_sender,
                draw_sender,
                update_sender,
            },
            draw_receiver,
            audio_receiver,
            update_receiver,
        ))
    }

    pub fn handle_event(
        &mut self,
        executor: &mut GameServerExecutor,
        root_scene: &RootScene,
        event: GameEvent,
    ) -> anyhow::Result<()> {
        match event {
            Event::UserEvent(GameUserEvent::UpdateDispatch(ids)) => {
                let callbacks = ids
                    .into_iter()
                    .filter_map(|id| self.update_sender.pop(id))
                    .collect::<Vec<_>>();
                for callback in callbacks {
                    callback(EventDispatchContext::new(executor, self, root_scene));
                }
            }

            event => {
                root_scene.handle_event(
                    &mut EventDispatchContext::new(executor, self, root_scene),
                    event,
                );
            }
        };
        Ok(())
    }

    pub fn run(
        mut self,
        mut executor: GameServerExecutor,
        event_loop: EventLoop<GameUserEvent>,
        root_scene: Arc<RootScene>,
    ) -> ! {
        use winit::event_loop::ControlFlow;
        event_loop.run(move |event, _target, control_flow| {
            // guarantee drop order
            fn unused<T>(_: &T) {}
            unused(&root_scene);
            unused(&self);
            match event {
                Event::MainEventsCleared => {
                    executor
                        .main_runner
                        .base
                        .run_single(true)
                        .expect("error running main runner");
                }

                Event::UserEvent(GameUserEvent::Exit(code)) => {
                    control_flow.set_exit_with_code(code)
                }

                event => self
                    .handle_event(&mut executor, &root_scene, event)
                    .expect("error handling events"),
            }

            match *control_flow {
                ControlFlow::ExitWithCode(_) => {
                    executor.stop();
                }

                _ => {
                    *control_flow = if executor.main_runner.base.container.does_run() {
                        ControlFlow::Poll
                    } else {
                        ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(100))
                    }
                }
            };
        })
    }

    pub(crate) fn create_graphics_context(&self) -> anyhow::Result<GraphicsContext> {
        pollster::block_on(GraphicsContext::new(self.common.clone()))
    }
}

pub struct EventDispatchContext<'a> {
    pub executor: &'a mut GameServerExecutor,
    pub event: &'a mut EventContext,
    pub root_scene: &'a RootScene,
}

impl<'a> EventDispatchContext<'a> {
    pub fn new(
        executor: &'a mut GameServerExecutor,
        event_context: &'a mut EventContext,
        root_scene: &'a RootScene,
    ) -> Self {
        Self {
            executor,
            event: event_context,
            root_scene,
        }
    }
}

pub trait Executable {
    fn executor(&mut self) -> &mut GameServerExecutor;
    fn event(&mut self) -> &mut EventContext;

    fn execute_draw<F>(&mut self, callback: F) -> anyhow::Result<()>
    where
        F: DrawDispatch,
    {
        self.event().draw_sender.execute(callback)
    }

    fn execute_draw_sync<F, R>(&mut self, callback: F) -> anyhow::Result<R>
    where
        R: 'static + Send,
        F: for<'a> FnOnce(DrawDispatchContext<'a>) -> R + 'static + Send,
    {
        if let Some(server) = self.executor().main_runner.base.container.draw.as_mut() {
            Ok(callback(DrawDispatchContext::new(
                &mut server.context,
                &server.root_scene,
            )))
        } else {
            let (sender, receiver) = mpsc::channels();
            self.execute_draw(move |context| {
                sender
                    .send(callback(context))
                    .context("unable to send value back to event thread")
                    .log_error();
                // this error can only happen if the below `recv` calls were not called
                // for some reason
            })
            .context("unable to execute sync-type callback")?;
            receiver.recv().context("unable to receive callback result")
        }
    }
}

impl<'a> Executable for EventDispatchContext<'a> {
    fn executor(&mut self) -> &mut GameServerExecutor {
        self.executor
    }

    fn event(&mut self) -> &mut EventContext {
        self.event
    }
}
