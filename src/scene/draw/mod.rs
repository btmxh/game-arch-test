use anyhow::Context;

use crate::{graphics::context::DrawContext, ui::scenes::UIDrawScene};

use self::{clear::ClearScreen, content::Content};

pub mod clear;
pub mod content;

pub struct DrawRoot {
    clear: ClearScreen,
    pub content: Content,
    pub ui: Option<UIDrawScene>,
}

impl DrawRoot {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            clear: ClearScreen,
            content: Content::new().context("unable to initialize content scene")?,
            ui: None,
        })
    }

    pub fn draw(&mut self, context: &mut DrawContext) -> anyhow::Result<()> {
        self.clear.draw(context);
        self.content.draw(context)?;
        if let Some(ui) = self.ui.as_mut() {
            ui.draw(context);
        }
        Ok(())
    }
}
