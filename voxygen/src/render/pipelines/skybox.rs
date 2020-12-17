use super::super::{AaMode, GlobalsLayouts, Mesh, Quad, Vertex as VertexTrait};
use bytemuck::{Pod, Zeroable};
use std::mem;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Vertex {
    pub pos: [f32; 3],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: Self::STRIDE,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[wgpu::VertexAttributeDescriptor {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float3,
            }],
        }
    }
}

impl VertexTrait for Vertex {
    const STRIDE: wgpu::BufferAddress = mem::size_of::<Self>() as wgpu::BufferAddress;
}

pub struct SkyboxPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl SkyboxPipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
        sc_desc: &wgpu::SwapChainDescriptor,
        layouts: &GlobalsLayouts,
        aa_mode: AaMode,
    ) -> Self {
        common::span!(_guard, "SkyboxPipeline::new");
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Skybox pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[&layouts.globals, &layouts.shadow_textures],
            });

        let samples = match aa_mode {
            AaMode::None | AaMode::Fxaa => 1,
            // TODO: Ensure sampling in the shader is exactly between the 4 texels
            AaMode::MsaaX4 => 4,
            AaMode::MsaaX8 => 8,
            AaMode::MsaaX16 => 16,
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Skybox pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                polygon_mode: wgpu::PolygonMode::Fill,
                clamp_depth: false,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format: sc_desc.format,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::GreaterEqual,
                stencil: wgpu::StencilStateDescriptor {
                    front: wgpu::StencilStateFaceDescriptor::IGNORE,
                    back: wgpu::StencilStateFaceDescriptor::IGNORE,
                    read_mask: !0,
                    write_mask: !0,
                },
            }),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: None,
                vertex_buffers: &[Vertex::desc()],
            },
            sample_count: samples,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        Self {
            pipeline: render_pipeline,
        }
    }
}

// TODO: generate mesh in vertex shader
pub fn create_mesh() -> Mesh<Vertex> {
    let mut mesh = Mesh::new();

    // -x
    #[rustfmt::skip]
    mesh.push_quad(Quad::new(
        Vertex { pos: [-1.0, -1.0, -1.0] },
        Vertex { pos: [-1.0,  1.0, -1.0] },
        Vertex { pos: [-1.0,  1.0,  1.0] },
        Vertex { pos: [-1.0, -1.0,  1.0] },
    ));
    // +x
    #[rustfmt::skip]
    mesh.push_quad(Quad::new(
        Vertex { pos: [ 1.0, -1.0,  1.0] },
        Vertex { pos: [ 1.0,  1.0,  1.0] },
        Vertex { pos: [ 1.0,  1.0, -1.0] },
        Vertex { pos: [ 1.0, -1.0, -1.0] },
    ));
    // -y
    #[rustfmt::skip]
    mesh.push_quad(Quad::new(
        Vertex { pos: [ 1.0, -1.0, -1.0] },
        Vertex { pos: [-1.0, -1.0, -1.0] },
        Vertex { pos: [-1.0, -1.0,  1.0] },
        Vertex { pos: [ 1.0, -1.0,  1.0] },
    ));
    // +y
    #[rustfmt::skip]
    mesh.push_quad(Quad::new(
        Vertex { pos: [ 1.0,  1.0,  1.0] },
        Vertex { pos: [-1.0,  1.0,  1.0] },
        Vertex { pos: [-1.0,  1.0, -1.0] },
        Vertex { pos: [ 1.0,  1.0, -1.0] },
    ));
    // -z
    #[rustfmt::skip]
    mesh.push_quad(Quad::new(
        Vertex { pos: [-1.0, -1.0, -1.0] },
        Vertex { pos: [ 1.0, -1.0, -1.0] },
        Vertex { pos: [ 1.0,  1.0, -1.0] },
        Vertex { pos: [-1.0,  1.0, -1.0] },
    ));
    // +z
    #[rustfmt::skip]
    mesh.push_quad(Quad::new(
        Vertex { pos: [-1.0,  1.0,  1.0] },
        Vertex { pos: [ 1.0,  1.0,  1.0] },
        Vertex { pos: [ 1.0, -1.0,  1.0] },
        Vertex { pos: [-1.0, -1.0,  1.0] },
    ));

    mesh
}
