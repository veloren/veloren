use super::{
    super::{
        buffer::Buffer,
        consts::Consts,
        instances::Instances,
        model::{DynamicModel, Model, SubModel},
        pipelines::{
            clouds, figure, fluid, lod_terrain, particle, postprocess, shadow, skybox, sprite,
            terrain, ui, ColLights, GlobalsBindGroup, Light, Shadow,
        },
    },
    spans::{self, OwningSpan, Span},
    Renderer, ShadowMap, ShadowMapRenderer,
};
use core::{num::NonZeroU32, ops::Range};
use std::sync::Arc;
use vek::Aabr;

pub struct Drawer<'frame> {
    encoder: Option<wgpu::CommandEncoder>,
    pub renderer: &'frame mut Renderer,
    tex: wgpu::SwapChainTexture,
    globals: &'frame GlobalsBindGroup,
}

impl<'frame> Drawer<'frame> {
    pub fn new(
        mut encoder: wgpu::CommandEncoder,
        renderer: &'frame mut Renderer,
        tex: wgpu::SwapChainTexture,
        globals: &'frame GlobalsBindGroup,
    ) -> Self {
        renderer.tracer.start_span(&mut encoder, &spans::Id::Frame);

        Self {
            encoder: Some(encoder),
            renderer,
            tex,
            globals,
        }
    }

