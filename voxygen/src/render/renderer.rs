// Library
use vek::*;
use winit;
use lazy_static::lazy_static;
use rendy::{
    hal,
    command::RenderPassInlineEncoder,
    wsi::Surface,
    graph::{
        Graph,
        GraphBuilder,
        NodeBuffer,
        NodeImage,
        present::PresentNode,
        render::RenderPass,
    },
    factory::{Config, Factory},
    memory::MemoryUsageValue,
    resource::buffer::Buffer,
    mesh::AsVertex,
    shader::{Shader, ShaderKind, SourceLanguage, StaticShaderInfo},
};

// Crate
use crate::VoxygenErr;

// Local
use super::{
    model::Model,
    mesh::Mesh,
    shader_set::ShaderSet,
    Pipeline,
    RenderErr,
    Backend,
};

lazy_static! {
    static ref VS: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/test/shader.vert"),
        ShaderKind::Vertex,
        SourceLanguage::GLSL,
        "main",
    );

    static ref FS: StaticShaderInfo = StaticShaderInfo::new(
        concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/test/shader.frag"),
        ShaderKind::Fragment,
        SourceLanguage::GLSL,
        "main",
    );
}

#[derive(Debug)]
struct TriangleRenderPass<B: hal::Backend> {
    vertex: Option<Buffer<B>>,
}

impl<B: hal::Backend, T: ?Sized> RenderPass<B, T> for TriangleRenderPass<B> {
    fn name() -> &'static str { "triangle" }

    fn vertices() -> Vec<(
        Vec<hal::pso::Element<hal::format::Format>>,
        hal::pso::ElemStride,
    )> { vec![rendy::mesh::PosColor::VERTEX.gfx_vertex_input_desc()] }

    fn load_shader_sets<'a>(
        storage: &'a mut Vec<B::ShaderModule>,
        factory: &mut Factory<B>,
        _aux: &mut T,
    ) -> Vec<hal::pso::GraphicsShaderSet<'a, B>> {
        storage.clear();
        storage.push(VS.module(factory).unwrap());
        storage.push(FS.module(factory).unwrap());

        vec![hal::pso::GraphicsShaderSet {
            vertex: hal::pso::EntryPoint {
                entry: "main",
                module: &storage[0],
                specialization: hal::pso::Specialization::default(),
            },
            fragment: Some(hal::pso::EntryPoint {
                entry: "main",
                module: &storage[1],
                specialization: hal::pso::Specialization::default(),
            }),
            hull: None,
            domain: None,
            geometry: None,
        }]
    }

    fn build<'a>(
        _factory: &mut Factory<B>,
        _aux: &mut T,
        buffers: &mut [NodeBuffer<'a, B>],
        images: &mut [NodeImage<'a, B>],
        sets: &[impl AsRef<[B::DescriptorSetLayout]>],
    ) -> Self {
        assert!(buffers.is_empty());
        assert!(images.is_empty());
        assert_eq!(sets.len(), 1);
        assert!(sets[0].as_ref().is_empty());

        Self {
            vertex: None,
        }
    }

    fn prepare(&mut self, factory: &mut Factory<B>, _aux: &T) -> bool {
        if self.vertex.is_some() {
            return false;
        } else {
            let mut vbuf = factory.create_buffer(
                512,
                rendy::mesh::PosColor::VERTEX.stride as u64 * 3,
                (hal::buffer::Usage::VERTEX, MemoryUsageValue::Dynamic)
            ).unwrap();

            unsafe {
                factory.upload_visible_buffer(&mut vbuf, 0, &[
                    rendy::mesh::PosColor {
                        position: [0.0, -1.0, 0.0].into(),
                        color: [1.0, 0.0, 0.0, 1.0].into(),
                    },
                    rendy::mesh::PosColor {
                        position: [1.0, 1.0, 0.0].into(),
                        color: [0.0, 1.0, 0.0, 1.0].into(),
                    },
                    rendy::mesh::PosColor {
                        position: [-1.0, 1.0, 0.0].into(),
                        color: [0.0, 0.0, 1.0, 1.0].into(),
                    },
                ]).unwrap();
            }

            self.vertex = Some(vbuf);

            true
        }
    }

    fn draw(
        &mut self,
        _layouts: &[B::PipelineLayout],
        pipelines: &[B::GraphicsPipeline],
        mut encoder: RenderPassInlineEncoder<'_, B>,
        _index: usize,
        _aux: &T,
    ) {
        let vbuf = self.vertex.as_ref().unwrap();
        encoder.bind_graphics_pipeline(&pipelines[0]);
        encoder.bind_vertex_buffers(0, Some((vbuf.raw(), 0)));
        encoder.draw(0..3, 0..1);
    }

    fn dispose(self, _factory: &mut Factory<B>, _aux: &mut T) {}
}

pub struct Renderer {
    //surface: Surface<Backend>,
    graph: Graph<Backend, ()>,
    factory: Factory<Backend>,
}

impl Renderer {
    pub fn new(
        window: winit::Window,
    ) -> Result<Self, VoxygenErr> {
        let config: Config = Config::default();

        let mut factory = Factory::<Backend>::new(config)
            .map_err(|err| VoxygenErr::Other(err))?;

        let surface = factory.create_surface(window);

        let mut graph_builder = GraphBuilder::<Backend, ()>::new();

        let color_img = graph_builder.create_image(
            surface.kind(),
            1,
            hal::format::Format::Rgba8Unorm,
            MemoryUsageValue::Data,
            Some(hal::command::ClearValue::Color([1.0; 4].into())),
        );

        graph_builder.add_node(
            TriangleRenderPass::builder()
                .with_image(color_img)
        );

        graph_builder.add_node(
            PresentNode::builder(surface)
                .with_image(color_img)
        );

        let graph = graph_builder.build(&mut factory, &mut ())
            .map_err(|err| VoxygenErr::Other(err))?;

        Ok(Self {
            graph,
            factory,
        })
    }

    pub fn clear(&mut self, col: Rgba<f32>) {
    }

    pub fn flush(&mut self) {
        self.graph.run(&mut self.factory, &mut ());
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        //self.graph.dispose(&mut self.factory, &mut ());
        //self.factory.dispose();
    }
}
