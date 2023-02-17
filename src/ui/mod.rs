use std::sync::atomic::{AtomicUsize, Ordering};

use event::{UICursorEvent, UIFocusEvent, UIPropagatingEvent};
use utils::geom::{UIPos, UIRect, UISize};

use crate::{exec::main_ctx::MainContext, graphics::context::DrawContext};

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
    ////  pub root_scene: &'a RootScene,
}

pub trait Widget: Send + Sync {
    fn id(&self) -> WidgetId;

    fn handle_propagating_event(
        &self,
        _ctx: &mut EventContext,
        event: UIPropagatingEvent,
    ) -> Option<UIPropagatingEvent> {
        Some(event)
    }

    fn handle_focus_event(
        &self,
        _ctx: &mut EventContext,
        event: UIFocusEvent,
    ) -> Option<UIFocusEvent> {
        Some(event)
    }

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

#[derive(Clone, Copy, Debug)]
pub struct UISizeConstraint {
    pub min: UISize,
    pub max: UISize,
}

impl UISizeConstraint {
    pub fn new(min: UISize, max: UISize) -> Self {
        Self { min, max }
    }

    pub fn exact(size: UISize) -> Self {
        Self::new(size, size)
    }

    pub fn test(&self, size: &UISize) -> bool {
        self.min.width <= size.width
            && size.width <= self.max.width
            && self.min.height <= size.height
            && size.height <= self.max.height
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Alignment {
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
}

impl Alignment {
    pub fn new(horizontal: HorizontalAlignment, vertical: VerticalAlignment) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum HorizontalAlignment {
    Left,
    Right,
    Center,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Bottom,
    Middle,
}

impl HorizontalAlignment {
    pub fn calc_x_offset(self, container_width: f32, width: f32) -> f32 {
        match self {
            HorizontalAlignment::Left => 0.0,
            HorizontalAlignment::Right => container_width - width,
            HorizontalAlignment::Center => (container_width - width) * 0.5,
        }
    }
}

impl VerticalAlignment {
    pub fn calc_y_offset(self, container_height: f32, height: f32) -> f32 {
        match self {
            VerticalAlignment::Top => 0.0,
            VerticalAlignment::Bottom => container_height - height,
            VerticalAlignment::Middle => (container_height - height) * 0.5,
        }
    }
}

pub trait Axis: 'static {
    type MainAlignment: Send + Sync + Clone + Copy + 'static;
    type CrossAlignment: Send + Sync + Clone + Copy + 'static;
    type OtherAxis: Axis<
        MainAlignment = Self::CrossAlignment,
        CrossAlignment = Self::MainAlignment,
        OtherAxis = Self,
    >;

    fn get_pos(pos: UIPos) -> f32;
    fn get_size(size: UISize) -> f32;

    fn new_pos(this_axis: f32, other_axis: f32) -> UIPos;
    fn new_size(this_axis: f32, other_axis: f32) -> UISize;

    fn calc_align_offset(alignment: Self::MainAlignment, container_size: f32, size: f32) -> f32;
}

pub struct AxisX;
pub struct AxisY;

impl Axis for AxisX {
    type MainAlignment = HorizontalAlignment;
    type CrossAlignment = VerticalAlignment;

    type OtherAxis = AxisY;

    fn get_pos(pos: UIPos) -> f32 {
        pos.x
    }

    fn get_size(size: UISize) -> f32 {
        size.width
    }

    fn new_pos(this_axis: f32, other_axis: f32) -> UIPos {
        UIPos::new(this_axis, other_axis)
    }

    fn new_size(this_axis: f32, other_axis: f32) -> UISize {
        UISize::new(this_axis, other_axis)
    }

    fn calc_align_offset(alignment: Self::MainAlignment, container_size: f32, size: f32) -> f32 {
        alignment.calc_x_offset(container_size, size)
    }
}

impl Axis for AxisY {
    type MainAlignment = VerticalAlignment;
    type CrossAlignment = HorizontalAlignment;

    type OtherAxis = AxisX;

    fn get_pos(pos: UIPos) -> f32 {
        pos.y
    }

    fn get_size(size: UISize) -> f32 {
        size.height
    }

    fn new_pos(this_axis: f32, other_axis: f32) -> UIPos {
        UIPos::new(other_axis, this_axis)
    }

    fn new_size(this_axis: f32, other_axis: f32) -> UISize {
        UISize::new(other_axis, this_axis)
    }

    fn calc_align_offset(alignment: Self::MainAlignment, container_size: f32, size: f32) -> f32 {
        alignment.calc_y_offset(container_size, size)
    }
}

#[derive(Default)]
pub struct Padding {
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,
}

impl Padding {
    fn remove_padding(&self, size: UISize) -> UISize {
        let width = 0.0f32.max(size.width - self.left - self.right);
        let height = 0.0f32.max(size.height - self.top - self.bottom);
        UISize::new(width, height)
    }

    pub fn apply_to_constraints(
        &self,
        constraints: &UISizeConstraint,
    ) -> (UISizeConstraint, UIPos) {
        (
            UISizeConstraint {
                min: self.remove_padding(constraints.min),
                max: self.remove_padding(constraints.max),
            },
            UIPos::new(self.left, self.top),
        )
    }
}
