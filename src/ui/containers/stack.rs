use std::{iter::Map, sync::Arc};

use crate::{
    ui::{
        acquire_widget_id,
        utils::geom::{UIPos, UIRect, UISize},
        Alignment, UISizeConstraint, Widget, WidgetId,
    },
    utils::mutex::{Mutex, MutexGuard},
};

use super::{ContainerHint, ContainerWidget};

pub struct StackChild {
    widget: Arc<dyn Widget>,
    alignment: Alignment,
    size: UISize,
}

pub struct Stack {
    children: Mutex<Vec<StackChild>>,
    hover_children: Mutex<Vec<Arc<dyn Widget>>>,
    bounds: Mutex<UIRect>,
    id: WidgetId,
}

fn map_child(child: &StackChild) -> Arc<dyn Widget> {
    child.widget.clone()
}

impl ContainerWidget for Stack {
    fn container_id(&self) -> WidgetId {
        self.id
    }

    fn set_container_position(&self, position: UIPos) {
        self.bounds.lock().pos = position;
    }

    fn get_container_bounds(&self) -> UIRect {
        *self.bounds.lock()
    }

    fn container_hints() -> ContainerHint {
        ContainerHint::empty()
    }

    type ChildrenGuard<'a> = MutexGuard<'a, Vec<StackChild>>;
    type ChildrenIterator<'c> =
        Map<std::slice::Iter<'c, StackChild>, fn(&StackChild) -> Arc<dyn Widget>>;

    fn lock_children(&self) -> Self::ChildrenGuard<'_> {
        self.children.lock()
    }

    fn iterate_child_widgets<'c>(
        &self,
        guard: &'c Self::ChildrenGuard<'_>,
    ) -> Self::ChildrenIterator<'c> {
        guard.iter().map(map_child)
    }

    fn hover_widgets(&self) -> MutexGuard<'_, Vec<Arc<dyn Widget>>> {
        self.hover_children.lock()
    }

    fn layout_container(&self, size_constraints: &UISizeConstraint) -> UISize {
        let mut container_size = size_constraints.min;
        let child_size_constraints = UISizeConstraint {
            min: UISize::ZERO,
            max: size_constraints.max,
        };

        let mut children = self.children.lock();

        for StackChild { widget, size, .. } in children.iter_mut() {
            *size = widget.layout(&child_size_constraints);
            debug_assert!(child_size_constraints.test(size));
            container_size.width = container_size.width.max(size.width);
            container_size.height = container_size.height.max(size.height);
        }

        self.bounds.lock().size = container_size;

        for StackChild {
            widget,
            size,
            alignment,
        } in children.iter_mut()
        {
            let x = alignment
                .horizontal
                .calc_x_offset(container_size.width, size.width);
            let y = alignment
                .vertical
                .calc_y_offset(container_size.height, size.height);
            widget.set_position(UIPos::new(x, y));
        }

        container_size
    }
}

impl Stack {
    pub fn new() -> Self {
        Self {
            id: acquire_widget_id(),
            children: Mutex::new(Vec::new()),
            bounds: Mutex::new(UIRect::ZERO),
            hover_children: Mutex::new(Vec::new()),
        }
    }

    pub fn push(&self, widget: impl Widget + 'static, alignment: Alignment) {
        self.push_arc(Arc::new(widget), alignment)
    }

    pub fn push_arc(&self, widget: Arc<dyn Widget>, alignment: Alignment) {
        self.children.lock().push(StackChild {
            widget,
            alignment,
            size: UISize::ZERO,
        })
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}