    pub fn shadow_pass(&mut self) -> Option<ShadowPassDrawer> {
        if let ShadowMap::Enabled(ref shadow_renderer) = self.renderer.shadow_map {
            let mut render_pass =
                self.encoder
                    .as_mut()
                    .unwrap()
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("shadow pass"),
                        color_attachments: &[],
                        depth_stencil_attachment: Some(
                            wgpu::RenderPassDepthStencilAttachmentDescriptor {
                                attachment: &shadow_renderer.directed_depth.view,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(1.0),
                                    store: true,
                                }),
                                stencil_ops: None,
                            },
                        ),
                    });

            let mut render_pass = OwningSpan::start(
                &self.renderer.tracer,
                render_pass,
                spans::Id::DirectedShadows,
            );
            render_pass.set_bind_group(0, &self.globals.bind_group, &[]);

            Some(ShadowPassDrawer {
                render_pass,
                renderer: &self.renderer,
                shadow_renderer,
            })
        } else {
            None
        }
    }

    pub fn first_pass(&mut self) -> FirstPassDrawer {
        let render_pass =
            self.encoder
                .as_mut()
                .unwrap()
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("first pass"),
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
                                load: wgpu::LoadOp::Clear(0.0),
                                store: true,
                            }),
                            stencil_ops: None,
                        },
                    ),
                });

        let mut render_pass =
            OwningSpan::start(&self.renderer.tracer, render_pass, spans::Id::PassOne);

        render_pass.set_bind_group(0, &self.globals.bind_group, &[]);
        render_pass.set_bind_group(1, &self.renderer.shadow_bind.bind_group, &[]);

        FirstPassDrawer {
            render_pass,
            renderer: &self.renderer,
            figures_called: false,
        }
    }

    pub fn second_pass(&mut self) -> SecondPassDrawer {
        let render_pass =
            self.encoder
                .as_mut()
                .unwrap()
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("second pass (clouds)"),
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

        let mut render_pass =
            OwningSpan::start(&self.renderer.tracer, render_pass, spans::Id::PassTwo);

        render_pass.set_bind_group(0, &self.globals.bind_group, &[]);
        render_pass.set_bind_group(1, &self.renderer.shadow_bind.bind_group, &[]);

        SecondPassDrawer {
            render_pass,
            renderer: &self.renderer,
        }
    }

    pub fn third_pass(&mut self) -> ThirdPassDrawer {
        let render_pass =
            self.encoder
                .as_mut()
                .unwrap()
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("third pass (postprocess + ui)"),
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

        let mut render_pass =
            OwningSpan::start(&self.renderer.tracer, render_pass, spans::Id::PassThree);

        render_pass.set_bind_group(0, &self.globals.bind_group, &[]);

        ThirdPassDrawer {
            render_pass,
            renderer: &self.renderer,
        }
    }

    pub fn draw_point_shadows<'data: 'frame>(
        &mut self,
        matrices: &[shadow::PointLightMatrix; 126],
        chunks: impl Clone
        + Iterator<Item = (&'data Model<terrain::Vertex>, &'data terrain::BoundLocals)>,
    ) {
        if let ShadowMap::Enabled(ref shadow_renderer) = self.renderer.shadow_map {
            self.renderer
                .tracer
                .start_span(self.encoder.as_mut().unwrap(), &spans::Id::PointShadows);
            const STRIDE: usize = std::mem::size_of::<shadow::PointLightMatrix>();
            let data = bytemuck::cast_slice(matrices);

            for face in 0..6 {
                // TODO: view creation cost?
                let view =
                    shadow_renderer
                        .point_depth
                        .tex
                        .create_view(&wgpu::TextureViewDescriptor {
                            label: Some("Point shadow cubemap face"),
                            format: None,
                            dimension: Some(wgpu::TextureViewDimension::D2),
                            aspect: wgpu::TextureAspect::DepthOnly,
                            base_mip_level: 0,
                            level_count: None,
                            base_array_layer: face,
                            array_layer_count: NonZeroU32::new(1),
                        });

                let label = format!("point shadow face-{} pass", face);
                let mut render_pass =
                    self.encoder
                        .as_mut()
                        .unwrap()
                        .begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some(&label),
                            color_attachments: &[],
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachmentDescriptor {
                                    attachment: &view,
                                    depth_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(1.0),
                                        store: true,
                                    }),
                                    stencil_ops: None,
                                },
                            ),
                        });

                render_pass.set_pipeline(&shadow_renderer.point_pipeline.pipeline);
                render_pass.set_bind_group(0, &self.globals.bind_group, &[]);

                (0../*20*/1).for_each(|point_light| {
                    render_pass.set_push_constants(
                        wgpu::ShaderStage::all(),
                        0,
                        &data[(6 * (point_light + 1) * STRIDE + face as usize * STRIDE)
                            ..(6 * (point_light + 1) * STRIDE + (face + 1) as usize * STRIDE)],
                    );
                    chunks.clone().for_each(|(model, locals)| {
                        render_pass.set_bind_group(1, &locals.bind_group, &[]);
                        render_pass.set_vertex_buffer(0, model.buf().slice(..));
                        render_pass.draw(0..model.len() as u32, 0..1);
                    });
                });
            }
            self.renderer
                .tracer
                .end_span(self.encoder.as_mut().unwrap(), &spans::Id::PointShadows);
        }
    }

    /// Clear all the shadow textures, useful if directed shadows (shadow_pass)
    /// and point light shadows (draw_point_shadows) are unused and thus the
    /// textures will otherwise not be cleared after either their
    /// initialization or their last use
    /// NOTE: could simply use the above passes except `draw_point_shadows`
    /// requires an array of matrices that could be a pain to construct
    /// simply for clearing
    pub fn clear_shadows(&mut self) {
        if let ShadowMap::Enabled(ref shadow_renderer) = self.renderer.shadow_map {
            self.encoder
                .as_mut()
                .unwrap()
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("clear directed shadow pass"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(
                        wgpu::RenderPassDepthStencilAttachmentDescriptor {
                            attachment: &shadow_renderer.directed_depth.view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: true,
                            }),
                            stencil_ops: None,
                        },
                    ),
                });

            for face in 0..6 {
                // TODO: view creation cost?
                let view =
                    shadow_renderer
                        .point_depth
                        .tex
                        .create_view(&wgpu::TextureViewDescriptor {
                            label: Some("Point shadow cubemap face"),
                            format: None,
                            dimension: Some(wgpu::TextureViewDimension::D2),
                            aspect: wgpu::TextureAspect::DepthOnly,
                            base_mip_level: 0,
                            level_count: None,
                            base_array_layer: face,
                            array_layer_count: NonZeroU32::new(1),
                        });

                let label = format!("clear point shadow face-{} pass", face);
                self.encoder
                    .as_mut()
                    .unwrap()
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some(&label),
                        color_attachments: &[],
                        depth_stencil_attachment: Some(
                            wgpu::RenderPassDepthStencilAttachmentDescriptor {
                                attachment: &view,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(1.0),
                                    store: true,
                                }),
                                stencil_ops: None,
                            },
                        ),
                    });
            }
        }
    }
}

