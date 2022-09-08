//! Based on: https://community.arm.com/cfs-file/__key/communityserver-blogs-components-weblogfiles/00-00-00-20-66/siggraph2015_2D00_mmg_2D00_marius_2D00_notes.pdf
//!
//! See additional details in the [NUM_SIZES] docs

use super::super::{BloomConfig, Consts};
use bytemuck::{Pod, Zeroable};
use vek::*;

// TODO: auto-tune the number of passes to maintain roughly constant blur per
// unit of FOV so changing resolution / FOV doesn't change the blur appearance
// significantly.
//
/// Blurring is performed while downsampling to the smaller sizes in steps and
/// then upsampling back up to the original resolution. Each level is half the
/// size in both dimensions from the previous. For instance with 5 distinct
/// sizes there is a total of 8 passes going from the largest to the smallest to
/// the largest again:
///
/// 1 -> 1/2 -> 1/4 -> 1/8 -> 1/16 -> 1/8 -> 1/4 -> 1/2 -> 1
///                           ~~~~
///     [downsampling]      smallest      [upsampling]
///
/// The textures used for downsampling are re-used when upsampling.
///
/// Additionally, instead of clearing them the colors are added together in an
/// attempt to obtain a more concentrated bloom near bright areas rather than
/// a uniform blur. In the example above, the added layers would include 1/8,
/// 1/4, and 1/2. The smallest size is not upsampled to and the original full
/// resolution has no blurring and we are already combining the bloom into the
/// full resolution image in a later step, so they are not included here. The 3
/// extra layers added in mean the total luminosity of the final blurred bloom
/// image will be 4 times more than the input image. To account for this, we
/// divide the bloom intensity by 4 before applying it.
///
/// Nevertheless, we have not fully evaluated how this visually compares to the
/// bloom obtained without adding with the previous layers so there is the
/// potential for further artistic investigation here.
///
/// NOTE: This constant includes the full resolution size and it is
/// assumed that there will be at least one smaller image to downsample to and
/// upsample back from (otherwise no blurring would be done). Thus, the minimum
/// valid value is 2 and panicking indexing operations we perform assume this
/// will be at least 2.
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
        bloom_config: &BloomConfig,
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
            (!bloom_config.uniform_blur).then_some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                // We don't really use this but we need something here..
                alpha: wgpu::BlendComponent::REPLACE,
            }),
        );

        Self {
            downsample_filtered: downsample_filtered_pipeline,
            downsample: downsample_pipeline,
            upsample: upsample_pipeline,
        }
    }
}
