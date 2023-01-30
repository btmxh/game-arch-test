use glam::{Mat3, Vec2};

use crate::{
    graphics::{
        context::DrawContext, quad_renderer::QuadRenderer, wrappers::texture::TextureHandle,
    },
    utils::clock::{Clock, SteadyClock},
};

#[derive(Default)]
pub struct Background {
    render_data: Option<(QuadRenderer, TextureHandle)>,
    offset: Vec2,
    clock: SteadyClock,
}

impl Background {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self, renderer: QuadRenderer, texture: TextureHandle) {
        self.render_data = Some((renderer, texture));
    }

    fn lerp_vec2(amt: Vec2, min: Vec2, max: Vec2) -> Vec2 {
        Vec2::new(
            min.x + (max.x - min.x) * amt.x,
            min.y + (max.y - min.y) * amt.y,
        )
    }

    pub fn set_offset(&mut self, offset: Vec2) {
        self.offset = offset;
    }

    pub fn draw(&mut self, context: &mut DrawContext) -> anyhow::Result<()> {
        if let Some((renderer, texture)) = self.render_data.as_mut() {
            const OFFSET_FACTOR_VECTOR: Vec2 = Vec2::new(0.995, 0.998);
            const BOUNDS_NEG_1: [Vec2; 2] = [Vec2::new(0.0, 0.0), OFFSET_FACTOR_VECTOR];
            const BOUNDS_POS_1: [Vec2; 2] = [
                Vec2::new(1.0 - OFFSET_FACTOR_VECTOR.x, 1.0 - OFFSET_FACTOR_VECTOR.y),
                Vec2::new(1.0, 1.0),
            ];
            const HALF: Vec2 = Vec2::new(0.5, 0.5);
            let texture = texture.get(context);
            let normalized_offset = self.offset.mul_add(HALF, HALF);
            let bounds = [
                Self::lerp_vec2(normalized_offset, BOUNDS_NEG_1[0], BOUNDS_POS_1[0]),
                Self::lerp_vec2(normalized_offset, BOUNDS_NEG_1[1], BOUNDS_POS_1[1]),
            ];
            let angle = self.clock.now() as f32 * 0.01;
            let transform = Mat3::from_angle(angle);
            let radius = Vec2::new(1.0, 1.0);
            renderer.draw(
                context,
                *texture,
                &QuadRenderer::FULL_WINDOW_POS_BOUNDS,
                &bounds,
                &radius,
                &transform,
            );
        }

        Ok(())
    }
}
