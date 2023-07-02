use std::{sync::Arc, time::Duration};

use anyhow::Context;

use crate::{
    context::{draw::DrawContext, init::InitContext},
    enclose,
    test::{
        assert::{assert_false, assert_unreachable},
        result::TestResult,
        tree::{LeafTestNode, ParentTestNode},
    },
    utils::args::args,
};

pub struct Scene {
    no_draw: Arc<LeafTestNode>,
}

impl Scene {
    pub fn new(
        context: &mut InitContext,
        node: &Arc<ParentTestNode>,
    ) -> anyhow::Result<Option<Self>> {
        if !args().headless {
            return Ok(None);
        }

        let node = node.new_child_parent("headless");
        node.new_child_leaf("not_visible")
            .update(Self::test_not_visible(context));

        let no_draw = node.new_child_leaf("no_draw");
        context
            .event
            .update_sender
            .set_timeout(
                Duration::from_secs(5),
                enclose!((no_draw) move |_| {
                    if !no_draw.finished() {
                        no_draw.update(Ok(()));
                    }
                }),
            )
            .context("unable to set timeout for no_draw test")?;

        Ok(Some(Self { no_draw }))
    }

    fn test_not_visible(context: &mut InitContext) -> TestResult {
        assert_false(
            context
                .event
                .display
                .get_winit_window()
                .is_visible()
                .unwrap_or_default(),
            "Main window should not be visible in headless mode",
        )?;
        Ok(())
    }

    fn test_not_draw() -> TestResult {
        assert_unreachable("Scene::draw() should not be called in headless mode")?;
        Ok(())
    }

    pub fn draw(&self, _draw: &mut DrawContext) {
        self.no_draw.update(Self::test_not_draw())
    }
}
