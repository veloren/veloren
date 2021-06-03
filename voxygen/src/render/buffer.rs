use bytemuck::Pod;
use wgpu::util::DeviceExt;

pub struct Buffer<T: Copy + Pod> {
    pub(super) buf: wgpu::Buffer,
    // Size in number of elements
    // TODO: determine if this is a good name
    len: usize,
    phantom_data: std::marker::PhantomData<T>,
}

impl<T: Copy + Pod> Buffer<T> {
    pub fn new(device: &wgpu::Device, usage: wgpu::BufferUsage, data: &[T]) -> Self {
        let contents = bytemuck::cast_slice(data);

        Self {
            buf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents,
                usage,
            }),
            len: data.len(),
            phantom_data: std::marker::PhantomData,
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize { self.len }
}

pub struct DynamicBuffer<T: Copy + Pod>(Buffer<T>);

impl<T: Copy + Pod> DynamicBuffer<T> {
    pub fn new(device: &wgpu::Device, len: usize, usage: wgpu::BufferUsage) -> Self {
        let buffer = Buffer {
            buf: device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                mapped_at_creation: false,
                size: len as u64 * std::mem::size_of::<T>() as u64,
                usage: usage | wgpu::BufferUsage::COPY_DST,
            }),
            len,
            phantom_data: std::marker::PhantomData,
        };
        Self(buffer)
    }

    pub fn update(&self, queue: &wgpu::Queue, vals: &[T], offset: usize) {
        if !vals.is_empty() {
            queue.write_buffer(
                &self.buf,
                offset as u64 * std::mem::size_of::<T>() as u64,
                bytemuck::cast_slice(vals),
            )
        }
    }
}

impl<T: Copy + Pod> std::ops::Deref for DynamicBuffer<T> {
    type Target = Buffer<T>;

    fn deref(&self) -> &Self::Target { &self.0 }
}
