use anyhow::Context;
use glutin::prelude::GlConfig;
use image::EncodableLayout;
use winit::dpi::PhysicalSize;

use crate::{
    enclose,
    events::{GameEvent, GameUserEvent},
    exec::{
        executor::GameServerExecutor,
        main_ctx::MainContext,
        server::{GameServerSendChannel, ServerChannels},
    },
    graphics::{
        blur::BlurRenderer,
        quad_renderer::QuadRenderer,
        wrappers::texture::{TextureHandle, TextureType},
    },
};

pub struct Background {
    pub test_texture: TextureHandle,
    pub blur: BlurRenderer,
    pub renderer: QuadRenderer,
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
        let test_texture = Self::init_test_texture(
            executor,
            &mut main_ctx.channels,
            blur.clone(),
            renderer.clone(),
        )
        .context("unable to initialize test texture")?;
        Ok(Self {
            test_texture,
            blur,
            renderer,
        })
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

            GameServerExecutor::execute_draw_event(&channel, move |server, _| {
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

                // server.set_draw_callback(move |server| {
                //     if let Some(texture) = blur.output_texture_handle().try_get(server) {
                //         let viewport_size = server.display_size;
                //         let vw = viewport_size.width.get() as f32;
                //         let vh = viewport_size.height.get() as f32;
                //         let tw = width as f32;
                //         let th = height as f32;
                //         let var = vw / vh;
                //         let tar = tw / th;
                //         let (hw, hh) = if var < tar {
                //             (0.5 * var / tar, 0.5)
                //         } else {
                //             (0.5, 0.5 * tar / var)
                //         };
                //         renderer.draw(
                //             server,
                //             *texture,
                //             &[[0.5 - hw, 0.5 + hh].into(), [0.5 + hw, 0.5 - hh].into()],
                //         );
                //     }

                //     Ok(())
                // });

                [GameUserEvent::Execute(Box::new(|ctx, _, root| {
                    root.background.update_blur_texture(ctx, None, 32.0)
                }))]
            })?;
            Ok(())
        }));
        Ok(test_texture)
    }

    fn update_blur_texture(
        &mut self,
        main_ctx: &mut MainContext,
        size: Option<PhysicalSize<u32>>,
        blur_factor: f32,
    ) -> anyhow::Result<()> {
        self.blur.redraw(
            &mut main_ctx.channels.draw,
            size.unwrap_or_else(|| main_ctx.display.get_size()),
            self.test_texture.clone(),
            0.0,
            blur_factor,
        )?;
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
                self.update_blur_texture(
                    main_ctx,
                    Some(PhysicalSize {
                        width: width.get(),
                        height: height.get(),
                    }),
                    32.0,
                )?;
            }

            _ => {}
        }
        Ok(false)
    }
}
