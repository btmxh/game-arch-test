use crate::{
    display::EventSender,
    exec::{
        dispatch::{DispatchList, DispatchMsg, EventDispatch},
        executor::GameServerExecutor,
        server::{audio, draw, update, ServerChannels},
        task::TaskExecutor,
    },
    scene::main::RootScene,
    test::manager::{new_test_manager, RealArcTestManager},
    utils::error::ResultExt,
};

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use tracing_appender::non_blocking::WorkerGuard;
use winit::{event::Event, event_loop::EventLoop};

use crate::{
    context::draw::DrawDispatch,
    display::Display,
    events::{GameEvent, GameUserEvent},
    utils::mpsc,
};

use super::draw::DrawDispatchContext;

pub struct EventContext {
    pub test_manager: RealArcTestManager,
    pub task_executor: TaskExecutor,
    pub channels: ServerChannels,
    pub dispatch_list: DispatchList,
    pub event_sender: EventSender,
    pub display: Display,
}

impl EventContext {
    #[allow(clippy::type_complexity)]
    pub fn new(
        display: Display,
        event_sender: EventSender,
    ) -> anyhow::Result<(
        Self,
        mpsc::Receiver<draw::Message>,
        mpsc::Receiver<audio::Message>,
        mpsc::Receiver<update::Message>,
    )> {
        let (draw_sender, draw_receiver) = mpsc::channels();
        let (audio_sender, audio_receiver) = mpsc::channels();
        let (update_sender, update_receiver) = mpsc::channels();
        let mut dispatch_list = DispatchList::new();

        Ok((
            Self {
                test_manager: new_test_manager(&update_sender, &mut dispatch_list, &event_sender)
                    .context("unable to create test manager")?,
                task_executor: TaskExecutor::new(),
                display,
                event_sender,
                dispatch_list,
                channels: ServerChannels {
                    audio: audio_sender,
                    draw: draw_sender,
                    update: update_sender,
                },
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
            Event::UserEvent(GameUserEvent::Dispatch(msg)) => match msg {
                DispatchMsg::ExecuteDispatch(ids) => {
                    for dispatch in ids
                        .into_iter()
                        .filter_map(|id| self.dispatch_list.pop(id))
                        .collect::<Vec<_>>()
                    {
                        dispatch(EventDispatchContext::new(executor, self, root_scene));
                    }
                }
            },

            Event::UserEvent(GameUserEvent::Execute(callback)) => {
                callback(EventDispatchContext::new(executor, self, root_scene));
            }

            event => {
                root_scene.handle_event(
                    &mut EventHandleContext::new(executor, self, root_scene),
                    event,
                );
            }
        };
        Ok(())
    }

    pub fn set_timeout(
        &mut self,
        timeout: Duration,
        callback: impl EventDispatch,
    ) -> anyhow::Result<()> {
        let id = self.dispatch_list.push(callback);
        self.channels.update.set_timeout(timeout, id)?;
        Ok(())
    }

    pub fn execute_blocking_task<F>(&mut self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.task_executor.execute(f)
    }

    pub fn execute_draw_sync<F>(&mut self, _callback: F) -> anyhow::Result<()>
    where
        F: DrawDispatch,
    {
        todo!()
    }

    pub fn run(
        mut self,
        mut executor: GameServerExecutor,
        event_loop: EventLoop<GameUserEvent>,
        root_scene: Arc<RootScene>,
        guard: Option<WorkerGuard>,
    ) -> ! {
        use winit::event_loop::ControlFlow;
        event_loop.run(move |event, _target, control_flow| {
            // guarantee drop order
            fn unused<T>(_: &T) {}
            unused(&root_scene);
            unused(&self);
            unused(&guard);
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

pub type EventHandleContext<'a> = EventDispatchContext<'a>;

pub trait Executable {
    fn executor(&mut self) -> &mut GameServerExecutor;
    fn event(&mut self) -> &mut EventContext;

    fn execute_draw<F>(&mut self, callback: F) -> anyhow::Result<()>
    where
        F: DrawDispatch,
    {
        self.event().channels.draw.execute(callback)
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
