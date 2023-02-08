use std::sync::Arc;

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
    scene::{main::RootScene, Scene},
    utils::{
        clock::{Clock, SteadyClock},
        error::ResultExt,
        mpsc::Sender,
        mutex::Mutex,
    },
};

pub enum LoadTextureResult {
    Pending(JoinToken<PhysicalSize<u32>>),
    Done(PhysicalSize<u32>),
}

pub struct Background {
    renderer: QuadRenderer,
    texture: TextureHandle,
    post_processed_texture: Mutex<Option<TextureHandle>>,
    offset: Mutex<Vec2>,
    clock: SteadyClock,
    blur: Mutex<BlurRenderer>,
    load_texture_result: Mutex<LoadTextureResult>,
    screen_framebuffer: Mutex<DefaultTextureFramebuffer>,
}

impl Scene for Background {
    fn handle_event<'a>(
        self: Arc<Self>,
        ctx: &mut MainContext,
        _: &RootScene,
        event: GameEvent<'a>,
    ) -> Option<GameEvent<'a>> {
        match &event {
            GameEvent::UserEvent(GameUserEvent::CheckedResize {
                display_size: PhysicalSize { width, height },
                ..
            }) => {
                self.resize(
                    ctx,
                    PhysicalSize {
                        width: width.get(),
                        height: height.get(),
                    },
                    1.0,
                )
                .context("unable to handle resize event")
                .log_error();
            }

            GameEvent::WindowEvent {
                window_id,
                event: WindowEvent::CursorMoved { position, .. },
            } if *window_id == ctx.display.get_window_id() => self.cursor_moved(ctx, position),

            _ => {}
        }

        Some(event)
    }

    fn draw(self: Arc<Self>, ctx: &mut crate::graphics::context::DrawContext) {
        if let Some(texture) = &*self.post_processed_texture.lock() {
            const OFFSET_FACTOR_VECTOR: Vec2 = Vec2::new(0.995, 0.998);
            const BOUNDS_NEG_1: [Vec2; 2] = [Vec2::new(0.0, 0.0), OFFSET_FACTOR_VECTOR];
            const BOUNDS_POS_1: [Vec2; 2] = [
                Vec2::new(1.0 - OFFSET_FACTOR_VECTOR.x, 1.0 - OFFSET_FACTOR_VECTOR.y),
                Vec2::new(1.0, 1.0),
            ];
            const HALF: Vec2 = Vec2::new(0.5, 0.5);
            let normalized_offset = self.offset.lock().mul_add(HALF, HALF);
            let bounds = [
                lerp_vec2(normalized_offset, BOUNDS_NEG_1[0], BOUNDS_POS_1[0]),
                lerp_vec2(normalized_offset, BOUNDS_NEG_1[1], BOUNDS_POS_1[1]),
            ];
            let angle = self.clock.now() as f32 * 0.01;
            let transform = Mat3::from_angle(angle);
            let radius = Vec2::new(1.0, 1.0);
            self.renderer.draw(
                ctx,
                *texture.get(ctx),
                &QuadRenderer::FULL_WINDOW_POS_BOUNDS,
                &bounds,
                &radius,
                &transform,
            );
        }
    }
}

fn lerp_vec2(amt: Vec2, min: Vec2, max: Vec2) -> Vec2 {
    Vec2::new(
        min.x + (max.x - min.x) * amt.x,
        min.y + (max.y - min.y) * amt.y,
    )
}

