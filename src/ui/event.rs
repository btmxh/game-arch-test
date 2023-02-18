use std::path::PathBuf;

use winit::{
    event::{ElementState, Ime, KeyboardInput, MouseButton, MouseScrollDelta},
    window::Theme,
};

use super::{utils::geom::UIPos, Visibility};

#[derive(Clone, Debug, PartialEq)]
pub enum DragDropAction {
    Hover(PathBuf),
    Drop(PathBuf),
    CancelDrop,
}

// applied to only the focused widget
#[derive(Clone, Debug, PartialEq)]
pub enum UIFocusEvent {
    Focus(bool),
    ReceivedCharacter(char),
    Ime(Ime),
    KeyboardInput(KeyboardInput),

    TestEvent(u32),
}

// propagated from the root widget
#[derive(Clone, Debug, PartialEq)]
pub enum UIPropagatingEvent {
    ThemeChanged(Theme),
    DragDrop(DragDropAction),
    MouseWheel(MouseScrollDelta),
    MouseInput {
        state: ElementState,
        button: MouseButton,
    },
    VisibilityChanged(Visibility),
    TestHover,
}

impl UIPropagatingEvent {
    pub fn only_propagate_hover(&self) -> bool {
        !matches!(self, UIPropagatingEvent::ThemeChanged(_))
            && !matches!(self, UIPropagatingEvent::VisibilityChanged(_))
    }
}

// special cursor events, entered and exited
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UICursorEvent {
    CursorEntered,
    CursorExited,
    CursorMoved(UIPos),
}
