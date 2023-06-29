use std::sync::Arc;

use wgpu::{
    Color, CommandEncoderDescriptor, Operations, RenderPassColorAttachment, RenderPassDescriptor,
};

use crate::{
    graphics::context::{DrawContext, DrawingContext},
    scene::Scene,
};

pub struct Clear;

impl Scene for Clear {
    fn draw(self: Arc<Self>, ctx: &mut DrawContext, drawing: &DrawingContext) {
        let mut encoder = ctx
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
                view: &drawing.surface_texture_view,
                resolve_target: None,
            })],
            label: Some("clear render pass"),
            depth_stencil_attachment: None,
        });

        ctx.queue.submit(std::iter::once(encoder.finish()));
    }
}
