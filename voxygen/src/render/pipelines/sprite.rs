use super::{
    super::{
        buffer::Buffer, AaMode, GlobalsLayouts, Mesh, TerrainLayout, Texture, Vertex as VertexTrait,
    },
    lod_terrain, GlobalModel,
};
use bytemuck::{Pod, Zeroable};
use std::mem;
use vek::*;

pub const VERT_PAGE_SIZE: u32 = 256;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Vertex {
    pos_norm: u32,
    // Because we try to restrict terrain sprite data to a 128×128 block
    // we need an offset into the texture atlas.
    atlas_pos: u32,
    /* ____BBBBBBBBGGGGGGGGRRRRRRRR
     * col: u32 = "v_col",
     * .....NNN
     * A = AO
     * N = Normal
     *norm: u32, */
}

// TODO: fix?
/*impl fmt::Display for Vertex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vertex")
            .field("pos_norm", &Vec3::<f32>::from(self.pos))
            .field(
                "atlas_pos",
                &Vec2::new(self.atlas_pos & 0xFFFF, (self.atlas_pos >> 16) & 0xFFFF),
            )
            .finish()
    }
}*/

impl Vertex {
    // NOTE: Limit to 16 (x) × 16 (y) × 32 (z).
    #[allow(clippy::collapsible_else_if)]
    pub fn new(atlas_pos: Vec2<u16>, pos: Vec3<f32>, norm: Vec3<f32>) -> Self {
        const VERT_EXTRA_NEG_XY: i32 = 128;
        const VERT_EXTRA_NEG_Z: i32 = 128; // NOTE: change if number of bits changes below, also we might not need this if meshing always produces positives values for sprites (I have no idea)

        #[allow(clippy::bool_to_int_with_if)]
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
            pos_norm: (((pos.x as i32 + VERT_EXTRA_NEG_XY) & 0x00FF) as u32) // NOTE: temp hack, this doesn't need 8 bits
                | (((pos.y as i32 + VERT_EXTRA_NEG_XY) & 0x00FF) as u32) << 8
                | (((pos.z as i32 + VERT_EXTRA_NEG_Z).clamp(0, 1 << 12) as u32) & 0x0FFF) << 16
                | (norm_bits & 0x7) << 29,
            atlas_pos: ((atlas_pos.x as u32) & 0xFFFF) | ((atlas_pos.y as u32) & 0xFFFF) << 16,
        }
    }
}

impl Default for Vertex {
    fn default() -> Self { Self::new(Vec2::zero(), Vec3::zero(), Vec3::zero()) }
}

impl VertexTrait for Vertex {
    const QUADS_INDEX: Option<wgpu::IndexFormat> = Some(wgpu::IndexFormat::Uint16);
    const STRIDE: wgpu::BufferAddress = mem::size_of::<Self>() as wgpu::BufferAddress;
}

pub struct SpriteVerts(Buffer<Vertex>);
//pub struct SpriteVerts(Texture);

pub(in super::super) fn create_verts_buffer(
    device: &wgpu::Device,
    mesh: Mesh<Vertex>,
) -> SpriteVerts {
    // TODO: type Buffer by wgpu::BufferUsage
    SpriteVerts(Buffer::new(
        device,
        wgpu::BufferUsage::STORAGE,
        mesh.vertices(),
    ))
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Instance {
    inst_mat0: [f32; 4],
    inst_mat1: [f32; 4],
    inst_mat2: [f32; 4],
    inst_mat3: [f32; 4],
    pos_ori_door: u32,
    inst_vert_page: u32,
    inst_light: f32,
    inst_glow: f32,
    model_wind_sway: f32,
    model_z_scale: f32,
}

impl Instance {
    pub fn new(
        mat: Mat4<f32>,
        wind_sway: f32,
        z_scale: f32,
        pos: Vec3<i32>,
        ori_bits: u8,
        light: f32,
        glow: f32,
        vert_page: u32,
        is_door: bool,
    ) -> Self {
        const EXTRA_NEG_Z: i32 = 32768;

        let mat_arr = mat.into_col_arrays();
        Self {
            inst_mat0: mat_arr[0],
            inst_mat1: mat_arr[1],
            inst_mat2: mat_arr[2],
            inst_mat3: mat_arr[3],
            pos_ori_door: ((pos.x as u32) & 0x003F)
                | ((pos.y as u32) & 0x003F) << 6
                | (((pos.z + EXTRA_NEG_Z).clamp(0, 1 << 16) as u32) & 0xFFFF) << 12
                | (u32::from(ori_bits) & 0x7) << 29
                | (u32::from(is_door) & 1) << 28,
            inst_vert_page: vert_page,
            inst_light: light,
            inst_glow: glow,
            model_wind_sway: wind_sway,
            model_z_scale: z_scale,
        }
    }

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 10] = wgpu::vertex_attr_array![
            0 => Float32x4,
            1 => Float32x4,
            2 => Float32x4,
            3 => Float32x4,
            4 => Uint32,
            5 => Uint32,
            6 => Float32,
            7 => Float32,
            8 => Float32,
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
    fn default() -> Self {
        Self::new(
            Mat4::identity(),
            0.0,
            0.0,
            Vec3::zero(),
            0,
            1.0,
            0.0,
            0,
            false,
        )
    }
}

// TODO: ColLightsWrapper instead?
pub struct Locals;

pub struct SpriteGlobalsBindGroup {
    pub(in super::super) bind_group: wgpu::BindGroup,
}

pub struct SpriteLayout {
    pub globals: wgpu::BindGroupLayout,
}

impl SpriteLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        let mut entries = GlobalsLayouts::base_globals_layout();
        debug_assert_eq!(15, entries.len()); // To remember to adjust the bindings below
        entries.extend_from_slice(&[
            // sprite_verts
            wgpu::BindGroupLayoutEntry {
                binding: 15,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: core::num::NonZeroU64::new(mem::size_of::<Vertex>() as u64),
                },
                count: None,
            },
        ]);

        Self {
            globals: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &entries,
            }),
        }
    }

    fn bind_globals_inner(
        &self,
        device: &wgpu::Device,
        global_model: &GlobalModel,
        lod_data: &lod_terrain::LodData,
        noise: &Texture,
        sprite_verts: &SpriteVerts,
    ) -> wgpu::BindGroup {
        let mut entries = GlobalsLayouts::bind_base_globals(global_model, lod_data, noise);

        entries.extend_from_slice(&[
            // sprite_verts
            wgpu::BindGroupEntry {
                binding: 15,
                resource: sprite_verts.0.buf.as_entire_binding(),
            },
        ]);

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.globals,
            entries: &entries,
        })
    }

    pub fn bind_globals(
        &self,
        device: &wgpu::Device,
        global_model: &GlobalModel,
        lod_data: &lod_terrain::LodData,
        noise: &Texture,
        sprite_verts: &SpriteVerts,
    ) -> SpriteGlobalsBindGroup {
        let bind_group =
            self.bind_globals_inner(device, global_model, lod_data, noise, sprite_verts);

        SpriteGlobalsBindGroup { bind_group }
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
                    &layout.globals,
                    &global_layout.shadow_textures,
                    // Note: mergable with globals
                    &global_layout.col_light,
                    &terrain_layout.locals,
                ],
            });

        let samples = aa_mode.samples();

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sprite pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: vs_module,
                entry_point: "main",
                buffers: &[Instance::desc()],
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
                    },
                    wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Uint,
                        blend: None,
                        write_mask: wgpu::ColorWrite::ALL,
                    },
                ],
            }),
        });

        Self {
            pipeline: render_pipeline,
        }
    }
}
