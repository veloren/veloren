use super::super::{AaMode, Consts, GlobalsLayouts};
use bytemuck::{Pod, Zeroable};
use vek::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Locals {
    proj_mat_inv: [[f32; 4]; 4],
    view_mat_inv: [[f32; 4]; 4],
}

impl Default for Locals {
    fn default() -> Self { Self::new(Mat4::identity(), Mat4::identity()) }
}

impl Locals {
    pub fn new(proj_mat_inv: Mat4<f32>, view_mat_inv: Mat4<f32>) -> Self {
        Self {
            proj_mat_inv: proj_mat_inv.into_col_arrays(),
            view_mat_inv: view_mat_inv.into_col_arrays(),
        }
    }
}

pub struct BindGroup {
    pub(in super::super) bind_group: wgpu::BindGroup,
}

pub struct PostProcessLayout {
    pub layout: wgpu::BindGroupLayout,
}

impl PostProcessLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            layout: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // src color
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            filtering: true,
                            comparison: false,
                        },
                        count: None,
                    },
                    // Locals
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            }),
        }
    }

    pub fn bind(
        &self,
        device: &wgpu::Device,
        src_color: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        locals: &Consts<Locals>,
    ) -> BindGroup {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src_color),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: locals.buf().as_entire_binding(),
                },
            ],
        });

        BindGroup { bind_group }
    }
}

pub struct PostProcessPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl PostProcessPipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
        sc_desc: &wgpu::SwapChainDescriptor,
        global_layout: &GlobalsLayouts,
        layout: &PostProcessLayout,
    ) -> Self {
        common_base::span!(_guard, "PostProcessPipeline::new");
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Post process pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[&global_layout.globals, &layout.layout],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Post process pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: vs_module,
                entry_point: "main",
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                polygon_mode: wgpu::PolygonMode::Fill,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: sc_desc.format,
                    color_blend: wgpu::BlendState::REPLACE,
                    alpha_blend: wgpu::BlendState::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
        });

        Self {
            pipeline: render_pipeline,
        }
    }
}
