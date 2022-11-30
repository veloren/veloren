use crate::{render::pipelines::rain_occlusion, scene::terrain::RAIN_OCCLUSION_CHUNKS};

use super::{
    super::{texture::Texture, RenderError, ShadowMapMode},
    Renderer,
};
use common::{terrain::TerrainChunkSize, vol::RectVolSize};
use vek::*;

/// A type that holds rain occlusion map data.  Since rain occlusion mapping may
/// not be supported on all platforms, we try to keep it separate.
pub struct RainOcclusionMapRenderer {
    pub depth: Texture,

    pub terrain_pipeline: rain_occlusion::RainOcclusionPipeline,
    pub figure_pipeline: rain_occlusion::RainOcclusionFigurePipeline,
    pub layout: rain_occlusion::RainOcclusionLayout,
}

pub enum RainOcclusionMap {
    Enabled(RainOcclusionMapRenderer),
    /// Dummy texture
    Disabled(Texture),
}

impl RainOcclusionMap {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        directed: Option<rain_occlusion::RainOcclusionPipeline>,
        figure: Option<rain_occlusion::RainOcclusionFigurePipeline>,
        view: Option<Texture>,
    ) -> Self {
        if let (Some(terrain_pipeline), Some(figure_pipeline), Some(depth)) =
            (directed, figure, view)
        {
            let layout = rain_occlusion::RainOcclusionLayout::new(device);

            Self::Enabled(RainOcclusionMapRenderer {
                depth,
                terrain_pipeline,
                figure_pipeline,
                layout,
            })
        } else {
            Self::Disabled(Self::create_dummy_tex(device, queue))
        }
    }

    fn create_dummy_tex(device: &wgpu::Device, queue: &wgpu::Queue) -> Texture {
        let tex = {
            let tex = wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: 4,
                    height: 4,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::RENDER_ATTACHMENT,
            };

            let view = wgpu::TextureViewDescriptor {
                label: None,
                format: Some(wgpu::TextureFormat::Depth24Plus),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::DepthOnly,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            };

            let sampler_info = wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual),
                ..Default::default()
            };

            Texture::new_raw(device, &tex, &view, &sampler_info)
        };

        // Clear to 1.0
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Dummy rain occlusion tex clearing encoder"),
        });

        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Clear dummy rain occlusion texture"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &tex.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        queue.submit(std::iter::once(encoder.finish()));

        tex
    }

    /// Create texture and view for rain ocllusion maps.
    /// Returns (point, directed)
    pub(super) fn create_view(
        device: &wgpu::Device,
        mode: &ShadowMapMode,
    ) -> Result<Texture, RenderError> {
        // (Attempt to) apply resolution factor to rain occlusion map resolution.
        let resolution_factor = mode.resolution.clamped(0.25, 4.0);

        let max_texture_size = Renderer::max_texture_size_raw(device);
        let size =
            (RAIN_OCCLUSION_CHUNKS as f32).sqrt().ceil() as u32 * TerrainChunkSize::RECT_SIZE * 2;

        // Limit to max texture size, rather than erroring.
        let size = size.map(|e| {
            let size = e as f32 * resolution_factor;
            // NOTE: We know 0 <= e since we clamped the resolution factor to be between
            // 0.25 and 4.0.
            if size <= max_texture_size as f32 {
                size as u32
            } else {
                max_texture_size
            }
        });

        let levels = 1;
        // Limit to max texture size rather than erroring.
        let two_size = size.map(|e| {
            u32::checked_next_power_of_two(e)
                .filter(|&e| e <= max_texture_size)
                .unwrap_or(max_texture_size)
        });
        let min_size = size.reduce_min();
        let max_size = size.reduce_max();
        let _min_two_size = two_size.reduce_min();
        let _max_two_size = two_size.reduce_max();
        // For rotated shadow maps, the maximum size of a pixel along any axis is the
        // size of a diagonal along that axis.
        let diag_size = size.map(f64::from).magnitude();
        let diag_cross_size = f64::from(min_size) / f64::from(max_size) * diag_size;
        let (diag_size, _diag_cross_size) =
            if 0.0 < diag_size && diag_size <= f64::from(max_texture_size) {
                // NOTE: diag_cross_size must be non-negative, since it is the ratio of a
                // non-negative and a positive number (if max_size were zero,
                // diag_size would be 0 too).  And it must be <= diag_size,
                // since min_size <= max_size.  Therefore, if diag_size fits in a
                // u16, so does diag_cross_size.
                (diag_size as u32, diag_cross_size as u32)
            } else {
                // Limit to max texture resolution rather than error.
                (max_texture_size, max_texture_size)
            };
        let diag_two_size = u32::checked_next_power_of_two(diag_size)
            .filter(|&e| e <= max_texture_size)
            // Limit to max texture resolution rather than error.
            .unwrap_or(max_texture_size)
            // Make sure we don't try to create a zero sized texture (divided by 4 below)
            .max(4);

        let rain_occlusion_tex = wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: diag_two_size,
                height: diag_two_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::RENDER_ATTACHMENT,
        };

        let rain_occlusion_view = wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Depth24Plus),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        };

        let sampler_info = wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        };

        let rain_occlusion_tex = Texture::new_raw(
            device,
            &rain_occlusion_tex,
            &rain_occlusion_view,
            &sampler_info,
        );

        Ok(rain_occlusion_tex)
    }

    pub fn texture(&self) -> &Texture {
        match self {
            Self::Enabled(renderer) => &renderer.depth,
            Self::Disabled(dummy) => dummy,
        }
    }
}
