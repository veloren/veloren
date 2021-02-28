use super::super::{AaMode, GlobalsLayouts, Mesh, Quad, Vertex as VertexTrait};
use bytemuck::{Pod, Zeroable};
use std::mem;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Vertex {
    pub pos: [f32; 3],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: Self::STRIDE,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
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
        common_base::span!(_guard, "SkyboxPipeline::new");
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
                polygon_mode: wgpu::PolygonMode::Fill,
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
                clamp_depth: false,
            }),
            multisample: wgpu::MultisampleState {
                count: samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: sc_desc.format,
                    blend: None,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
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
