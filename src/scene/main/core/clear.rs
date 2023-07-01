use wgpu::{
    Color, CommandEncoderDescriptor, Operations, RenderPassColorAttachment, RenderPassDescriptor,
};

use crate::context::draw::DrawContext;

pub struct Scene;

impl Scene {
    pub fn draw(&self, context: &mut DrawContext) {
        let mut encoder =
            context
                .graphics
                .device
                .create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("clear command encoder"),
                });
        encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(RenderPassColorAttachment {
                ops: Operations {
                    load: wgpu::LoadOp::Clear(Color::BLUE),
                    store: true,
                },
                view: &context.frame.surface_texture_view,
                resolve_target: None,
            })],
            label: Some("clear render pass"),
            depth_stencil_attachment: None,
        });

        context
            .graphics
            .queue
            .submit(std::iter::once(encoder.finish()));
    }
}