impl<'frame> Drop for Drawer<'frame> {
    fn drop(&mut self) {
        // TODO: submitting things to the queue can let the gpu start on them sooner
        // maybe we should submit each render pass to the queue as they are produced?
        self.renderer
            .tracer
            .end_span(self.encoder.as_mut().unwrap(), &spans::Id::Frame);
        self.renderer
            .tracer
            .resolve_timestamps(self.encoder.as_mut().unwrap());
        self.renderer
            .queue
            .submit(std::iter::once(self.encoder.take().unwrap().finish()));
        // NOTE: this introduces blocking on GPU work
        self.renderer
            .tracer
            .record_timestamps(&self.renderer.device)
    }
}

// Shadow pass
pub struct ShadowPassDrawer<'pass> {
    render_pass: OwningSpan<'pass, wgpu::RenderPass<'pass>>,
    pub renderer: &'pass Renderer,
    shadow_renderer: &'pass ShadowMapRenderer,
}

impl<'pass> ShadowPassDrawer<'pass> {
    pub fn draw_figure_shadows(&mut self) -> FigureShadowDrawer<'_, 'pass> {
        let mut render_pass = Span::start(
            &self.renderer.tracer,
            &mut *self.render_pass,
            spans::Id::DirectedFigureShadows,
        );
        render_pass.set_pipeline(&self.shadow_renderer.figure_directed_pipeline.pipeline);

        FigureShadowDrawer { render_pass }
    }

    pub fn draw_terrain_shadows(&mut self) -> TerrainShadowDrawer<'_, 'pass> {
        let mut render_pass = Span::start(
            &self.renderer.tracer,
            &mut *self.render_pass,
            spans::Id::DirectedTerrainShadows,
        );
        render_pass.set_pipeline(&self.shadow_renderer.terrain_directed_pipeline.pipeline);

        TerrainShadowDrawer { render_pass }
    }
}

pub struct FigureShadowDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Span<'pass_ref, wgpu::RenderPass<'pass>>,
}

impl<'pass_ref, 'pass: 'pass_ref> FigureShadowDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass>(
        &mut self,
        model: SubModel<'data, terrain::Vertex>,
        locals: &'data figure::BoundLocals,
    ) {
        self.render_pass.set_bind_group(1, &locals.bind_group, &[]);
        self.render_pass.set_vertex_buffer(0, model.buf());
        self.render_pass.draw(0..model.len(), 0..1);
    }
}

pub struct TerrainShadowDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Span<'pass_ref, wgpu::RenderPass<'pass>>,
}

impl<'pass_ref, 'pass: 'pass_ref> TerrainShadowDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass>(
        &mut self,
        model: &'data Model<terrain::Vertex>,
        locals: &'data terrain::BoundLocals,
    ) {
        self.render_pass.set_bind_group(1, &locals.bind_group, &[]);
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..));
        self.render_pass.draw(0..model.len() as u32, 0..1);
    }
}

// First pass
pub struct FirstPassDrawer<'pass> {
    pub(super) render_pass: OwningSpan<'pass, wgpu::RenderPass<'pass>>,
    pub renderer: &'pass Renderer,
    // TODO: hack
    figures_called: bool,
}

impl<'pass> FirstPassDrawer<'pass> {
    pub fn draw_skybox<'data: 'pass>(&mut self, model: &'data Model<skybox::Vertex>) {
        let mut render_pass = Span::start(
            &self.renderer.tracer,
            &mut *self.render_pass,
            spans::Id::Skybox,
        );
        render_pass.set_pipeline(&self.renderer.skybox_pipeline.pipeline);
        render_pass.set_vertex_buffer(0, model.buf().slice(..));
        render_pass.draw(0..model.len() as u32, 0..1);
    }

    pub fn draw_lod_terrain<'data: 'pass>(&mut self, model: &'data Model<lod_terrain::Vertex>) {
        let mut render_pass = Span::start(
            &self.renderer.tracer,
            &mut *self.render_pass,
            spans::Id::Lod,
        );
        render_pass.set_pipeline(&self.renderer.lod_terrain_pipeline.pipeline);
        render_pass.set_vertex_buffer(0, model.buf().slice(..));
        render_pass.draw(0..model.len() as u32, 0..1);
    }

