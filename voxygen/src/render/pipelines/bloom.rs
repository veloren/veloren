//! Based on: https://community.arm.com/cfs-file/__key/communityserver-blogs-components-weblogfiles/00-00-00-20-66/siggraph2015_2D00_mmg_2D00_marius_2D00_notes.pdf

use super::super::Consts;
use bytemuck::{Pod, Zeroable};
use vek::*;

// TODO: auto-tune the number of passes to maintain roughly constant blur per
// unit of FOV so changing resolution / FOV doesn't change the blur appearance
// significantly
/// Each level is a multiple of 2 smaller in both dimensions.
/// For a total of 8 passes from the largest to the smallest to the largest
/// again.
pub const NUM_SIZES: usize = 5;

pub struct BindGroup {
    pub(in super::super) bind_group: wgpu::BindGroup,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Locals {
    halfpixel: [f32; 2],
}

impl Locals {
    pub fn new(source_texture_resolution: Vec2<f32>) -> Self {
        Self {
            halfpixel: source_texture_resolution.map(|e| 0.5 / e).into_array(),
        }
    }
}

pub struct BloomLayout {
    pub layout: wgpu::BindGroupLayout,
}

impl BloomLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            layout: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // Color source
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            filtering: true,
                            comparison: false,
                        },
                        count: None,
                    },
                    // halfpixel
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::FRAGMENT,
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
        half_pixel: Consts<Locals>,
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
                    resource: half_pixel.buf().as_entire_binding(),
                },
            ],
        });

        BindGroup { bind_group }
    }
}

pub struct BloomPipelines {
    pub downsample_filtered: wgpu::RenderPipeline,
    pub downsample: wgpu::RenderPipeline,
    pub upsample: wgpu::RenderPipeline,
}

impl BloomPipelines {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        downsample_filtered_fs_module: &wgpu::ShaderModule,
        downsample_fs_module: &wgpu::ShaderModule,
        upsample_fs_module: &wgpu::ShaderModule,
        target_format: wgpu::TextureFormat,
        layout: &BloomLayout,
    ) -> Self {
        common_base::span!(_guard, "BloomPipelines::new");
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Bloom pipelines layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[&layout.layout],
            });

        let create_pipeline = |label, fs_module, blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
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
                    cull_mode: None,
                    clamp_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
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
                        format: target_format,
                        blend,
                        write_mask: wgpu::ColorWrite::ALL,
                    }],
                }),
            })
        };

        let downsample_filtered_pipeline = create_pipeline(
            "Bloom downsample filtered pipeline",
            downsample_filtered_fs_module,
            None,
        );
        let downsample_pipeline =
            create_pipeline("Bloom downsample pipeline", downsample_fs_module, None);
        let upsample_pipeline = create_pipeline(
            "Bloom upsample pipeline",
            upsample_fs_module,
            Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                // we don't really use this, but... we need something here
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
        );

        Self {
            downsample_filtered: downsample_filtered_pipeline,
            downsample: downsample_pipeline,
            upsample: upsample_pipeline,
        }
    }
}
