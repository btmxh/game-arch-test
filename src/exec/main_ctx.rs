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
        wrappers::{
            framebuffer::DefaultTextureFramebuffer,
            texture::{TextureHandle, TextureType},
            vertex_array::VertexArrayHandle,
        },
    },
    utils::{clock::debug_get_time, enclose::enclose, error::ResultExt},
};

use super::{
    dispatch::{DispatchId, DispatchList},
    executor::GameServerExecutor,
    server::{GameServerSendChannel, ServerChannels},
    task::CancellationToken,
};

pub struct MainContext {
    pub test_texture: TextureHandle,
    pub blur: BlurRenderer,
    pub renderer: QuadRenderer,
    pub screen_framebuffer: DefaultTextureFramebuffer,
    pub dummy_vao: VertexArrayHandle,
    pub frequency_profiling: bool,
    pub vsync: bool,
    pub channels: ServerChannels,
    pub dispatch_list: DispatchList,
    pub event_loop_proxy: EventLoopProxy<GameUserEvent>,
    pub display: Display,
    // resize throttling
    // port of https://blog.webdevsimplified.com/2022-03/debounce-vs-throttle/
    resize_should_wait: bool,
    resize_size: Option<PhysicalSize<NonZeroU32>>,
}

impl MainContext {
    pub fn new(
        executor: &mut GameServerExecutor,
        display: Display,
        event_loop_proxy: EventLoopProxy<GameUserEvent>,
        mut channels: ServerChannels,
    ) -> anyhow::Result<Self> {
        let dummy_vao = VertexArrayHandle::new(&mut channels.draw, "dummy vertex array")?;
        let renderer = QuadRenderer::new(dummy_vao.clone(), &mut channels.draw)
            .context("quad renderer initialization failed")?;
        let blur = BlurRenderer::new(dummy_vao.clone(), &mut channels.draw)
            .context("blur renderer initialization failed")?;

        let mut screen_framebuffer =
            DefaultTextureFramebuffer::new(&mut channels.draw, "screen framebuffer")
                .context("screen framebuffer initialization failed")?;
        screen_framebuffer.resize(&mut channels.draw, display.get_size())?;

        let test_texture =
            Self::init_test_texture(executor, &mut channels, blur.clone(), renderer.clone())?;

        Ok(Self {
            renderer,
            blur,
            dummy_vao,
            test_texture,
            display,
            event_loop_proxy,
            dispatch_list: DispatchList::new(),
            channels,
            vsync: true,
            frequency_profiling: false,
            screen_framebuffer,
            resize_should_wait: false,
            resize_size: None,
        })
    }

    fn resize(
        &mut self,
        executor: &mut GameServerExecutor,
        size: PhysicalSize<NonZeroU32>,
    ) -> anyhow::Result<()> {
        executor.execute_draw_sync(&mut self.channels.draw, move |server| {
            server.resize(size);
            Ok(())
        })?;
        self.update_blur_texture(32.0)?;
        Ok(())
    }

    #[allow(unused_mut)]
    fn init_test_texture(
        executor: &mut GameServerExecutor,
        channels: &mut ServerChannels,
        blur: BlurRenderer,
        renderer: QuadRenderer,
    ) -> anyhow::Result<TextureHandle> {
        let test_texture =
            TextureHandle::new_args(&mut channels.draw, "test texture", TextureType::E2D)?;

        let channel = channels.draw.clone_sender();
        let node_handle = channels.draw.generate_id();
        executor.execute_blocking_task(enclose!((test_texture) move |token| {
            let img = image::io::Reader::open("BG.jpg")
                .context("unable to load test texture")?
                .decode()
                .context("unable to decode test texture")?
                .into_rgba8();
            if token.is_cancelled() {
                return Ok(())
            }
            let width = img.width();
            let height = img.height();

            GameServerExecutor::execute_draw_event(&channel, move |server| {
                let tex_handle = test_texture.get(server);
                tex_handle.bind();
                unsafe {
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

                server.draw_tree.create_root(node_handle, move |server| {
                    if let Some(texture) = blur.output_texture_handle().try_get(server) {
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
                            *texture,
                            &[[0.5 - hw, 0.5 + hh].into(), [0.5 + hw, 0.5 - hh].into()],
                        );
                    }

                    Ok(())
                });

                [GameUserEvent::Execute(Box::new(|ctx| {
                    ctx.update_blur_texture(32.0)
                }))]
            })?;
            Ok(())
        }));
        Ok(test_texture)
    }

