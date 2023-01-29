use crate::{
    graphics::{blur::BlurRenderer, context::DrawContext, quad_renderer::QuadRenderer},
    ui::scenes::UIDrawScene,
};

use self::{bg::Background, clear::ClearScreen};

pub mod bg;
pub mod clear;

pub struct DrawRoot {
    clear: ClearScreen,
    pub background: Option<Background>,
    pub ui: Option<UIDrawScene>,
}

impl DrawRoot {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            clear: ClearScreen,
            background: None,
            ui: None,
        })
    }

    pub fn draw(&mut self, context: &mut DrawContext) -> anyhow::Result<()> {
        self.clear.draw(context);
        self.background
            .as_mut()
            .map(|bg| bg.draw(context))
            .transpose()?;
        if let Some(ui) = self.ui.as_mut() {
            ui.draw(context);
        }
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
