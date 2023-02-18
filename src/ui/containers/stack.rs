use std::{iter::Map, sync::Arc};

use crate::{
    ui::{
        acquire_widget_id,
        utils::geom::{UIPos, UIRect, UISize},
        Alignment, Padding, UISizeConstraint, Visibility, Widget, WidgetId,
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
    padding: Mutex<Padding>,
    visibility: Mutex<Visibility>,
}

fn map_child(child: &StackChild) -> Arc<dyn Widget> {
    child.widget.clone()
}

impl ContainerWidget for Stack {
    fn container_id(&self) -> WidgetId {
        self.id
    }

    fn set_container_bounds(&self, bounds: UIRect) {
        *self.bounds.lock() = bounds;
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
        let (size_constraints, pos_offset) =
            self.padding.lock().apply_to_constraints(size_constraints);
        let mut container_size = size_constraints.min;
        let child_size_constraints = UISizeConstraint {
            min: UISize::ZERO,
            max: size_constraints.max,
        };

        let mut children = self.children.lock();

        for StackChild { widget, size, .. } in children.iter_mut() {
            *size = widget.layout(&child_size_constraints);
            // special case: size.width
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
            if size.width == UISize::FIT_CONTAINER {
                size.width = container_size.width;
            }

            if size.height == UISize::FIT_CONTAINER {
                size.height = container_size.height;
            }

            let x = alignment
                .horizontal
                .calc_x_offset(container_size.width, size.width)
                + pos_offset.x;
            let y = alignment
                .vertical
                .calc_y_offset(container_size.height, size.height)
                + pos_offset.y;
            widget.set_bounds(UIRect::new(UIPos::new(x, y), *size));
        }

        container_size
    }

    fn get_visibility(&self) -> Visibility {
        *self.visibility.lock()
    }

    fn set_visibility(&self, visibility: Visibility) {
        *self.visibility.lock() = visibility;
    }
}

impl Stack {
    pub fn new() -> Self {
        Self {
            id: acquire_widget_id(),
            children: Mutex::new(Vec::new()),
            bounds: Mutex::new(UIRect::ZERO),
            hover_children: Mutex::new(Vec::new()),
            padding: Mutex::new(Padding::default()),
            visibility: Mutex::new(Visibility::Visible),
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
