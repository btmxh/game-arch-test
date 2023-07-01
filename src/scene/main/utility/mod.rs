use anyhow::Context;

use crate::{
    context::{event::EventHandleContext, init::InitContext},
    events::GameEvent,
};
pub mod close;
pub mod error;
pub mod freq_profile;
pub mod update_delay_test;
pub mod vsync;

pub struct Scene {
    vsync: vsync::Scene,
    freq_profile: freq_profile::Scene,
    update_delay_test: update_delay_test::ArcScene,
    close: close::Scene,
    error: error::Scene,
}

impl Scene {
    pub fn new(context: &mut InitContext) -> anyhow::Result<Self> {
        Ok(Self {
            vsync: vsync::Scene::new(context).context("unable to initialize VSync scene")?,
            freq_profile: freq_profile::Scene::new(),
            update_delay_test: update_delay_test::Scene::new(),
            close: close::Scene,
            error: error::Scene,
        })
    }

    pub fn handle_event<'a>(
        &self,
        context: &mut EventHandleContext,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        let event = self.vsync.handle_event(context, event)?;
        let event = self.freq_profile.handle_event(context, event)?;
        let event = self
            .update_delay_test
            .clone()
            .handle_event(context, event)?;
        let event = self.close.handle_event(context, event)?;
        let event = self.error.handle_event(context, event)?;
        Some(event)
    }
}
