use super::super::{AaMode, Bound, Consts, GlobalsLayouts, Vertex as VertexTrait};
use bytemuck::{Pod, Zeroable};
use std::mem;
use vek::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
    pub normal: [f32; 3],
}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4, 2 => Float32x3];
        wgpu::VertexBufferLayout {
            array_stride: Self::STRIDE,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

impl VertexTrait for Vertex {
    const QUADS_INDEX: Option<wgpu::IndexFormat> = None;
    const STRIDE: wgpu::BufferAddress = mem::size_of::<Self>() as wgpu::BufferAddress;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Locals {
    /// pos is [f32; 4] instead of [f32; 3] so that Locals' size is a multiple
    /// of 8 bytes (which is required by gfx), the last component is ignored
    /// by the shader
    pub pos: [f32; 4],
    pub color: [f32; 4],
    /// quaternion as [x, y, z, w]
    pub ori: [f32; 4],
}

pub type BoundLocals = Bound<Consts<Locals>>;

impl From<Vec3<f32>> for Vertex {
    fn from(pos: Vec3<f32>) -> Vertex {
        Vertex {
            pos: [pos.x, pos.y, pos.z],
            color: [1.0; 4],
            normal: [0.0, 0.0, 1.0],
        }
    }
}

impl From<(Vec3<f32>, [f32; 4], Vec3<f32>)> for Vertex {
    fn from((pos, color, normal): (Vec3<f32>, [f32; 4], Vec3<f32>)) -> Vertex {
        Vertex {
            pos: [pos.x, pos.y, pos.z],
            color,
            normal: [normal.x, normal.y, normal.z],
        }
    }
}

pub struct DebugPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl DebugPipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
        global_layouts: &GlobalsLayouts,
        layout: &DebugLayout,
        aa_mode: AaMode,
    ) -> Self {
        common_base::span!(_guard, "DebugPipeline::new");
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Debug pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[
                    &global_layouts.globals,
                    &global_layouts.shadow_textures,
                    &layout.locals,
                ],
            });

        let samples = aa_mode.samples();

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Debug pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: vs_module,
                entry_point: "main",
                buffers: &[Vertex::desc()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                clamp_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::GreaterEqual,
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
            fragment: Some(wgpu::FragmentState {
                module: fs_module,
                entry_point: "main",
                targets: &[
                    wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrite::ALL,
                    },
                    wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Uint,
                        blend: None,
                        write_mask: wgpu::ColorWrite::empty(),
                    },
                ],
            }),
        });

        Self {
            pipeline: render_pipeline,
        }
    }
}

pub struct DebugLayout {
    pub locals: wgpu::BindGroupLayout,
}

impl DebugLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            locals: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
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
