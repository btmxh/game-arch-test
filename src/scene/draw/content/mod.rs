use crate::graphics::context::DrawContext;

use self::bg::Background;

pub mod bg;

pub struct Content {
    pub background: Background,
}

impl Content {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            background: Background::new(),
        })
    }

    pub fn draw(&mut self, context: &mut DrawContext) -> anyhow::Result<()> {
        self.background.draw(context)?;
        Ok(())
    }
}