    pub fn draw_figures(&mut self) -> FigureDrawer<'_, 'pass> {
        let mut render_pass = Span::start(
            &self.renderer.tracer,
            &mut *self.render_pass,
            if !self.figures_called {
                spans::Id::Figures1
            } else {
                spans::Id::Figures2
            },
        );
        self.figures_called = true;
        render_pass.set_pipeline(&self.renderer.figure_pipeline.pipeline);

        FigureDrawer { render_pass }
    }

    pub fn draw_terrain<'data: 'pass>(&mut self) -> TerrainDrawer<'_, 'pass> {
        let mut render_pass = Span::start(
            &self.renderer.tracer,
            &mut *self.render_pass,
            spans::Id::Terrain,
        );
        render_pass.set_pipeline(&self.renderer.terrain_pipeline.pipeline);

        TerrainDrawer {
            render_pass,

            col_lights: None,
        }
    }

    pub fn draw_particles(&mut self) -> ParticleDrawer<'_, 'pass> {
        let mut render_pass = Span::start(
            &self.renderer.tracer,
            &mut *self.render_pass,
            spans::Id::Particles,
        );
        render_pass.set_pipeline(&self.renderer.particle_pipeline.pipeline);

        ParticleDrawer { render_pass }
    }

    pub fn draw_sprites<'data: 'pass>(
        &mut self,
        col_lights: &'data ColLights<sprite::Locals>,
    ) -> SpriteDrawer<'_, 'pass> {
        let mut render_pass = Span::start(
            &self.renderer.tracer,
            &mut *self.render_pass,
            spans::Id::Sprites,
        );
        self.render_pass
            .set_pipeline(&self.renderer.sprite_pipeline.pipeline);
        self.render_pass
            .set_bind_group(4, &col_lights.bind_group, &[]);

        SpriteDrawer { render_pass }
    }

    pub fn draw_fluid<'data: 'pass>(
        &mut self,
        waves: &'data fluid::BindGroup,
    ) -> FluidDrawer<'_, 'pass> {
        let mut render_pass = Span::start(
            &self.renderer.tracer,
            &mut *self.render_pass,
            spans::Id::Fluid,
        );
        render_pass.set_pipeline(&self.renderer.fluid_pipeline.pipeline);
        render_pass.set_bind_group(2, &waves.bind_group, &[]);

        FluidDrawer { render_pass }
    }
}

pub struct FigureDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Span<'pass_ref, wgpu::RenderPass<'pass>>,
}

impl<'pass_ref, 'pass: 'pass_ref> FigureDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass>(
        &mut self,
        model: SubModel<'data, terrain::Vertex>,
        locals: &'data figure::BoundLocals,
        // TODO: don't rebind this every time once they are shared between figures
        col_lights: &'data ColLights<figure::Locals>,
    ) {
        self.render_pass.set_bind_group(2, &locals.bind_group, &[]);
        self.render_pass
            .set_bind_group(3, &col_lights.bind_group, &[]);
        self.render_pass.set_vertex_buffer(0, model.buf());
        self.render_pass.draw(0..model.len(), 0..1);
    }
}

pub struct TerrainDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Span<'pass_ref, wgpu::RenderPass<'pass>>,
    col_lights: Option<&'pass_ref Arc<ColLights<terrain::Locals>>>,
}

impl<'pass_ref, 'pass: 'pass_ref> TerrainDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass>(
        &mut self,
        model: &'data Model<terrain::Vertex>,
        col_lights: &'data Arc<ColLights<terrain::Locals>>,
        locals: &'data terrain::BoundLocals,
    ) {
        let col_lights = if let Some(col_lights) = self
            .col_lights
            // Check if we are still using the same atlas texture as the previous drawn
            // chunk
            .filter(|current_col_lights| Arc::ptr_eq(current_col_lights, col_lights))
        {
            col_lights
        } else {
            self.render_pass
                .set_bind_group(3, &col_lights.bind_group, &[]); // TODO: put this in slot 2
            self.col_lights = Some(col_lights);
            col_lights
        };

        self.render_pass.set_bind_group(2, &locals.bind_group, &[]); // TODO: put this in slot 3
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..));
        self.render_pass.draw(0..model.len() as u32, 0..1)
    }
}

