use anyhow::{bail, Context};
use glutin::prelude::GlConfig;
use image::EncodableLayout;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::WindowEvent,
};

use crate::{
    enclose,
    events::{GameEvent, GameUserEvent},
    exec::{
        executor::GameServerExecutor,
        main_ctx::MainContext,
        server::{GameServerSendChannel, ServerChannels},
        task::{Cancellable, CancellationToken, JoinToken, Joinable, TryJoinTaskResult},
    },
    graphics::{
        blur::BlurRenderer,
        quad_renderer::QuadRenderer,
        wrappers::{
            framebuffer::{DefaultTextureFramebuffer, Framebuffer},
            texture::{TextureHandle, TextureType},
        },
        Vec2,
    },
    utils::error::ResultExt,
};

pub struct Background {
    pub blur: BlurRenderer,
    pub renderer: QuadRenderer,
    pub texture: TextureHandle,
    pub cancel_load_texture: CancellationToken,
    pub join_load_texture: Option<JoinToken<anyhow::Result<PhysicalSize<u32>>>>,
    pub texture_dimensions: Option<PhysicalSize<u32>>,
    pub screen_framebuffer: DefaultTextureFramebuffer,
}

impl Background {
    pub fn new(
        executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
    ) -> anyhow::Result<Self> {
        let renderer = QuadRenderer::new(main_ctx.dummy_vao.clone(), &mut main_ctx.channels.draw)
            .context("quad renderer initialization failed")?;
        let blur = BlurRenderer::new(main_ctx.dummy_vao.clone(), &mut main_ctx.channels.draw)
            .context("blur renderer initialization failed")?;
        let mut screen_framebuffer =
            DefaultTextureFramebuffer::new(&mut main_ctx.channels.draw, "screen framebuffer")
                .context("screen framebuffer initialization failed")?;
        screen_framebuffer.resize(&mut main_ctx.channels.draw, main_ctx.display.get_size())?;
        let (texture, cancel_load_texture, join_load_texture) = Self::init_test_texture(
            executor,
            &mut main_ctx.channels,
            blur.clone(),
            renderer.clone(),
        )
        .context("unable to initialize test texture")?;
        Ok(Self {
            texture,
            blur,
            renderer,
            cancel_load_texture,
            join_load_texture: Some(join_load_texture),
            texture_dimensions: None,
            screen_framebuffer,
        })
    }

