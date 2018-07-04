// Ui
use conrod::backend::gfx::Renderer as ui_renderer;
use ui::{
    UI,
    ShaderResourceView,
    ui_resources,
    ImageMap,
};

use gfx;
use gfx::{Device, Encoder, handle::RenderTargetView, handle::DepthStencilView};
use gfx_device_gl;

use model_object;
use model_object::{ModelObject, Constants};
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
    model_pipeline: Pipeline<model_object::pipe::Init<'static>>,
    ui_renderer: ui_renderer<'static, ui_resources>,
}

impl Renderer {
    pub fn new(device: gfx_device_gl::Device, mut factory: gfx_device_gl::Factory, color_view: ColorView, depth_view: DepthView) -> Renderer {
        let ui_renderer = ui_renderer::new(&mut factory, &color_view, 1.0).unwrap();

        Renderer {
            device,
            color_view,
            depth_view,
            encoder: factory.create_command_buffer().into(),
            model_pipeline: Pipeline::new(
                &mut factory,
                model_object::pipe::new(),
                include_bytes!("../shaders/vert.glsl"),
                include_bytes!("../shaders/frag.glsl"),
            ),
            factory,
            ui_renderer,
        }
    }

    pub fn render_ui(&mut self, ui: &UI, window_size: &[f64; 2]) {
        let primitives = ui.get_primitives();
        let image_map = ui.get_image_map();

        self.ui_renderer.fill(&mut self.encoder, (window_size[0] as f32, window_size[1] as f32), primitives, &image_map);
        self.ui_renderer.draw(&mut self.factory, &mut self.encoder, image_map);
    }

    pub fn factory_mut<'a>(&'a mut self) -> &'a mut gfx_device_gl::Factory {
        &mut self.factory
    }

    pub fn color_view<'a>(&'a self) -> &'a ColorView {
        &self.color_view
    }

    pub fn depth_view<'a>(&'a self) -> &'a DepthView {
        &self.depth_view
    }

    pub fn set_views<'a>(&'a mut self, cv: ColorView, dv: DepthView) {
        self.color_view = cv;
        self.depth_view = dv;
    }

    pub fn begin_frame(&mut self) {
        self.encoder.clear(&self.color_view, [0.3, 0.3, 0.6, 1.0]);
        self.encoder.clear_depth(&self.depth_view, 1.0);
    }

    pub fn update_model_object(&mut self, mo: &ModelObject, constants: Constants) {
        self.encoder.update_buffer(mo.constants(), &[constants], 0).unwrap();
    }

    pub fn render_model_object(&mut self, mo: &ModelObject) {
        let pipeline_data = mo.get_pipeline_data(self);
        self.encoder.draw(&mo.slice(), self.model_pipeline.pso(), &pipeline_data);
    }

    pub fn end_frame(&mut self) {
        self.encoder.flush(&mut self.device);
        self.device.cleanup();
    }
}
