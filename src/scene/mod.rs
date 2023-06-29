use std::sync::Arc;

use trait_set::trait_set;

use crate::{
    events::GameEvent,
    exec::main_ctx::MainContext,
    graphics::context::{DrawContext, DrawingContext},
};

use self::main::RootScene;

pub mod main;

#[derive(Default)]
pub struct SceneContainer {
    scenes: Vec<Arc<dyn Scene>>,
}

trait_set! {
    pub trait EventHandler = Fn(&mut MainContext, &RootScene, GameEvent<'static>) -> Option<GameEvent<'static>>
            + Send
            + Sync;
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

    pub fn push_event_handler<F>(&mut self, event_handler: F)
    where
        F: EventHandler + 'static,
    {
        struct EventHandlerScene<F> {
            event_handler: F,
        }

        impl<F> Scene for EventHandlerScene<F>
        where
            F: EventHandler + 'static,
        {
            fn handle_event<'a>(
                self: Arc<Self>,
                ctx: &mut MainContext,
                root_scene: &RootScene,
                event: GameEvent<'a>,
            ) -> Option<GameEvent<'a>> {
                if let Some(event) = event.to_static() {
                    (self.event_handler)(ctx, root_scene, event)
                } else {
                    None
                }
            }
        }

        let scene = EventHandlerScene { event_handler };
        self.push(scene);
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

    fn draw(self: Arc<Self>, _ctx: &mut DrawContext, _drawing: &DrawingContext) {}
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

    fn draw(self: Arc<Self>, ctx: &mut DrawContext, drawing: &DrawingContext) {
        for scene in self.scenes.iter() {
            scene.clone().draw(ctx, drawing);
        }
    }
}
