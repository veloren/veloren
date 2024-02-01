use super::RenderError;
use image::DynamicImage;
use wgpu::Extent3d;

/// Represents an image that has been uploaded to the GPU.
pub struct Texture {
    pub tex: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    size: Extent3d,
    /// TODO: consider making Texture generic over the format
    format: wgpu::TextureFormat,
}

impl Texture {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &DynamicImage,
        filter_method: Option<wgpu::FilterMode>,
        address_mode: Option<wgpu::AddressMode>,
    ) -> Result<Self, RenderError> {
        let format = match image {
            DynamicImage::ImageLuma8(_) => wgpu::TextureFormat::R8Unorm,
            DynamicImage::ImageLumaA8(_) => panic!("ImageLuma8 unsupported"),
            DynamicImage::ImageRgb8(_) => panic!("ImageRgb8 unsupported"),
            DynamicImage::ImageRgba8(_) => wgpu::TextureFormat::Rgba8UnormSrgb,
            DynamicImage::ImageLuma16(_) => panic!("ImageLuma16 unsupported"),
            DynamicImage::ImageLumaA16(_) => panic!("ImageLumaA16 unsupported"),
            DynamicImage::ImageRgb16(_) => panic!("ImageRgb16 unsupported"),
            DynamicImage::ImageRgba16(_) => panic!("ImageRgba16 unsupported"),
            _ => panic!("unsupported format"),
        };

        // TODO: Actually handle images that aren't in rgba format properly.
        let buffer = image.as_flat_samples_u8().ok_or_else(|| {
            RenderError::CustomError(
                "We currently do not support color formats using more than 4 bytes / pixel.".into(),
            )
        })?;

        let bytes_per_pixel = u32::from(buffer.layout.channels);

        let size = Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        };

        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            buffer.as_slice(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(image.width() * bytes_per_pixel),
                rows_per_image: Some(image.height()),
            },
            Extent3d {
                width: image.width(),
                height: image.height(),
                depth_or_array_layers: 1,
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
            format: Some(format),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        Ok(Self {
            tex,
            view,
            sampler: device.create_sampler(&sampler_info),
            size,
            format,
        })
    }

    pub fn new_dynamic(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
    ) -> Self {
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let tex_info = wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // TODO: nondynamic version doesn't seeem to have different usage, unify code?
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        };

        let sampler_info = wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        };

        let view_info = wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        };

        let texture = Self::new_raw(device, &tex_info, &view_info, &sampler_info);
        texture.clear(queue); // Needs to be fully initialized for partial writes to work on Dx12 AMD
        texture
    }

    /// Note: the user is responsible for making sure the texture is fully
    /// initialized before doing partial writes on Dx12 AMD: https://github.com/gfx-rs/wgpu/issues/1306
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
            format: texture_info.format,
        }
    }

    /// Clears the texture data to 0
    pub fn clear(&self, queue: &wgpu::Queue) {
        let size = self.size;
        let byte_len = size.width as usize
            * size.height as usize
            * size.depth_or_array_layers as usize
            * self.format.block_size(None).unwrap() as usize;
        let zeros = vec![0; byte_len];

        self.update(queue, [0, 0], [size.width, size.height], &zeros);
    }

    /// Update a texture with the given data (used for updating the glyph cache
    /// texture).
    pub fn update(&self, queue: &wgpu::Queue, offset: [u32; 2], size: [u32; 2], data: &[u8]) {
        let bytes_per_pixel = self.format.block_size(None).unwrap();

        debug_assert_eq!(
            data.len(),
            size[0] as usize * size[1] as usize * bytes_per_pixel as usize
        );
        // TODO: Only works for 2D images
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.tex,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: offset[0],
                    y: offset[1],
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(size[0] * bytes_per_pixel),
                rows_per_image: Some(size[1]),
            },
            Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
        );
    }

    // TODO: remove `get` from this name
    /// Get dimensions of the represented image.
    pub fn get_dimensions(&self) -> vek::Vec3<u32> {
        vek::Vec3::new(
            self.size.width,
            self.size.height,
            self.size.depth_or_array_layers,
        )
    }
}
