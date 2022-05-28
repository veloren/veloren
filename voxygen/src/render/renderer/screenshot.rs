use super::super::pipelines::blit;
use tracing::error;

pub type ScreenshotFn = Box<dyn FnOnce(Result<image::DynamicImage, String>) + Send>;

pub struct TakeScreenshot {
    bind_group: blit::BindGroup,
    view: wgpu::TextureView,
    texture: wgpu::Texture,
    buffer: wgpu::Buffer,
    screenshot_fn: ScreenshotFn,
    // Dimensions used for copying from the screenshot texture to a buffer
    width: u32,
    height: u32,
    bytes_per_pixel: u8,
    // Texture format
    tex_format: wgpu::TextureFormat,
}

impl TakeScreenshot {
    pub fn new(
        device: &wgpu::Device,
        blit_layout: &blit::BlitLayout,
        sampler: &wgpu::Sampler,
        // Used to determine the resolution and texture format
        sc_desc: &wgpu::SwapChainDescriptor,
        // Function that is given the image after downloading it from the GPU
        // This is executed in a background thread
        screenshot_fn: ScreenshotFn,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("screenshot tex"),
            size: wgpu::Extent3d {
                width: sc_desc.width,
                height: sc_desc.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: sc_desc.format,
            usage: wgpu::TextureUsage::COPY_SRC
                | wgpu::TextureUsage::SAMPLED
                | wgpu::TextureUsage::RENDER_ATTACHMENT,
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("screenshot tex view"),
            format: Some(sc_desc.format),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let bind_group = blit_layout.bind(device, &view, sampler);

        let bytes_per_pixel = sc_desc.format.describe().block_size;
        let padded_bytes_per_row = padded_bytes_per_row(sc_desc.width, bytes_per_pixel);

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screenshot download buffer"),
            size: (padded_bytes_per_row * sc_desc.height) as u64,
            usage: wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            bind_group,
            texture,
            view,
            buffer,
            screenshot_fn,
            width: sc_desc.width,
            height: sc_desc.height,
            bytes_per_pixel,
            tex_format: sc_desc.format,
        }
    }

    /// Get the texture view for the screenshot
    /// This can then be used as a render attachment
    pub fn texture_view(&self) -> &wgpu::TextureView { &self.view }

    /// Get the bind group used for blitting the screenshot to the current
    /// swapchain image
    pub fn bind_group(&self) -> &wgpu::BindGroup { &self.bind_group.bind_group }

    /// Call this after rendering to the screenshot texture
    ///
    /// Issues a command to copy from the texture to a buffer and returns a
    /// closure that needs to be called after submitting the encoder
    /// to the queue. When called, the closure will spawn a new thread for
    /// async mapping of the buffer and downloading of the screenshot.
    pub fn copy_to_buffer(self, encoder: &mut wgpu::CommandEncoder) -> impl FnOnce() {
        // Calculate padded bytes per row
        let padded_bytes_per_row = padded_bytes_per_row(self.width, self.bytes_per_pixel);
        // Copy image to a buffer
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &self.buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: core::num::NonZeroU32::new(padded_bytes_per_row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        move || {
            // Send buffer to another thread for async mapping, downloading, and passing to
            // the given handler function (which probably saves it to the disk)
            std::thread::Builder::new()
                .name("screenshot".into())
                .spawn(move || {
                    self.download_and_handle_internal();
                })
                .expect("Failed to spawn screenshot thread");
        }
    }

    fn download_and_handle_internal(self) {
        // Calculate padded bytes per row
        let padded_bytes_per_row = padded_bytes_per_row(self.width, self.bytes_per_pixel);
        let singlethread_rt = match tokio::runtime::Builder::new_current_thread().build() {
            Ok(rt) => rt,
            Err(err) => {
                error!(?err, "Could not create tokio runtime");
                return;
            },
        };

        // Map buffer
        let buffer_slice = self.buffer.slice(..);
        let buffer_map_future = buffer_slice.map_async(wgpu::MapMode::Read);

        // Wait on buffer mapping
        let mut pixel_bytes = match singlethread_rt.block_on(buffer_map_future) {
            // Buffer is mapped and we can read it
            Ok(()) => {
                // Copy to a Vec
                let padded_buffer = buffer_slice.get_mapped_range();
                let mut pixel_bytes = Vec::new();
                padded_buffer
                    .chunks(padded_bytes_per_row as usize)
                    .map(|padded_chunk| {
                        &padded_chunk[..self.width as usize * self.bytes_per_pixel as usize]
                    })
                    .for_each(|row| pixel_bytes.extend_from_slice(row));
                pixel_bytes
            },
            // Error
            Err(err) => {
                error!(
                    ?err,
                    "Failed to map buffer for downloading a screenshot from the GPU"
                );
                return;
            },
        };

        // Construct image
        let image = match self.tex_format {
            wgpu::TextureFormat::Bgra8UnormSrgb => {
                let (pixels, rest) = pixel_bytes.as_chunks_mut();
                assert!(
                    rest.is_empty(),
                    "Always valid because each pixel uses four bytes"
                );
                // Swap blue and red components to get a RGBA texture.
                for [b, _g, r, _a] in pixels {
                    std::mem::swap(b, r);
                }
                Ok(pixel_bytes)
            },
            wgpu::TextureFormat::Rgba8UnormSrgb => Ok(pixel_bytes),
            format => Err(format!(
                "Unhandled format for screenshot texture: {:?}",
                format,
            )),
        }
        .map(|pixel_bytes| {
            let image = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_vec(
                self.width,
                self.height,
                pixel_bytes,
            )
            .expect(
                "Failed to create ImageBuffer! Buffer was not large enough. This should not occur",
            );
            image::DynamicImage::ImageRgba8(image)
        });

        // Call supplied handler
        (self.screenshot_fn)(image);
    }
}

// Graphics API requires a specific alignment for buffer copies
fn padded_bytes_per_row(width: u32, bytes_per_pixel: u8) -> u32 {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let unpadded_bytes_per_row = width * bytes_per_pixel as u32;
    let padding = (align - unpadded_bytes_per_row % align) % align;
    unpadded_bytes_per_row + padding
}