    fn update_blur_texture(&mut self, blur_factor: f32) -> anyhow::Result<()> {
        self.blur.redraw(
            &mut self.channels.draw,
            self.display.get_size(),
            self.test_texture.clone(),
            0.0,
            blur_factor,
        )?;
        Ok(())
    }

    #[allow(clippy::blocks_in_if_conditions)]
    pub fn handle_event(
        &mut self,
        executor: &mut GameServerExecutor,
        event: GameEvent<'_>,
    ) -> anyhow::Result<()> {
        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
            } if self.display.get_window_id() == window_id => {
                self.event_loop_proxy
                    .send_event(GameUserEvent::Exit)
                    .map_err(|e| anyhow::format_err!("{}", e))
                    .context("unable to send event to event loop")?;
            }

            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(size),
            } if self.display.get_window_id() == window_id => {
                const THROTTLE_DURATION: Duration = Duration::from_millis(100);
                fn resize_timeout_func(
                    slf: &mut MainContext,
                    executor: &mut GameServerExecutor,
                ) -> anyhow::Result<()> {
                    if let Some(size) = slf.resize_size.take() {
                        slf.resize(executor, size)?;
                        slf.resize_size = None;
                        slf.set_timeout(THROTTLE_DURATION, |slf, executor, _| {
                            resize_timeout_func(slf, executor)
                        })?;
                    } else {
                        slf.resize_should_wait = false;
                    }

                    Ok(())
                }

                let width = NonZeroU32::new(size.width);
                let height = NonZeroU32::new(size.height);
                let size =
                    width.and_then(|width| height.map(|height| PhysicalSize::new(width, height)));
                if let Some(size) = size {
                    // throttle
                    if self.resize_should_wait {
                        self.resize_size = Some(size);
                    } else {
                        self.resize(executor, size)?;
                        self.resize_should_wait = true;
                        self.set_timeout(THROTTLE_DURATION, |slf, executor, _| {
                            resize_timeout_func(slf, executor)
                        })?;
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
                    .execute_draw_sync(&mut self.channels.draw, move |s| {
                        s.set_swap_interval(interval)?;
                        Ok(Box::new(()))
                    })
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
                self.set_timeout(Duration::from_secs_f64(test_duration), move |_, _, _| {
                    tracing::info!("delay: {}s", debug_get_time() - time - test_duration);
                    Ok(())
                })?;
            }

            Event::UserEvent(GameUserEvent::Dispatch(msg)) => {
                for (id, d) in self.dispatch_list.handle_dispatch_msg(msg).into_iter() {
                    d(self, executor, id)?;
                }
            }

            Event::UserEvent(GameUserEvent::Execute(callback)) => {
                callback(self).log_error();
            }

            Event::UserEvent(GameUserEvent::Error(e)) => {
                tracing::error!("GameUserEvent::Error caught: {}", e);
            }

            _ => {}
        };
        Ok(())
    }

    pub fn set_timeout<F>(
        &mut self,
        timeout: Duration,
        callback: F,
    ) -> anyhow::Result<(DispatchId, CancellationToken)>
    where
        F: FnOnce(&mut MainContext, &mut GameServerExecutor, DispatchId) -> anyhow::Result<()>
            + 'static,
    {
        let cancel_token = CancellationToken::new();
        let id = self.dispatch_list.push(callback, cancel_token.clone());
        self.channels.update.set_timeout(timeout, id)?;
        Ok((id, cancel_token))
    }
}
