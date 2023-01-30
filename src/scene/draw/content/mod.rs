use crate::graphics::{blur::BlurRenderer, context::DrawContext, quad_renderer::QuadRenderer};

use self::bg::Background;

pub mod bg;

pub struct Content {
    pub background: Option<Background>,
}

impl Content {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self { background: None })
    }

    pub fn initialize_background(
        &mut self,
        blur: BlurRenderer,
        renderer: QuadRenderer,
    ) -> anyhow::Result<()> {
        self.background = Some(Background::new(blur, renderer));
        Ok(())
    }

    pub fn draw(&mut self, context: &mut DrawContext) -> anyhow::Result<()> {
        self.background
            .as_mut()
            .map(|bg| bg.draw(context))
            .transpose()?;
        Ok(())
    }
}
