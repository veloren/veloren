use bytemuck::Pod;
use wgpu::util::DeviceExt;

pub struct Buffer<T: Copy + Pod> {
    pub buf: wgpu::Buffer,
    // bytes
    count: usize,
    phantom_data: std::marker::PhantomData<T>,
}

impl<T: Copy + Pod> Buffer<T> {
    pub fn new(device: &wgpu::Device, cap: u64, usage: wgpu::BufferUsage) -> Self {
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

    pub fn new_with_data(device: &wgpu::Device, usage: wgpu::BufferUsage, data: &[T]) -> Self {
        let contents = bytemuck::cast_slice(data);

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

    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vals: &[T], offset: u64) {
        if !vals.is_empty() {
            queue.write_buffer(&self.buf, offset, bytemuck::cast_slice(vals))
        }
    }

    pub fn count(&self) -> usize { self.count }
}
