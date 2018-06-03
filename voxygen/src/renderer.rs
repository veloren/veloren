use gfx;
use gfx::{Device, Encoder, handle::RenderTargetView, handle::DepthStencilView};
use gfx_device_gl;

use vertex_buffer;
use vertex_buffer::{VertexBuffer, Constants};
use pipeline::Pipeline;

pub type ColorFormat = gfx::format::Srgba8;
pub type DepthFormat = gfx::format::DepthStencil;

pub type ColorView = RenderTargetView<gfx_device_gl::Resources, ColorFormat>;
pub type DepthView = DepthStencilView<gfx_device_gl::Resources, DepthFormat>;

pub struct Renderer {
    device: gfx_device_gl::Device,
    color_view: ColorView,
    depth_view: DepthView,
    factory: gfx_device_gl::Factory,
    encoder: Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    voxel_pipeline: Pipeline<vertex_buffer::pipe::Init<'static>>,
}

impl Renderer {
    pub fn new(device: gfx_device_gl::Device, mut factory: gfx_device_gl::Factory, color_view: ColorView, depth_view: DepthView) -> Renderer {

        Renderer {
            device,
            color_view,
            depth_view,
            encoder: factory.create_command_buffer().into(),
            voxel_pipeline: Pipeline::new(
                &mut factory,
                vertex_buffer::pipe::new(),
                include_bytes!("../shaders/vert.glsl"),
                include_bytes!("../shaders/frag.glsl"),
            ),
            factory,
        }
    }

    pub fn factory_mut<'a>(&'a mut self) -> &'a mut gfx_device_gl::Factory {
        &mut self.factory
    }

    pub fn color_view<'a>(&'a self) -> &'a ColorView {
        &self.color_view
    }

    pub fn begin_frame(&mut self) {
        self.encoder.clear(&self.color_view, [0.3, 0.3, 0.6, 1.0]);
        self.encoder.clear_depth(&self.depth_view, 1.0);
    }

    // TODO: Make this accept a VoxelModel, not a VertexBuffer
    pub fn render_vertex_buffer(&mut self, vbuf: &VertexBuffer, constants: Constants) {
        self.encoder.update_buffer(&vbuf.data().constants, &[constants], 0).unwrap();
        self.encoder.draw(&vbuf.slice(), self.voxel_pipeline.pso(), vbuf.data());
    }

    pub fn end_frame(&mut self) {
        self.encoder.flush(&mut self.device);
        self.device.cleanup();
    }
}
