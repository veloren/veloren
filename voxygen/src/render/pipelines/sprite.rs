use super::super::{AaMode, Bound, Consts, GlobalsLayouts, TerrainLayout, Vertex as VertexTrait};
use bytemuck::{Pod, Zeroable};
use core::fmt;
use std::mem;
use vek::*;

// pub const VERT_PAGE_SIZE: u32 = 256;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Vertex {
    pos: [f32; 3],
    // Because we try to restrict terrain sprite data to a 128×128 block
    // we need an offset into the texture atlas.
    atlas_pos: u32,
    // ____BBBBBBBBGGGGGGGGRRRRRRRR
    // col: u32 = "v_col",
    // ...AANNN
    // A = AO
    // N = Normal
    norm_ao: u32,
}

impl fmt::Display for Vertex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vertex")
            .field("pos", &Vec3::<f32>::from(self.pos))
            .field(
                "atlas_pos",
                &Vec2::new(self.atlas_pos & 0xFFFF, (self.atlas_pos >> 16) & 0xFFFF),
            )
            .field("norm_ao", &self.norm_ao)
            .finish()
    }
}

impl Vertex {
    // NOTE: Limit to 16 (x) × 16 (y) × 32 (z).
    #[allow(clippy::collapsible_else_if)]
    pub fn new(
        atlas_pos: Vec2<u16>,
        pos: Vec3<f32>,
        norm: Vec3<f32>, /* , col: Rgb<f32>, ao: f32 */
    ) -> Self {
        let norm_bits = if norm.x != 0.0 {
            if norm.x < 0.0 { 0 } else { 1 }
        } else if norm.y != 0.0 {
            if norm.y < 0.0 { 2 } else { 3 }
        } else {
            if norm.z < 0.0 { 4 } else { 5 }
        };

        Self {
            // pos_norm: ((pos.x as u32) & 0x003F)
            //     | ((pos.y as u32) & 0x003F) << 6
            //     | (((pos + EXTRA_NEG_Z).z.max(0.0).min((1 << 16) as f32) as u32) & 0xFFFF) << 12
            //     | if meta { 1 } else { 0 } << 28
            //     | (norm_bits & 0x7) << 29,
            pos: pos.into_array(),
            atlas_pos: ((atlas_pos.x as u32) & 0xFFFF) | ((atlas_pos.y as u32) & 0xFFFF) << 16,
            norm_ao: norm_bits,
        }
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Uint32, 2 => Uint32];
        wgpu::VertexBufferLayout {
            array_stride: Self::STRIDE,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

impl VertexTrait for Vertex {
    const QUADS_INDEX: Option<wgpu::IndexFormat> = Some(wgpu::IndexFormat::Uint16);
    const STRIDE: wgpu::BufferAddress = mem::size_of::<Self>() as wgpu::BufferAddress;
}

/* pub fn create_verts_buffer(renderer: &mut Renderer, mut mesh: Mesh<Vertex>) -> Buffer<Vertex> {
    renderer.ensure_sufficient_index_length::<Vertex>(VERT_PAGE_SIZE as usize);
    // TODO: type buffer by Usage
    Buffer::new(
        &renderer.device,
        wgpu::BufferUsage::STORAGE,
        mesh.vertices(),
    )
    //let mut verts = mesh.vertices_mut_vec();
    //let format = wgpu::TextureFormat::Rg32Uint;

    // TODO: temp
    //const WIDTH: u32 = 8192;
    //let height = verts.len() as u32 / WIDTH;
    // Fill in verts to full texture size
    //verts.resize_with(height as usize * WIDTH as usize, Vertex::default);

    /*let texture_info = wgpu::TextureDescriptor {
        label: Some("Sprite verts"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height,
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
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
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

    renderer.create_texture_with_data_raw::<8>(
        &texture_info,
        &view_info,
        &sampler_info,
        bytemuck::cast_slice(verts),
    )*/
}
*/

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Instance {
    pos_ori: u32,
    inst_mat0: [f32; 4],
    inst_mat1: [f32; 4],
    inst_mat2: [f32; 4],
    inst_mat3: [f32; 4],
    inst_light: [f32; 4],
    inst_wind_sway: f32,
}

impl Instance {
    pub fn new(
        mat: Mat4<f32>,
        wind_sway: f32,
        pos: Vec3<i32>,
        ori_bits: u8,
        light: f32,
        glow: f32,
    ) -> Self {
        const EXTRA_NEG_Z: i32 = 32768;

        let mat_arr = mat.into_col_arrays();
        Self {
            pos_ori: ((pos.x as u32) & 0x003F)
                | ((pos.y as u32) & 0x003F) << 6
                | (((pos + EXTRA_NEG_Z).z.max(0).min(1 << 16) as u32) & 0xFFFF) << 12
                | (u32::from(ori_bits) & 0x7) << 29,
            inst_mat0: mat_arr[0],
            inst_mat1: mat_arr[1],
            inst_mat2: mat_arr[2],
            inst_mat3: mat_arr[3],
            inst_light: [light, glow, 1.0, 1.0],
            inst_wind_sway: wind_sway,
        }
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
            3 => Uint32,
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4,
            7 => Float32x4,
            8 => Float32x4,
            9 => Float32,
        ];
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &ATTRIBUTES,
        }
    }
}

