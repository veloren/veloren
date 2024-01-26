use super::super::{
    AaMode, Bound, Consts, FigureLayout, GlobalsLayouts, TerrainLayout, TerrainVertex,
};
use bytemuck::{Pod, Zeroable};
use vek::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Default)]
pub struct Locals {
    rain_occlusion_matrices: [[f32; 4]; 4],
    rain_occlusion_texture_mat: [[f32; 4]; 4],
    /// A rotation of the direction of the rain, relative to the players
    /// velocity.
    rain_dir_mat: [[f32; 4]; 4],
    /// A value to offset the rain, to make it move over time.
    integrated_rain_vel: f32,
    rain_density: f32,
    // To keep 16-byte-aligned.
    occlusion_dummy: [f32; 2],
}
/// Make sure Locals is 16-byte-aligned.
const _: () = assert!(core::mem::size_of::<Locals>() % 16 == 0);

impl Locals {
    pub fn new(
        rain_occlusion_matrices: Mat4<f32>,
        rain_occlusion_texture_mat: Mat4<f32>,
        rain_dir_mat: Mat4<f32>,
        rain_density: f32,
        integrated_rain_vel: f32,
    ) -> Self {
        Self {
            rain_occlusion_matrices: rain_occlusion_matrices.into_col_arrays(),
            rain_occlusion_texture_mat: rain_occlusion_texture_mat.into_col_arrays(),
            rain_dir_mat: rain_dir_mat.into_col_arrays(),
            integrated_rain_vel,
            rain_density,
            occlusion_dummy: [0.0; 2],
        }
    }
}

pub type BoundLocals = Bound<Consts<Locals>>;

pub struct RainOcclusionLayout {
    pub locals: wgpu::BindGroupLayout,
}

impl RainOcclusionLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            locals: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            }),
        }
    }

    pub fn bind_locals(&self, device: &wgpu::Device, locals: Consts<Locals>) -> BoundLocals {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.locals,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: locals.buf().as_entire_binding(),
            }],
        });

        BoundLocals {
            bind_group,
            with: locals,
        }
    }
}

pub struct RainOcclusionFigurePipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl RainOcclusionFigurePipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        global_layout: &GlobalsLayouts,
        figure_layout: &FigureLayout,
        aa_mode: AaMode,
    ) -> Self {
        common_base::span!(_guard, "new");

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Rain occlusion figure pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[&global_layout.globals, &figure_layout.locals],
            });

        let samples = aa_mode.samples();

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rain occlusion figure pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: vs_module,
                entry_point: "main",
                buffers: &[TerrainVertex::desc()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: true,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState::IGNORE,
                    back: wgpu::StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: wgpu::DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState {
                count: samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: None,
            multiview: None,
        });

        Self {
            pipeline: render_pipeline,
        }
    }
}

pub struct RainOcclusionPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl RainOcclusionPipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        global_layout: &GlobalsLayouts,
        terrain_layout: &TerrainLayout,
        aa_mode: AaMode,
    ) -> Self {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Rain occlusion pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[&global_layout.globals, &terrain_layout.locals],
            });

        let samples = aa_mode.samples();

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rain occlusion pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: vs_module,
                entry_point: "main",
                buffers: &[TerrainVertex::desc()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: true,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState::IGNORE,
                    back: wgpu::StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: wgpu::DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState {
                count: samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: None,
            multiview: None,
        });

        Self {
            pipeline: render_pipeline,
        }
    }
}
