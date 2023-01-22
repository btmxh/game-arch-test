use winit::dpi::PhysicalSize;

use crate::graphics::{blur::BlurRenderer, context::DrawContext, quad_renderer::QuadRenderer};

pub struct Background {
    blur: BlurRenderer,
    renderer: QuadRenderer,
    texture_dimensions: PhysicalSize<u32>,
}

impl Background {
    pub fn new(
        blur: BlurRenderer,
        renderer: QuadRenderer,
        texture_dimensions: PhysicalSize<u32>,
    ) -> Self {
        Self {
            blur,
            renderer,
            texture_dimensions,
        }
    }

    pub fn draw(&mut self, context: &mut DrawContext) -> anyhow::Result<()> {
        if let Some(texture) = self.blur.output_texture_handle().try_get(context) {
            let viewport_size = context.display_size;
            let vw = viewport_size.width.get() as f32;
            let vh = viewport_size.height.get() as f32;
            let tw = self.texture_dimensions.width as f32;
            let th = self.texture_dimensions.height as f32;
            let var = vw / vh;
            let tar = tw / th;
            let (hw, hh) = if var < tar {
                (0.5 * var / tar, 0.5)
            } else {
                (0.5, 0.5 * tar / var)
            };
            self.renderer.draw(
                &context,
                *texture,
                &[[0.5 - hw, 0.5 + hh].into(), [0.5 + hw, 0.5 - hh].into()],
            );
        }

        Ok(())
    }
}