impl Default for Instance {
    fn default() -> Self { Self::new(Mat4::identity(), 0.0, Vec3::zero(), 0, 1.0, 0.0) }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Locals {
    // Each matrix performs rotatation, translation, and scaling, relative to the sprite
    // origin, for all sprite instances.  The matrix will be in an array indexed by the
    // sprite instance's orientation (0 through 7).
    mat: [[f32; 4]; 4],
    wind_sway: [f32; 4],
    offs: [f32; 4],
}

pub type BoundLocals = Bound<Consts<Locals>>;

impl Default for Locals {
    fn default() -> Self { Self::new(Mat4::identity(), Vec3::one(), Vec3::zero(), 0.0) }
}

impl Locals {
    pub fn new(mat: Mat4<f32>, scale: Vec3<f32>, offs: Vec3<f32>, wind_sway: f32) -> Self {
        Self {
            mat: mat.into_col_arrays(),
            wind_sway: [scale.x, scale.y, scale.z, wind_sway],
            offs: [offs.x, offs.y, offs.z, 0.0],
        }
    }
}

pub struct SpriteLayout {
    pub locals: wgpu::BindGroupLayout,
}

impl SpriteLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            locals: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // locals
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // instance buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,    
                        visibility: wgpu::ShaderStage::Vertex,
                        ty: wgpu::BufferBindingType::Buffer {
                            ty: wgpu::BufferBindingType:: 
                        }
                    },
                ],
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

pub struct SpritePipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl SpritePipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
        sc_desc: &wgpu::SwapChainDescriptor,
        global_layout: &GlobalsLayouts,
        layout: &SpriteLayout,
        terrain_layout: &TerrainLayout,
        aa_mode: AaMode,
    ) -> Self {
        common_base::span!(_guard, "SpritePipeline::new");
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Sprite pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[
                    &global_layout.globals,
                    &global_layout.shadow_textures,
                    &terrain_layout.locals,
                    &layout.locals,
                    &global_layout.col_light,
                ],
            });

        let samples = match aa_mode {
            AaMode::None | AaMode::Fxaa => 1,
            // TODO: Ensure sampling in the shader is exactly between the 4 texels
            AaMode::MsaaX4 => 4,
            AaMode::MsaaX8 => 8,
            AaMode::MsaaX16 => 16,
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sprite pipeline"),
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
                cull_mode: Some(wgpu::Face::Back),
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
                    // TODO: can we remove sprite transparency?
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
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
        });

        Self {
            pipeline: render_pipeline,
        }
    }
}
