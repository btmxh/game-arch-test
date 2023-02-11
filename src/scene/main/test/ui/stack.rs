use std::sync::Arc;

use crate::{exec::main_ctx::MainContext, test::tree::ParentTestNode};

pub fn test(main_ctx: &mut MainContext, node: &Arc<ParentTestNode>) -> anyhow::Result<()> {
    let node = node.new_child_parent("stack_test");
    layout_tests::test(main_ctx, &node);
    // propagating_tests::test(main_ctx, &node)?;
    // cursor_tests::test(main_ctx, &node)?;
    draw_tests::test(main_ctx, &node)?;
    Ok(())
}

mod layout_tests {
    use std::{borrow::Cow, sync::Arc};

    use crate::{
        exec::main_ctx::MainContext,
        scene::main::test::ui::TestWidgetBuilder,
        test::{
            assert::{assert_equals_err, assert_true},
            result::TestResult,
            tree::{LeafTestNode, ParentTestNode},
        },
        ui::{
            containers::stack::Stack,
            utils::geom::{UIPos, UIRect, UISize},
            Alignment, HorizontalAlignment, UISizeConstraint, VerticalAlignment, Widget,
        },
    };

    #[rustfmt::skip]
    pub(super) fn test(
        _: &mut MainContext,
        node: &Arc<ParentTestNode>,
    ) {
        let node = node.new_child_parent("layout");
        do_test(&node, "single_child_center_middle", [
            (100.0, 200.0, HorizontalAlignment::Middle, VerticalAlignment::Center),
        ], [
            (50.0, 50.0, 50.0, 50.0, [(0.0, 0.0, 50.0, 50.0)]),
            (100.0, 100.0, 100.0, 100.0, [(0.0, 0.0, 100.0, 100.0)]),
            (200.0, 200.0, 200.0, 200.0, [(50.0, 0.0, 100.0, 200.0)]),
            (500.0, 500.0, 500.0, 500.0, [(200.0, 150.0, 100.0, 200.0)]),
            (0.0, 0.0, 1e9, 1e9, [(0.0, 0.0, 100.0, 200.0)]),
        ]);

        do_test(&node, "single_child_top_left", [
            (100.0, 200.0, HorizontalAlignment::Left, VerticalAlignment::Top),
        ], [
            (50.0, 50.0, 50.0, 50.0, [(0.0, 0.0, 50.0, 50.0)]),
            (100.0, 100.0, 100.0, 100.0, [(0.0, 0.0, 100.0, 100.0)]),
            (200.0, 200.0, 200.0, 200.0, [(0.0, 0.0, 100.0, 200.0)]),
            (500.0, 500.0, 500.0, 500.0, [(0.0, 0.0, 100.0, 200.0)]),
            (0.0, 0.0, 1e9, 1e9, [(0.0, 0.0, 100.0, 200.0)]),
        ]);

        do_test(&node, "single_child_bottom_right", [
            (100.0, 200.0, HorizontalAlignment::Right, VerticalAlignment::Bottom),
        ], [
            (50.0, 50.0, 50.0, 50.0, [(0.0, 0.0, 50.0, 50.0)]),
            (100.0, 100.0, 100.0, 100.0, [(0.0, 0.0, 100.0, 100.0)]),
            (200.0, 200.0, 200.0, 200.0, [(100.0, 0.0, 100.0, 200.0)]),
            (500.0, 500.0, 500.0, 500.0, [(400.0, 300.0, 100.0, 200.0)]),
            (0.0, 0.0, 1e9, 1e9, [(0.0, 0.0, 100.0, 200.0)]),
        ]);

        do_test(&node, "lazy_child", [
            // setitng pref_size to 0x0 is equivalent to always picking the minimum size
            (0.0, 0.0, HorizontalAlignment::Middle, VerticalAlignment::Center),
            (0.0, 0.0, HorizontalAlignment::Left, VerticalAlignment::Top),
            (0.0, 0.0, HorizontalAlignment::Right, VerticalAlignment::Bottom),
        ], [
            (0.0, 0.0, 100.0, 200.0, [(0.0, 0.0, 0.0, 0.0); 3]),
            (100.0, 200.0, 100.0, 200.0, [
                (50.0, 100.0, 0.0, 0.0),
                (0.0, 0.0, 0.0, 0.0),
                (100.0, 200.0, 0.0, 0.0),
            ]),
            (100.0, 200.0, 300.0, 400.0, [
                (50.0, 100.0, 0.0, 0.0),
                (0.0, 0.0, 0.0, 0.0),
                (100.0, 200.0, 0.0, 0.0),
            ]),
        ]);

        do_test(&node, "greedy_child", [
            // f32::INFINITY is mouthful, we will just use 1e9 instead
            // setitng pref_size to INF is equivalent to always picking the maximum size
            (1e9, 1e9, HorizontalAlignment::Middle, VerticalAlignment::Center),
            (1e9, 1e9, HorizontalAlignment::Left, VerticalAlignment::Top),
            (1e9, 1e9, HorizontalAlignment::Right, VerticalAlignment::Bottom),
        ], [
            (0.0, 0.0, 100.0, 200.0, [(0.0, 0.0, 100.0, 200.0); 3]),
            (100.0, 200.0, 100.0, 200.0, [(0.0, 0.0, 100.0, 200.0); 3]),
            (100.0, 200.0, 300.0, 400.0, [(0.0, 0.0, 300.0, 400.0); 3]),
        ])
    }

