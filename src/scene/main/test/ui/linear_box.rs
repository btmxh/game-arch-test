use std::sync::Arc;

use crate::{exec::main_ctx::MainContext, test::tree::ParentTestNode};

pub fn test(main_ctx: &mut MainContext, node: &Arc<ParentTestNode>) -> anyhow::Result<()> {
    let node = node.new_child_parent("linear_box_test");
    layout_tests::test(main_ctx, &node);
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
            containers::linear_box::LinearBox,
            utils::geom::{UIPos, UIRect, UISize},
            Axis, AxisY, HorizontalAlignment, UISizeConstraint, Widget,
        },
    };

    #[rustfmt::skip]
    pub(super) fn test(
        _: &mut MainContext,
        node: &Arc<ParentTestNode>,
    ) {
        let node = node.new_child_parent("layout");
        do_test(
            AxisY,
            &node,
            "simple_y",
            [
                (200.0, 300.0, HorizontalAlignment::Left),
                (300.0, 400.0, HorizontalAlignment::Middle),
                (400.0, 100.0, HorizontalAlignment::Right),
            ],
            [
                (
                    0.0, 0.0, 1000.0, 1000.0,
                    [
                        // container size: 400x808 (default spacing is 4)
                        (0.0, 0.0, 200.0, 300.0),
                        (50.0, 304.0, 300.0, 400.0),
                        (0.0, 708.0, 400.0, 100.0)
                    ]
                )
            ]
        );
    }

    fn do_test<A: Axis, const N: usize>(
        _: A,
        node: &Arc<ParentTestNode>,
        name: impl Into<Cow<'static, str>>,
        widget_builders: [(
            /*width:*/ f32,
            /*height:*/ f32,
            /*align:*/ <A as Axis>::CrossAlignment,
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
        node.update(test_body::<A, N>(&node, widget_builders, expected_results));
    }

    fn test_body<A: Axis, const N: usize>(
        node: &Arc<LeafTestNode>,
        widget_builders: impl IntoIterator<
            Item = (
                /*width:*/ f32,
                /*height:*/ f32,
                /*align:*/ <A as Axis>::CrossAlignment,
            ),
        >,
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
            .map(|(i, (width, height, align))| {
                (
                    TestWidgetBuilder::new().pref_size(width, height).build(
                        i,
                        node.full_name().to_owned(),
                        false,
                        false,
                        false,
                    ),
                    align,
                )
            })
            .collect::<Vec<_>>();

        let stack = Arc::new(LinearBox::<A>::new());

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
