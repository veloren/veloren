use super::super::{AaMode, GlobalsLayouts, Vertex as VertexTrait};
use bytemuck::{Pod, Zeroable};
use std::mem;
use vek::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Vertex {
    pub pos: [f32; 3],
    // ____BBBBBBBBGGGGGGGGRRRRRRRR
    // col: u32 = "v_col",
    // ...AANNN
    // A = AO
    // N = Normal
    norm_ao: u32,
}

impl Vertex {
    #[allow(clippy::collapsible_else_if)]
    pub fn new(pos: Vec3<f32>, norm: Vec3<f32>) -> Self {
        let norm_bits = if norm.x != 0.0 {
            if norm.x < 0.0 { 0 } else { 1 }
        } else if norm.y != 0.0 {
            if norm.y < 0.0 { 2 } else { 3 }
        } else {
            if norm.z < 0.0 { 4 } else { 5 }
        };

        Self {
            pos: pos.into_array(),
            norm_ao: norm_bits,
        }
    }

    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        const ATTRIBUTES: [wgpu::VertexAttributeDescriptor; 2] =
            wgpu::vertex_attr_array![0 => Float3, 1 => Uint];
        wgpu::VertexBufferDescriptor {
            stride: Self::STRIDE,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

impl VertexTrait for Vertex {
    const STRIDE: wgpu::BufferAddress = mem::size_of::<Self>() as wgpu::BufferAddress;
}

#[derive(Copy, Clone)]
pub enum ParticleMode {
    CampfireSmoke = 0,
    CampfireFire = 1,
    GunPowderSpark = 2,
    Shrapnel = 3,
    FireworkBlue = 4,
    FireworkGreen = 5,
    FireworkPurple = 6,
    FireworkRed = 7,
    FireworkWhite = 8,
    FireworkYellow = 9,
    Leaf = 10,
    Firefly = 11,
    Bee = 12,
    GroundShockwave = 13,
    HealingBeam = 14,
    EnergyNature = 15,
    FlameThrower = 16,
    FireShockwave = 17,
    FireBowl = 18,
    Snow = 19,
    Explosion = 20,
    Ice = 21,
    LifestealBeam = 22,
    CultistFlame = 23,
    StaticSmoke = 24,
    Blood = 25,
    Enraged = 26,
    BigShrapnel = 27,
    Laser = 28,
}

impl ParticleMode {
    pub fn into_uint(self) -> u32 { self as u32 }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Instance {
    // created_at time, so we can calculate time relativity, needed for relative animation.
    // can save 32 bits per instance, for particles that are not relatively animated.
    inst_time: f32,

    // The lifespan in seconds of the particle
    inst_lifespan: f32,

    // a seed value for randomness
    // can save 32 bits per instance, for particles that don't need randomness/uniqueness.
    inst_entropy: f32,

    // modes should probably be seperate shaders, as a part of scaling and optimisation efforts.
    // can save 32 bits per instance, and have cleaner tailor made code.
    inst_mode: i32,

    // A direction for particles to move in
    inst_dir: [f32; 3],

    // a triangle is: f32 x 3 x 3 x 1  = 288 bits
    // a quad is:     f32 x 3 x 3 x 2  = 576 bits
    // a cube is:     f32 x 3 x 3 x 12 = 3456 bits
    // this vec is:   f32 x 3 x 1 x 1  = 96 bits (per instance!)
    // consider using a throw-away mesh and
    // positioning the vertex verticies instead,
    // if we have:
    // - a triangle mesh, and 3 or more instances.
    // - a quad mesh, and 6 or more instances.
    // - a cube mesh, and 36 or more instances.
    inst_pos: [f32; 3],
}

impl Instance {
    pub fn new(
        inst_time: f64,
        lifespan: f32,
        inst_mode: ParticleMode,
        inst_pos: Vec3<f32>,
    ) -> Self {
        use rand::Rng;
        Self {
            inst_time: inst_time as f32,
            inst_lifespan: lifespan,
            inst_entropy: rand::thread_rng().gen(),
            inst_mode: inst_mode as i32,
            inst_pos: inst_pos.into_array(),
            inst_dir: [0.0, 0.0, 0.0],
        }
    }

    pub fn new_directed(
        inst_time: f64,
        lifespan: f32,
        inst_mode: ParticleMode,
        inst_pos: Vec3<f32>,
        inst_pos2: Vec3<f32>,
    ) -> Self {
        use rand::Rng;
        Self {
            inst_time: inst_time as f32,
            inst_lifespan: lifespan,
            inst_entropy: rand::thread_rng().gen(),
            inst_mode: inst_mode as i32,
            inst_pos: inst_pos.into_array(),
            inst_dir: (inst_pos2 - inst_pos).into_array(),
        }
    }

    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        const ATTRIBUTES: [wgpu::VertexAttributeDescriptor; 6] = wgpu::vertex_attr_array![2 => Float, 3 => Float, 4 => Float, 5 => Int, 6 => Float3, 7 => Float3];
        wgpu::VertexBufferDescriptor {
            stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &ATTRIBUTES,
        }
    }
}

impl Default for Instance {
    fn default() -> Self { Self::new(0.0, 0.0, ParticleMode::CampfireSmoke, Vec3::zero()) }
}

pub struct ParticlePipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl ParticlePipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
        sc_desc: &wgpu::SwapChainDescriptor,
        global_layout: &GlobalsLayouts,
        aa_mode: AaMode,
    ) -> Self {
        common::span!(_guard, "ParticlePipeline::new");
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Particle pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[&global_layout.globals],
            });

        let samples = match aa_mode {
            AaMode::None | AaMode::Fxaa => 1,
            // TODO: Ensure sampling in the shader is exactly between the 4 texels
            AaMode::MsaaX4 => 4,
            AaMode::MsaaX8 => 8,
            AaMode::MsaaX16 => 16,
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Particle pipeline"),
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
                color_blend: wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha_blend: wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilStateDescriptor {
                    front: wgpu::StencilStateFaceDescriptor::IGNORE,
                    back: wgpu::StencilStateFaceDescriptor::IGNORE,
                    read_mask: !0,
                    write_mask: !0,
                },
            }),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: None,
                vertex_buffers: &[Vertex::desc(), Instance::desc()],
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