pub struct ParticleDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Span<'pass_ref, wgpu::RenderPass<'pass>>,
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

pub struct SpriteDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Span<'pass_ref, wgpu::RenderPass<'pass>>,
}

impl<'pass_ref, 'pass: 'pass_ref> SpriteDrawer<'pass_ref, 'pass> {
    pub fn in_chunk<'data: 'pass>(
        &mut self,
        terrain_locals: &'data terrain::BoundLocals,
    ) -> ChunkSpriteDrawer<'_, 'pass> {
        self.render_pass
            .set_bind_group(2, &terrain_locals.bind_group, &[]);

        ChunkSpriteDrawer {
            render_pass: &mut self.render_pass,
        }
    }
}
pub struct ChunkSpriteDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: &'pass_ref mut wgpu::RenderPass<'pass>,
}

impl<'pass_ref, 'pass: 'pass_ref> ChunkSpriteDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass>(
        &mut self,
        model: &'data Model<sprite::Vertex>,
        instances: &'data Instances<sprite::Instance>,
        locals: &'data sprite::BoundLocals,
    ) {
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..));
        self.render_pass
            .set_vertex_buffer(1, instances.buf().slice(..));
        self.render_pass.set_bind_group(3, &locals.bind_group, &[]);
        self.render_pass
            .draw(0..model.len() as u32, 0..instances.count() as u32);
    }
}

pub struct FluidDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Span<'pass_ref, wgpu::RenderPass<'pass>>,
}

impl<'pass_ref, 'pass: 'pass_ref> FluidDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass>(
        &mut self,
        model: &'data Model<fluid::Vertex>,
        locals: &'data terrain::BoundLocals,
    ) {
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..));
        self.render_pass.set_bind_group(3, &locals.bind_group, &[]);
        self.render_pass.draw(0..model.len() as u32, 0..1);
    }
}

// Second pass: clouds
pub struct SecondPassDrawer<'pass> {
    render_pass: OwningSpan<'pass, wgpu::RenderPass<'pass>>,
    renderer: &'pass Renderer,
}

impl<'pass> SecondPassDrawer<'pass> {
    pub fn draw_clouds(&mut self) {
        self.render_pass
            .set_pipeline(&self.renderer.clouds_pipeline.pipeline);
        self.render_pass
            .set_bind_group(2, &self.renderer.locals.clouds_bind.bind_group, &[]);
        self.render_pass.draw(0..3, 0..1);
    }
}

// Third pass: postprocess + ui
pub struct ThirdPassDrawer<'pass> {
    render_pass: OwningSpan<'pass, wgpu::RenderPass<'pass>>,
    renderer: &'pass Renderer,
}

impl<'pass> ThirdPassDrawer<'pass> {
    pub fn draw_post_process(&mut self) {
        let mut render_pass = Span::start(
            &self.renderer.tracer,
            &mut *self.render_pass,
            spans::Id::Postprocess,
        );
        render_pass.set_pipeline(&self.renderer.postprocess_pipeline.pipeline);
        render_pass.set_bind_group(1, &self.renderer.locals.postprocess_bind.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    pub fn draw_ui(&mut self) -> UiDrawer<'_, 'pass> {
        let mut render_pass =
            Span::start(&self.renderer.tracer, &mut *self.render_pass, spans::Id::Ui);
        render_pass.set_pipeline(&self.renderer.ui_pipeline.pipeline);

        UiDrawer { render_pass }
    }
}

pub struct UiDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Span<'pass_ref, wgpu::RenderPass<'pass>>,
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
        //texture: &'data ui::TextureBindGroup,
        buf: &'data DynamicModel<ui::Vertex>,
        scissor: Aabr<u16>,
    ) -> PreparedUiDrawer<'_, 'pass> {
        // Note: not actually prepared yet
        // we do this to avoid having to write extra code for the set functions
        let mut prepared = PreparedUiDrawer {
            render_pass: &mut *self.render_pass,
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

    //pub fn set_texture<'data: 'pass>(&mut self, texture: &'data
    // ui::TextureBindGroup) {    self.render_pass.set_bind_group(1,
    // &texture.bind_group, &[]);
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
