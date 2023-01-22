use crate::graphics::context::DrawContext;

pub struct Background;

impl Background {
    pub fn draw(&mut self, _: &mut DrawContext) -> anyhow::Result<()> {
        Ok(())
    }
}
