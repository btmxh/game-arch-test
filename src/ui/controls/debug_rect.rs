use std::ops::{Add, Div};

use glam::{Mat3, Vec2};

use crate::{
    graphics::quad_renderer::QuadRenderer,
    ui::{
        common::{SharedState, UIDrawContext, UIStateOpacity, UIStatePos, UIStateTransform},
        utils::geom::UISize,
    },
};

#[derive(Clone, Copy)]
pub struct State {
    pub radius: Vec2,
}

pub struct EventScene {
    pub state: SharedState<State>,
}

pub struct DrawScene {
    pub state: SharedState<State>,
    pub renderer: QuadRenderer,
}

impl State {
    pub fn draw(
        &self,
        ctx: &UIDrawContext,
        self_pos: &UIStatePos,
        self_size: &UISize,
        self_opacity: &UIStateOpacity,
        self_transform: &UIStateTransform,
        parent_opacity: f32,
        parent_transform: &Mat3,
    ) {
        let UISize { width, height } = ctx.context.ui_size;

        let base_transform = Mat3::from_scale_angle_translation(
            Vec2::new(2.0 / width, 2.0 / height),
            0.0,
            Vec2::new(-1.0, -1.0),
        );
        let transform = parent_transform
            .mul_mat3(self_transform)
            .mul_mat3(&base_transform);
        // renderer does not support custom opacity
        let _opacity = parent_opacity * **self_opacity;
        let radius = self.radius.div(Vec2::new(width, height));

        ctx.quad_renderer.draw(
            ctx.context,
            0,
            &[
                Vec2::from(*self_pos),
                Vec2::from(*self_pos).add(Vec2::from(*self_size)),
            ],
            &QuadRenderer::FULL_TEXTURE_TEX_BOUNDS,
            &radius,
            &transform,
        )
    }
}
