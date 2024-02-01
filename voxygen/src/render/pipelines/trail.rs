use super::super::{AaMode, GlobalsLayouts, Vertex as VertexTrait};
use bytemuck::{Pod, Zeroable};
use std::{
    mem,
    ops::{Add, Mul},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, PartialEq)]
pub struct Vertex {
    pub pos: [f32; 3],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];
        wgpu::VertexBufferLayout {
            array_stride: Self::STRIDE,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }

    pub fn zero() -> Self { Self { pos: [0.0; 3] } }
}

impl Mul<f32> for Vertex {
    type Output = Self;

    fn mul(self, val: f32) -> Self::Output {
        Self {
            pos: self.pos.map(|a| a * val),
        }
    }
}

impl Add<Vertex> for Vertex {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            pos: [
                self.pos[0] + other.pos[0],
                self.pos[1] + other.pos[1],
                self.pos[2] + other.pos[2],
            ],
        }
    }
}

impl VertexTrait for Vertex {
    const QUADS_INDEX: Option<wgpu::IndexFormat> = Some(wgpu::IndexFormat::Uint16);
    const STRIDE: wgpu::BufferAddress = mem::size_of::<Self>() as wgpu::BufferAddress;
}

pub struct TrailPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl TrailPipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
        global_layout: &GlobalsLayouts,
        aa_mode: AaMode,
        format: wgpu::TextureFormat,
    ) -> Self {
        common_base::span!(_guard, "TrailPipeline::new");
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Trail pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[&global_layout.globals, &global_layout.shadow_textures],
            });

        let samples = aa_mode.samples();

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Trail pipeline"),
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
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::GreaterEqual,
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
            fragment: Some(wgpu::FragmentState {
                module: fs_module,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        Self {
            pipeline: render_pipeline,
        }
    }
}
