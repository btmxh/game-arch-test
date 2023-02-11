use std::sync::Arc;

use crate::{exec::main_ctx::MainContext, test::tree::ParentTestNode};

pub fn test(main_ctx: &mut MainContext, node: &Arc<ParentTestNode>) -> anyhow::Result<()> {
    let node = node.new_child_parent("stack_test");
    layout_tests::test(main_ctx, &node);
    // propagating_tests::test(main_ctx, &node)?;
    // cursor_tests::test(main_ctx, &node)?;
    // draw_tests::test(main_ctx, &node)?;
    Ok(())
}

mod layout_tests {
    use std::{borrow::Cow, sync::Arc};

    use crate::{
        exec::main_ctx::MainContext,
        scene::main::test::ui::TestWidgetBuilder,
        test::{
            assert::assert_equals_err,
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
        do_test(&node, "1", [
            (100.0, 200.0, HorizontalAlignment::Middle, VerticalAlignment::Center),
        ], [
            (50.0, 50.0, [(0.0, 0.0, 50.0, 50.0)]),
            (100.0, 100.0, [(0.0, 0.0, 100.0, 100.0)]),
            (200.0, 200.0, [(50.0, 0.0, 100.0, 200.0)]),
            (500.0, 500.0, [(200.0, 150.0, 100.0, 200.0)]),
        ]);
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
                /*container_width:*/ f32,
                /*container_height:*/ f32,
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
                /*container_width:*/ f32,
                /*container_height:*/ f32,
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

        for (test_case_index, (container_width, container_height, child_layouts)) in
            expected_results.into_iter().enumerate()
        {
            let size = UISize::new(container_width, container_height);
            stack.layout(&UISizeConstraint::exact(size));

            assert_equals_err(
                &stack.get_bounds().size,
                &size,
                format!("container bounds mismatch in test case {test_case_index}"),
            )?;

            let msg = format!("child bounds mismatch in test case {test_case_index}");
            for (i, (x, y, w, h)) in child_layouts.into_iter().enumerate() {
                let widget = &widgets.get(i).expect("widgets.len() == N").0;
                let expected_bounds = UIRect::new(UIPos::new(x, y), UISize::new(w, h));

                assert_equals_err(&widget.get_bounds(), &expected_bounds, msg.clone())?;
            }
        }

        Ok(())
    }
}
