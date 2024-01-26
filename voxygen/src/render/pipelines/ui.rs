use super::super::{Bound, Consts, GlobalsLayouts, Quad, Texture, Tri, Vertex as VertexTrait};
use bytemuck::{Pod, Zeroable};
use std::mem;
use vek::*;

/// The format of textures that the UI sources image data from.
///
/// Note, the is not directly used in all relevant locations, but still helps to
/// more clearly document the that this is the format being used. Notably,
/// textures are created via `renderer.create_dynamic_texture(...)` and
/// `renderer.create_texture(&DynamicImage::ImageRgba(image), ...)` (TODO:
/// update if we have to refactor when implementing the RENDER_ATTACHMENT
/// usage).
const UI_IMAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    center: [f32; 2],
    // Used calculating where to sample scaled images.
    scale: [f32; 2],
    mode: u32,
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
            0 => Float32x2, 1 => Float32x2, 2 => Float32x4,
            3 => Float32x2, 4 => Float32x2,    5 => Uint32,
        ];
        wgpu::VertexBufferLayout {
            array_stride: Self::STRIDE,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

impl VertexTrait for Vertex {
    const QUADS_INDEX: Option<wgpu::IndexFormat> = None;
    const STRIDE: wgpu::BufferAddress = mem::size_of::<Self>() as wgpu::BufferAddress;
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

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct TexLocals {
    texture_size: [u32; 2],
}

impl From<Vec2<u32>> for TexLocals {
    fn from(texture_size: Vec2<u32>) -> Self {
        Self {
            texture_size: texture_size.into_array(),
        }
    }
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

#[derive(Clone, Copy)]
pub enum Mode {
    Text,
    Image {
        scale: Vec2<f32>,
    },
    Geometry,
    /// Draw an image from the texture at `tex` in the fragment shader, with the
    /// source rectangle rotated to face north (TODO: detail on what "north"
    /// means here).
    ImageSourceNorth {
        scale: Vec2<f32>,
    },
    /// Draw an image from the texture at `tex` in the fragment shader, with the
    /// target rectangle rotated to face north. (TODO: detail on what "target"
    /// means)
    ImageTargetNorth {
        scale: Vec2<f32>,
    },
}

impl Mode {
    fn value(self) -> u32 {
        match self {
            Mode::Text => MODE_TEXT,
            Mode::Image { .. } => MODE_IMAGE,
            Mode::Geometry => MODE_GEOMETRY,
            Mode::ImageSourceNorth { .. } => MODE_IMAGE_SOURCE_NORTH,
            Mode::ImageTargetNorth { .. } => MODE_IMAGE_TARGET_NORTH,
        }
    }

    /// Gets the scaling of the displayed image compared to the source.
    fn scale(self) -> Vec2<f32> {
        match self {
            Mode::ImageSourceNorth { scale } | Mode::ImageTargetNorth { scale } => scale,
            Mode::Image { scale } => scale,
            Mode::Text | Mode::Geometry => Vec2::one(),
        }
    }
}

pub type BoundLocals = Bound<Consts<Locals>>;

pub struct TextureBindGroup {
    pub(in super::super) bind_group: wgpu::BindGroup,
}

pub struct UiLayout {
    locals: wgpu::BindGroupLayout,
    texture: wgpu::BindGroupLayout,
}

impl UiLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            locals: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // locals
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            }),
            texture: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // tex_locals
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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

    pub fn bind_texture(
        &self,
        device: &wgpu::Device,
        texture: &Texture,
        tex_locals: Consts<TexLocals>,
    ) -> TextureBindGroup {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.texture,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: tex_locals.buf().as_entire_binding(),
                },
            ],
        });

        TextureBindGroup { bind_group }
    }
}

