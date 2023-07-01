use std::{num::NonZeroU32, sync::Arc, time::Duration};

use anyhow::Context;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
};

use crate::{
    context::event::{EventDispatchContext, EventHandleContext, Executable},
    events::{GameEvent, GameUserEvent},
    utils::{args::args, error::ResultExt, mutex::Mutex},
};

pub struct ResizeThrottleState {
    // for resize throttling
    // port of https://blog.webdevsimplified.com/2022-03/debounce-vs-throttle/
    resize_should_wait: bool,
    resize_size: Option<PhysicalSize<NonZeroU32>>,
}

pub struct Scene {
    state: Mutex<ResizeThrottleState>,
}

pub type ArcScene = Arc<Scene>;

impl Scene {
    const THROTTLE_DURATION: Duration = Duration::from_millis(100);
    pub fn new() -> ArcScene {
        Arc::new(Self {
            state: Mutex::new(ResizeThrottleState {
                resize_should_wait: false,
                resize_size: None,
            }),
        })
    }

    pub fn handle_event<'a>(
        self: ArcScene,
        context: &mut EventHandleContext,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(size),
            } if context.event.display.get_window_id() == window_id => {
                let width = NonZeroU32::new(size.width);
                let height = NonZeroU32::new(size.height);
                let size = width.zip(height).map(|(w, h)| PhysicalSize::new(w, h));
                if let Some(size) = size {
                    if args().throttle_resize {
                        let mut state = self.state.lock();
                        if state.resize_should_wait {
                            state.resize_size = Some(size);
                        } else {
                            Self::resize(context, size, false);
                            state.resize_should_wait = true;
                            self.clone()
                                .set_timeout(context)
                                .context("error while setting throttle timeout")
                                .log_error();
                        }
                    } else {
                        Self::resize(context, size, !args().block_event_loop);
                    }
                }
                None
            }

            event => Some(event),
        }
    }

    fn resize(
        context: &mut EventHandleContext,
        display_size: PhysicalSize<NonZeroU32>,
        block: bool,
    ) {
        if block {
            context.execute_draw_sync(move |context| context.graphics.resize(display_size))
        } else {
            context.execute_draw(move |context| context.graphics.resize(display_size))
        }
        .context("unable to send resize execute request to draw server")
        .log_error();
        context.root_scene.handle_event(
            context,
            GameEvent::UserEvent(GameUserEvent::CheckedResize { display_size }),
        );
    }

    fn resize_timeout_func(
        self: Arc<Self>,
        context: &mut EventDispatchContext,
    ) -> anyhow::Result<()> {
        let mut state = self.state.lock();
        if let Some(size) = state.resize_size.take() {
            Self::resize(context, size, false);
            state.resize_size = None;
            self.clone().set_timeout(context)?;
        } else {
            state.resize_should_wait = false;
        }

        Ok(())
    }

    fn set_timeout(self: Arc<Self>, context: &mut EventHandleContext) -> anyhow::Result<()> {
        context
            .event
            .set_timeout(Self::THROTTLE_DURATION, move |mut context| {
                self.resize_timeout_func(&mut context).log_error();
            })?;

        Ok(())
    }
}
