use std::path::PathBuf;

use winit::{
    event::{ElementState, Ime, KeyboardInput, MouseButton, MouseScrollDelta},
    window::Theme,
};

use super::utils::geom::UIPos;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorPosition {
    pub relative: UIPos,
    pub absolute: UIPos,
}

pub enum DragDropAction {
    Hover(PathBuf),
    Drop(PathBuf),
    CancelDrop,
}

// applied to only the focused widget
pub enum UIFocusEvent {
    Focus(bool),
    ReceivedCharacter(char),
    Ime(Ime),
    KeyboardInput(KeyboardInput),
}

// propagated from the root widget
pub enum UIPropagatingEvent {
    ThemeChanged(Theme),
    DragDrop(DragDropAction),
    MouseWheel(MouseScrollDelta),
    MouseInput {
        state: ElementState,
        button: MouseButton,
    },
}

// special cursor events, entered and exited
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UICursorEvent {
    CursorEntered,
    CursorExited,
    CursorMoved(CursorPosition),
}
