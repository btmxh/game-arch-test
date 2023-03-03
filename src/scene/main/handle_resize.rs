use std::{num::NonZeroU32, sync::Arc, time::Duration};

use anyhow::Context;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
};

use crate::{
    events::{GameEvent, GameUserEvent},
    exec::{main_ctx::MainContext, server::draw::ServerSendChannelExt},
    scene::Scene,
    ui::utils::geom::UISize,
    utils::{args::args, error::ResultExt, mutex::Mutex},
};

use super::RootScene;

pub struct ResizeThrottleState {
    // for resize throttling
    // port of https://blog.webdevsimplified.com/2022-03/debounce-vs-throttle/
    resize_should_wait: bool,
    resize_size: Option<(PhysicalSize<NonZeroU32>, UISize)>,
}

pub struct HandleResize {
    state: Mutex<ResizeThrottleState>,
}

impl Scene for HandleResize {
    fn handle_event<'a>(
        self: Arc<Self>,
        main_ctx: &mut MainContext,
        root_scene: &RootScene,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(size),
            } if main_ctx.display.get_window_id() == window_id => {
                let width = NonZeroU32::new(size.width);
                let height = NonZeroU32::new(size.height);
                let ui_size = size.to_logical(main_ctx.display.get_scale_factor()).into();
                let size = width.zip(height).map(|(w, h)| PhysicalSize::new(w, h));
                if let Some(size) = size {
                    if args().throttle_resize {
                        let mut state = self.state.lock();
                        if state.resize_should_wait {
                            state.resize_size = Some((size, ui_size));
                        } else {
                            Self::resize(main_ctx, root_scene, size, ui_size, false);
                            state.resize_should_wait = true;
                            self.clone()
                                .set_timeout(main_ctx)
                                .context("error while setting throttle timeout")
                                .log_error();
                        }
                    } else {
                        Self::resize(
                            main_ctx,
                            root_scene,
                            size,
                            ui_size,
                            !args().block_event_loop,
                        );
                    }
                }
                None
            }

            event => Some(event),
        }
    }

    fn draw(self: Arc<Self>, _: &mut crate::graphics::context::DrawContext) {}
}

impl HandleResize {
    const THROTTLE_DURATION: Duration = Duration::from_millis(100);
    pub fn new() -> Self {
        Self {
            state: Mutex::new(ResizeThrottleState {
                resize_should_wait: false,
                resize_size: None,
            }),
        }
    }

    fn resize(
        main_ctx: &mut MainContext,
        root_scene: &RootScene,
        display_size: PhysicalSize<NonZeroU32>,
        ui_size: UISize,
        block: bool,
    ) {
        if block {
            main_ctx
                .execute_draw_sync(move |context, _| {
                    context.resize(display_size, ui_size);
                    Ok(())
                })
                .and_then(std::convert::identity)
        } else {
            main_ctx.channels.draw.execute(move |context, _| {
                context.resize(display_size, ui_size);
            })
        }
        .context("unable to send resize execute request to draw server")
        .log_error();
        root_scene.handle_event(
            main_ctx,
            GameEvent::UserEvent(GameUserEvent::CheckedResize {
                display_size,
                ui_size,
            }),
        );
    }

    fn resize_timeout_func(
        self: Arc<Self>,
        main_ctx: &mut MainContext,
        root_scene: &mut RootScene,
    ) -> anyhow::Result<()> {
        let mut state = self.state.lock();
        if let Some((size, ui_size)) = state.resize_size.take() {
            Self::resize(main_ctx, root_scene, size, ui_size, false);
            state.resize_size = None;
            self.clone().set_timeout(main_ctx)?;
        } else {
            state.resize_should_wait = false;
        }

        Ok(())
    }

    fn set_timeout(self: Arc<Self>, main_ctx: &mut MainContext) -> anyhow::Result<()> {
        main_ctx.set_timeout(Self::THROTTLE_DURATION, move |main_ctx, root_scene| {
            self.resize_timeout_func(main_ctx, root_scene).log_error();
            Ok(())
        })?;

        Ok(())
    }
}

impl Default for HandleResize {
    fn default() -> Self {
        Self::new()
    }
}
