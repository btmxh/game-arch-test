use std::mem::size_of;

use glam::{Mat3, Vec2};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, ColorTargetState, ColorWrites, CommandEncoder,
    Face, FragmentState, FrontFace, MultisampleState, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveState, PrimitiveTopology, PushConstantRange, RenderPipeline, RenderPipelineDescriptor,
    Sampler, SamplerBindingType, ShaderStages, TextureFormat, TextureSampleType, TextureView,
    TextureViewDimension, VertexState,
};

use crate::context::draw::{FrameContext, GraphicsContext};

pub struct QuadRenderer {
    pipeline: RenderPipeline,
    texture_bind_group_layout: BindGroupLayout,
}

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct QuadPushConstants {
    pos_bounds: [Vec2; 2],
    radius: Vec2,
    tex_bounds: [Vec2; 2],
    transform: Mat3,
    _padding: [f32; 1],
}

impl QuadRenderer {
    pub const FULL_WINDOW_POS_BOUNDS: [Vec2; 2] = [Vec2::new(-1.0, -1.0), Vec2::new(1.0, 1.0)];
    pub const FULL_TEXTURE_TEX_BOUNDS: [Vec2; 2] = [Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0)];
    pub const MAX_PUSH_CONSTANT_SIZE: usize = std::mem::size_of::<QuadPushConstants>();

    pub fn new(context: &GraphicsContext, target_format: TextureFormat) -> anyhow::Result<Self> {
        let shader = context
            .device
            .create_shader_module(wgpu::include_wgsl!("quad.wgsl"));
        let texture_bind_group_layout =
            context
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("quad renderer texture label"),
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            ty: wgpu::BindingType::Texture {
                                sample_type: TextureSampleType::Float { filterable: true },
                                view_dimension: TextureViewDimension::D2,
                                multisampled: false,
                            },
                            visibility: ShaderStages::FRAGMENT,
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            ty: BindingType::Sampler(SamplerBindingType::Filtering),
                            visibility: ShaderStages::FRAGMENT,
                            count: None,
                        },
                    ],
                });
        let pipeline_layout = context
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("quad renderer"),
                push_constant_ranges: &[PushConstantRange {
                    stages: ShaderStages::VERTEX_FRAGMENT,
                    range: 0..u32::try_from(size_of::<QuadPushConstants>())
                        .expect("sizeof(QuadPushConstants) should fit in an u32"),
                }],
                bind_group_layouts: &[&texture_bind_group_layout],
            });
        let pipeline = context
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("quad renderer pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: target_format,
                        blend: Some(BlendState::ALPHA_BLENDING),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleStrip,
                    strip_index_format: None,
                    front_face: FrontFace::Ccw,
                    polygon_mode: PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                    cull_mode: Some(Face::Back),
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

        Ok(Self {
            pipeline,
            texture_bind_group_layout,
        })
    }

    pub fn create_texture_bind_group(
        &self,
        context: &GraphicsContext,
        view: &TextureView,
        sampler: &Sampler,
    ) -> BindGroup {
        context.device.create_bind_group(&BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
            label: Some("texture bind group for quad renderer"),
        })
    }

    pub fn draw(
        &self,
        _context: &GraphicsContext,
        frame: &FrameContext,
        encoder: &mut CommandEncoder,
        bind_group: &BindGroup,
        pos_bounds: &[Vec2; 2],
        tex_bounds: &[Vec2; 2],
        radius: &Vec2,
        transform: &Mat3,
    ) {
        let mut render_pass = frame.begin_direct_render_pass(encoder, Some("quad renderer pass"));
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        let quad = QuadPushConstants {
            pos_bounds: *pos_bounds,
            tex_bounds: *tex_bounds,
            radius: *radius,
            transform: *transform,
            _padding: Default::default(),
        };
        render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, bytemuck::bytes_of(&quad));
        render_pass.draw(0..4, 0..1);
    }
}

#[test]
fn test_send_sync() {
    use crate::{assert_send, assert_sync};
    // `QuadRenderer` is basically two GLGfxHandles, therefore
    // it should be both Send and Sync.
    assert_send!(QuadRenderer);
    assert_sync!(QuadRenderer);
}
