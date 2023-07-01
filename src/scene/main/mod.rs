use std::{ops::Not, sync::Arc};

use crate::draw_option;
use anyhow::Context;

use crate::{
    context::draw::DrawContext,
    context::{event::EventHandleContext, init::InitContext},
    events::GameEvent,
    utils::args::args,
};

pub mod content;
pub mod core;
pub mod handle_resize;
pub mod test;
pub mod utility;

pub struct RootScene {
    handle_resize: handle_resize::ArcScene,
    core: core::Scene,
    test: Option<test::Scene>,
    #[allow(dead_code)]
    content: Option<content::Scene>,
    utility: utility::Scene,
}

impl RootScene {
    pub fn new(context: &mut InitContext) -> anyhow::Result<Arc<Self>> {
        Ok(Arc::new(Self {
            handle_resize: handle_resize::Scene::new(),
            core: core::Scene::new().context("unable to initialize handle core scene")?,
            test: args()
                .test
                .then(|| test::Scene::new(context))
                .transpose()
                .context("unable to initialize test scene")?,
            content: args().test.not().then(content::Scene::new),
            utility: utility::Scene::new(context).context("unable to initialize utility scene")?,
        }))
    }

    pub fn handle_event<'a>(
        &self,
        context: &mut EventHandleContext,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        let event = self.handle_resize.clone().handle_event(context, event)?;
        let event = self.core.handle_event(context, event)?;
        let event = self.utility.handle_event(context, event)?;
        Some(event)
    }

    pub fn draw(&self, context: &mut DrawContext) {
        self.core.draw(context);
        draw_option!(self.test, context);
    }
}

#[test]
fn test_sync() {
    use crate::assert_sync;

    assert_sync!(RootScene);
}

#[macro_export]
macro_rules! draw_option {
    ($scene: expr, $($args: expr),*) => {
        if let Some(scene) = $scene.as_ref() {
            scene.draw($($args)*);
        }
    };
}

#[macro_export]
macro_rules! handle_event_option {
    ($scene: expr, $($args: expr),*) => {
        let event = if let Some(scene) = $scene.as_ref() {
            scene.handle_event($($args)*)?
        } else {
            event
        }
    };
}
