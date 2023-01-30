use anyhow::Context;
use glam::{Mat3, Vec2};
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
        main_ctx::MainContext,
        server::GameServerSendChannel,
        task::{JoinToken, Joinable, TryJoinTaskResult},
    },
    graphics::{
        blur::BlurRenderer,
        quad_renderer::QuadRenderer,
        wrappers::{
            framebuffer::{DefaultTextureFramebuffer, Framebuffer},
            texture::{TextureHandle, TextureType},
        },
    },
    scene::draw::DrawRoot,
    utils::error::ResultExt,
};

use crate::scene::draw::content::bg::Background as BackgroundDraw;

pub enum LoadTextureResult {
    Pending(JoinToken<PhysicalSize<u32>>),
    Done(PhysicalSize<u32>),
}

pub struct Background {
    pub blur: BlurRenderer,
    pub renderer: QuadRenderer,
    pub texture: TextureHandle,
    pub load_texture_result: LoadTextureResult,
    pub screen_framebuffer: DefaultTextureFramebuffer,
}

impl Background {
    pub fn new(main_ctx: &mut MainContext) -> anyhow::Result<Self> {
        let renderer = QuadRenderer::new(main_ctx.dummy_vao.clone(), &mut main_ctx.channels.draw)
            .context("quad renderer initialization failed")?;
        let blur = BlurRenderer::new(main_ctx.dummy_vao.clone(), &mut main_ctx.channels.draw)
            .context("blur renderer initialization failed")?;
        let mut screen_framebuffer =
            DefaultTextureFramebuffer::new(&mut main_ctx.channels.draw, "screen framebuffer")
                .context("screen framebuffer initialization failed")?;
        screen_framebuffer.resize(&mut main_ctx.channels.draw, main_ctx.display.get_size())?;
        let (texture, join_load_texture) =
            Self::init_test_texture(main_ctx, blur.clone(), renderer.clone())
                .context("unable to initialize test texture")?;
        Ok(Self {
            texture,
            blur,
            renderer,
            load_texture_result: LoadTextureResult::Pending(join_load_texture),
            screen_framebuffer,
        })
    }

    #[allow(unused_mut)]
    fn init_test_texture(
        main_ctx: &mut MainContext,
        blur: BlurRenderer,
        renderer: QuadRenderer,
    ) -> anyhow::Result<(TextureHandle, JoinToken<PhysicalSize<u32>>)> {
        let test_texture = TextureHandle::new_args(
            &mut main_ctx.channels.draw,
            "test texture",
            TextureType::E2D,
        )?;

        let channel = main_ctx.channels.draw.clone_sender();
        let proxy = main_ctx.event_loop_proxy.clone();
        let (sender, join_token) = JoinToken::new();

        main_ctx.execute_blocking_task(enclose!((test_texture) move || {
            let result: anyhow::Result<PhysicalSize<u32>> = (|| {
                let img = image::io::Reader::open("BG.jpg")
                    .context("unable to load test texture")?
                    .decode()
                    .context("unable to decode test texture")?
                    .into_rgba8();
                let img_size = PhysicalSize::new(img.width(), img.height());

                channel.execute_draw_event(move |context, root| {
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

                    Self::get_bg_draw(root).init(renderer, blur.output_texture_handle());

                    [GameUserEvent::Execute(Box::new(|ctx, root| {
                        root.content.background.resize(ctx, ctx.display.get_size(), 1.0)
                    }))]
                })?;

                Ok(img_size)
            })();

            match result {
                Ok(result) => sender.send(result).log_warn(),
                Err(err) => proxy.send_event(GameUserEvent::Error(err)).log_warn(),
            };
        }));
        Ok((test_texture, join_token))
    }

    fn resize(
        &mut self,
        main_ctx: &mut MainContext,
        size: PhysicalSize<u32>,
        blur_factor: f32,
    ) -> anyhow::Result<()> {
        let texture_dimensions = match &self.load_texture_result {
            LoadTextureResult::Pending(join_load_texture) => match join_load_texture.try_join() {
                TryJoinTaskResult::JoinedResultTaken => {
                    tracing::warn!("Texture loading task failed, the error (if present) was reported to the event loop via a GameUserEvent::Error event");
                    None
                }
                TryJoinTaskResult::Joined(result) => Some(result),
                _ => None,
            },

            LoadTextureResult::Done(result) => Some(*result),
        };
        if let Some(texture_dimensions) = texture_dimensions {
            self.screen_framebuffer
                .resize(&mut main_ctx.channels.draw, size)?;
            let screen_framebuffer = self.screen_framebuffer.framebuffer.clone();
            let renderer = self.renderer.clone();
            let texture = self.texture.clone();
            main_ctx
                .channels
                .draw
                .execute_draw_event(move |context, _| {
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
                        &QuadRenderer::FULL_WINDOW_POS_BOUNDS,
                        &[[0.5 - hw, 0.5 + hh].into(), [0.5 + hw, 0.5 - hh].into()],
                        &Vec2::ZERO,
                        &Mat3::IDENTITY,
                    );
                    Framebuffer::unbind_static();
                    []
                })?;
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
        main_ctx: &mut MainContext,
        event: &GameEvent,
    ) -> anyhow::Result<bool> {
        match event {
            GameEvent::UserEvent(GameUserEvent::CheckedResize {
                display_size: PhysicalSize { width, height },
                ..
            }) => {
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
                    (*x as f32 / width as f32) * 2.0 - 1.0,
                    -((*y as f32 / height as f32) * 2.0 - 1.0),
                );
                fn interpolate(factor: f32) -> f32 {
                    let sign = factor.signum();
                    let abs = factor.abs();
                    let new_abs = 1.0 - (1.0 - abs).powf(3.0);
                    sign * new_abs
                }
                offset.x = interpolate(offset.x);
                offset.y = interpolate(offset.y);
                main_ctx.channels.draw.execute_draw_event(move |_, root| {
                    Self::get_bg_draw(root).set_offset(offset);
                    []
                })?;
            }

            _ => {}
        }
        Ok(false)
    }

    fn get_bg_draw(draw: &mut DrawRoot) -> &mut BackgroundDraw {
        &mut draw.content.background
    }
}
