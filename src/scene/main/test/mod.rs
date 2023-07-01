use anyhow::Context;

use crate::{
    context::{draw::DrawContext, init::InitContext},
    draw_option,
};

pub mod headless;
pub mod timeout_delay;

pub struct Scene {
    headless: Option<headless::Scene>,
    #[allow(dead_code)]
    timeout_delay: timeout_delay::Scene,
}

impl Scene {
    pub fn new(context: &mut InitContext) -> anyhow::Result<Self> {
        let node = &context
            .event
            .test_manager
            .as_ref()
            .expect("TestManager must exist in test mode")
            .root
            .clone();
        let slf = Self {
            headless: headless::Scene::new(context, node)
                .context("unable to create Headless test scene")?,
            timeout_delay: timeout_delay::Scene::new(context, node)
                .context("unable to initiate TimeoutDelay tests")?,
        };

        context
            .event
            .test_manager
            .as_ref()
            .expect("TestManager must exist in test mode")
            .finish_init();
        Ok(slf)
    }

    pub fn draw(&self, draw: &mut DrawContext) {
        draw_option!(self.headless, draw);
    }
}
