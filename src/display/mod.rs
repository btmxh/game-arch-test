use anyhow::Context;
use glutin::{
    config::{Config, ConfigTemplateBuilder},
    display::GetGlDisplay,
};
use glutin_winit::DisplayBuilder;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoopWindowTarget,
    window::{Window, WindowBuilder},
};

pub struct Display {
    window: Window,
}

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

    pub fn kys(&mut self) {
        println!("hello");
    }

    pub fn get_raw_window_handle(&self) -> RawWindowHandle {
        self.window.raw_window_handle()
    }

    pub fn get_raw_display_handle(&self) -> RawDisplayHandle {
        self.window.raw_display_handle()
    }
}
