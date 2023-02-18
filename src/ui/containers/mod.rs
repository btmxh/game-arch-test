use std::{collections::HashMap, sync::Arc};

use bitflags::bitflags;

use crate::{graphics::context::DrawContext, utils::mutex::MutexGuard};

use super::{
    event::{UICursorEvent, UIFocusEvent, UIPropagatingEvent},
    utils::geom::{UIPos, UIRect, UISize},
    EventContext, UISizeConstraint, Visibility, Widget, WidgetId,
};

pub mod linear_box;
pub mod stack;

bitflags! {
    pub struct ContainerHint : u32 {
        const NO_OVERLAP = 0x1;
    }
}

pub trait ContainerWidget: Widget {
    fn container_id(&self) -> WidgetId;
    fn layout_container(&self, size_constraints: &UISizeConstraint) -> UISize;
    fn set_container_bounds(&self, bounds: UIRect);
    fn get_container_bounds(&self) -> UIRect;

    fn container_hints() -> ContainerHint;

    type ChildrenGuard<'a>;
    type ChildrenIterator<'c>: DoubleEndedIterator<Item = Arc<dyn Widget>>;

    fn lock_children(&self) -> Self::ChildrenGuard<'_>;
    fn iterate_child_widgets<'c>(
        &self,
        guard: &'c Self::ChildrenGuard<'_>,
    ) -> Self::ChildrenIterator<'c>;

    fn hover_widgets(&self) -> MutexGuard<'_, Vec<Arc<dyn Widget>>>;

    fn handle_focus_event_impl(
        &self,
        _ctx: &mut EventContext,
        event: UIFocusEvent,
    ) -> Option<UIFocusEvent> {
        Some(event)
    }

    fn handle_propagating_event_impl(
        &self,
        _ctx: &mut EventContext,
        event: UIPropagatingEvent,
    ) -> Option<UIPropagatingEvent> {
        Some(event)
    }

    fn handle_cursor_event_impl(
        &self,
        _ctx: &mut EventContext,
        event: UICursorEvent,
    ) -> Option<UICursorEvent> {
        Some(event)
    }

    fn get_visibility(&self) -> Visibility;
    fn set_visibility(&self, visibility: Visibility);
}

impl<T: ContainerWidget> Widget for T {
    fn id(&self) -> WidgetId {
        self.container_id()
    }

    fn layout(&self, size_constraints: &UISizeConstraint) -> UISize {
        self.layout_container(size_constraints)
    }

    fn set_bounds(&self, bounds: UIRect) {
        self.set_container_bounds(bounds);
    }

    fn get_bounds(&self) -> UIRect {
        self.get_container_bounds()
    }

    fn handle_focus_event(
        self: Arc<Self>,
        ctx: &mut EventContext,
        event: UIFocusEvent,
    ) -> Option<UIFocusEvent> {
        if !self.get_visibility().handle_event() {
            return Some(event);
        }
        self.handle_focus_event_impl(ctx, event)
    }

    fn handle_propagating_event(
        self: Arc<Self>,
        ctx: &mut EventContext,
        event: UIPropagatingEvent,
    ) -> Option<UIPropagatingEvent> {
        if !self.get_visibility().handle_event()
            && !matches!(event, UIPropagatingEvent::VisibilityChanged(_))
        {
            return Some(event);
        }
        self.handle_propagating_event_impl(ctx, event)
            .and_then(|mut event| {
                if event.only_propagate_hover() {
                    let hover_widgets = self.hover_widgets();
                    for widget in hover_widgets.iter() {
                        if let Some(evt) = widget.clone().handle_propagating_event(ctx, event) {
                            event = evt;
                        } else {
                            return None;
                        }
                    }
                } else {
                    let guard = self.lock_children();
                    for widget in self.iterate_child_widgets(&guard).rev() {
                        if let Some(evt) = widget.clone().handle_propagating_event(ctx, event) {
                            event = evt;
                        } else {
                            return None;
                        }
                    }
                }

                Some(event)
            })
    }

    fn handle_cursor_event(
        self: Arc<Self>,
        ctx: &mut EventContext,
        event: UICursorEvent,
    ) -> Option<UICursorEvent> {
        if !self.get_visibility().handle_event() {
            return Some(event);
        }
        self.handle_cursor_event_impl(ctx, event)
            .and_then(|event| match event {
                UICursorEvent::CursorEntered => Some(event),
                UICursorEvent::CursorExited => {
                    let mut hover_widgets = self.hover_widgets();
                    for widget in hover_widgets.iter() {
                        widget.clone().handle_cursor_event(ctx, event);
                    }
                    hover_widgets.clear();

                    Some(event)
                }
                UICursorEvent::CursorMoved(position) => {
                    let mut hover_widgets = self.hover_widgets();
                    let mut last_hover_widgets = hover_widgets
                        .iter()
                        .map(|widget| (widget.id(), widget.clone()))
                        .collect::<HashMap<_, _>>();
                    hover_widgets.clear();
                    let children = self.lock_children();
                    for widget in self.iterate_child_widgets(&children).rev() {
                        let id = widget.id();
                        let bounds = widget.get_bounds();

                        if !bounds.contains(position) {
                            continue;
                        }

                        if last_hover_widgets.remove(&id).is_none() {
                            widget
                                .clone()
                                .handle_cursor_event(ctx, UICursorEvent::CursorEntered);
                        }

                        hover_widgets.push(widget.clone());

                        widget.handle_cursor_event(
                            ctx,
                            UICursorEvent::CursorMoved(UIPos::new(
                                position.x - bounds.pos.x,
                                position.y - bounds.pos.y,
                            )),
                        )?;
                    }

                    last_hover_widgets.values().for_each(|widget| {
                        widget
                            .clone()
                            .handle_cursor_event(ctx, UICursorEvent::CursorExited);
                    });

                    Some(event)
                }
            })
    }

    fn draw(&self, ctx: &mut DrawContext) {
        if !self.get_visibility().draw() {
            return;
        }

        let old_len = ctx.transform_stack.len();
        ctx.transform_stack.push();
        ctx.transform_stack.translate(self.get_bounds().pos);

        let children = self.lock_children();
        for widget in self.iterate_child_widgets(&children) {
            widget.draw(ctx);
        }

        ctx.transform_stack.pop();
        debug_assert!(old_len == ctx.transform_stack.len());
    }
}
