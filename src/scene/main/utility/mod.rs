use anyhow::Context;

use crate::{context::event::EventDispatchContext, events::GameEvent};
pub mod close;
pub mod freq_profile;
pub mod update_delay_test;
pub mod vsync;

pub struct Scene {
    vsync: vsync::Scene,
    freq_profile: freq_profile::Scene,
    update_delay_test: update_delay_test::ArcScene,
    close: close::Scene,
}

impl Scene {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            vsync: vsync::Scene::new().context("unable to initialize VSync scene")?,
            freq_profile: freq_profile::Scene::new(),
            update_delay_test: update_delay_test::Scene::new(),
            close: close::Scene,
        })
    }

    pub fn handle_event<'a>(
        &self,
        context: &mut EventDispatchContext,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        let event = self.vsync.handle_event(context, event)?;
        let event = self.freq_profile.handle_event(context, event)?;
        let event = self
            .update_delay_test
            .clone()
            .handle_event(context, event)?;
        let event = self.close.handle_event(context, event)?;
        Some(event)
    }
}
