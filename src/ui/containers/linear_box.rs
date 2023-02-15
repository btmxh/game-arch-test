use std::{iter::Map, sync::Arc};

use crate::{
    ui::{
        acquire_widget_id,
        utils::geom::{UIPos, UIRect, UISize},
        Axis, Padding, UISizeConstraint, Widget, WidgetId,
    },
    utils::mutex::{Mutex, MutexGuard},
};

use super::{ContainerHint, ContainerWidget};

pub struct LinearBoxChild<A: Axis> {
    widget: Arc<dyn Widget>,
    alignment: <A as Axis>::CrossAlignment,
    size: UISize,
}

pub struct LinearBox<A: Axis> {
    id: WidgetId,
    children: Mutex<Vec<LinearBoxChild<A>>>,
    hover: Mutex<Vec<Arc<dyn Widget>>>,
    bounds: Mutex<UIRect>,
    spacing: Mutex<f32>,
    padding: Mutex<Padding>,
}

impl<A: Axis> LinearBox<A> {
    pub fn new(spacing: f32) -> Self {
        Self {
            id: acquire_widget_id(),
            children: Mutex::new(Vec::new()),
            hover: Mutex::new(Vec::new()),
            bounds: Mutex::new(UIRect::ZERO),
            spacing: Mutex::new(spacing),
            padding: Mutex::new(Padding::default()),
        }
    }

    pub fn push(&self, child: impl Widget + 'static, alignment: <A as Axis>::CrossAlignment) {
        self.push_arc(Arc::new(child), alignment);
    }

    pub fn push_arc(&self, child: Arc<dyn Widget>, alignment: <A as Axis>::CrossAlignment) {
        self.children.lock().push(LinearBoxChild {
            widget: child,
            alignment,
            size: UISize::ZERO,
        });
    }
}

impl<A: Axis> Default for LinearBox<A> {
    fn default() -> Self {
        Self::new(4.0)
    }
}

impl<A: Axis> ContainerWidget for LinearBox<A> {
    fn container_id(&self) -> WidgetId {
        self.id
    }

    fn layout_container(&self, size_constraints: &UISizeConstraint) -> UISize {
        let (size_constraints, pos_offset) =
            self.padding.lock().apply_to_constraints(size_constraints);
        let mut main_size: f32 = 0.0;
        let mut cross_size: f32 = 0.0;
        let mut children = self.children.lock();
        let spacing = *self.spacing.lock();
        for child in children.iter_mut() {
            let size_constraints = UISizeConstraint {
                min: UISize::ZERO,
                max: A::new_size(
                    A::get_size(size_constraints.max) - main_size,
                    <A as Axis>::OtherAxis::get_size(size_constraints.max),
                ),
            };

            let size = child.widget.layout(&size_constraints);

            main_size += A::get_size(size) + spacing;
            cross_size = cross_size.max(<A as Axis>::OtherAxis::get_size(size));
            child.size = size;
        }

        let mut main_pos = 0.0;
        for child in children.iter() {
            let mut child_pos = A::new_pos(
                main_pos,
                <A as Axis>::OtherAxis::calc_align_offset(
                    child.alignment,
                    cross_size,
                    <A as Axis>::OtherAxis::get_size(child.size),
                ),
            );

            child_pos.x += pos_offset.x;
            child_pos.y += pos_offset.y;

            child.widget.set_position(child_pos);
            main_pos += A::get_size(child.size) + spacing;
        }

        A::new_size(main_size, cross_size)
    }

    fn set_container_position(&self, position: UIPos) {
        self.bounds.lock().pos = position;
    }

    fn get_container_bounds(&self) -> UIRect {
        *self.bounds.lock()
    }

    fn container_hints() -> super::ContainerHint {
        ContainerHint::NO_OVERLAP
    }

    type ChildrenGuard<'a> = MutexGuard<'a, Vec<LinearBoxChild<A>>>;

    type ChildrenIterator<'c> =
        Map<std::slice::Iter<'c, LinearBoxChild<A>>, fn(&LinearBoxChild<A>) -> Arc<dyn Widget>>;

    fn lock_children(&self) -> Self::ChildrenGuard<'_> {
        self.children.lock()
    }

    fn iterate_child_widgets<'c>(
        &self,
        guard: &'c Self::ChildrenGuard<'_>,
    ) -> Self::ChildrenIterator<'c> {
        fn get_widget<A: Axis>(child: &LinearBoxChild<A>) -> Arc<dyn Widget> {
            child.widget.clone()
        }

        guard.iter().map(get_widget)
    }

    fn hover_widgets(&self) -> MutexGuard<'_, Vec<Arc<dyn Widget>>> {
        self.hover.lock()
    }
}
