use super::super::{AaMode, GlobalsLayouts, Renderer, Texture, Vertex as VertexTrait};
use bytemuck::{Pod, Zeroable};
use std::mem;
use vek::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Vertex {
    pos: [f32; 2],
}

impl Vertex {
    pub fn new(pos: Vec2<f32>) -> Self {
        Self {
            pos: pos.into_array(),
        }
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float2];
        wgpu::VertexBufferLayout {
            array_stride: Self::STRIDE,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

impl VertexTrait for Vertex {
    const STRIDE: wgpu::BufferAddress = mem::size_of::<Self>() as wgpu::BufferAddress;
}

pub struct LodData {
    pub map: Texture,
    pub alt: Texture,
    pub horizon: Texture,
    pub tgt_detail: u32,
}

impl LodData {
    pub fn dummy(renderer: &mut Renderer) -> Self {
        let map_size = Vec2::new(1, 1);
        let map_border = [0.0, 0.0, 0.0, 0.0];
        let map_image = [0];
        let alt_image = [0];
        let horizon_image = [0x_00_01_00_01];
        //let map_border = [0.0, 0.0, 0.0, 0.0];

        Self::new(
            renderer,
            map_size,
            &map_image,
            &alt_image,
            &horizon_image,
            1,
            //map_border.into(),
        )
    }

    pub fn new(
        renderer: &mut Renderer,
        map_size: Vec2<u32>,
        lod_base: &[u32],
        lod_alt: &[u32],
        lod_horizon: &[u32],
        tgt_detail: u32,
        //border_color: gfx::texture::PackedColor,
    ) -> Self {
        let mut create_texture = |format, data, filter| {
            let texture_info = wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: map_size.x,
                    height: map_size.y,
                    depth: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            };

            let sampler_info = wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: wgpu::FilterMode::Nearest,
                border_color: Some(wgpu::SamplerBorderColor::TransparentBlack),
                ..Default::default()
            };

            let view_info = wgpu::TextureViewDescriptor {
                label: None,
                format: Some(format),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            };

            renderer.create_texture_with_data_raw(
                &texture_info,
                &view_info,
                &sampler_info,
                bytemuck::cast_slice(data),
            )
        };
        let map = create_texture(
            wgpu::TextureFormat::Rgba8UnormSrgb,
            lod_base,
            wgpu::FilterMode::Linear,
        );
        //             SamplerInfo {
        //                 border: border_color,
        let alt = create_texture(
            wgpu::TextureFormat::Rgba8Unorm,
            lod_alt,
            wgpu::FilterMode::Linear,
        );
        //             SamplerInfo {
        //                 border: [0.0, 0.0, 0.0, 0.0].into(),
        let horizon = create_texture(
            wgpu::TextureFormat::Rgba8Unorm,
            lod_horizon,
            wgpu::FilterMode::Linear,
        );
        //             SamplerInfo {
        //                 border: [1.0, 0.0, 1.0, 0.0].into(),

        Self {
            map,
            alt,
            horizon,
            tgt_detail,
        }
    }
}

pub struct LodTerrainPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl LodTerrainPipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
        sc_desc: &wgpu::SwapChainDescriptor,
        global_layout: &GlobalsLayouts,
        aa_mode: AaMode,
    ) -> Self {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Lod terrain pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[&global_layout.globals, &global_layout.shadow_textures],
            });

        let samples = match aa_mode {
            AaMode::None | AaMode::Fxaa => 1,
            // TODO: Ensure sampling in the shader is exactly between the 4 texels
            AaMode::MsaaX4 => 4,
            AaMode::MsaaX8 => 8,
            AaMode::MsaaX16 => 16,
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Lod terrain pipeline"),
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
                cull_mode: wgpu::CullMode::Back,
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
