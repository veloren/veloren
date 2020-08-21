use super::RenderError;
use wgpu::util::DeviceExt;
use zerocopy::AsBytes;

#[derive(Clone)]
pub struct Buffer<T: Copy + AsBytes> {
    pub buf: wgpu::Buffer,
    // bytes
    count: usize,
    phantom_data: std::marker::PhantomData<T>,
}

impl<T: Copy + AsBytes> Buffer<T> {
    pub fn new(device: &mut wgpu::Device, cap: usize, usage: wgpu::BufferUsage) -> Self {
        Self {
            buf: device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                mapped_at_creation: false,
                size: cap,
                usage: usage | wgpu::BufferUsage::MAP_WRITE,
            }),
            count: 0,
            phantom_data: std::marker::PhantomData,
        }
    }

    pub fn new_with_data(device: &mut wgpu::Device, usage: wgpu::BufferUsage, data: &[T]) -> Self {
        let contents = data.as_bytes();

        Self {
            buf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents,
                usage: usage | wgpu::BufferUsage::MAP_WRITE,
            }),
            count: data.len(),
            phantom_data: std::marker::PhantomData,
        }
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vals: &[T],
        offset: usize,
    ) {
        queue.write_buffer(&self.buf, offset, vals.as_bytes())
    }

    pub fn count(&self) -> usize { self.count }
}