    fn do_test<const N: usize>(
        node: &Arc<ParentTestNode>,
        name: impl Into<Cow<'static, str>>,
        widget_builders: [(
            /*width:*/ f32,
            /*height:*/ f32,
            /*h_align:*/ HorizontalAlignment,
            /*v_align:*/ VerticalAlignment,
        ); N],
        expected_results: impl IntoIterator<
            Item = (
                /*container_min_width:*/ f32,
                /*container_min_height:*/ f32,
                /*container_max_width:*/ f32,
                /*container_max_height:*/ f32,
                /*child_layouts:*/ [(f32, f32, f32, f32); N],
            ),
        >,
    ) {
        let node = node.new_child_leaf(name);
        node.update(test_body(&node, widget_builders, expected_results));
    }

    fn test_body<const N: usize>(
        node: &Arc<LeafTestNode>,
        widget_builders: [(
            /*width:*/ f32,
            /*height:*/ f32,
            /*h_align:*/ HorizontalAlignment,
            /*v_align:*/ VerticalAlignment,
        ); N],
        expected_results: impl IntoIterator<
            Item = (
                /*container_min_width:*/ f32,
                /*container_min_height:*/ f32,
                /*container_max_width:*/ f32,
                /*container_max_height:*/ f32,
                /*child_layouts:*/ [(f32, f32, f32, f32); N],
            ),
        >,
    ) -> TestResult {
        let widgets = widget_builders
            .into_iter()
            .enumerate()
            .map(|(i, (width, height, h_align, v_align))| {
                (
                    TestWidgetBuilder::new().pref_size(width, height).build(
                        i,
                        node.full_name().to_owned(),
                        false,
                        false,
                        false,
                    ),
                    Alignment::new(h_align, v_align),
                )
            })
            .collect::<Vec<_>>();

        let stack = Arc::new(Stack::new());

        for (widget, alignment) in widgets.iter() {
            stack.push_arc(widget.clone(), *alignment)
        }

        for (test_case_index, (min_width, min_height, max_width, max_height, child_layouts)) in
            expected_results.into_iter().enumerate()
        {
            let constraints = UISizeConstraint::new(
                UISize::new(min_width, min_height),
                UISize::new(max_width, max_height),
            );
            stack.layout(&constraints);

            let container_size = stack.get_bounds().size;
            assert_true(constraints.test(&container_size), format!("container size does not fits (constraits: {constraints:?}, actual size: {container_size:?})"))?;

            for (i, (x, y, w, h)) in child_layouts.into_iter().enumerate() {
                let widget = &widgets.get(i).expect("widgets.len() == N").0;
                let expected_bounds = UIRect::new(UIPos::new(x, y), UISize::new(w, h));

                let msg =
                    format!("child (index: {i}) bounds mismatch in test case {test_case_index}");
                assert_equals_err(&widget.get_bounds(), &expected_bounds, msg)?;
            }
        }

        Ok(())
    }
}

mod draw_tests {
    use std::{collections::HashSet, sync::Arc};

    use anyhow::Context;

    use crate::{
        exec::main_ctx::MainContext,
        graphics::context::DrawContext,
        scene::main::test::ui::{TestWidgetBuilder, TestWidgetId},
        test::{assert::assert_equals, result::TestResult, tree::ParentTestNode},
        ui::{containers::stack::Stack, Alignment, HorizontalAlignment, VerticalAlignment, Widget},
    };

    pub(super) fn test(
        main_ctx: &mut MainContext,
        node: &Arc<ParentTestNode>,
    ) -> anyhow::Result<()> {
        let node = node.new_child_parent("draw");
        do_test(
            main_ctx,
            &node,
            "12345",
            [1, 2, 3, 4, 5],
            r#"
1
2
3
4
5
"#,
        )?;
        Ok(())
    }

    fn do_test<const N: usize>(
        main_ctx: &mut MainContext,
        node: &Arc<ParentTestNode>,
        name: &'static str,
        widget_test_ids: [TestWidgetId; N],
        expected_log: &'static str,
    ) -> anyhow::Result<()> {
        let node = node.new_child_leaf(name);
        debug_assert!(
            widget_test_ids.len()
                == widget_test_ids
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>()
                    .len(),
            "widget test ids must be unique"
        );

        let stack = Arc::new(Stack::new());

        for id in widget_test_ids {
            let widget = TestWidgetBuilder::new().build(
                id,
                node.full_name().to_owned(),
                false,
                false,
                false,
            );
            stack.push_arc(
                widget.clone(),
                Alignment::new(HorizontalAlignment::Middle, VerticalAlignment::Center),
            );
        }

        let name = node.full_name().to_owned();
        main_ctx
            .channels
            .draw
            .execute_draw_event(move |ctx, _| {
                stack.draw(ctx);
                node.update(test_body(ctx, name, expected_log));
                []
            })
            .context("unable to send test to run on draw server")?;

        Ok(())
    }

    fn test_body(ctx: &mut DrawContext, name: String, expected_log: &str) -> TestResult {
        let log = ctx.pop_test_log(name.as_str())?;
        let log = log.trim();
        let expected_log = expected_log.trim();

        assert_equals(log, expected_log, "draw log mismatch")?;

        Ok(())
    }
}
