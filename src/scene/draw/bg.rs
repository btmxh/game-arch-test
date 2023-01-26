use cgmath::{Matrix3, SquareMatrix, Vector2, Zero};

use crate::graphics::{
    blur::BlurRenderer, context::DrawContext, quad_renderer::QuadRenderer, Vec2,
};

pub struct Background {
    blur: BlurRenderer,
    renderer: QuadRenderer,
    offset: Vec2,
}

impl Background {
    pub fn new(blur: BlurRenderer, renderer: QuadRenderer) -> Self {
        Self {
            blur,
            renderer,
            offset: Vector2::zero(),
        }
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
        if let Some(texture) = self.blur.output_texture_handle().try_get(context) {
            const OFFSET_FACTOR_VECTOR: Vec2 = Vec2::new(0.995, 0.998);
            const BOUNDS_NEG_1: [Vec2; 2] = [Vec2::new(0.0, 0.0), OFFSET_FACTOR_VECTOR];
            const BOUNDS_POS_1: [Vec2; 2] = [
                Vec2::new(1.0 - OFFSET_FACTOR_VECTOR.x, 1.0 - OFFSET_FACTOR_VECTOR.y),
                Vec2::new(1.0, 1.0),
            ];
            let normalized_offset = self.offset.map(|v| (v + 1.0) * 0.5);
            let bounds = [
                Self::lerp_vec2(normalized_offset, BOUNDS_NEG_1[0], BOUNDS_POS_1[0]),
                Self::lerp_vec2(normalized_offset, BOUNDS_NEG_1[1], BOUNDS_POS_1[1]),
            ];
            self.renderer.draw(
                context,
                *texture,
                &QuadRenderer::FULL_WINDOW_POS_BOUNDS,
                &bounds,
                &Vec2::zero(),
                &Matrix3::identity(),
            );
        }

        Ok(())
    }
}
