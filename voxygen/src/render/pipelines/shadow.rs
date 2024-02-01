use super::super::{
    AaMode, Bound, Consts, DebugLayout, DebugVertex, FigureLayout, GlobalsLayouts, TerrainLayout,
    TerrainVertex,
};
use bytemuck::{Pod, Zeroable};
use vek::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Locals {
    shadow_matrices: [[f32; 4]; 4],
    texture_mats: [[f32; 4]; 4],
}

impl Locals {
    pub fn new(shadow_mat: Mat4<f32>, texture_mat: Mat4<f32>) -> Self {
        Self {
            shadow_matrices: shadow_mat.into_col_arrays(),
            texture_mats: texture_mat.into_col_arrays(),
        }
    }
}

impl Default for Locals {
    fn default() -> Self { Self::new(Mat4::identity(), Mat4::identity()) }
}

pub type BoundLocals = Bound<Consts<Locals>>;

pub struct ShadowLayout {
    pub locals: wgpu::BindGroupLayout,
}

impl ShadowLayout {
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

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct PointLightMatrix([[f32; 4]; 4]);

impl PointLightMatrix {
    pub fn new(shadow_mat: Mat4<f32>) -> Self { Self(shadow_mat.into_col_arrays()) }
}

impl Default for PointLightMatrix {
    fn default() -> Self { Self::new(Mat4::identity()) }
}

pub struct ShadowFigurePipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl ShadowFigurePipeline {
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
                label: Some("Directed figure shadow pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[&global_layout.globals, &figure_layout.locals],
            });

        let samples = aa_mode.samples();

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Directed shadow figure pipeline"),
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
                cull_mode: Some(wgpu::Face::Front),
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
                    read_mask: !0,
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

pub struct ShadowPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl ShadowPipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        global_layout: &GlobalsLayouts,
        terrain_layout: &TerrainLayout,
        aa_mode: AaMode,
    ) -> Self {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Directed shadow pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[&global_layout.globals, &terrain_layout.locals],
            });

        let samples = aa_mode.samples();

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Directed shadow pipeline"),
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
                cull_mode: Some(wgpu::Face::Front),
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
                    read_mask: !0,
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
pub struct PointShadowPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl PointShadowPipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        global_layout: &GlobalsLayouts,
        terrain_layout: &TerrainLayout,
        aa_mode: AaMode,
    ) -> Self {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Point shadow pipeline layout"),
                push_constant_ranges: &[wgpu::PushConstantRange {
                    stages: wgpu::ShaderStages::all(),
                    range: 0..64,
                }],
                bind_group_layouts: &[&global_layout.globals, &terrain_layout.locals],
            });

        let samples = aa_mode.samples();

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Point shadow pipeline"),
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
                unclipped_depth: false,
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
                    read_mask: !0,
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

pub struct ShadowDebugPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl ShadowDebugPipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        global_layout: &GlobalsLayouts,
        debug_layout: &DebugLayout,
        aa_mode: AaMode,
    ) -> Self {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Directed shadow debug pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[&global_layout.globals, &debug_layout.locals],
            });

        let samples = aa_mode.samples();

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Directed shadow debug pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: vs_module,
                entry_point: "main",
                buffers: &[DebugVertex::desc()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Front),
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
                    read_mask: !0,
                    write_mask: !0,
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
