use crate::graphics::{blur::BlurRenderer, context::DrawContext, quad_renderer::QuadRenderer};

use self::{bg::Background, clear::ClearScreen};

pub mod bg;
pub mod clear;

pub struct DrawRoot {
    clear: ClearScreen,
    pub background: Option<Background>,
}

impl DrawRoot {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            clear: ClearScreen,
            background: None,
        })
    }

    pub fn draw(&mut self, context: &mut DrawContext) -> anyhow::Result<()> {
        self.clear.draw(context);
        self.background
            .as_mut()
            .map(|bg| bg.draw(context))
            .transpose()?;
        Ok(())
    }

    pub fn initialize_background(
        &mut self,
        blur: BlurRenderer,
        renderer: QuadRenderer,
    ) -> anyhow::Result<()> {
        self.background = Some(Background::new(blur, renderer));
        Ok(())
    }
}
