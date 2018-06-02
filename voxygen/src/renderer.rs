use gfx;
use gfx::{Device, Encoder, handle::RenderTargetView, handle::DepthStencilView};
use gfx_device_gl;

use mesh;
use mesh::{Mesh, Vertex, Constants};
use vertex_buffer::VertexBuffer;
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
    voxel_pipeline: Pipeline<mesh::pipe::Init<'static>>,

    test_tri: VertexBuffer,
}

impl Renderer {
    pub fn new(device: gfx_device_gl::Device, mut factory: gfx_device_gl::Factory, color_view: ColorView, depth_view: DepthView) -> Renderer {

        let mut mesh = Mesh::new();
        mesh.add(&[
            Vertex { pos: [0., 1., 0.], norm: [0., 0., 1.], col: [1., 0., 0.] },
            Vertex { pos: [-1., -1., 0.], norm: [0., 0., 1.], col: [0., 1., 0.] },
            Vertex { pos: [1., -1., 0.], norm: [0., 0., 1.], col: [0., 0., 1.] },
        ]);

        Renderer {
            test_tri: VertexBuffer::new(&mut factory, &color_view, &mesh),

            device,
            color_view,
            depth_view,
            encoder: factory.create_command_buffer().into(),
            voxel_pipeline: Pipeline::new(
                &mut factory,
                mesh::pipe::new(),
                include_bytes!("../shaders/vert.glsl"),
                include_bytes!("../shaders/frag.glsl"),
            ),
            factory,
        }
    }

    pub fn factory_mut<'a>(&'a mut self) -> &'a mut gfx_device_gl::Factory {
        &mut self.factory
    }

    pub fn begin_frame(&mut self) {
        self.encoder.clear(&self.color_view, [0.3, 0.3, 0.6, 1.0]);
        self.encoder.clear_depth(&self.depth_view, 1.0);

        const CONSTANTS: Constants = Constants {
            trans: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ],
        };

        let data = self.test_tri.data();
        self.encoder.update_buffer(&data.constants, &[CONSTANTS], 0).unwrap();

        let slice = self.test_tri.slice();
        self.encoder.draw(&slice, self.voxel_pipeline.pso(), data);
    }

    pub fn end_frame(&mut self) {
        self.encoder.flush(&mut self.device);
        self.device.cleanup();
    }
}
