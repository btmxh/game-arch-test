use glam::Mat3;
use winit::event::Event;

use crate::{
    events::{GameEvent, GameUserEvent},
    exec::{executor::GameServerExecutor, main_ctx::MainContext},
    graphics::{context::DrawContext, quad_renderer::QuadRenderer},
    utils::error::ResultExt,
};

use super::{
    common::{UIDrawContext, UIEventContext},
    utils::geom::UISize,
};

pub mod center_test;

pub struct UIEventScene {
    center_test: center_test::EventScene,
}

pub struct UIDrawScene {
    center_test: center_test::DrawScene,
    quad_renderer: QuadRenderer,
}

impl UIEventScene {
    pub fn new(executor: &mut GameServerExecutor, main_ctx: &mut MainContext) -> Self {
        let mut slf = Self {
            center_test: center_test::EventScene::new(),
        };
        let draw = slf.create_predraw(&mut Self::event_ctx(executor, main_ctx));
        GameServerExecutor::execute_draw_event(&main_ctx.channels.draw, move |_, draw_root| {
            draw_root.ui = Some(draw);
            []
        })
        .log_error();
        slf.resize(
            executor,
            main_ctx,
            main_ctx
                .display
                .get_size()
                .to_logical(main_ctx.display.get_scale_factor())
                .into(),
        );
        slf
    }

    pub fn create_predraw(&self, ctx: &mut UIEventContext) -> UIDrawScene {
        UIDrawScene {
            center_test: self.center_test.create_predraw(ctx),
            quad_renderer: QuadRenderer::new(
                ctx.main_ctx.dummy_vao.clone(),
                &mut ctx.main_ctx.channels.draw,
            )
            .expect("unable to create quad renderer"),
        }
    }

    fn event_ctx<'a>(
        executor: &'a mut GameServerExecutor,
        main_ctx: &'a mut MainContext,
    ) -> UIEventContext<'a> {
        UIEventContext { executor, main_ctx }
    }

    pub fn handle_event(
        &mut self,
        executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
        event: &GameEvent,
    ) -> anyhow::Result<bool> {
        if let Event::UserEvent(GameUserEvent::CheckedResize { ui_size, .. }) = event {
            self.resize(executor, main_ctx, *ui_size)
        }

        Ok(false)
    }

    fn resize(
        &mut self,
        executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
        ui_size: UISize,
    ) {
        self.center_test
            .resize(&mut Self::event_ctx(executor, main_ctx), ui_size);
    }
}

impl UIDrawScene {
    pub fn draw(&mut self, context: &mut DrawContext) {
        self.center_test.draw(
            &mut UIDrawContext {
                context,
                quad_renderer: &mut self.quad_renderer,
            },
            1.0,
            &Mat3::IDENTITY,
        );
    }
}
