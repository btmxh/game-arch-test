use display::Display;
use executor::server::draw;
use glutin::{display::GetGlDisplay, prelude::GlDisplay, surface::SurfaceAttributes};
use winit::{dpi::PhysicalSize, event_loop::EventLoop};

pub mod display;
pub mod executor;
pub mod utils;

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new();
    let (mut display, gl_config) =
        Display::new_display(&event_loop, PhysicalSize::new(1280, 720), "hello")?;
    let (draw_server, draw_channels) = draw::SendServer::new(gl_config, &display)?;
    event_loop.run(move |_event, _target, _control_flow| {
        display.kys(); 
    });
}
