use super::super::{pipelines::shadow, texture::Texture, RenderError, ShadowMapMode};
use vek::*;

/// A type that holds shadow map data.  Since shadow mapping may not be
/// supported on all platforms, we try to keep it separate.
pub struct ShadowMapRenderer {
    pub directed_depth: Texture,

    pub point_depth: Texture,

    pub point_pipeline: shadow::PointShadowPipeline,
    pub terrain_directed_pipeline: shadow::ShadowPipeline,
    pub figure_directed_pipeline: shadow::ShadowFigurePipeline,
    pub debug_directed_pipeline: shadow::ShadowDebugPipeline,
    pub layout: shadow::ShadowLayout,
}

pub enum ShadowMap {
    Enabled(ShadowMapRenderer),
    Disabled {
        dummy_point: Texture, // Cube texture
        dummy_directed: Texture,
    },
}

impl ShadowMap {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        point: Option<shadow::PointShadowPipeline>,
        directed: Option<shadow::ShadowPipeline>,
        figure: Option<shadow::ShadowFigurePipeline>,
        debug: Option<shadow::ShadowDebugPipeline>,
        shadow_views: Option<(Texture, Texture)>,
    ) -> Self {
        if let (
            Some(point_pipeline),
            Some(terrain_directed_pipeline),
            Some(figure_directed_pipeline),
            Some(debug_directed_pipeline),
            Some(shadow_views),
        ) = (point, directed, figure, debug, shadow_views)
        {
            let (point_depth, directed_depth) = shadow_views;

            let layout = shadow::ShadowLayout::new(device);

            Self::Enabled(ShadowMapRenderer {
                directed_depth,
                point_depth,

                point_pipeline,
                terrain_directed_pipeline,
                figure_directed_pipeline,
                debug_directed_pipeline,

                layout,
            })
        } else {
            let (dummy_point, dummy_directed) = Self::create_dummy_shadow_tex(device, queue);
            Self::Disabled {
                dummy_point,
                dummy_directed,
            }
        }
    }

    fn create_dummy_shadow_tex(device: &wgpu::Device, queue: &wgpu::Queue) -> (Texture, Texture) {
        let make_tex = |view_dim, depth| {
            let tex = wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: 4,
                    height: 4,
                    depth_or_array_layers: depth,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            };

            let view = wgpu::TextureViewDescriptor {
                label: None,
                format: Some(wgpu::TextureFormat::Depth24Plus),
                dimension: Some(view_dim),
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

        let cube_tex = make_tex(wgpu::TextureViewDimension::Cube, 6);
        let tex = make_tex(wgpu::TextureViewDimension::D2, 1);

        // Clear to 1.0
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Dummy shadow tex clearing encoder"),
        });
        let mut clear = |tex: &Texture| {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear dummy shadow texture"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &tex.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        };
        clear(&cube_tex);
        clear(&tex);
        #[allow(clippy::drop_non_drop)]
        drop(clear);
        queue.submit(std::iter::once(encoder.finish()));

        (cube_tex, tex)
    }

    /// Create textures and views for shadow maps.
    /// Returns (point, directed)
    pub(super) fn create_shadow_views(
        device: &wgpu::Device,
        size: (u32, u32),
        mode: &ShadowMapMode,
        max_texture_size: u32,
    ) -> Result<(Texture, Texture), RenderError> {
        // (Attempt to) apply resolution factor to shadow map resolution.
        let resolution_factor = mode.resolution.clamped(0.25, 4.0);

        // Limit to max texture size, rather than erroring.
        let size = Vec2::new(size.0, size.1).map(|e| {
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

        let point_shadow_tex = wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: diag_two_size / 4,
                height: diag_two_size / 4,
                depth_or_array_layers: 6,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };

        let point_shadow_view = wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Depth24Plus),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        };

        let directed_shadow_tex = wgpu::TextureDescriptor {
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
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };

        let directed_shadow_view = wgpu::TextureViewDescriptor {
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

        let point_shadow_tex =
            Texture::new_raw(device, &point_shadow_tex, &point_shadow_view, &sampler_info);
        let directed_shadow_tex = Texture::new_raw(
            device,
            &directed_shadow_tex,
            &directed_shadow_view,
            &sampler_info,
        );

        Ok((point_shadow_tex, directed_shadow_tex))
    }

    pub fn textures(&self) -> (&Texture, &Texture) {
        match self {
            Self::Enabled(renderer) => (&renderer.point_depth, &renderer.directed_depth),
            Self::Disabled {
                dummy_point,
                dummy_directed,
            } => (dummy_point, dummy_directed),
        }
    }

    pub fn is_enabled(&self) -> bool { matches!(self, Self::Enabled(_)) }
}
