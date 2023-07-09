use crate::{
    context::{draw::DrawContext, event::EventDispatchContext},
    events::GameEvent,
    handle_event_option,
};

mod clear;
mod redraw;
mod surface_creation;

pub struct Scene {
    surface_creation: Option<surface_creation::Scene>,
    redraw: redraw::Scene,
    clear: clear::Scene,
}

impl Scene {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            surface_creation: surface_creation::Scene::new(),
            redraw: redraw::Scene,
            clear: clear::Scene,
        })
    }

    pub fn handle_event<'a>(
        &self,
        context: &mut EventDispatchContext,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        handle_event_option!(self.surface_creation, event, context);
        let event = self.redraw.handle_event(context, event)?;
        Some(event)
    }

    pub fn draw(&self, context: &mut DrawContext) {
        self.clear.draw(context);
    }
}
