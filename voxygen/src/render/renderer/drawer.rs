use super::{
    super::{
        buffer::Buffer,
        consts::Consts,
        instances::Instances,
        model::{DynamicModel, Model, SubModel},
        pipelines::{
            clouds, figure, fluid, lod_terrain, particle, postprocess, skybox, sprite, terrain, ui,
            ColLights, GlobalsBindGroup, Light, Shadow,
        },
    },
    Renderer,
};
use std::ops::Range;
use vek::Aabr;

pub struct Drawer<'a> {
    encoder: Option<wgpu::CommandEncoder>,
    renderer: &'a mut Renderer,
    tex: wgpu::SwapChainTexture,
    globals: &'a GlobalsBindGroup,
}

impl<'a> Drawer<'a> {
    pub fn new(
        encoder: wgpu::CommandEncoder,
        renderer: &'a mut Renderer,
        tex: wgpu::SwapChainTexture,
        globals: &'a GlobalsBindGroup,
    ) -> Self {
        Self {
            encoder: Some(encoder),
            renderer,
            tex,
            globals,
        }
    }

    pub fn first_pass(&mut self) -> FirstPassDrawer {
        let mut render_pass =
            self.encoder
                .as_mut()
                .unwrap()
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &self.renderer.tgt_color_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: Some(
                        wgpu::RenderPassDepthStencilAttachmentDescriptor {
                            attachment: &self.renderer.tgt_depth_view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: true,
                            }),
                            stencil_ops: None,
                        },
                    ),
                });

        render_pass.set_bind_group(0, &self.globals.bind_group, &[]);

        FirstPassDrawer {
            render_pass,
            renderer: &self.renderer,
        }
    }

    pub fn second_pass(&mut self) -> SecondPassDrawer {
        let mut render_pass =
            self.encoder
                .as_mut()
                .unwrap()
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &self.renderer.tgt_color_pp_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                });

        render_pass.set_bind_group(0, &self.globals.bind_group, &[]);

        SecondPassDrawer {
            render_pass,
            renderer: &self.renderer,
        }
    }

    pub fn third_pass(&mut self) -> ThirdPassDrawer {
        let mut render_pass =
            self.encoder
                .as_mut()
                .unwrap()
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &self.tex.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                });

        render_pass.set_bind_group(0, &self.globals.bind_group, &[]);

        ThirdPassDrawer {
            render_pass,
            renderer: &self.renderer,
        }
    }
}

impl<'a> Drop for Drawer<'a> {
    fn drop(&mut self) {
        // TODO: submitting things to the queue can let the gpu start on them sooner
        // maybe we should submit each render pass to the queue as they are produced?
        self.renderer
            .queue
            .submit(std::iter::once(self.encoder.take().unwrap().finish()));
    }
}

pub struct FirstPassDrawer<'a> {
    pub(super) render_pass: wgpu::RenderPass<'a>,
    pub renderer: &'a Renderer,
}

impl<'a> FirstPassDrawer<'a> {
    pub fn draw_skybox<'b: 'a>(&mut self, model: &'b Model<skybox::Vertex>) {
        self.render_pass
            .set_pipeline(&self.renderer.skybox_pipeline.pipeline);
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..));
        self.render_pass.draw(0..model.len() as u32, 0..1);
    }

    pub fn draw_lod_terrain<'b: 'a>(&mut self, model: &'b Model<lod_terrain::Vertex>) {
        self.render_pass
            .set_pipeline(&self.renderer.lod_terrain_pipeline.pipeline);
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..));
        self.render_pass.draw(0..model.len() as u32, 0..1);
    }

    pub fn draw_figure<'b: 'a>(
        &mut self,
        model: SubModel<'b, terrain::Vertex>,
        locals: &'b figure::BoundLocals,
        col_lights: &'b ColLights<figure::Locals>,
    ) {
        self.render_pass
            .set_pipeline(&self.renderer.figure_pipeline.pipeline);
        self.render_pass.set_bind_group(1, &locals.bind_group, &[]);
        self.render_pass
            .set_bind_group(2, &col_lights.bind_group, &[]);
        self.render_pass.set_vertex_buffer(0, model.buf());
        self.render_pass.draw(0..model.len(), 0..1);
    }

    pub fn draw_terrain<'b: 'a>(
        &mut self,
        model: &'b Model<terrain::Vertex>,
        locals: &'b terrain::BoundLocals,
        col_lights: &'b ColLights<terrain::Locals>,
    ) {
        self.render_pass
            .set_pipeline(&self.renderer.terrain_pipeline.pipeline);
        self.render_pass.set_bind_group(1, &locals.bind_group, &[]);
        self.render_pass
            .set_bind_group(2, &col_lights.bind_group, &[]);
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..));
        self.render_pass.draw(0..model.len() as u32, 0..1)
    }

    pub fn draw_particles(&mut self) -> ParticleDrawer<'_, 'a> {
        self.render_pass
            .set_pipeline(&self.renderer.particle_pipeline.pipeline);

        ParticleDrawer {
            render_pass: &mut self.render_pass,
        }
    }

    pub fn draw_sprite<'b: 'a>(
        &mut self,
        model: &'b Model<sprite::Vertex>,
        instances: &'b Instances<sprite::Instance>,
        terrain_locals: &'b terrain::BoundLocals,
        locals: &'b sprite::BoundLocals,
        col_lights: &'b ColLights<sprite::Locals>,
    ) {
        self.render_pass
            .set_pipeline(&self.renderer.sprite_pipeline.pipeline);
        self.render_pass
            .set_bind_group(1, &terrain_locals.bind_group, &[]);
        self.render_pass.set_bind_group(2, &locals.bind_group, &[]);
        self.render_pass
            .set_bind_group(3, &col_lights.bind_group, &[]);
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..));
        self.render_pass
            .set_vertex_buffer(1, instances.buf().slice(..));
        self.render_pass
            .draw(0..model.len() as u32, 0..instances.count() as u32);
    }

    pub fn draw_fluid<'b: 'a>(&mut self, waves: &'b fluid::BindGroup) -> FluidDrawer<'_, 'a> {
        self.render_pass
            .set_pipeline(&self.renderer.fluid_pipeline.pipeline);
        self.render_pass.set_bind_group(1, &waves.bind_group, &[]);

        FluidDrawer {
            render_pass: &mut self.render_pass,
        }
    }
}

