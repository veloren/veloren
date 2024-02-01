use super::super::pipelines::blit;
use common_base::prof_span;
use crossbeam_channel;
use tracing::error;

pub type ScreenshotFn = Box<dyn FnOnce(Result<image::RgbImage, String>) + Send>;

pub struct TakeScreenshot {
    bind_group: blit::BindGroup,
    view: wgpu::TextureView,
    texture: wgpu::Texture,
    buffer: wgpu::Buffer,
    screenshot_fn: ScreenshotFn,
    // Dimensions used for copying from the screenshot texture to a buffer
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
    // Texture format
    tex_format: wgpu::TextureFormat,
}

impl TakeScreenshot {
    pub fn new(
        device: &wgpu::Device,
        blit_layout: &blit::BlitLayout,
        sampler: &wgpu::Sampler,
        // Used to determine the resolution and texture format
        surface_config: &wgpu::SurfaceConfiguration,
        // Function that is given the image after downloading it from the GPU
        // This is executed in a background thread
        screenshot_fn: ScreenshotFn,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("screenshot tex"),
            size: wgpu::Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface_config.format,
            usage: wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("screenshot tex view"),
            format: Some(surface_config.format),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let bind_group = blit_layout.bind(device, &view, sampler);

        let bytes_per_pixel = surface_config.format.block_size(None).unwrap();
        let padded_bytes_per_row = padded_bytes_per_row(surface_config.width, bytes_per_pixel);

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screenshot download buffer"),
            size: (padded_bytes_per_row * surface_config.height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            bind_group,
            texture,
            view,
            buffer,
            screenshot_fn,
            width: surface_config.width,
            height: surface_config.height,
            bytes_per_pixel,
            tex_format: surface_config.format,
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
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &self.buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
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
            // Send buffer to another thread for async
            // mapping, downloading, and passing to the given handler function
            // (which probably saves it to the disk)
            std::thread::Builder::new()
                .name("screenshot".into())
                .spawn(move || {
                    self.download_and_handle_internal();
                })
                .expect("Failed to spawn screenshot thread");
        }
    }

    /// Don't call this from the main loop, it will block for a while
    fn download_and_handle_internal(self) {
        prof_span!("download_and_handle_internal");
        // Calculate padded bytes per row
        let padded_bytes_per_row = padded_bytes_per_row(self.width, self.bytes_per_pixel);

        // Map buffer
        let buffer = std::sync::Arc::new(self.buffer);
        let buffer2 = std::sync::Arc::clone(&buffer);
        let buffer_slice = buffer.slice(..);
        let (map_result_sender, map_result_receiver) = crossbeam_channel::bounded(1);
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            map_result_sender
                .send(result)
                .expect("seems like the receiver broke, which should not happen");
        });
        let result = match map_result_receiver.recv() {
            Ok(result) => result,
            Err(e) => {
                error!(
                    ?e,
                    "map_async never send the result for the screenshot mapping"
                );
                return;
            },
        };
        let padded_buffer;
        let buffer_slice = buffer2.slice(..);
        let rows = match result {
            Ok(()) => {
                // Copy to a Vec
                padded_buffer = buffer_slice.get_mapped_range();
                padded_buffer
                    .chunks(padded_bytes_per_row as usize)
                    .map(|padded_chunk| {
                        &padded_chunk[..self.width as usize * self.bytes_per_pixel as usize]
                    })
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

        // Note: we don't use bytes_per_pixel here since we expect only certain formats
        // below.
        let bytes_per_rgb = 3;
        let mut pixel_bytes =
            Vec::with_capacity(self.width as usize * self.height as usize * bytes_per_rgb);
        // Construct image
        let image = match self.tex_format {
            wgpu::TextureFormat::Bgra8UnormSrgb => {
                prof_span!("copy image");
                rows.for_each(|row| {
                    let (pixels, rest) = row.as_chunks();
                    assert!(
                        rest.is_empty(),
                        "Always valid because each pixel uses four bytes"
                    );
                    // Swap blue and red components and drop alpha to get a RGB texture.
                    for &[b, g, r, _a] in pixels {
                        pixel_bytes.extend_from_slice(&[r, g, b])
                    }
                });

                Ok(pixel_bytes)
            },
            wgpu::TextureFormat::Rgba8UnormSrgb => {
                prof_span!("copy image");
                rows.for_each(|row| {
                    let (pixels, rest) = row.as_chunks();
                    assert!(
                        rest.is_empty(),
                        "Always valid because each pixel uses four bytes"
                    );
                    // Drop alpha to get a RGB texture.
                    for &[r, g, b, _a] in pixels {
                        pixel_bytes.extend_from_slice(&[r, g, b])
                    }
                });

                Ok(pixel_bytes)
            },
            format => Err(format!(
                "Unhandled format for screenshot texture: {:?}",
                format,
            )),
        }
        .map(|pixel_bytes| {
            image::RgbImage::from_vec(self.width, self.height, pixel_bytes).expect(
                "Failed to create ImageBuffer! Buffer was not large enough. This should not occur",
            )
        });
        // Call supplied handler
        (self.screenshot_fn)(image);
    }
}

// Graphics API requires a specific alignment for buffer copies
fn padded_bytes_per_row(width: u32, bytes_per_pixel: u32) -> u32 {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let unpadded_bytes_per_row = width * bytes_per_pixel;
    let padding = (align - unpadded_bytes_per_row % align) % align;
    unpadded_bytes_per_row + padding
}
