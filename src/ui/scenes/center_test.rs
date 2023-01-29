use glam::Mat3;

use crate::ui::{
    common::{
        UIDrawContext, UIEventContext, UISizeConstraint, UIStateOpacity, UIStateSize,
        UIStateTransform,
    },
    containers::center,
    utils::{
        geom::{UIPos, UISize},
        helpers::{ZERO_POS, ZERO_SIZE},
    },
};

pub struct UISizeHint(Option<UISize>);

impl UISizeConstraint for UISizeHint {
    fn min_size(&self) -> Option<UISize> {
        self.0
    }

    fn max_size(&self) -> Option<UISize> {
        self.0
    }

    fn pref_size(&self) -> Option<UISize> {
        self.0
    }
}

mod debug_rect_test {
    use glam::{Mat3, Vec2};

    use crate::{
        ui::{
            common::{
                new_shared_state, SharedState, UIDrawContext, UIEventContext, UIStateOpacity,
                UIStatePos, UIStateSize, UIStateTransform,
            },
            controls::debug_rect,
            utils::{
                geom::{UIPos, UISize},
                helpers::{ZERO_POS, ZERO_SIZE},
            },
        },
        utils::mutex::MutexGuard,
    };

    use super::UISizeHint;

    pub struct State {
        pub state: debug_rect::State,
        pub pos: UIStatePos,
        pub size: UIStateSize<UISizeHint>,
    }

    pub struct EventScene {
        state: SharedState<State>,
    }

    pub struct DrawScene {
        state: SharedState<State>,
    }

    pub type PreDrawScene = DrawScene;

    impl State {
        pub fn new() -> Self {
            Self {
                state: debug_rect::State {
                    radius: Vec2::new(4.0, 4.0),
                },
                pos: ZERO_POS,
                size: UIStateSize(ZERO_SIZE, UISizeHint(Some(UISize::new(100.0, 100.0)))),
            }
        }

        pub fn relocate(&mut self, new_pos: UIPos) {
            self.pos = new_pos;
        }

        pub fn resize(&mut self, new_size: UISize) {
            *self.size = new_size;
        }

        pub(super) fn opacity() -> UIStateOpacity {
            UIStateOpacity(1.0)
        }

        pub(super) fn transform() -> UIStateTransform {
            UIStateTransform(Mat3::IDENTITY)
        }
    }

    impl Default for State {
        fn default() -> Self {
            Self::new()
        }
    }

    impl EventScene {
        pub fn new() -> Self {
            Self {
                state: new_shared_state(State::new()),
            }
        }

        pub fn create_predraw(&self, _: &mut UIEventContext) -> PreDrawScene {
            PreDrawScene {
                state: self.state.clone(),
            }
        }

        pub fn lock_state(&self) -> MutexGuard<'_, State> {
            self.state.lock()
        }
    }

    impl DrawScene {
        pub fn draw(&self, ctx: &UIDrawContext, parent_opacity: f32, parent_transform: &Mat3) {
            let (state, pos, size) = {
                let lock_state = self.state.lock();
                (lock_state.state, lock_state.pos, lock_state.size.0)
            };
            let opacity = State::opacity();
            let transform = State::transform();
            state.draw(
                ctx,
                &pos,
                &size,
                &opacity,
                &transform,
                parent_opacity,
                parent_transform,
            )
        }
    }
}

pub struct State {
    pos: UIPos,
    size: UIStateSize<UISizeHint>,
}

pub struct EventScene {
    container_state: State,
    child: debug_rect_test::EventScene,
}
pub struct DrawScene {
    child: debug_rect_test::DrawScene,
}
pub type PreDrawScene = DrawScene;

impl State {
    pub fn relocate(
        &mut self,
        ctx: &mut UIEventContext,
        new_pos: UIPos,
        child: &mut debug_rect_test::State,
    ) {
        let mut token = center::relocate(ctx, &mut self.pos, new_pos, &child.pos);
        if let Some(new_child_pos) = token.new_child_pos.take() {
            child.relocate(new_child_pos);
        }
    }

    pub fn resize(
        &mut self,
        ctx: &mut UIEventContext,
        new_size: UISize,
        child_state: &mut debug_rect_test::State,
    ) {
        let mut token = center::resize(
            ctx,
            &mut self.pos,
            &mut self.size,
            new_size,
            &child_state.pos,
            &child_state.size,
        );
        if let Some(new_child_pos) = token.new_child_pos.take() {
            child_state.relocate(new_child_pos);
        }
        if let Some(new_child_size) = token.new_child_size.take() {
            child_state.resize(new_child_size);
        }
    }

    fn opacity() -> UIStateOpacity {
        UIStateOpacity(1.0)
    }

    fn transform() -> UIStateTransform {
        UIStateTransform(Mat3::IDENTITY)
    }
}

impl EventScene {
    pub fn new() -> Self {
        Self {
            container_state: State {
                pos: ZERO_POS,
                size: UIStateSize(ZERO_SIZE, UISizeHint(None)),
            },
            child: debug_rect_test::EventScene::new(),
        }
    }

    pub fn create_predraw(&self, ctx: &mut UIEventContext) -> PreDrawScene {
        PreDrawScene {
            child: self.child.create_predraw(ctx),
        }
    }

    pub fn relocate(&mut self, ctx: &mut UIEventContext, new_pos: UIPos) {
        let mut child_state = self.child.lock_state();
        self.container_state
            .relocate(ctx, new_pos, &mut *child_state);
    }

    pub fn resize(&mut self, ctx: &mut UIEventContext, new_size: UISize) {
        let mut child_state = self.child.lock_state();
        self.container_state
            .resize(ctx, new_size, &mut *child_state);
    }
}

impl Default for EventScene {
    fn default() -> Self {
        Self::new()
    }
}

impl DrawScene {
    pub fn draw(&self, ctx: &mut UIDrawContext, parent_opacity: f32, parent_transform: &Mat3) {
        let cont_opacity = State::opacity();
        let cont_transform = State::transform();
        let token = center::draw(
            ctx,
            &cont_opacity,
            &cont_transform,
            parent_opacity,
            parent_transform,
        );
        self.child
            .draw(ctx, token.self_opacity, &token.self_transform)
    }
}
