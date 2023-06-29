use anyhow::Context;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoopWindowTarget,
    window::{Window, WindowBuilder, WindowId},
};

use crate::utils::args::args;

pub struct Display {
    window: Window,
}

pub struct SendRawHandle(pub RawWindowHandle, pub RawDisplayHandle);
unsafe impl Send for SendRawHandle {}

impl Display {
    pub fn new_display<T>(
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

    pub fn get_raw_window_handle(&self) -> RawWindowHandle {
        self.window.raw_window_handle()
    }

    pub fn get_raw_display_handle(&self) -> RawDisplayHandle {
        self.window.raw_display_handle()
    }

    pub fn get_raw_handles(&self) -> SendRawHandle {
        SendRawHandle(self.get_raw_window_handle(), self.get_raw_display_handle())
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
