use glutin::config::{Config, ConfigTemplateBuilder};
use glutin_winit::DisplayBuilder;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoopWindowTarget,
    window::{Window, WindowBuilder, WindowId},
};

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
    ) -> anyhow::Result<(Display, Config)> {
        let (window, gl_config) = DisplayBuilder::new()
            .with_window_builder(Some(
                WindowBuilder::new().with_inner_size(size).with_title(title),
            ))
            .build(event_loop, ConfigTemplateBuilder::new(), |mut config| {
                config.next().expect("no OpenGL config found")
            })
            .map_err(|e| anyhow::format_err!("{}", e))?;
        Ok((
            Display {
                window: window.unwrap(),
            },
            gl_config,
        ))
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
}