pub struct UiPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl UiPipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
        surface_config: &wgpu::SurfaceConfiguration,
        global_layout: &GlobalsLayouts,
        layout: &UiLayout,
    ) -> Self {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Ui pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[&global_layout.globals, &layout.locals, &layout.texture],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UI pipeline"),
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
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: fs_module,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
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

pub fn create_quad(
    rect: Aabr<f32>,
    uv_rect: Aabr<f32>,
    color: Rgba<f32>,
    mode: Mode,
) -> Quad<Vertex> {
    create_quad_vert_gradient(rect, uv_rect, color, color, mode)
}

pub fn create_quad_vert_gradient(
    rect: Aabr<f32>,
    uv_rect: Aabr<f32>,
    top_color: Rgba<f32>,
    bottom_color: Rgba<f32>,
    mode: Mode,
) -> Quad<Vertex> {
    let top_color = top_color.into_array();
    let bottom_color = bottom_color.into_array();

    let center = if let Mode::ImageSourceNorth { .. } = mode {
        uv_rect.center().into_array()
    } else {
        rect.center().into_array()
    };
    let scale = mode.scale().into_array();
    let mode_val = mode.value();
    let v = |pos, uv, color| Vertex {
        pos,
        uv,
        center,
        color,
        scale,
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
    let scale = mode.scale().into_array();
    let mode_val = mode.value();
    let v = |pos, uv| Vertex {
        pos,
        uv,
        center,
        color: color.into_array(),
        scale,
        mode: mode_val,
    };
    Tri::new(
        v([tri[0][0], tri[0][1]], [uv_tri[0][0], uv_tri[0][1]]),
        v([tri[1][0], tri[1][1]], [uv_tri[1][0], uv_tri[1][1]]),
        v([tri[2][0], tri[2][1]], [uv_tri[2][0], uv_tri[2][1]]),
    )
}

// Premultiplying alpha on the GPU before placing images into the textures that
// will be sampled from in the UI pipeline.
//
// Steps:
//
// 1. Upload new image via `Device::create_texture_with_data`.
//
//    (NOTE: Initially considered: Creating a storage buffer to read from in the
//    shader via `Device::create_buffer_init`, with `MAP_WRITE` flag to avoid
//    staging buffer. However, with GPUs combining usages other than `COPY_SRC`
//    with `MAP_WRITE` may be less ideal. Plus, by copying into a texture first
//    we can get free srgb conversion when fetching colors from the texture. In
//    the future, we may want to branch based on the whether the GPU is
//    integrated and avoid this extra copy.)
//
// 2. Run render pipeline to multiply by alpha reading from this texture and
//    writing to the final texture (this can either be in an atlas or in an
//    independent texture if the image is over a certain size threshold).
//
//    (NOTE: Initially considered: using a compute pipeline and writing to the
//     final texture as a storage texture. However, the srgb format can't be
//     used with storage texture and there is not yet the capability to create
//     non-srgb views of srgb textures.)
//
// Info needed:
//
// * source texture (texture binding)
// * target texture (render attachment)
// * source image dimensions (push constant)
// * target texture dimensions (push constant)
// * position in the target texture (push constant)
//
// TODO: potential optimizations
// * what is the overhead of this draw call call? at some point we may be better
//   off converting very small images on the cpu and/or batching these into a
//   single draw call
// * what is the overhead of creating new small textures? for processing many
//   small images would it be useful to create a single texture the same size as
//   our cache texture and use Queue::write_texture?
// * is using create_buffer_init and reading directly from that (with manual
//   srgb conversion) worth avoiding staging buffer/copy-to-texture for
//   integrated GPUs?
// * premultipying alpha in a release asset preparation step

pub struct PremultiplyAlphaLayout {
    source_texture: wgpu::BindGroupLayout,
}

impl PremultiplyAlphaLayout {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            source_texture: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    // source_texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            }),
        }
    }
}

pub struct PremultiplyAlphaPipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl PremultiplyAlphaPipeline {
    pub fn new(
        device: &wgpu::Device,
        vs_module: &wgpu::ShaderModule,
        fs_module: &wgpu::ShaderModule,
        layout: &PremultiplyAlphaLayout,
    ) -> Self {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Premultiply alpha pipeline layout"),
            bind_group_layouts: &[&layout.source_texture],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::VERTEX,
                range: 0..core::mem::size_of::<PremultiplyAlphaParams>() as u32,
            }],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Premultiply alpha pipeline"),
            layout: Some(&pipeline_layout),
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
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: fs_module,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: UI_IMAGE_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        Self { pipeline }
    }
}

/// Uploaded as push constant.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct PremultiplyAlphaParams {
    /// Size of the source image.
    source_size_xy: u32,
    /// Offset to place the image at in the target texture.
    ///
    /// Origin is the top-left.
    target_offset_xy: u32,
    /// Size of the target texture.
    target_size_xy: u32,
}

