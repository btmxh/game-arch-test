use std::{mem::size_of, sync::Arc};

use glam::{Mat3, Vec2};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, ColorTargetState, ColorWrites, CommandEncoder,
    Face, FragmentState, FrontFace, MultisampleState, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveState, PrimitiveTopology, PushConstantRange, RenderPipeline, RenderPipelineDescriptor,
    Sampler, SamplerBindingType, ShaderStages, TextureSampleType, TextureView,
    TextureViewDimension, VertexState,
};

use crate::{enclose, exec::main_ctx::MainContext, utils::mutex::Mutex};

use super::context::{DrawContext, DrawingContext};

struct State {
    pipeline: RenderPipeline,
    texture_bind_group_layout: BindGroupLayout,
}

#[derive(Clone)]
pub struct QuadRenderer {
    state: Arc<Mutex<Option<State>>>,
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

    pub fn new(main_ctx: &mut MainContext) -> anyhow::Result<Self> {
        let state = Arc::new(Mutex::new(None));
        #[allow(unused_mut)]
        main_ctx.execute_draw_sync(enclose!(
            (state) move |draw, _| {
                let shader = draw
                    .device
                    .create_shader_module(wgpu::include_wgsl!("quad.wgsl"));
                let texture_bind_group_layout =
                    draw.device
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
                let pipeline_layout =
                    draw.device
                        .create_pipeline_layout(&PipelineLayoutDescriptor {
                            label: Some("quad renderer"),
                            push_constant_ranges: &[PushConstantRange {
                                stages: ShaderStages::VERTEX_FRAGMENT,
                                range: 0..u32::try_from(size_of::<QuadPushConstants>())
                                    .expect("sizeof(QuadPushConstants) should fit in an u32"),
                            }],
                            bind_group_layouts: &[&texture_bind_group_layout],
                        });
                let pipeline = draw
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
                                format: draw.surface_configuration.format,
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

                *state.lock() = Some(State {
                    pipeline,
                    texture_bind_group_layout,
                })
            }
        ))?;

        Ok(Self { state })
    }

    pub fn create_texture_bind_group(
        &self,
        context: &DrawContext,
        view: &TextureView,
        sampler: &Sampler,
    ) -> BindGroup {
        context.device.create_bind_group(&BindGroupDescriptor {
            layout: &self
                .state
                .lock()
                .as_ref()
                .expect("state should be available by now")
                .texture_bind_group_layout,
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
        _context: &DrawContext,
        drawing_context: &DrawingContext,
        encoder: &mut CommandEncoder,
        bind_group: &BindGroup,
        pos_bounds: &[Vec2; 2],
        tex_bounds: &[Vec2; 2],
        radius: &Vec2,
        transform: &Mat3,
    ) {
        if let Some(State { pipeline, .. }) = self.state.lock().as_ref() {
            let mut render_pass =
                drawing_context.begin_direct_render_pass(encoder, Some("quad renderer pass"));
            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            let quad = QuadPushConstants {
                pos_bounds: *pos_bounds,
                tex_bounds: *tex_bounds,
                radius: *radius,
                transform: *transform,
                _padding: Default::default(),
            };
            render_pass.set_push_constants(
                ShaderStages::VERTEX_FRAGMENT,
                0,
                bytemuck::bytes_of(&quad),
            );
            render_pass.draw(0..4, 0..1);
        }
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
