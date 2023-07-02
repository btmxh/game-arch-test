use anyhow::Context;
use winit::{
    dpi::PhysicalSize,
    event_loop::{EventLoop, EventLoopProxy, EventLoopWindowTarget},
    window::{Window, WindowBuilder, WindowId},
};

use crate::{events::GameUserEvent, utils::args::args};

pub struct Display {
    window: Window,
}

#[derive(Clone)]
pub struct EventSender {
    loop_proxy: EventLoopProxy<GameUserEvent>,
}

impl Display {
    pub fn new<T>(
        event_loop: &EventLoopWindowTarget<T>,
        size: PhysicalSize<u32>,
        title: &str,
    ) -> anyhow::Result<Display> {
        let span = tracing::trace_span!("Creating display window");
        let _enter = span.enter();
        let window_builder = WindowBuilder::new()
            .with_inner_size(size)
            .with_title(title)
            .with_visible(!args().headless);
        tracing::trace!("WindowBuilder structure: {:?}", window_builder);

        Ok(Display {
            window: window_builder
                .build(event_loop)
                .context("unable to create display")?,
        })
    }

    pub fn get_window_id(&self) -> WindowId {
        self.window.id()
    }

    pub fn get_size(&self) -> PhysicalSize<u32> {
        self.window.inner_size()
    }

    pub fn get_scale_factor(&self) -> f64 {
        self.window.scale_factor()
    }

    pub fn get_winit_window(&self) -> &Window {
        &self.window
    }
}

impl EventSender {
    pub fn new(event_loop: &EventLoop<GameUserEvent>) -> Self {
        Self {
            loop_proxy: event_loop.create_proxy(),
        }
    }
    pub fn send_event(&self, event: GameUserEvent) -> anyhow::Result<()> {
        self.loop_proxy
            .send_event(event)
            .map_err(|err| anyhow::anyhow!("{}", err))
            .context("Unable to send event to main event loop")
    }
}
