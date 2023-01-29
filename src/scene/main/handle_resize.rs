use std::{num::NonZeroU32, time::Duration};

use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
};

use crate::{
    events::{GameEvent, GameUserEvent},
    exec::{executor::GameServerExecutor, main_ctx::MainContext},
    ui::utils::geom::UISize,
    utils::args::args,
};

use super::EventRoot;

pub struct HandleResize {
    // for resize throttling
    // port of https://blog.webdevsimplified.com/2022-03/debounce-vs-throttle/
    resize_should_wait: bool,
    resize_size: Option<(PhysicalSize<NonZeroU32>, UISize)>,
}

impl HandleResize {
    const THROTTLE_DURATION: Duration = Duration::from_millis(100);
    pub fn new(_: &mut GameServerExecutor, _: &mut MainContext) -> anyhow::Result<Self> {
        Ok(Self {
            resize_should_wait: false,
            resize_size: None,
        })
    }

    fn resize(
        executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
        root_scene: &mut EventRoot,
        display_size: PhysicalSize<NonZeroU32>,
        ui_size: UISize,
        block: bool,
    ) -> anyhow::Result<()> {
        if block {
            executor.execute_draw_sync(&mut main_ctx.channels.draw, move |context, _| {
                context.resize(display_size, ui_size);
                Ok(())
            })?;
        } else {
            GameServerExecutor::execute_draw_event(&main_ctx.channels.draw, move |context, _| {
                context.resize(display_size, ui_size);
                []
            })?;
        }
        root_scene.handle_event(
            executor,
            main_ctx,
            GameEvent::UserEvent(GameUserEvent::CheckedResize {
                display_size,
                ui_size,
            }),
        )?;
        Ok(())
    }

    fn resize_timeout_func(
        &mut self,
        main_ctx: &mut MainContext,
        executor: &mut GameServerExecutor,
        root_scene: &mut EventRoot,
    ) -> anyhow::Result<()> {
        if let Some((size, ui_size)) = self.resize_size.take() {
            Self::resize(executor, main_ctx, root_scene, size, ui_size, false)?;
            self.resize_size = None;
            Self::set_timeout(main_ctx)?;
        } else {
            self.resize_should_wait = false;
        }

        Ok(())
    }

    fn set_timeout(main_ctx: &mut MainContext) -> anyhow::Result<()> {
        main_ctx.set_timeout(
            Self::THROTTLE_DURATION,
            |main_ctx, executor, root_scene, _| {
                if let Some(mut slf) = root_scene.handle_resize.take() {
                    slf.resize_timeout_func(main_ctx, executor, root_scene)?;
                    root_scene.handle_resize = Some(slf);
                }

                Ok(())
            },
        )?;

        Ok(())
    }

    pub fn handle_event(
        &mut self,
        executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
        root_scene: &mut EventRoot,
        event: &GameEvent,
    ) -> anyhow::Result<bool> {
        Ok(match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(size),
            } if main_ctx.display.get_window_id() == *window_id => {
                let width = NonZeroU32::new(size.width);
                let height = NonZeroU32::new(size.height);
                let ui_size = size.to_logical(main_ctx.display.get_scale_factor()).into();
                let size =
                    width.and_then(|width| height.map(|height| PhysicalSize::new(width, height)));
                if let Some(size) = size {
                    if args().throttle_resize {
                        if self.resize_should_wait {
                            self.resize_size = Some((size, ui_size));
                        } else {
                            Self::resize(executor, main_ctx, root_scene, size, ui_size, false)?;
                            self.resize_should_wait = true;
                            Self::set_timeout(main_ctx)?;
                        }
                    } else {
                        Self::resize(
                            executor,
                            main_ctx,
                            root_scene,
                            size,
                            ui_size,
                            !args().block_event_loop,
                        )?;
                    }
                }
                true
            }

            _ => false,
        })
    }
}
