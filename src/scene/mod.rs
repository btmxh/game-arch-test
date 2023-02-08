use std::sync::Arc;

use crate::{events::GameEvent, exec::main_ctx::MainContext, graphics::context::DrawContext};

use self::main::RootScene;

pub mod main;

#[derive(Default)]
pub struct SceneContainer {
    scenes: Vec<Arc<dyn Scene>>,
}

impl SceneContainer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, scene: impl Scene + 'static) {
        self.push_arc(Arc::new(scene))
    }

    pub fn push_arc(&mut self, scene: Arc<dyn Scene>) {
        self.scenes.push(scene)
    }

    pub fn push_all(&mut self, mut container: SceneContainer) {
        self.scenes.append(&mut container.scenes);
    }
}

pub trait Scene: Send + Sync {
    fn handle_event<'a>(
        self: Arc<Self>,
        _ctx: &mut MainContext,
        _root_scene: &RootScene,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        Some(event)
    }

    fn draw(self: Arc<Self>, _ctx: &mut DrawContext) {}
}

impl Scene for SceneContainer {
    fn handle_event<'a>(
        self: Arc<Self>,
        ctx: &mut MainContext,
        root_scene: &RootScene,
        mut event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        for scene in self.scenes.iter().rev() {
            if let Some(e) = scene.clone().handle_event(ctx, root_scene, event) {
                event = e;
            } else {
                return None;
            }
        }

        Some(event)
    }

    fn draw(self: Arc<Self>, ctx: &mut DrawContext) {
        for scene in self.scenes.iter() {
            scene.clone().draw(ctx);
        }
    }
}