    #[allow(unused_mut)]
    fn init_test_texture(
        executor: &mut GameServerExecutor,
        channels: &mut ServerChannels,
        blur: BlurRenderer,
        renderer: QuadRenderer,
    ) -> anyhow::Result<(
        TextureHandle,
        CancellationToken,
        JoinToken<anyhow::Result<PhysicalSize<u32>>>,
    )> {
        let test_texture =
            TextureHandle::new_args(&mut channels.draw, "test texture", TextureType::E2D)?;

        let channel = channels.draw.clone_sender();
        let cancel_token = CancellationToken::new();
        let (sender, join_token) = JoinToken::new();

        executor.execute_blocking_task(enclose!((test_texture, cancel_token) move || {
            let result: anyhow::Result<PhysicalSize<u32>> = (|| {
                let check_cancel = || {
                    if cancel_token.is_cancelled() {
                        bail!("cancelled")
                    }

                    Ok(())
                };
                check_cancel()?;
                let img = image::io::Reader::open("BG.jpg")
                    .context("unable to load test texture")?
                    .decode()
                    .context("unable to decode test texture")?
                    .into_rgba8();
                let img_size = PhysicalSize::new(img.width(), img.height());
                check_cancel()?;

                GameServerExecutor::execute_draw_event(&channel, move |context, root| {
                    let tex_handle = test_texture.get(context);
                    tex_handle.bind();
                    unsafe {
                        gl::TexImage2D(
                            gl::TEXTURE_2D,
                            0,
                            if context.gl_config.srgb_capable() {
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

                    root.initialize_background(blur, renderer).log_error();

                    [GameUserEvent::Execute(Box::new(|ctx, _, root| {
                        root.background.resize(ctx, ctx.display.get_size(), 1.0)
                    }))]
                })?;

                Ok(img_size)
            })();
            sender.send(result).log_warn();
        }));
        Ok((test_texture, cancel_token, join_token))
    }

    fn resize(
        &mut self,
        main_ctx: &mut MainContext,
        size: PhysicalSize<u32>,
        blur_factor: f32,
    ) -> anyhow::Result<()> {
        if let Some(join_load_texture) = self.join_load_texture.take() {
            match join_load_texture.try_join() {
                TryJoinTaskResult::JoinedResultTaken => {
                    panic!("task result taken (task probably panicked)")
                }
                TryJoinTaskResult::NotJoined => {
                    self.join_load_texture = Some(join_load_texture);
                }
                TryJoinTaskResult::Joined(result) => {
                    self.texture_dimensions = Some(result.context("unable to load texture")?)
                }
            }
        }
        if let Some(texture_dimensions) = self.texture_dimensions {
            self.screen_framebuffer
                .resize(&mut main_ctx.channels.draw, size)?;
            {
                let screen_framebuffer = self.screen_framebuffer.framebuffer.clone();
                let renderer = self.renderer.clone();
                let texture = self.texture.clone();
                GameServerExecutor::execute_draw_event(
                    &mut main_ctx.channels.draw,
                    move |context, _| {
                        screen_framebuffer.get(context).bind();
                        let viewport_size = context.display_size;
                        let vw = viewport_size.width.get() as f32;
                        let vh = viewport_size.height.get() as f32;
                        let tw = texture_dimensions.width as f32;
                        let th = texture_dimensions.height as f32;
                        let var = vw / vh;
                        let tar = tw / th;
                        let (hw, hh) = if var < tar {
                            (0.5 * var / tar, 0.5)
                        } else {
                            (0.5, 0.5 * tar / var)
                        };
                        renderer.draw(
                            context,
                            *texture.get(context),
                            &[[0.5 - hw, 0.5 + hh].into(), [0.5 + hw, 0.5 - hh].into()],
                        );
                        Framebuffer::unbind_static();
                        []
                    },
                )?;
            }
            self.blur.redraw(
                &mut main_ctx.channels.draw,
                size,
                self.screen_framebuffer.texture.clone(),
                0.0,
                blur_factor,
            )?;
        }
        Ok(())
    }

    pub fn handle_event(
        &mut self,
        _executor: &mut GameServerExecutor,
        main_ctx: &mut MainContext,
        event: &GameEvent,
    ) -> anyhow::Result<bool> {
        match event {
            GameEvent::UserEvent(GameUserEvent::CheckedResize(PhysicalSize { width, height })) => {
                self.resize(
                    main_ctx,
                    PhysicalSize {
                        width: width.get(),
                        height: height.get(),
                    },
                    1.0,
                )?;
            }

            GameEvent::WindowEvent {
                window_id,
                event:
                    WindowEvent::CursorMoved {
                        position: PhysicalPosition { x, y },
                        ..
                    },
            } if *window_id == main_ctx.display.get_window_id() => {
                let PhysicalSize { width, height } = main_ctx.display.get_size();
                let mut offset = Vec2::new(
                    ((*x as f32) / (width as f32)) * 2.0 - 1.0,
                    -(((*y as f32) / (height as f32)) * 2.0 - 1.0),
                );
                offset = offset.map(|factor| {
                    let sign = factor.signum();
                    let abs = factor.abs();
                    let new_abs = 1.0 - (1.0 - abs).powf(3.0);
                    sign * new_abs
                });
                tracing::info!("{:?}", offset);
                GameServerExecutor::execute_draw_event(
                    &mut main_ctx.channels.draw,
                    move |_, root| {
                        if let Some(background) = root.background.as_mut() {
                            background.set_offset(offset);
                        }
                        []
                    },
                )?;
            }

            _ => {}
        }
        Ok(false)
    }
}