pub struct ParticleDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: &'pass_ref mut wgpu::RenderPass<'pass>,
}

impl<'pass_ref, 'pass: 'pass_ref> ParticleDrawer<'pass_ref, 'pass> {
    // Note: if we ever need to draw less than the whole model, these APIs can be
    // changed
    pub fn draw<'data: 'pass>(
        &mut self,
        model: &'data Model<particle::Vertex>,
        instances: &'data Instances<particle::Instance>,
    ) {
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..));
        self.render_pass
            .set_vertex_buffer(1, instances.buf().slice(..));
        self.render_pass
            // TODO: since we cast to u32 maybe this should returned by the len/count functions?
            .draw(0..model.len() as u32, 0..instances.count() as u32);
    }
}

pub struct FluidDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: &'pass_ref mut wgpu::RenderPass<'pass>,
}

impl<'pass_ref, 'pass: 'pass_ref> FluidDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass>(
        &mut self,
        model: &'data Model<fluid::Vertex>,
        locals: &'data terrain::BoundLocals,
    ) {
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..));
        self.render_pass.set_bind_group(2, &locals.bind_group, &[]);
        self.render_pass.draw(0..model.len() as u32, 0..1);
    }
}

pub struct SecondPassDrawer<'a> {
    pub(super) render_pass: wgpu::RenderPass<'a>,
    pub renderer: &'a Renderer,
}

impl<'a> SecondPassDrawer<'a> {
    pub fn draw_clouds<'b: 'a>(&mut self) {
        self.render_pass
            .set_pipeline(&self.renderer.clouds_pipeline.pipeline);
        self.render_pass
            .set_bind_group(1, &self.renderer.locals.clouds_bind.bind_group, &[]);
        self.render_pass.draw(0..3, 0..1);
    }
}

pub struct ThirdPassDrawer<'a> {
    render_pass: wgpu::RenderPass<'a>,
    renderer: &'a Renderer,
}

impl<'a> ThirdPassDrawer<'a> {
    pub fn draw_post_process<'b: 'a>(&mut self) {
        self.render_pass
            .set_pipeline(&self.renderer.postprocess_pipeline.pipeline);
        self.render_pass
            .set_bind_group(1, &self.renderer.locals.postprocess_bind.bind_group, &[]);
        self.render_pass.draw(0..3, 0..1);
    }

    pub fn draw_ui(&mut self) -> UiDrawer<'_, 'a> {
        self.render_pass
            .set_pipeline(&self.renderer.ui_pipeline.pipeline);

        UiDrawer {
            render_pass: &mut self.render_pass,
        }
    }
}

pub struct UiDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: &'pass_ref mut wgpu::RenderPass<'pass>,
}

pub struct PreparedUiDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: &'pass_ref mut wgpu::RenderPass<'pass>,
}

impl<'pass_ref, 'pass: 'pass_ref> UiDrawer<'pass_ref, 'pass> {
    /// Set vertex buffer, initial scissor, and locals
    /// These can be changed later but this ensures that they don't have to be
    /// set with every draw call
    pub fn prepare<'data: 'pass>(
        &mut self,
        locals: &'data ui::BoundLocals,
        //texture: &'b ui::TextureBindGroup,
        buf: &'data DynamicModel<ui::Vertex>,
        scissor: Aabr<u16>,
    ) -> PreparedUiDrawer<'_, 'pass> {
        // Note: not actually prepared yet
        // we do this to avoid having to write extra code for the set functions
        let mut prepared = PreparedUiDrawer {
            render_pass: self.render_pass,
        };
        // Prepare
        prepared.set_locals(locals);
        //prepared.set_texture(texture);
        prepared.set_model(buf);
        prepared.set_scissor(scissor);

        prepared
    }
}

impl<'pass_ref, 'pass: 'pass_ref> PreparedUiDrawer<'pass_ref, 'pass> {
    pub fn set_locals<'data: 'pass>(&mut self, locals: &'data ui::BoundLocals) {
        self.render_pass.set_bind_group(1, &locals.bind_group, &[]);
    }

    //pub fn set_texture<'b: 'a>(&mut self, texture: &'b ui::TextureBindGroup) {
    //    self.render_pass.set_bind_group(1, &texture.bind_group, &[]);
    //}

    pub fn set_model<'data: 'pass>(&mut self, model: &'data DynamicModel<ui::Vertex>) {
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..))
    }

    pub fn set_scissor<'data: 'pass>(&mut self, scissor: Aabr<u16>) {
        let Aabr { min, max } = scissor;
        self.render_pass.set_scissor_rect(
            min.x as u32,
            min.y as u32,
            (max.x - min.x) as u32,
            (max.y - min.y) as u32,
        );
    }

    pub fn draw<'data: 'pass>(&mut self, texture: &'data ui::TextureBindGroup, verts: Range<u32>) {
        self.render_pass.set_bind_group(2, &texture.bind_group, &[]);
        self.render_pass.draw(verts, 0..1);
    }
}
