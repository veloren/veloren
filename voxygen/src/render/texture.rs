use super::RenderError;
use image::{DynamicImage, GenericImageView};
use vek::Vec2;
use wgpu::{util::DeviceExt, Extent3d};

/// Represents an image that has been uploaded to the GPU.
pub struct Texture {
    pub tex: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    size: Extent3d,
}

impl Texture {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &DynamicImage,
        filter_method: Option<wgpu::FilterMode>,
        addresse_mode: Option<wgpu::AddressMode>,
    ) -> Result<Self, RenderError> {
        // TODO: Actualy handle images that aren't in rgba format properly.
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
            usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
        });

        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        queue.write_texture(
            wgpu::TextureCopyViewBase {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &[buffer.as_slice()],
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
            address_mode_u: addresse_mode.unwrap_or(wgpu::AddressMode::ClampToEdge),
            address_mode_v: addresse_mode.unwrap_or(wgpu::AddressMode::ClampToEdge),
            address_mode_w: addresse_mode.unwrap_or(wgpu::AddressMode::ClampToEdge),
            mag_filter: filter_method.unwrap_or(wgpu::FilterMode::Nearest),
            min_filter: filter_method.unwrap_or(wgpu::FilterMode::Nearest),
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        };

        Ok(Self {
            tex,
            sampler: device.create_sampler(&sampler_info),
            size,
        })
    }

    pub fn new_dynamic(device: &wgpu::Device, width: u16, height: u16) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth: 1,
        };

        let tex_info = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
        });

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

        Self::new_raw(device, tex_info, sampler_info)
    }

    pub fn new_raw(
        device: &wgpu::Device,
        texture_info: wgpu::TextureDescriptor,
        sampler_info: wgpu::SamplerDescriptor,
    ) -> Self {
        Ok(Self {
            tex: device.create_texture(&texture_info),
            sampler: device.create_sampler(&sampler_info),
            size: texture_info.size,
        })
    }

    /// Update a texture with the given data (used for updating the glyph cache
    /// texture).
    pub fn update(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        offset: [u16; 2],
        size: [u16; 2],
        data: &[u8],
    ) -> Result<(), RenderError> {
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
            // TODO: I heard some rumors that there are other
            // formats that are not Rgba8
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: self.size.x * 4,
                rows_per_image: self.size.y,
            },
            wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth: 1,
            },
        );
    }

    /// Get dimensions of the represented image.
    pub fn get_dimensions(&self) -> Extent3d { self.size }
}
