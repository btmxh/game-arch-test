use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Context;
use wgpu::PresentMode;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};

use crate::{
    events::GameEvent,
    exec::{main_ctx::MainContext, server::draw::ServerSendChannelExt},
    scene::{main::RootScene, Scene},
    utils::error::ResultExt,
};

pub struct VSync {
    current_vsync: AtomicBool,
}

impl Scene for VSync {
    fn handle_event<'a>(
        self: Arc<Self>,
        ctx: &mut MainContext,
        _: &RootScene,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        match &event {
            Event::WindowEvent {
                window_id,
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Released,
                                virtual_keycode: Some(VirtualKeyCode::E),
                                ..
                            },
                        ..
                    },
            } if ctx.display.get_window_id() == *window_id => {
                self.toggle(ctx)
                    .context("unable to toggle VSync mode")
                    .log_warn();
            }

            _ => {}
        };

        Some(event)
    }
}

impl VSync {
    pub fn new(main_ctx: &mut MainContext) -> anyhow::Result<Self> {
        main_ctx.execute_draw_sync(|draw_ctx, _| {
            let present_modes = draw_ctx
                .surface
                .get_capabilities(&draw_ctx.adapter)
                .present_modes;
            for present_mode in present_modes {
                tracing::info!("Supported present mode: {:?}", present_mode);
            }
        })?;
        let slf = Self {
            current_vsync: AtomicBool::new(false),
        };
        slf.toggle(main_ctx)
            .context("unable to reset vsync to default state")?; // current_mode is now true
        Ok(slf)
    }

    pub fn toggle(&self, main_ctx: &mut MainContext) -> anyhow::Result<()> {
        let current_vsync = !self.current_vsync.load(Ordering::Relaxed);
        self.current_vsync.store(current_vsync, Ordering::Relaxed);
        let interval = if current_vsync {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        };
        main_ctx.channels.draw.execute(move |s, _| {
            s.set_swap_interval(interval)
                .with_context(|| format!("unable to set vsync swap interval to {interval:?}"))
                .log_error();
            tracing::info!("VSync swap interval set to {interval:?}");
        })?;

        Ok(())
    }
}
