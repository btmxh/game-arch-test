use std::collections::VecDeque;

use glutin::{
    config::{Api, ColorBufferType, Config, ConfigSurfaceTypes, ConfigTemplateBuilder},
    prelude::GlConfig,
};
use glutin_winit::DisplayBuilder;
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

#[derive(Debug)]
#[allow(dead_code)]
struct GLConfigInfo {
    color_buffer_type: Option<ColorBufferType>,
    float_pixels: bool,
    alpha_size: u8,
    depth_size: u8,
    stencil_size: u8,
    num_samples: u8,
    srgb_capable: bool,
    supports_transparency: Option<bool>,
    config_surface_types: ConfigSurfaceTypes,
    api: Api,
}

impl GLConfigInfo {
    pub fn new(config: &Config) -> Self {
        Self {
            alpha_size: config.alpha_size(),
            api: config.api(),
            color_buffer_type: config.color_buffer_type(),
            config_surface_types: config.config_surface_types(),
            depth_size: config.depth_size(),
            float_pixels: config.float_pixels(),
            num_samples: config.num_samples(),
            srgb_capable: config.srgb_capable(),
            stencil_size: config.stencil_size(),
            supports_transparency: config.supports_transparency(),
        }
    }
}

impl Display {
    fn choose_config<'a>(config: Box<dyn Iterator<Item = Config> + 'a>) -> Config {
        let mut config: VecDeque<Config> = config.collect();
        let x: Vec<GLConfigInfo> = config.iter().map(GLConfigInfo::new).collect();
        tracing::trace!("Available OpenGL configs: {:#?}", x);
        if let Some(index) = args().gl_config_index {
            let config = config
                .swap_remove_back(index)
                .expect("out of bounds config index");
            tracing::trace!("Selecting OpenGL config of index {}: {:?}", index, config);
            config
        } else {
            let (index, config) = config
                .into_iter()
                .enumerate()
                .max_by_key(|(_, config)| {
                    let mut score: i32 = 0;
                    if config.srgb_capable() {
                        if args().gl_disable_srgb {
                            return i32::MIN;
                        }
                        score += 20;
                    }

                    score += config.num_samples() as i32;
                    score += config.alpha_size() as i32;
                    match config.color_buffer_type() {
                        Some(ColorBufferType::Luminance(bit)) => {
                            score += bit as i32;
                        }

                        Some(ColorBufferType::Rgb {
                            r_size,
                            g_size,
                            b_size,
                        }) => {
                            score += ((r_size + g_size + b_size) / 3) as i32;
                        }

                        _ => {}
                    }

                    score
                })
                .ok_or_else(|| anyhow::format_err!("no OpenGL config found"))
                .unwrap();
            tracing::trace!(
                "Automatically selecting OpenGL config of index {}: {:?}",
                index,
                GLConfigInfo::new(&config)
            );
            config
        }
    }

    pub fn new_display<T>(
        event_loop: &EventLoopWindowTarget<T>,
        size: PhysicalSize<u32>,
        title: &str,
    ) -> anyhow::Result<(Display, Config)> {
        let span = tracing::trace_span!("Creating display window");
        let _enter = span.enter();
        let window_builder = WindowBuilder::new().with_inner_size(size).with_title(title);
        tracing::trace!("WindowBuilder structure: {:?}", window_builder);
        let (window, gl_config) = DisplayBuilder::new()
            .with_window_builder(Some(window_builder))
            .build(event_loop, ConfigTemplateBuilder::new(), |config| {
                Self::choose_config(config)
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

    pub fn get_scale_factor(&self) -> f64 {
        self.window.scale_factor()
    }
}