impl Background {
    pub fn new(main_ctx: &mut MainContext) -> anyhow::Result<Arc<Self>> {
        let renderer = QuadRenderer::new(main_ctx.dummy_vao.clone(), &mut main_ctx.channels.draw)
            .context("quad renderer initialization failed")?;
        let blur = Mutex::new(
            BlurRenderer::new(main_ctx.dummy_vao.clone(), &mut main_ctx.channels.draw)
                .context("blur renderer initialization failed")?,
        );
        let mut screen_framebuffer =
            DefaultTextureFramebuffer::new(&mut main_ctx.channels.draw, "screen framebuffer")
                .context("screen framebuffer initialization failed")?;
        screen_framebuffer.resize(&mut main_ctx.channels.draw, main_ctx.display.get_size())?;
        let texture = TextureHandle::new_args(
            &mut main_ctx.channels.draw,
            "test texture",
            TextureType::E2D,
        )
        .context("unable to initialize test texture")?;
        let (sender, join_token) = JoinToken::new();

        let slf = Arc::new(Self {
            texture: texture.clone(),
            post_processed_texture: Mutex::new(None),
            blur,
            renderer,
            load_texture_result: Mutex::new(LoadTextureResult::Pending(join_token)),
            screen_framebuffer: Mutex::new(screen_framebuffer),
            offset: Mutex::new(Vec2::ZERO),
            clock: SteadyClock::new(),
        });

        slf.init_test_texture(main_ctx, texture, sender)
            .context("unable to initialize test texture")?;

        Ok(slf)
    }

    #[allow(unused_mut)]
    fn init_test_texture(
        self: &Arc<Self>,
        main_ctx: &mut MainContext,
        test_texture: TextureHandle,
        sender: Sender<PhysicalSize<u32>>,
    ) -> anyhow::Result<()> {
        let channel = main_ctx.channels.draw.clone_sender();
        let proxy = main_ctx.event_loop_proxy.clone();

        let slf = self.clone();
        main_ctx.execute_blocking_task(enclose!((test_texture) move || {
            let result: anyhow::Result<PhysicalSize<u32>> = (|| {
                let img = image::io::Reader::open("BG.jpg")
                    .context("unable to load test texture")?
                    .decode()
                    .context("unable to decode test texture")?
                    .into_rgba8();
                let img_size = PhysicalSize::new(img.width(), img.height());

                channel.execute_draw_event(move |context, _| {
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

                    *slf.post_processed_texture.lock() = Some(slf.blur.lock().output_texture_handle());

                    [GameUserEvent::Execute(Box::new(move |ctx, _| {
                        slf.resize(ctx, ctx.display.get_size(), 1.0)
                    }))]
                })?;

                Ok(img_size)
            })();

            match result {
                Ok(result) => sender.send(result).log_warn(),
                Err(err) => proxy.send_event(GameUserEvent::Error(err)).log_warn(),
            };
        }));

        Ok(())
    }

    fn poll_texture_dimensions(result: &Mutex<LoadTextureResult>) -> Option<PhysicalSize<u32>> {
        let mut lock = result.lock();
        match &*lock {
            LoadTextureResult::Done(texture_dimensions) => Some(*texture_dimensions),
            LoadTextureResult::Pending(join_handle) => match join_handle.try_join() {
                TryJoinTaskResult::JoinedResultTaken => {
                    tracing::warn!("Texture loading task failed, the error (if present) was reported to the event loop via a GameUserEvent::Error event");
                    None
                }
                TryJoinTaskResult::Joined(result) => {
                    *lock = LoadTextureResult::Done(result);
                    Some(result)
                }
                _ => None,
            },
        }
    }

    fn resize(
        &self,
        main_ctx: &mut MainContext,
        size: PhysicalSize<u32>,
        blur_factor: f32,
    ) -> anyhow::Result<()> {
        if let Some(texture_dimensions) = Self::poll_texture_dimensions(&self.load_texture_result) {
            let (screen_framebuffer, screen_fb_texture) = {
                let mut lock = self.screen_framebuffer.lock();
                lock.resize(&mut main_ctx.channels.draw, size)
                    .context("unable to resize screen framebuffer")?;
                (lock.framebuffer.clone(), lock.texture.clone())
            };
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
            self.blur.lock().redraw(
                &mut main_ctx.channels.draw,
                size,
                screen_fb_texture,
                0.0,
                blur_factor,
            )?;
        }
        Ok(())
    }

    fn cursor_moved(&self, ctx: &mut MainContext, pos: &PhysicalPosition<f64>) {
        let PhysicalPosition { x, y } = pos;
        let PhysicalSize { width, height } = ctx.display.get_size();
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
        *self.offset.lock() = offset;
    }
}