/// An image upload that needs alpha premultiplication and which is in a pending
/// state.
///
/// From here we will use the `PremultiplyAlpha` pipeline to premultiply the
/// alpha while transfering the image to its destination texture.
pub(in super::super) struct PremultiplyUpload {
    source_bg: wgpu::BindGroup,
    source_size_xy: u32,
    /// The location in the final texture this will be placed at. Technically,
    /// we don't need this information at this point but it is convenient to
    /// store it here.
    offset: Vec2<u16>,
}

impl PremultiplyUpload {
    pub(in super::super) fn prepare(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &PremultiplyAlphaLayout,
        image: &image::RgbaImage,
        offset: Vec2<u16>,
    ) -> Self {
        // TODO: duplicating some code from `Texture` since:
        // 1. We don't need to create a sampler.
        // 2. Texture::new accepts &DynamicImage which isn't possible to create from
        //    &RgbaImage without cloning. (this might be addressed on zoomy worldgen
        //    branch)
        let image_size = wgpu::Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        };
        let source_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: image_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &source_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &(&**image)[..(image.width() as usize * image.height() as usize * 4)],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(image.width() * 4),
                rows_per_image: Some(image.height()),
            },
            image_size,
        );
        // Create view to use to create bind group
        let view = source_tex.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });
        let source_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout.source_texture,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });

        // NOTE: We assume the max texture size is less than u16::MAX.
        let source_size_xy = image_size.width + (image_size.height << 16);

        Self {
            source_bg,
            source_size_xy,
            offset,
        }
    }

    /// Semantically, this consumes the `PremultiplyUpload` but we need to keep
    /// the bind group alive to the end of the render pass and don't want to
    /// bother storing it somewhere else.
    pub(in super::super) fn draw_data(
        &self,
        target: &Texture,
    ) -> (&wgpu::BindGroup, PremultiplyAlphaParams) {
        let target_offset_xy = u32::from(self.offset.x) + (u32::from(self.offset.y) << 16);
        let target_dims = target.get_dimensions();
        // NOTE: We assume the max texture size is less than u16::MAX.
        let target_size_xy = target_dims.x + (target_dims.y << 16);
        (&self.source_bg, PremultiplyAlphaParams {
            source_size_xy: self.source_size_xy,
            target_offset_xy,
            target_size_xy,
        })
    }
}

use std::sync::Arc;
/// Per-target texture batched uploads
#[derive(Default)]
pub(in super::super) struct BatchedUploads {
    batches: Vec<(Arc<Texture>, Vec<PremultiplyUpload>)>,
}
#[derive(Default, Clone, Copy)]
pub struct UploadBatchId(usize);

impl BatchedUploads {
    /// Adds the provided upload to the batch indicated by the provided target
    /// texture and optional batch id. A new batch will be created if the batch
    /// id is invalid (doesn't refer to an existing batch) or the provided
    /// target texture isn't the same as the one associated with the
    /// provided batch id. Creating a new batch involves cloning the
    /// provided texture `Arc`.
    ///
    /// The id of the batch where the upload is ultimately submitted will be
    /// returned. This id can be used in subsequent calls to add items to
    /// the same batch (i.e. uploads for the same texture).
    ///
    /// Batch ids will reset every frame, however since we check that the
    /// texture matches, it is perfectly fine to use a stale id (just keep
    /// in mind that this will create a new batch). This also means that it is
    /// sufficient to use `UploadBatchId::default()` when calling this with
    /// new textures.
    pub(in super::super) fn submit(
        &mut self,
        target_texture: &Arc<Texture>,
        batch_id: UploadBatchId,
        upload: PremultiplyUpload,
    ) -> UploadBatchId {
        if let Some(batch) = self
            .batches
            .get_mut(batch_id.0)
            .filter(|b| Arc::ptr_eq(&b.0, target_texture))
        {
            batch.1.push(upload);
            batch_id
        } else {
            let new_batch_id = UploadBatchId(self.batches.len());
            self.batches
                .push((Arc::clone(target_texture), vec![upload]));
            new_batch_id
        }
    }

    pub(in super::super) fn take(&mut self) -> Vec<(Arc<Texture>, Vec<PremultiplyUpload>)> {
        core::mem::take(&mut self.batches)
    }
}
