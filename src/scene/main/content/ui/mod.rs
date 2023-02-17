use std::sync::Arc;

use winit::event::{Event, ModifiersState, WindowEvent};

use crate::{
    events::{GameEvent, GameUserEvent},
    exec::main_ctx::MainContext,
    graphics::context::DrawContext,
    scene::{main::RootScene, Scene},
    ui::{
        containers::stack::Stack,
        event::{DragDropAction, UICursorEvent, UIFocusEvent, UIPropagatingEvent},
        EventContext, UISizeConstraint, Widget,
    },
    utils::mutex::Mutex,
};

pub mod settings;

pub struct UI {
    pub root: Arc<Stack>,
    pub modifiers: Mutex<ModifiersState>,
    focused: Mutex<Option<Arc<dyn Widget>>>,
}

impl UI {
    pub fn new(main_ctx: &mut MainContext) -> anyhow::Result<Arc<Self>> {
        let slf = Arc::new(Self {
            root: Arc::new(Stack::new()),
            focused: Mutex::new(None),
            modifiers: Mutex::new(ModifiersState::default()),
        });

        settings::init(&slf);

        Ok(slf)
    }

    fn handle_win_event<'a>(
        self: Arc<Self>,
        main_ctx: &mut MainContext,
        event: WindowEvent<'a>,
    ) -> Option<WindowEvent<'a>> {
        let mut ctx = EventContext { main_ctx };
        // these kinds of events contain non-copyable data
        let event = match event {
            WindowEvent::DroppedFile(path) => {
                return if let Some(UIPropagatingEvent::DragDrop(DragDropAction::Drop(path))) =
                    self.root.handle_propagating_event(
                        &mut ctx,
                        UIPropagatingEvent::DragDrop(DragDropAction::Drop(path)),
                    ) {
                    Some(WindowEvent::DroppedFile(path))
                } else {
                    None
                };
            }

            WindowEvent::HoveredFile(path) => {
                return if let Some(UIPropagatingEvent::DragDrop(DragDropAction::Hover(path))) =
                    self.root.handle_propagating_event(
                        &mut ctx,
                        UIPropagatingEvent::DragDrop(DragDropAction::Hover(path)),
                    ) {
                    Some(WindowEvent::HoveredFile(path))
                } else {
                    None
                };
            }

            WindowEvent::Ime(ime) => {
                let lock = self.focused.lock();
                return if let Some(focus_widget) = lock.as_ref() {
                    if let Some(UIFocusEvent::Ime(ime)) =
                        focus_widget.handle_focus_event(&mut ctx, UIFocusEvent::Ime(ime))
                    {
                        Some(WindowEvent::Ime(ime))
                    } else {
                        None
                    }
                } else {
                    Some(WindowEvent::Ime(ime))
                };
            }

            e => e,
        };

        match &event {
            WindowEvent::HoveredFileCancelled => self
                .root
                .handle_propagating_event(
                    &mut ctx,
                    UIPropagatingEvent::DragDrop(DragDropAction::CancelDrop),
                )
                .is_some(),
            WindowEvent::ReceivedCharacter(ch) => self
                .focused
                .lock()
                .as_ref()
                .map(|w| {
                    w.handle_focus_event(&mut ctx, UIFocusEvent::ReceivedCharacter(*ch))
                        .is_some()
                })
                .unwrap_or(true),
            WindowEvent::KeyboardInput { input, .. } => self
                .focused
                .lock()
                .as_ref()
                .map(|w| {
                    w.handle_focus_event(&mut ctx, UIFocusEvent::KeyboardInput(*input))
                        .is_some()
                })
                .unwrap_or(true),
            WindowEvent::ModifiersChanged(mods) => {
                *self.modifiers.lock() = *mods;
                false
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale_factor = ctx.main_ctx.display.get_scale_factor();
                self.root
                    .handle_cursor_event(
                        &mut ctx,
                        UICursorEvent::CursorMoved(position.to_logical(scale_factor).into()),
                    )
                    .is_some()
            }
            WindowEvent::CursorEntered { .. } => self
                .root
                .handle_cursor_event(&mut ctx, UICursorEvent::CursorEntered)
                .is_some(),
            WindowEvent::CursorLeft { .. } => self
                .root
                .handle_cursor_event(&mut ctx, UICursorEvent::CursorExited)
                .is_some(),
            WindowEvent::MouseWheel { delta, .. } => self
                .root
                .handle_propagating_event(&mut ctx, UIPropagatingEvent::MouseWheel(*delta))
                .is_some(),
            WindowEvent::MouseInput { state, button, .. } => self
                .root
                .handle_propagating_event(
                    &mut ctx,
                    UIPropagatingEvent::MouseInput {
                        state: *state,
                        button: *button,
                    },
                )
                .is_some(),
            WindowEvent::ThemeChanged(theme) => self
                .root
                .handle_propagating_event(&mut ctx, UIPropagatingEvent::ThemeChanged(*theme))
                .is_some(),

            _ => true,
        }
        .then_some(event)
    }
}

impl Scene for UI {
    fn handle_event<'a>(
        self: Arc<Self>,
        ctx: &mut MainContext,
        _root_scene: &RootScene,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        if let Event::UserEvent(GameUserEvent::CheckedResize { ui_size, .. }) = &event {
            self.root.layout(&UISizeConstraint::exact(*ui_size));
        }
        if let Event::WindowEvent { window_id, event } = event {
            if window_id == ctx.display.get_window_id() {
                return self
                    .handle_win_event(ctx, event)
                    .map(|event| Event::WindowEvent { window_id, event });
            } else {
                Some(Event::WindowEvent { window_id, event })
            }
        } else {
            Some(event)
        }
    }

    fn draw(self: Arc<Self>, ctx: &mut DrawContext) {
        self.root.draw(ctx)
    }
}
