use super::RenderError;
use image::{DynamicImage, GenericImageView};
use wgpu::Extent3d;

/// Represents an image that has been uploaded to the GPU.
pub struct Texture {
    pub tex: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    size: Extent3d,
}

impl Texture {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &DynamicImage,
        filter_method: Option<wgpu::FilterMode>,
        address_mode: Option<wgpu::AddressMode>,
    ) -> Result<Self, RenderError> {
        // TODO: Actually handle images that aren't in rgba format properly.
        let buffer = image.as_flat_samples_u8().ok_or_else(|| {
            RenderError::CustomError(
                "We currently do not support color formats using more than 4 bytes / pixel.".into(),
            )
        })?;

        let size = Extent3d {
            width: image.width(),
            height: image.height(),
            depth: 1,
        };

        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });

        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        queue.write_texture(
            wgpu::TextureCopyViewBase {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            buffer.as_slice(),
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: image.width() * 4,
                rows_per_image: image.height(),
            },
            wgpu::Extent3d {
                width: image.width(),
                height: image.height(),
                depth: 1,
            },
        );

        let sampler_info = wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: address_mode.unwrap_or(wgpu::AddressMode::ClampToEdge),
            address_mode_v: address_mode.unwrap_or(wgpu::AddressMode::ClampToEdge),
            address_mode_w: address_mode.unwrap_or(wgpu::AddressMode::ClampToEdge),
            mag_filter: filter_method.unwrap_or(wgpu::FilterMode::Nearest),
            min_filter: filter_method.unwrap_or(wgpu::FilterMode::Nearest),
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        };

        let view = tex.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        Ok(Self {
            tex,
            view,
            sampler: device.create_sampler(&sampler_info),
            size,
        })
    }

    pub fn new_dynamic(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth: 1,
        };

        let tex_info = wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // TODO: nondynamic version doesn't seeem to have different usage, unify code?
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        };

        let sampler_info = wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        };

        let view_info = wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        };

        Self::new_raw(device, &tex_info, &view_info, &sampler_info)
    }

    pub fn new_raw(
        device: &wgpu::Device,
        texture_info: &wgpu::TextureDescriptor,
        view_info: &wgpu::TextureViewDescriptor,
        sampler_info: &wgpu::SamplerDescriptor,
    ) -> Self {
        let tex = device.create_texture(texture_info);
        let view = tex.create_view(view_info);

        Self {
            tex,
            view,
            sampler: device.create_sampler(sampler_info),
            size: texture_info.size,
        }
    }

    /// Update a texture with the given data (used for updating the glyph cache
    /// texture).
    pub fn update(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        offset: [u32; 2],
        size: [u32; 2],
        data: &[u8],
    ) {
        // Note: we only accept 4 bytes per pixel
        // (enforce this is API?)
        debug_assert_eq!(data.len(), size[0] as usize * size[1] as usize * 4);
        // TODO: Only works for 2D images
        queue.write_texture(
            wgpu::TextureCopyViewBase {
                texture: &self.tex,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: offset[0],
                    y: offset[1],
                    z: 0,
                },
            },
            data,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: size[0] * 4,
                rows_per_image: size[1],
            },
            wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth: 1,
            },
        );
    }

    /// Get dimensions of the represented image.
    pub fn get_dimensions(&self) -> vek::Vec3<u32> {
        vek::Vec3::new(self.size.width, self.size.height, self.size.depth)
    }
}
