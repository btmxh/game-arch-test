use std::{num::NonZeroU32, time::Duration};

use anyhow::Context;
use glutin::{prelude::GlConfig, surface::SwapInterval};
use image::EncodableLayout;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoopProxy,
};

use crate::{
    display::Display,
    events::{GameEvent, GameUserEvent},
    graphics::{
        blur::BlurRenderer,
        quad_renderer::QuadRenderer,
        wrappers::{texture::TextureHandle, vertex_array::VertexArrayHandle},
    },
    utils::{clock::debug_get_time, enclose::enclose, error::ResultExt},
};

use super::{
    dispatch::{DispatchId, DispatchList, ReturnMechanism},
    executor::GameServerExecutor,
    server::ServerChannels,
};

pub struct MainContext {
    pub test_texture: (TextureHandle, PhysicalSize<u32>),
    pub blur: BlurRenderer,
    pub renderer: QuadRenderer,
    pub dummy_vao: VertexArrayHandle,
    pub frequency_profiling: bool,
    pub vsync: bool,
    pub channels: ServerChannels,
    pub dispatch_list: DispatchList,
    pub event_loop_proxy: EventLoopProxy<GameUserEvent>,
    pub display: Display,
}

impl MainContext {
    #[allow(unused_mut)]
    pub fn new(
        executor: &mut GameServerExecutor,
        display: Display,
        event_loop_proxy: EventLoopProxy<GameUserEvent>,
        dispatch_list: DispatchList,
        mut channels: ServerChannels,
    ) -> anyhow::Result<Self> {
        let dummy_vao = {
            let vertex_array = VertexArrayHandle::new(&mut channels.draw);
            executor.execute_draw(
                &mut channels.draw,
                Some(ReturnMechanism::Sync),
                enclose!((vertex_array) move |s| {
                    s.handles
                        .create_vertex_array("dummy vertex array", vertex_array)?;
                    Ok(Box::new(()))
                }),
            )?;
            vertex_array
        };
        let renderer = QuadRenderer::new(executor, dummy_vao.clone(), &mut channels.draw)
            .context("quad renderer initialization failed")?;
        let blur = BlurRenderer::new(executor, dummy_vao.clone(), &mut channels.draw)
            .context("blur renderer initialization failed")?;
        let window_size = display.get_size();

        let test_texture = TextureHandle::new(&mut channels.draw);
        let img = image::io::Reader::open("BG.jpg")
            .context("unable to load test texture")?
            .decode()
            .context("unable to decode test texture")?
            .into_rgba8();
        let width = img.width();
        let height = img.height();

        executor
            .execute_draw(
                &mut channels.draw,
                Some(ReturnMechanism::Sync),
                enclose!((test_texture) move |server| {
                    let tex_handle = server
                        .handles
                        .create_texture("test texture", test_texture)?;
                    unsafe {
                        gl::BindTexture(gl::TEXTURE_2D, tex_handle);
                        gl::TexImage2D(
                            gl::TEXTURE_2D,
                            0,
                            if server.gl_config.srgb_capable() {
                                gl::SRGB8_ALPHA8.try_into().unwrap()
                            } else {
                                gl::RGBA8.try_into().unwrap()
                            },
                            img.width().try_into().unwrap(),
                            img.height().try_into().unwrap(),
                            0,
                            gl::RGBA,
                            gl::UNSIGNED_BYTE,
                            img.as_bytes().as_ptr() as *const _,
                        );
                        gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_MIN_FILTER,
                            gl::LINEAR_MIPMAP_LINEAR.try_into().unwrap(),
                        );
                        gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_MAG_FILTER,
                            gl::LINEAR.try_into().unwrap(),
                        );
                        gl::GenerateMipmap(gl::TEXTURE_2D);
                    };
                    Ok(Box::new(()))
                }),
            )
            .context("unable to initialize test texture (in draw server)")?;

        let node_handle = channels.draw.generate_id();
        executor.execute_draw(
            &mut channels.draw,
            Some(ReturnMechanism::Sync),
            enclose!((test_texture, blur, renderer) move |server| {
                blur.redraw(
                    server,
                    window_size,
                    *test_texture.get(server).unwrap(),
                    1.0,
                    3.0,
                )?;

                server.draw_tree.create_root(node_handle, enclose!((blur, renderer) move |server| {
                    let viewport_size = server.display_size;
                    let vw = viewport_size.width.get() as f32;
                    let vh = viewport_size.height.get() as f32;
                    let tw = width as f32;
                    let th = height as f32;
                    let var = vw / vh;
                    let tar = tw / th;
                    let (hw, hh) = if var < tar {
                        (0.5 * var / tar, 0.5)
                    } else {
                        (0.5, 0.5 * tar / var)
                    };
                    renderer.draw(
                        server,
                        *blur.output_texture_handle().get(server).unwrap(),
                        &[[0.5 - hw, 0.5 + hh].into(), [0.5 + hw, 0.5 - hh].into()],
                    );

                    Ok(())
                }));

                Ok(Box::new(()))
            }),
        )?;

        let slf = Self {
            renderer,
            blur,
            dummy_vao,
            test_texture: (test_texture, PhysicalSize { width, height }),
            display,
            event_loop_proxy,
            dispatch_list,
            channels,
            vsync: true,
            frequency_profiling: false,
        };
        Ok(slf)
    }

    fn update_blur_texture(
        &mut self,
        executor: &mut GameServerExecutor,
        window_size: PhysicalSize<u32>,
        blur_factor: f32,
    ) -> anyhow::Result<()> {
        let test_texture = self.test_texture.0.clone();
        let mut blur = self.blur.clone();
        // let renderer = self.renderer.clone();
        executor.execute_draw(
            &mut self.channels.draw,
            Some(ReturnMechanism::Sync),
            move |server| {
                blur.redraw(
                    server,
                    window_size,
                    *test_texture.get(server).unwrap(),
                    0.0,
                    blur_factor,
                )?;

                unsafe {
                    gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
                    gl::Viewport(
                        0,
                        0,
                        window_size.width.try_into().unwrap(),
                        window_size.height.try_into().unwrap(),
                    );
                };

                Ok(Box::new(()))
            },
        )?;
        Ok(())
    }

    #[allow(clippy::blocks_in_if_conditions)]
    pub async fn handle_event(
        &mut self,
        executor: &mut GameServerExecutor,
        event: GameEvent<'_>,
    ) -> anyhow::Result<()> {
        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
            } if self.display.get_window_id() == window_id => {
                self.event_loop_proxy.send_event(GameUserEvent::Exit)?;
            }

            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(size),
            } if self.display.get_window_id() == window_id => {
                let width = NonZeroU32::new(size.width);
                let height = NonZeroU32::new(size.height);
                if let Some(width) = width {
                    if let Some(height) = height {
                        self.channels.draw.resize(PhysicalSize { width, height })?;
                        self.update_blur_texture(executor, size, 32.0)?;
                    }
                }
            }

            Event::WindowEvent {
                window_id,
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Released,
                                virtual_keycode: Some(VirtualKeyCode::Q),
                                ..
                            },
                        ..
                    },
            } if self.display.get_window_id() == window_id => {
                self.frequency_profiling = !self.frequency_profiling;
                self.channels
                    .update
                    .set_frequency_profiling(self.frequency_profiling)?;
                self.channels
                    .draw
                    .set_frequency_profiling(self.frequency_profiling)?;
                self.channels
                    .audio
                    .set_frequency_profiling(self.frequency_profiling)?;
            }

            Event::WindowEvent {
                window_id,
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Released,
                                virtual_keycode: Some(VirtualKeyCode::R),
                                ..
                            },
                        ..
                    },
            } if self.display.get_window_id() == window_id => {
                self.vsync = !self.vsync;
                let interval = if self.vsync {
                    SwapInterval::Wait(NonZeroU32::new(1).unwrap())
                } else {
                    SwapInterval::DontWait
                };
                if executor
                    .execute_draw(
                        &mut self.channels.draw,
                        Some(ReturnMechanism::Sync),
                        move |s| {
                            s.set_swap_interval(interval)?;
                            Ok(Box::new(()))
                        },
                    )
                    .with_context(|| format!("unable to set vsync swap interval to {:?}", interval))
                    .log_warn()
                    .is_some()
                {
                    tracing::info!(
                        "VSync swap interval set to {} ({:?})",
                        interval != SwapInterval::DontWait,
                        interval
                    );
                }
            }

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
            } if self.display.get_window_id() == window_id => {
                let time = debug_get_time();
                let test_duration = 5.0;
                tracing::info!("{}", time);
                self.set_timeout(Duration::from_secs_f64(test_duration), move |_, _| {
                    tracing::info!("delay: {}s", debug_get_time() - time - test_duration);
                })?;
            }

            Event::UserEvent(GameUserEvent::Dispatch(msg)) => {
                self.dispatch_list
                    .handle_dispatch_msg(msg)
                    .into_iter()
                    .for_each(|(id, d)| {
                        d(self, id);
                    });
            }

            _ => {}
        };
        Ok(())
    }

    pub fn set_timeout<F>(&mut self, timeout: Duration, callback: F) -> anyhow::Result<DispatchId>
    where
        F: FnOnce(&mut MainContext, DispatchId) + 'static,
    {
        let id = self.dispatch_list.push(callback);
        self.channels.update.set_timeout(timeout, id)?;
        Ok(id)
    }
}
