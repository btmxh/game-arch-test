use crate::{
    context::{draw::DrawContext, event::EventDispatchContext},
    events::GameEvent,
};

pub mod clear;
pub mod redraw;

pub struct Scene {
    redraw: redraw::Scene,
    clear: clear::Scene,
}

impl Scene {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            redraw: redraw::Scene,
            clear: clear::Scene,
        })
    }

    pub fn handle_event<'a>(
        &self,
        context: &mut EventDispatchContext,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        let event = self.redraw.handle_event(context, event)?;
        Some(event)
    }

    pub fn draw(&self, context: &mut DrawContext) {
        self.clear.draw(context);
    }
}
