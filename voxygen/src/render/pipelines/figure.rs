use super::{
    super::{AaMode, Bound, Consts, GlobalsLayouts, Mesh, Model},
    terrain::Vertex,
};
use crate::mesh::greedy::GreedyMesh;
use bytemuck::{Pod, Zeroable};
use vek::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Locals {
    model_mat: [[f32; 4]; 4],
    highlight_col: [f32; 4],
    model_light: [f32; 4],
    model_glow: [f32; 4],
    atlas_offs: [i32; 4],
    model_pos: [f32; 3],
    flags: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct BoneData {
    bone_mat: [[f32; 4]; 4],
    normals_mat: [[f32; 4]; 4],
}

pub type BoundLocals = Bound<(Consts<Locals>, Consts<BoneData>)>;

impl Locals {
    pub fn new(
        model_mat: anim::vek::Mat4<f32>,
        col: Rgb<f32>,
        pos: anim::vek::Vec3<f32>,
        atlas_offs: Vec2<i32>,
        is_player: bool,
        light: f32,
        glow: (Vec3<f32>, f32),
    ) -> Self {
        let mut flags = 0;
        flags |= is_player as u32;

        Self {
            model_mat: model_mat.into_col_arrays(),
            highlight_col: [col.r, col.g, col.b, 1.0],
            model_pos: pos.into_array(),
            atlas_offs: Vec4::from(atlas_offs).into_array(),
            model_light: [light, 1.0, 1.0, 1.0],
            model_glow: [glow.0.x, glow.0.y, glow.0.z, glow.1],
            flags,
        }
    }
}

impl Default for Locals {
    fn default() -> Self {
        Self::new(
            anim::vek::Mat4::identity(),
            Rgb::broadcast(1.0),
            anim::vek::Vec3::default(),
            Vec2::default(),
            false,
            1.0,
            (Vec3::zero(), 0.0),
        )
    }
}

impl BoneData {
    pub fn new(bone_mat: anim::vek::Mat4<f32>, normals_mat: anim::vek::Mat4<f32>) -> Self {
        Self {
            bone_mat: bone_mat.into_col_arrays(),
            normals_mat: normals_mat.into_col_arrays(),
        }
    }
}

impl Default for BoneData {
    fn default() -> Self { Self::new(anim::vek::Mat4::identity(), anim::vek::Mat4::identity()) }
}

pub struct FigureModel {
    pub opaque: Option<Model<Vertex>>,
    /* TODO: Consider using mipmaps instead of storing multiple texture atlases for different
     * LOD levels. */
}

impl FigureModel {
    /// Start a greedy mesh designed for figure bones.
    pub fn make_greedy<'a>() -> GreedyMesh<'a> {
        // NOTE: Required because we steal two bits from the normal in the shadow uint
        // in order to store the bone index.  The two bits are instead taken out
        // of the atlas coordinates, which is why we "only" allow 1 << 15 per
        // coordinate instead of 1 << 16.
        let max_size = Vec2::new((1 << 15) - 1, (1 << 15) - 1);
        GreedyMesh::new(max_size, crate::mesh::greedy::general_config())
    }
}

pub type BoneMeshes = (Mesh<Vertex>, anim::vek::Aabb<f32>);

pub struct FigureLayout {
    pub locals: wgpu::BindGroupLayout,
}

impl FigureLayout {
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
                    // bone data
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
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

    pub fn bind_locals(
        &self,
        device: &wgpu::Device,
        locals: Consts<Locals>,
        bone_data: Consts<BoneData>,
    ) -> BoundLocals {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.locals,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: locals.buf().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bone_data.buf().as_entire_binding(),
                },
            ],
        });

        BoundLocals {
            bind_group,
            with: (locals, bone_data),
        }
    }
}

pub struct FigurePipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl FigurePipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
        global_layout: &GlobalsLayouts,
        layout: &FigureLayout,
        aa_mode: AaMode,
    ) -> Self {
        common_base::span!(_guard, "FigurePipeline::new");
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Figure pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[
                    &global_layout.globals,
                    &global_layout.shadow_textures,
                    &global_layout.col_light,
                    &layout.locals,
                ],
            });

        let samples = aa_mode.samples();

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Figure pipeline"),
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
                        blend: None,
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
