use super::super::{AaMode, GlobalsLayouts, Quad, Tri};
use bytemuck::{Pod, Zeroable};
use vek::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    center: [f32; 2],
    mode: u32,
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        use std::mem;
        wgpu::VertexBufferDescriptor {
            stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float2, 1 => Float2, 2 => Float4, 3 => Float2, 4 => Uint],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Locals {
    pos: [f32; 4],
}

impl From<Vec4<f32>> for Locals {
    fn from(pos: Vec4<f32>) -> Self {
        Self {
            pos: pos.into_array(),
        }
    }
}

impl Default for Locals {
    fn default() -> Self { Self { pos: [0.0; 4] } }
}

/// Draw text from the text cache texture `tex` in the fragment shader.
pub const MODE_TEXT: u32 = 0;
/// Draw an image from the texture at `tex` in the fragment shader.
pub const MODE_IMAGE: u32 = 1;
/// Ignore `tex` and draw simple, colored 2D geometry.
pub const MODE_GEOMETRY: u32 = 2;
/// Draw an image from the texture at `tex` in the fragment shader, with the
/// source rectangle rotated to face north.
///
/// FIXME: Make more principled.
pub const MODE_IMAGE_SOURCE_NORTH: u32 = 3;
/// Draw an image from the texture at `tex` in the fragment shader, with the
/// target rectangle rotated to face north.
///
/// FIXME: Make more principled.
pub const MODE_IMAGE_TARGET_NORTH: u32 = 5;

pub enum Mode {
    Text,
    Image,
    Geometry,
    ImageSourceNorth,
    ImageTargetNorth,
}

impl Mode {
    fn value(self) -> u32 {
        match self {
            Mode::Text => MODE_TEXT,
            Mode::Image => MODE_IMAGE,
            Mode::Geometry => MODE_GEOMETRY,
            Mode::ImageSourceNorth => MODE_IMAGE_SOURCE_NORTH,
            Mode::ImageTargetNorth => MODE_IMAGE_TARGET_NORTH,
        }
    }
}

pub struct UILayout {
    pub locals: wgpu::BindGroupLayout,
}

impl UILayout {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            locals: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // locals
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::UniformBuffer {
                            dynamic: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            component_type: wgpu::TextureComponentType::Float,
                            dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                        count: None,
                    },
                ],
            }),
        }
    }
}

pub struct UIPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl UIPipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
        sc_desc: &wgpu::SwapChainDescriptor,
        global_layout: &GlobalsLayouts,
        layout: &UILayout,
        aa_mode: AaMode,
    ) -> Self {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("UI pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[&global_layout.globals, &layout.locals],
            });

        let samples = match aa_mode {
            AaMode::None | AaMode::Fxaa => 1,
            // TODO: Ensure sampling in the shader is exactly between the 4 texels
            AaMode::SsaaX4 => 1,
            AaMode::MsaaX4 => 4,
            AaMode::MsaaX8 => 8,
            AaMode::MsaaX16 => 16,
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UI pipeline"),
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
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilStateDescriptor {
                    front: wgpu::StencilStateFaceDescriptor::IGNORE,
                    back: wgpu::StencilStateFaceDescriptor::IGNORE,
                    read_mask: !0,
                    write_mask: !0,
                },
            }),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
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

pub fn create_quad(
    rect: Aabr<f32>,
    uv_rect: Aabr<f32>,
    color: Rgba<f32>,
    mode: Mode,
) -> Quad<Vertex> {
    create_quad_vert_gradient(rect, uv_rect, color, color, mode)
}

#[allow(clippy::many_single_char_names)]
pub fn create_quad_vert_gradient(
    rect: Aabr<f32>,
    uv_rect: Aabr<f32>,
    top_color: Rgba<f32>,
    bottom_color: Rgba<f32>,
    mode: Mode,
) -> Quad<Vertex> {
    let top_color = top_color.into_array();
    let bottom_color = bottom_color.into_array();

    let center = if let Mode::ImageSourceNorth = mode {
        uv_rect.center().into_array()
    } else {
        rect.center().into_array()
    };
    let mode_val = mode.value();
    let v = |pos, uv, color| Vertex {
        pos,
        uv,
        center,
        color,
        mode: mode_val,
    };
    let aabr_to_lbrt = |aabr: Aabr<f32>| (aabr.min.x, aabr.min.y, aabr.max.x, aabr.max.y);

    let (l, b, r, t) = aabr_to_lbrt(rect);
    let (uv_l, uv_b, uv_r, uv_t) = aabr_to_lbrt(uv_rect);

    match (uv_b > uv_t, uv_l > uv_r) {
        (true, true) => Quad::new(
            v([r, t], [uv_l, uv_b], top_color),
            v([l, t], [uv_l, uv_t], top_color),
            v([l, b], [uv_r, uv_t], bottom_color),
            v([r, b], [uv_r, uv_b], bottom_color),
        ),
        (false, false) => Quad::new(
            v([r, t], [uv_l, uv_b], top_color),
            v([l, t], [uv_l, uv_t], top_color),
            v([l, b], [uv_r, uv_t], bottom_color),
            v([r, b], [uv_r, uv_b], bottom_color),
        ),
        _ => Quad::new(
            v([r, t], [uv_r, uv_t], top_color),
            v([l, t], [uv_l, uv_t], top_color),
            v([l, b], [uv_l, uv_b], bottom_color),
            v([r, b], [uv_r, uv_b], bottom_color),
        ),
    }
}

pub fn create_tri(
    tri: [[f32; 2]; 3],
    uv_tri: [[f32; 2]; 3],
    color: Rgba<f32>,
    mode: Mode,
) -> Tri<Vertex> {
    let center = [0.0, 0.0];
    let mode_val = mode.value();
    let v = |pos, uv| Vertex {
        pos,
        uv,
        center,
        color: color.into_array(),
        mode: mode_val,
    };
    Tri::new(
        v([tri[0][0], tri[0][1]], [uv_tri[0][0], uv_tri[0][1]]),
        v([tri[1][0], tri[1][1]], [uv_tri[1][0], uv_tri[1][1]]),
        v([tri[2][0], tri[2][1]], [uv_tri[2][0], uv_tri[2][1]]),
    )
}
