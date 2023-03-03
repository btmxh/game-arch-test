use std::{sync::Arc, time::Duration};

use anyhow::Context;

use crate::{
    enclose,
    exec::main_ctx::MainContext,
    graphics::context::DrawContext,
    scene::{Scene, SceneContainer},
    test::{
        assert::{assert_false, assert_unreachable},
        result::TestResult,
        tree::{LeafTestNode, ParentTestNode},
    },
    utils::args::args,
};

pub struct Headless {
    no_draw: Arc<LeafTestNode>,
}

impl Headless {
    #[allow(clippy::new_ret_no_self)]
    #[allow(unused_mut)]
    pub fn new(
        main_ctx: &mut MainContext,
        node: &Arc<ParentTestNode>,
    ) -> anyhow::Result<SceneContainer> /* acts as an Option<Self> */ {
        if !args().headless {
            return Ok(SceneContainer::new());
        }

        let mut container = SceneContainer::new();
        let node = node.new_child_parent("headless");
        node.new_child_leaf("not_visible")
            .update(Self::test_not_visible(main_ctx));

        let no_draw = node.new_child_leaf("no_draw");
        main_ctx
            .set_timeout(
                Duration::from_secs(5),
                enclose!((no_draw) move |_, _| {
                    if !no_draw.finished() {
                        no_draw.update(Ok(()));
                    }
                    Ok(())
                }),
            )
            .context("unable to set timeout for no_draw test")?;

        container.push(Self { no_draw });
        Ok(container)
    }

    fn test_not_visible(main_ctx: &mut MainContext) -> TestResult {
        assert_false(
            main_ctx
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
}

impl Scene for Headless {
    fn draw(self: Arc<Self>, _ctx: &mut DrawContext) {
        self.no_draw.update(Self::test_not_draw())
    }
}
