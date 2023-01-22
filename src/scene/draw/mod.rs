use crate::graphics::context::DrawContext;

use self::{bg::Background, clear::ClearScreen};

pub mod bg;
pub mod clear;

pub struct DrawRoot {
    clear: ClearScreen,
    background: Background,
}

impl DrawRoot {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            clear: ClearScreen,
            background: Background,
        })
    }

    pub fn draw(&mut self, context: &mut DrawContext) -> anyhow::Result<()> {
        self.clear.draw(context);
        self.background.draw(context)?;
        Ok(())
    }
}
