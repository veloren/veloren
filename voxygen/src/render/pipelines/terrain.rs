use super::super::{AaMode, Bound, Consts, GlobalsLayouts, Vertex as VertexTrait};
use bytemuck::{Pod, Zeroable};
use std::mem;
use vek::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Vertex {
    pos_norm: u32,
    atlas_pos: u32,
}

impl Vertex {
    /// NOTE: meta is true when the terrain vertex is touching water.
    pub fn new(atlas_pos: Vec2<u16>, pos: Vec3<f32>, norm: Vec3<f32>, meta: bool) -> Self {
        const EXTRA_NEG_Z: f32 = 32768.0;

        #[allow(clippy::bool_to_int_with_if)]
        let norm_bits = if norm.x != 0.0 {
            if norm.x < 0.0 { 0 } else { 1 }
        } else if norm.y != 0.0 {
            if norm.y < 0.0 { 2 } else { 3 }
        } else if norm.z < 0.0 {
            4
        } else {
            5
        };
        Self {
            pos_norm: ((pos.x as u32) & 0x003F) << 0
                | ((pos.y as u32) & 0x003F) << 6
                | (((pos + EXTRA_NEG_Z).z.clamp(0.0, (1 << 16) as f32) as u32) & 0xFFFF) << 12
                | u32::from(meta) << 28
                | (norm_bits & 0x7) << 29,
            atlas_pos: ((atlas_pos.x as u32) & 0xFFFF) << 0 | ((atlas_pos.y as u32) & 0xFFFF) << 16,
        }
    }

    pub fn new_figure(atlas_pos: Vec2<u16>, pos: Vec3<f32>, norm: Vec3<f32>, bone_idx: u8) -> Self {
        let norm_bits = u32::from(norm.x.min(norm.y).min(norm.z) >= 0.0);
        let axis_bits = if norm.x != 0.0 {
            0
        } else if norm.y != 0.0 {
            1
        } else {
            2
        };
        Self {
            pos_norm: pos
                .map2(Vec3::new(0, 9, 18), |e, shift| {
                    (((e * 2.0 + 256.0) as u32) & 0x1FF) << shift
                })
                .reduce_bitor()
                | (((bone_idx & 0xF) as u32) << 27)
                | (norm_bits << 31),
            atlas_pos: ((atlas_pos.x as u32) & 0x7FFF) << 2
                | ((atlas_pos.y as u32) & 0x7FFF) << 17
                | axis_bits & 3,
        }
    }

    pub fn make_col_light(
        // 0 to 31
        light: u8,
        // 0 to 31
        glow: u8,
        col: Rgb<u8>,
        ao: bool,
    ) -> [u8; 4] {
        //[col.r, col.g, col.b, light]
        // It would be nice for this to be cleaner, but we want to squeeze 5 fields into
        // 4. We can do this because both `light` and `glow` go from 0 to 31,
        // meaning that they can both fit into 5 bits. If we steal a bit from
        // red and blue each (not green, human eyes are more sensitive to
        // changes in green) then we get just enough to expand the nibbles of
        // the alpha field enough to fit both `light` and `glow`.
        //
        // However, we now have a problem. In the shader code with use hardware
        // filtering to get at the `light` and `glow` attributes (but not
        // colour, that remains constant across a block). How do we resolve this
        // if we're twiddling bits? The answer is to very carefully manipulate
        // the bit pattern such that the fields we want to filter (`light` and
        // `glow`) always sit as the higher bits of the fields. Then, we can do
        // some modulation magic to extract them from the filtering unharmed and use
        // unfiltered texture access (i.e: `texelFetch`) to access the colours, plus a
        // little bit-fiddling.
        //
        // TODO: This isn't currently working (no idea why). See `srgb.glsl` for current
        // impl that intead does manual bit-twiddling and filtering.
        [
            (light.min(31) << 3) | ((col.r >> 1) & 0b111),
            (glow.min(31) << 3) | ((col.b >> 1) & 0b111),
            (col.r & 0b11110000) | (col.b >> 4),
            (col.g & 0xFE) | ao as u8,
        ]
    }

    pub fn make_col_light_figure(
        // 0 to 31
        light: u8,
        glowy: bool,
        shiny: bool,
        col: Rgb<u8>,
    ) -> [u8; 4] {
        let attr = 0 | ((glowy as u8) << 0) | ((shiny as u8) << 1);
        [
            (light.min(31) << 3) | ((col.r >> 1) & 0b111),
            (attr.min(31) << 3) | ((col.b >> 1) & 0b111),
            (col.r & 0b11110000) | (col.b >> 4),
            col.g, // Green is lucky, it remains unscathed
        ]
    }

    /// Set the bone_idx for an existing figure vertex.
    pub fn set_bone_idx(&mut self, bone_idx: u8) {
        self.pos_norm = (self.pos_norm & !(0xF << 27)) | ((bone_idx as u32 & 0xF) << 27);
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Uint32,1 => Uint32];
        wgpu::VertexBufferLayout {
            array_stride: Self::STRIDE,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

impl VertexTrait for Vertex {
    // Note: I think it's u32 due to figures??
    // potentiall optimize by splitting
    const QUADS_INDEX: Option<wgpu::IndexFormat> = Some(wgpu::IndexFormat::Uint32);
    const STRIDE: wgpu::BufferAddress = mem::size_of::<Self>() as wgpu::BufferAddress;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
// TODO: new function and private fields??
pub struct Locals {
    model_offs: [f32; 3],
    load_time: f32,
    atlas_offs: [i32; 4],
}

impl Locals {
    pub fn new(model_offs: Vec3<f32>, atlas_offs: Vec2<u32>, load_time: f32) -> Self {
        Self {
            model_offs: model_offs.into_array(),
            load_time,
            atlas_offs: Vec4::new(atlas_offs.x as i32, atlas_offs.y as i32, 0, 0).into_array(),
        }
    }
}

impl Default for Locals {
    fn default() -> Self {
        Self {
            model_offs: [0.0; 3],
            load_time: 0.0,
            atlas_offs: [0; 4],
        }
    }
}

pub type BoundLocals = Bound<Consts<Locals>>;

pub struct TerrainLayout {
    pub locals: wgpu::BindGroupLayout,
}

impl TerrainLayout {
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

pub struct TerrainPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl TerrainPipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
        global_layout: &GlobalsLayouts,
        layout: &TerrainLayout,
        aa_mode: AaMode,
    ) -> Self {
        common_base::span!(_guard, "TerrainPipeline::new");
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Terrain pipeline layout"),
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
            label: Some("Terrain pipeline"),
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
