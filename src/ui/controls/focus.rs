use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Weak,
};

use winit::event::{ElementState, MouseButton};

use crate::ui::{
    acquire_widget_id,
    event::UIPropagatingEvent,
    utils::geom::{UIRect, UISize},
    EventContext, UISizeConstraint, Widget, WidgetId,
};

// must be a child in a stack with zero padding
pub struct Focus {
    id: WidgetId,
    owner: Weak<dyn Widget>,
    focused: AtomicBool,
}

impl Focus {
    pub fn new(owner: Weak<dyn Widget>) -> Self {
        Self {
            id: acquire_widget_id(),
            owner,
            focused: AtomicBool::new(false),
        }
    }
}

impl Widget for Focus {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _: &UISizeConstraint) -> UISize {
        UISize::new(UISize::FIT_CONTAINER, UISize::FIT_CONTAINER)
    }

    fn get_bounds(&self) -> UIRect {
        self.owner
            .upgrade()
            .map(|w| w.get_bounds())
            .unwrap_or_default()
    }

    fn set_bounds(&self, bounds: UIRect) {
        const EPSILON: f32 = 1e-4;
        debug_assert!(bounds.pos.x.abs() < EPSILON);
        debug_assert!(bounds.pos.y.abs() < EPSILON);
    }

    fn handle_propagating_event(
        self: Arc<Self>,
        ctx: &mut EventContext,
        event: UIPropagatingEvent,
    ) -> Option<UIPropagatingEvent> {
        match &event {
            UIPropagatingEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
            } => {
                ctx.main_ctx.set_focus_widget(Some(self));
                return None;
            }

            UIPropagatingEvent::VisibilityChanged(visibility) if !visibility.handle_event() => {
                if self.focused.load(Ordering::Relaxed) {
                    ctx.main_ctx.set_focus_widget(None);
                }
            }

            _ => {}
        }

        Some(event)
    }

    fn focus_changed(&self, _: &mut EventContext, new_focus: bool) {
        self.focused.store(new_focus, Ordering::Relaxed);
    }
}
