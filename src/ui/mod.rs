use std::sync::atomic::{AtomicUsize, Ordering};

use event::{UICursorEvent, UIFocusEvent, UIPropagatingEvent};
use utils::geom::{UIPos, UIRect, UISize};

use crate::{exec::main_ctx::MainContext, graphics::context::DrawContext, scene::main::RootScene};

pub mod containers;
pub mod controls;
pub mod event;
pub mod utils;

pub type WidgetId = usize;

static WIDGET_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn acquire_widget_id() -> usize {
    WIDGET_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub struct EventContext<'a> {
    pub main_ctx: &'a mut MainContext,
    pub root_scene: &'a RootScene,
}

pub trait Widget {
    fn id(&self) -> WidgetId;

    fn handle_propagating_event(
        &self,
        _ctx: &mut EventContext,
        event: UIPropagatingEvent,
    ) -> Option<UIPropagatingEvent> {
        Some(event)
    }

    fn handle_focus_event(&self, _ctx: &mut EventContext, _event: UIFocusEvent) {}

    fn handle_cursor_event(
        &self,
        _ctx: &mut EventContext,
        event: UICursorEvent,
    ) -> Option<UICursorEvent> {
        Some(event)
    }

    fn draw(&self, _ctx: &mut DrawContext) {}

    fn layout(&self, size_constraints: &UISizeConstraint) -> UISize;
    fn set_position(&self, position: UIPos);
    fn get_bounds(&self) -> UIRect;
}

pub struct UISizeConstraint {
    pub min: UISize,
    pub max: UISize,
}

impl UISizeConstraint {
    pub fn test(&self, size: &UISize) -> bool {
        self.min.width <= size.width
            && size.width <= self.max.width
            && self.min.height <= size.height
            && size.height <= self.max.height
    }
}

pub struct Alignment {
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
}

#[derive(Clone, Copy, Debug, Hash)]
pub enum HorizontalAlignment {
    Left,
    Right,
    Middle,
}

#[derive(Clone, Copy, Debug, Hash)]
pub enum VerticalAlignment {
    Top,
    Bottom,
    Center,
}

impl HorizontalAlignment {
    pub fn calc_x_offset(self, container_width: f32, width: f32) -> f32 {
        match self {
            HorizontalAlignment::Left => 0.0,
            HorizontalAlignment::Right => container_width - width,
            HorizontalAlignment::Middle => (container_width - width) * 0.5,
        }
    }
}

impl VerticalAlignment {
    pub fn calc_y_offset(self, container_height: f32, height: f32) -> f32 {
        match self {
            VerticalAlignment::Top => 0.0,
            VerticalAlignment::Bottom => container_height - height,
            VerticalAlignment::Center => (container_height - height) * 0.5,
        }
    }
}
