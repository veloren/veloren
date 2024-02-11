use crate::render::Bound;

use super::{
    super::{
        buffer::Buffer,
        instances::Instances,
        model::{DynamicModel, Model, SubModel},
        pipelines::{
            blit, bloom, clouds, debug, figure, fluid, lod_object, lod_terrain, particle, rope,
            shadow, skybox, sprite, terrain, trail, ui, AtlasTextures, FigureSpriteAtlasData,
            GlobalsBindGroup, TerrainAtlasData,
        },
        AltIndices, CullingMode,
    },
    rain_occlusion_map::{RainOcclusionMap, RainOcclusionMapRenderer},
    Renderer, ShadowMap, ShadowMapRenderer,
};
use common_base::prof_span;
use core::ops::Range;
use std::sync::Arc;
use vek::Aabr;
use wgpu_profiler::scope::{ManualOwningScope, OwningScope, Scope};
#[cfg(feature = "egui-ui")]
use {common_base::span, egui_wgpu_backend::ScreenDescriptor, egui_winit_platform::Platform};

/// Gpu timing label prefix associated with the UI alpha premultiplication pass.
pub const UI_PREMULTIPLY_PASS: &str = "ui_premultiply_pass";

// Currently available pipelines
enum Pipelines<'frame> {
    Interface(&'frame super::InterfacePipelines),
    All(&'frame super::Pipelines),
    // Should never be in this state for now but we need this to account for super::State::Nothing
    None,
}

impl<'frame> Pipelines<'frame> {
    fn ui(&self) -> Option<&ui::UiPipeline> {
        match self {
            Pipelines::Interface(pipelines) => Some(&pipelines.ui),
            Pipelines::All(pipelines) => Some(&pipelines.ui),
            Pipelines::None => None,
        }
    }

    fn premultiply_alpha(&self) -> Option<&ui::PremultiplyAlphaPipeline> {
        match self {
            Pipelines::Interface(pipelines) => Some(&pipelines.premultiply_alpha),
            Pipelines::All(pipelines) => Some(&pipelines.premultiply_alpha),
            Pipelines::None => None,
        }
    }

    fn blit(&self) -> Option<&blit::BlitPipeline> {
        match self {
            Pipelines::Interface(pipelines) => Some(&pipelines.blit),
            Pipelines::All(pipelines) => Some(&pipelines.blit),
            Pipelines::None => None,
        }
    }

    fn all(&self) -> Option<&super::Pipelines> {
        match self {
            Pipelines::All(pipelines) => Some(pipelines),
            Pipelines::Interface(_) | Pipelines::None => None,
        }
    }
}

// Borrow the fields we need from the renderer so that the GpuProfiler can be
// disjointedly borrowed mutably
struct RendererBorrow<'frame> {
    queue: &'frame wgpu::Queue,
    device: &'frame wgpu::Device,
    #[cfg(feature = "egui-ui")]
    surface_config: &'frame wgpu::SurfaceConfiguration,
    shadow: Option<&'frame super::Shadow>,
    pipelines: Pipelines<'frame>,
    locals: &'frame super::locals::Locals,
    views: &'frame super::Views,
    pipeline_modes: &'frame super::PipelineModes,
    quad_index_buffer_u16: &'frame Buffer<u16>,
    quad_index_buffer_u32: &'frame Buffer<u32>,
    ui_premultiply_uploads: &'frame mut ui::BatchedUploads,
    #[cfg(feature = "egui-ui")]
    egui_render_pass: &'frame mut egui_wgpu_backend::RenderPass,
}

pub struct Drawer<'frame> {
    surface_view: wgpu::TextureView,
    encoder: Option<ManualOwningScope<'frame, wgpu::CommandEncoder>>,
    borrow: RendererBorrow<'frame>,
    surface_texture: Option<wgpu::SurfaceTexture>,
    globals: &'frame GlobalsBindGroup,
    // Texture and other info for taking a screenshot
    // Writes to this instead in the third pass if it is present
    taking_screenshot: Option<super::screenshot::TakeScreenshot>,
}

impl<'frame> Drawer<'frame> {
    pub fn new(
        encoder: wgpu::CommandEncoder,
        renderer: &'frame mut Renderer,
        surface_texture: wgpu::SurfaceTexture,
        globals: &'frame GlobalsBindGroup,
    ) -> Self {
        let taking_screenshot = renderer.take_screenshot.take().map(|screenshot_fn| {
            super::screenshot::TakeScreenshot::new(
                &renderer.device,
                &renderer.layouts.blit,
                &renderer.sampler,
                &renderer.surface_config,
                screenshot_fn,
            )
        });

        let (pipelines, shadow) = match &renderer.state {
            super::State::Interface { pipelines, .. } => (Pipelines::Interface(pipelines), None),
            super::State::Complete {
                pipelines, shadow, ..
            } => (Pipelines::All(pipelines), Some(shadow)),
            super::State::Nothing => (Pipelines::None, None),
        };

        let borrow = RendererBorrow {
            queue: &renderer.queue,
            device: &renderer.device,
            #[cfg(feature = "egui-ui")]
            surface_config: &renderer.surface_config,
            shadow,
            pipelines,
            locals: &renderer.locals,
            views: &renderer.views,
            pipeline_modes: &renderer.pipeline_modes,
            quad_index_buffer_u16: &renderer.quad_index_buffer_u16,
            quad_index_buffer_u32: &renderer.quad_index_buffer_u32,
            ui_premultiply_uploads: &mut renderer.ui_premultiply_uploads,
            #[cfg(feature = "egui-ui")]
            egui_render_pass: &mut renderer.egui_renderpass,
        };

        let encoder =
            ManualOwningScope::start("frame", &mut renderer.profiler, encoder, borrow.device);

        // Create a view to the surface texture.
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                label: Some("Surface texture view"),
                ..Default::default()
            });

        Self {
            surface_view,
            encoder: Some(encoder),
            borrow,
            surface_texture: Some(surface_texture),
            globals,
            taking_screenshot,
        }
    }

    /// Get the pipeline modes.
    pub fn pipeline_modes(&self) -> &super::PipelineModes { self.borrow.pipeline_modes }

    /// Returns None if the rain occlusion renderer is not enabled at some
    /// level, the pipelines are not available yet or clouds are disabled.
    pub fn rain_occlusion_pass(&mut self) -> Option<RainOcclusionPassDrawer> {
        if !self.borrow.pipeline_modes.cloud.is_enabled() {
            return None;
        }

        if let RainOcclusionMap::Enabled(ref rain_occlusion_renderer) = self.borrow.shadow?.rain_map
        {
            let encoder = self.encoder.as_mut().unwrap();
            let device = self.borrow.device;
            let mut render_pass = encoder.scoped_render_pass(
                "rain_occlusion_pass",
                device,
                &wgpu::RenderPassDescriptor {
                    label: Some("rain occlusion pass"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &rain_occlusion_renderer.depth.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                },
            );

            render_pass.set_bind_group(0, &self.globals.bind_group, &[]);

            Some(RainOcclusionPassDrawer {
                render_pass,
                borrow: &self.borrow,
                rain_occlusion_renderer,
            })
        } else {
            None
        }
    }

    /// Returns None if the shadow renderer is not enabled at some level or the
    /// pipelines are not available yet
    pub fn shadow_pass(&mut self) -> Option<ShadowPassDrawer> {
        if !self.borrow.pipeline_modes.shadow.is_map() {
            return None;
        }

        if let ShadowMap::Enabled(ref shadow_renderer) = self.borrow.shadow?.map {
            let encoder = self.encoder.as_mut().unwrap();
            let device = self.borrow.device;
            let mut render_pass =
                encoder.scoped_render_pass("shadow_pass", device, &wgpu::RenderPassDescriptor {
                    label: Some("shadow pass"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &shadow_renderer.directed_depth.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            render_pass.set_bind_group(0, &self.globals.bind_group, &[]);

            Some(ShadowPassDrawer {
                render_pass,
                borrow: &self.borrow,
                shadow_renderer,
            })
        } else {
            None
        }
    }

    /// Returns None if all the pipelines are not available
    pub fn first_pass(&mut self) -> Option<FirstPassDrawer> {
        let pipelines = self.borrow.pipelines.all()?;
        // Note: this becomes Some once pipeline creation is complete even if shadows
        // are not enabled
        let shadow = self.borrow.shadow?;

        let encoder = self.encoder.as_mut().unwrap();
        let device = self.borrow.device;
        let mut render_pass =
            encoder.scoped_render_pass("first_pass", device, &wgpu::RenderPassDescriptor {
                label: Some("first pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.borrow.views.tgt_color,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.borrow.views.tgt_mat,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.borrow.views.tgt_depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_bind_group(0, &self.globals.bind_group, &[]);
        render_pass.set_bind_group(1, &shadow.bind.bind_group, &[]);

        Some(FirstPassDrawer {
            render_pass,
            borrow: &self.borrow,
            pipelines,
            globals: self.globals,
        })
    }

    /// Returns None if the volumetrics pipeline is not available
    pub fn volumetric_pass(&mut self) -> Option<VolumetricPassDrawer> {
        let pipelines = &self.borrow.pipelines.all()?;
        let shadow = self.borrow.shadow?;

        let encoder = self.encoder.as_mut().unwrap();
        let device = self.borrow.device;
        let mut render_pass =
            encoder.scoped_render_pass("volumetric_pass", device, &wgpu::RenderPassDescriptor {
                label: Some("volumetric pass (clouds)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.borrow.views.tgt_color_pp,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_bind_group(0, &self.globals.bind_group, &[]);
        render_pass.set_bind_group(1, &shadow.bind.bind_group, &[]);

        Some(VolumetricPassDrawer {
            render_pass,
            borrow: &self.borrow,
            clouds_pipeline: &pipelines.clouds,
        })
    }

    /// Returns None if the trail pipeline is not available
    pub fn transparent_pass(&mut self) -> Option<TransparentPassDrawer> {
        let pipelines = &self.borrow.pipelines.all()?;
        let shadow = self.borrow.shadow?;

        let encoder = self.encoder.as_mut().unwrap();
        let device = self.borrow.device;
        let mut render_pass =
            encoder.scoped_render_pass("transparent_pass", device, &wgpu::RenderPassDescriptor {
                label: Some("transparent pass (trails)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.borrow.views.tgt_color_pp,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.borrow.views.tgt_depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_bind_group(0, &self.globals.bind_group, &[]);
        render_pass.set_bind_group(1, &shadow.bind.bind_group, &[]);

        Some(TransparentPassDrawer {
            render_pass,
            borrow: &self.borrow,
            trail_pipeline: &pipelines.trail,
        })
    }

    /// To be ran between the second pass and the third pass
    /// does nothing if the ingame pipelines are not yet ready
    /// does nothing if bloom is disabled
    pub fn run_bloom_passes(&mut self) {
        let locals = &self.borrow.locals;
        let views = &self.borrow.views;

        let bloom_pipelines = match self.borrow.pipelines.all() {
            Some(super::Pipelines { bloom: Some(p), .. }) => p,
            _ => return,
        };

        // TODO: consider consolidating optional bloom bind groups and optional pipeline
        // into a single structure?
        let (bloom_tgts, bloom_binds) =
            match views.bloom_tgts.as_ref().zip(locals.bloom_binds.as_ref()) {
                Some((t, b)) => (t, b),
                None => return,
            };

        let device = self.borrow.device;
        let mut encoder = self.encoder.as_mut().unwrap().scope("bloom", device);

        let mut run_bloom_pass = |bind, view, label: String, pipeline, load| {
            let pass_label = format!("bloom {} pass", label);
            let mut render_pass =
                encoder.scoped_render_pass(&label, device, &wgpu::RenderPassDescriptor {
                    label: Some(&pass_label),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        resolve_target: None,
                        view,
                        ops: wgpu::Operations {
                            store: wgpu::StoreOp::Store,
                            load,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

            render_pass.set_bind_group(0, bind, &[]);
            render_pass.set_pipeline(pipeline);
            render_pass.draw(0..3, 0..1);
        };

        // Downsample filter passes
        (0..bloom::NUM_SIZES - 1).for_each(|index| {
            let bind = &bloom_binds[index].bind_group;
            let view = &bloom_tgts[index + 1];
            // Do filtering during the first downsample
            // NOTE: We currently blur all things without filtering by brightness.
            // This is left in for those that might want to experminent with filtering by
            // brightness, and it is used to filter out NaNs/Infs that would infect all the
            // pixels they are blurred with.
            let (label, pipeline) = if index == 0 {
                (
                    format!("downsample filtered {}", index + 1),
                    &bloom_pipelines.downsample_filtered,
                )
            } else {
                (
                    format!("downsample {}", index + 1),
                    &bloom_pipelines.downsample,
                )
            };
            run_bloom_pass(
                bind,
                view,
                label,
                pipeline,
                wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            );
        });

        // Upsample filter passes
        (0..bloom::NUM_SIZES - 1).for_each(|index| {
            let bind = &bloom_binds[bloom::NUM_SIZES - 1 - index].bind_group;
            let view = &bloom_tgts[bloom::NUM_SIZES - 2 - index];
            let label = format!("upsample {}", index + 1);
            run_bloom_pass(
                bind,
                view,
                label,
                &bloom_pipelines.upsample,
                if index + 2 == bloom::NUM_SIZES {
                    // Clear for the final image since that is just stuff from the previous frame.
                    wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                } else {
                    // Add to less blurred images to get gradient of blur instead of a smudge>
                    // https://catlikecoding.com/unity/tutorials/advanced-rendering/bloom/
                    wgpu::LoadOp::Load
                },
            );
        });
    }

    /// Runs render passes with alpha premultiplication pipeline to complete any
    /// pending uploads.
    fn run_ui_premultiply_passes(&mut self) {
        prof_span!("run_ui_premultiply_passes");
        let Some(premultiply_alpha) = self.borrow.pipelines.premultiply_alpha() else {
            return;
        };
        let encoder = self.encoder.as_mut().unwrap();
        let device = self.borrow.device;

        let targets = self.borrow.ui_premultiply_uploads.take();

        for (i, (target_texture, uploads)) in targets.into_iter().enumerate() {
            prof_span!("ui premultiply pass");
            let profile_name = format!("{UI_PREMULTIPLY_PASS} {i}");
            let label = format!("ui premultiply pass {i}");
            let mut render_pass =
                encoder.scoped_render_pass(&profile_name, device, &wgpu::RenderPassDescriptor {
                    label: Some(&label),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target_texture.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            render_pass.set_pipeline(&premultiply_alpha.pipeline);
            for upload in &uploads {
                let (source_bind_group, push_constant_data) = upload.draw_data(&target_texture);
                let bytes = bytemuck::bytes_of(&push_constant_data);
                render_pass.set_bind_group(0, source_bind_group, &[]);
                render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, bytes);
                render_pass.draw(0..6, 0..1);
            }
        }
    }

    /// Prepares the third pass drawer to be used.
    ///
    /// Note, this automatically calls the internal `run_ui_premultiply_passes`
    /// to complete any pending image uploads for the UI.
    pub fn third_pass(&mut self) -> ThirdPassDrawer {
        self.run_ui_premultiply_passes();

        let encoder = self.encoder.as_mut().unwrap();
        let device = self.borrow.device;
        let mut render_pass =
            encoder.scoped_render_pass("third_pass", device, &wgpu::RenderPassDescriptor {
                label: Some("third pass (postprocess + ui)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    // If a screenshot was requested render to that as an intermediate texture
                    // instead
                    view: self
                        .taking_screenshot
                        .as_ref()
                        .map_or(&self.surface_view, |s| s.texture_view()),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        render_pass.set_bind_group(0, &self.globals.bind_group, &[]);

        ThirdPassDrawer {
            render_pass,
            borrow: &self.borrow,
        }
    }

    #[cfg(feature = "egui-ui")]
    pub fn draw_egui(&mut self, platform: &mut Platform, scale_factor: f32) {
        span!(guard, "Draw egui");

        let output = platform.end_frame(None);

        let paint_jobs = platform.context().tessellate(output.shapes);

        let screen_descriptor = ScreenDescriptor {
            physical_width: self.borrow.surface_config.width,
            physical_height: self.borrow.surface_config.height,
            scale_factor,
        };

        self.borrow
            .egui_render_pass
            .add_textures(
                self.borrow.device,
                self.borrow.queue,
                &output.textures_delta,
            )
            .expect("Failed to update egui textures");
        self.borrow.egui_render_pass.update_buffers(
            self.borrow.device,
            self.borrow.queue,
            &paint_jobs,
            &screen_descriptor,
        );

        self.borrow
            .egui_render_pass
            .execute(
                self.encoder.as_mut().unwrap(),
                self.taking_screenshot
                    .as_ref()
                    .map_or(&self.surface_view, |s| s.texture_view()),
                &paint_jobs,
                &screen_descriptor,
                None,
            )
            .expect("Failed to draw egui");

        self.borrow
            .egui_render_pass
            .remove_textures(output.textures_delta)
            .expect("Failed to remove unused egui textures");

        drop(guard);
    }

    /// Does nothing if the shadow pipelines are not available or shadow map
    /// rendering is disabled
    pub fn draw_point_shadows<'data>(
        &mut self,
        matrices: &[shadow::PointLightMatrix; 126],
        chunks: impl Clone
        + Iterator<Item = (&'data Model<terrain::Vertex>, &'data terrain::BoundLocals)>,
    ) {
        if !self.borrow.pipeline_modes.shadow.is_map() {
            return;
        }

        if let Some(ShadowMap::Enabled(ref shadow_renderer)) = self.borrow.shadow.map(|s| &s.map) {
            let device = self.borrow.device;
            let mut encoder = self
                .encoder
                .as_mut()
                .unwrap()
                .scope("point shadows", device);
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
                            mip_level_count: None,
                            base_array_layer: face,
                            array_layer_count: Some(1),
                        });

                let label = format!("point shadow face-{} pass", face);
                let mut render_pass =
                    encoder.scoped_render_pass(&label, device, &wgpu::RenderPassDescriptor {
                        label: Some(&label),
                        color_attachments: &[],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                render_pass.set_pipeline(&shadow_renderer.point_pipeline.pipeline);
                set_quad_index_buffer::<terrain::Vertex>(&mut render_pass, &self.borrow);
                render_pass.set_bind_group(0, &self.globals.bind_group, &[]);

                (0../*20*/1).for_each(|point_light| {
                    render_pass.set_push_constants(
                        wgpu::ShaderStages::VERTEX_FRAGMENT,
                        0,
                        &data[(6 * (point_light + 1) * STRIDE + face as usize * STRIDE)
                            ..(6 * (point_light + 1) * STRIDE + (face + 1) as usize * STRIDE)],
                    );
                    chunks.clone().for_each(|(model, locals)| {
                        render_pass.set_bind_group(1, &locals.bind_group, &[]);
                        render_pass.set_vertex_buffer(0, model.buf().slice(..));
                        render_pass.draw_indexed(0..model.len() as u32 / 4 * 6, 0, 0..1);
                    });
                });
            }
        }
    }

    /// Clear all the shadow textures, useful if directed shadows (shadow_pass)
    /// and point light shadows (draw_point_shadows) are unused and thus the
    /// textures will otherwise not be cleared after either their
    /// initialization or their last use
    /// NOTE: could simply use the above passes except `draw_point_shadows`
    /// requires an array of matrices that could be a pain to construct
    /// simply for clearing
    ///
    /// Does nothing if the shadow pipelines are not available (although they
    /// aren't used here they are needed for the ShadowMap to exist)
    pub fn clear_shadows(&mut self) {
        if let Some(ShadowMap::Enabled(ref shadow_renderer)) = self.borrow.shadow.map(|s| &s.map) {
            let device = self.borrow.device;
            let encoder = self.encoder.as_mut().unwrap();
            let _ = encoder.scoped_render_pass(
                "clear_directed_shadow",
                device,
                &wgpu::RenderPassDescriptor {
                    label: Some("clear directed shadow pass"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &shadow_renderer.directed_depth.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                },
            );

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
                            mip_level_count: None,
                            base_array_layer: face,
                            array_layer_count: Some(1),
                        });

                let label = format!("clear point shadow face-{} pass", face);
                let _ = encoder.scoped_render_pass(&label, device, &wgpu::RenderPassDescriptor {
                    label: Some(&label),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }
        }
    }
}

impl<'frame> Drop for Drawer<'frame> {
    fn drop(&mut self) {
        let mut encoder = self.encoder.take().unwrap();

        // If taking a screenshot and the blit pipeline is available
        // NOTE: blit pipeline should always be available for now so we don't report an
        // error if it isn't
        let download_and_handle_screenshot = self
            .taking_screenshot
            .take()
            .zip(self.borrow.pipelines.blit())
            .map(|(screenshot, blit)| {
                // Image needs to be copied from the screenshot texture to the swapchain texture
                let mut render_pass = encoder.scoped_render_pass(
                    "screenshot blit",
                    self.borrow.device,
                    &wgpu::RenderPassDescriptor {
                        label: Some("Blit screenshot pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.surface_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    },
                );
                render_pass.set_pipeline(&blit.pipeline);
                render_pass.set_bind_group(0, screenshot.bind_group(), &[]);
                render_pass.draw(0..3, 0..1);
                drop(render_pass);
                // Issues a command to copy from the texture to a buffer and returns a closure
                // that will send the buffer off to another thread to be mapped
                // and processed.
                screenshot.copy_to_buffer(&mut encoder)
            });

        let (mut encoder, profiler) = encoder.end_scope();
        profiler.resolve_queries(&mut encoder);

        // It is recommended to only do one submit per frame
        self.borrow.queue.submit(std::iter::once(encoder.finish()));
        // Need to call this after submit so the async mapping doesn't occur before
        // copying the screenshot to the buffer which will be mapped.
        if let Some(f) = download_and_handle_screenshot {
            f();
        }
        self.surface_texture.take().unwrap().present();

        profiler
            .end_frame()
            .expect("Gpu profiler error! Maybe there was an unclosed scope?");
    }
}

// Shadow pass
#[must_use]
pub struct ShadowPassDrawer<'pass> {
    render_pass: OwningScope<'pass, wgpu::RenderPass<'pass>>,
    borrow: &'pass RendererBorrow<'pass>,
    shadow_renderer: &'pass ShadowMapRenderer,
}

impl<'pass> ShadowPassDrawer<'pass> {
    pub fn draw_figure_shadows(&mut self) -> FigureShadowDrawer<'_, 'pass> {
        let mut render_pass = self
            .render_pass
            .scope("directed_figure_shadows", self.borrow.device);

        render_pass.set_pipeline(&self.shadow_renderer.figure_directed_pipeline.pipeline);
        set_quad_index_buffer::<terrain::Vertex>(&mut render_pass, self.borrow);

        FigureShadowDrawer { render_pass }
    }

    pub fn draw_terrain_shadows(&mut self) -> TerrainShadowDrawer<'_, 'pass> {
        let mut render_pass = self
            .render_pass
            .scope("directed_terrain_shadows", self.borrow.device);

        render_pass.set_pipeline(&self.shadow_renderer.terrain_directed_pipeline.pipeline);
        set_quad_index_buffer::<terrain::Vertex>(&mut render_pass, self.borrow);

        TerrainShadowDrawer { render_pass }
    }

    pub fn draw_debug_shadows(&mut self) -> DebugShadowDrawer<'_, 'pass> {
        let mut render_pass = self
            .render_pass
            .scope("directed_debug_shadows", self.borrow.device);

        render_pass.set_pipeline(&self.shadow_renderer.debug_directed_pipeline.pipeline);
        set_quad_index_buffer::<debug::Vertex>(&mut render_pass, self.borrow);

        DebugShadowDrawer { render_pass }
    }
}

#[must_use]
pub struct RainOcclusionPassDrawer<'pass> {
    render_pass: OwningScope<'pass, wgpu::RenderPass<'pass>>,
    borrow: &'pass RendererBorrow<'pass>,
    rain_occlusion_renderer: &'pass RainOcclusionMapRenderer,
}

impl<'pass> RainOcclusionPassDrawer<'pass> {
    pub fn draw_figure_shadows(&mut self) -> FigureShadowDrawer<'_, 'pass> {
        let mut render_pass = self
            .render_pass
            .scope("directed_figure_rain_occlusion", self.borrow.device);

        render_pass.set_pipeline(&self.rain_occlusion_renderer.figure_pipeline.pipeline);
        set_quad_index_buffer::<terrain::Vertex>(&mut render_pass, self.borrow);

        FigureShadowDrawer { render_pass }
    }

    pub fn draw_terrain_shadows(&mut self) -> TerrainShadowDrawer<'_, 'pass> {
        let mut render_pass = self
            .render_pass
            .scope("directed_terrain_rain_occlusion", self.borrow.device);

        render_pass.set_pipeline(&self.rain_occlusion_renderer.terrain_pipeline.pipeline);
        set_quad_index_buffer::<terrain::Vertex>(&mut render_pass, self.borrow);

        TerrainShadowDrawer { render_pass }
    }
}

#[must_use]
pub struct FigureShadowDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Scope<'pass_ref, wgpu::RenderPass<'pass>>,
}

impl<'pass_ref, 'pass: 'pass_ref> FigureShadowDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass>(
        &mut self,
        model: SubModel<'data, terrain::Vertex>,
        locals: &'data figure::BoundLocals,
    ) {
        self.render_pass.set_bind_group(1, &locals.bind_group, &[]);
        self.render_pass.set_vertex_buffer(0, model.buf());
        self.render_pass
            .draw_indexed(0..model.len() / 4 * 6, 0, 0..1);
    }
}

#[must_use]
pub struct TerrainShadowDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Scope<'pass_ref, wgpu::RenderPass<'pass>>,
}

impl<'pass_ref, 'pass: 'pass_ref> TerrainShadowDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass>(
        &mut self,
        model: &'data Model<terrain::Vertex>,
        locals: &'data terrain::BoundLocals,
        alt_indices: &'data AltIndices,
        culling_mode: CullingMode,
    ) {
        let index_range = match culling_mode {
            // Don't bother rendering shadows when underground
            // TODO: Does this break point shadows in certain cases?
            CullingMode::Underground => return, //0..alt_indices.underground_end as u32,
            CullingMode::Surface => alt_indices.deep_end as u32..model.len() as u32,
            CullingMode::None => 0..model.len() as u32,
        };

        // Don't render anything if there's nothing to render!
        if index_range.is_empty() {
            return;
        }

        let submodel = model.submodel(index_range);

        self.render_pass.set_bind_group(1, &locals.bind_group, &[]);
        self.render_pass.set_vertex_buffer(0, submodel.buf());
        self.render_pass
            .draw_indexed(0..submodel.len() / 4 * 6, 0, 0..1);
    }
}

#[must_use]
pub struct DebugShadowDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Scope<'pass_ref, wgpu::RenderPass<'pass>>,
}

impl<'pass_ref, 'pass: 'pass_ref> DebugShadowDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass>(
        &mut self,
        model: &'data Model<debug::Vertex>,
        locals: &'data debug::BoundLocals,
    ) {
        self.render_pass.set_bind_group(1, &locals.bind_group, &[]);
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..));
        self.render_pass.draw(0..model.len() as u32, 0..1);
    }
}

// First pass
#[must_use]
pub struct FirstPassDrawer<'pass> {
    pub(super) render_pass: OwningScope<'pass, wgpu::RenderPass<'pass>>,
    borrow: &'pass RendererBorrow<'pass>,
    pipelines: &'pass super::Pipelines,
    globals: &'pass GlobalsBindGroup,
}

impl<'pass> FirstPassDrawer<'pass> {
    pub fn draw_skybox<'data: 'pass>(&mut self, model: &'data Model<skybox::Vertex>) {
        let mut render_pass = self.render_pass.scope("skybox", self.borrow.device);

        render_pass.set_pipeline(&self.pipelines.skybox.pipeline);
        set_quad_index_buffer::<skybox::Vertex>(&mut render_pass, self.borrow);
        render_pass.set_vertex_buffer(0, model.buf().slice(..));
        render_pass.draw(0..model.len() as u32, 0..1);
    }

    pub fn draw_debug(&mut self) -> DebugDrawer<'_, 'pass> {
        let mut render_pass = self.render_pass.scope("debug", self.borrow.device);

        render_pass.set_pipeline(&self.pipelines.debug.pipeline);
        set_quad_index_buffer::<debug::Vertex>(&mut render_pass, self.borrow);

        DebugDrawer { render_pass }
    }

    pub fn draw_lod_terrain<'data: 'pass>(&mut self, model: &'data Model<lod_terrain::Vertex>) {
        let mut render_pass = self.render_pass.scope("lod_terrain", self.borrow.device);

        render_pass.set_pipeline(&self.pipelines.lod_terrain.pipeline);
        set_quad_index_buffer::<lod_terrain::Vertex>(&mut render_pass, self.borrow);
        render_pass.set_vertex_buffer(0, model.buf().slice(..));
        render_pass.draw_indexed(0..model.len() as u32 / 4 * 6, 0, 0..1);
    }

    pub fn draw_figures(&mut self) -> FigureDrawer<'_, 'pass> {
        let mut render_pass = self.render_pass.scope("figures", self.borrow.device);

        render_pass.set_pipeline(&self.pipelines.figure.pipeline);
        // Note: figures use the same vertex type as the terrain
        set_quad_index_buffer::<terrain::Vertex>(&mut render_pass, self.borrow);

        FigureDrawer { render_pass }
    }

    pub fn draw_terrain(&mut self) -> TerrainDrawer<'_, 'pass> {
        let mut render_pass = self.render_pass.scope("terrain", self.borrow.device);

        render_pass.set_pipeline(&self.pipelines.terrain.pipeline);
        set_quad_index_buffer::<terrain::Vertex>(&mut render_pass, self.borrow);

        TerrainDrawer {
            render_pass,
            atlas_textures: None,
        }
    }

    pub fn draw_particles(&mut self) -> ParticleDrawer<'_, 'pass> {
        let mut render_pass = self.render_pass.scope("particles", self.borrow.device);

        render_pass.set_pipeline(&self.pipelines.particle.pipeline);
        set_quad_index_buffer::<particle::Vertex>(&mut render_pass, self.borrow);

        ParticleDrawer { render_pass }
    }

    pub fn draw_ropes(&mut self) -> RopeDrawer<'_, 'pass> {
        let mut render_pass = self.render_pass.scope("ropes", self.borrow.device);

        render_pass.set_pipeline(&self.pipelines.rope.pipeline);
        set_quad_index_buffer::<rope::Vertex>(&mut render_pass, self.borrow);

        RopeDrawer { render_pass }
    }

    pub fn draw_sprites<'data: 'pass>(
        &mut self,
        globals: &'data sprite::SpriteGlobalsBindGroup,
        atlas_textures: &'data AtlasTextures<sprite::Locals, FigureSpriteAtlasData>,
    ) -> SpriteDrawer<'_, 'pass> {
        let mut render_pass = self.render_pass.scope("sprites", self.borrow.device);

        render_pass.set_pipeline(&self.pipelines.sprite.pipeline);
        set_quad_index_buffer::<sprite::Vertex>(&mut render_pass, self.borrow);
        render_pass.set_bind_group(0, &globals.bind_group, &[]);
        render_pass.set_bind_group(2, &atlas_textures.bind_group, &[]);

        SpriteDrawer {
            render_pass,
            globals: self.globals,
        }
    }

    pub fn draw_lod_objects(&mut self) -> LodObjectDrawer<'_, 'pass> {
        let mut render_pass = self.render_pass.scope("lod objects", self.borrow.device);

        render_pass.set_pipeline(&self.pipelines.lod_object.pipeline);
        set_quad_index_buffer::<lod_object::Vertex>(&mut render_pass, self.borrow);

        LodObjectDrawer { render_pass }
    }

    pub fn draw_fluid(&mut self) -> FluidDrawer<'_, 'pass> {
        let mut render_pass = self.render_pass.scope("fluid", self.borrow.device);

        render_pass.set_pipeline(&self.pipelines.fluid.pipeline);
        set_quad_index_buffer::<fluid::Vertex>(&mut render_pass, self.borrow);

        FluidDrawer { render_pass }
    }
}

#[must_use]
pub struct DebugDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Scope<'pass_ref, wgpu::RenderPass<'pass>>,
}

impl<'pass_ref, 'pass: 'pass_ref> DebugDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass>(
        &mut self,
        model: &'data Model<debug::Vertex>,
        locals: &'data debug::BoundLocals,
    ) {
        self.render_pass.set_bind_group(2, &locals.bind_group, &[]);
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..));
        self.render_pass.draw(0..model.len() as u32, 0..1);
    }
}

#[must_use]
pub struct FigureDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Scope<'pass_ref, wgpu::RenderPass<'pass>>,
}

impl<'pass_ref, 'pass: 'pass_ref> FigureDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass>(
        &mut self,
        model: SubModel<'data, terrain::Vertex>,
        locals: &'data figure::BoundLocals,
        // TODO: don't rebind this every time once they are shared between figures
        atlas_textures: &'data AtlasTextures<figure::Locals, FigureSpriteAtlasData>,
    ) {
        self.render_pass
            .set_bind_group(2, &atlas_textures.bind_group, &[]);
        self.render_pass.set_bind_group(3, &locals.bind_group, &[]);
        self.render_pass.set_vertex_buffer(0, model.buf());
        self.render_pass
            .draw_indexed(0..model.len() / 4 * 6, 0, 0..1);
    }
}

#[must_use]
pub struct TerrainDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Scope<'pass_ref, wgpu::RenderPass<'pass>>,
    atlas_textures: Option<&'pass_ref Arc<AtlasTextures<terrain::Locals, TerrainAtlasData>>>,
}

impl<'pass_ref, 'pass: 'pass_ref> TerrainDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass>(
        &mut self,
        model: &'data Model<terrain::Vertex>,
        atlas_textures: &'data Arc<AtlasTextures<terrain::Locals, TerrainAtlasData>>,
        locals: &'data terrain::BoundLocals,
        alt_indices: &'data AltIndices,
        culling_mode: CullingMode,
    ) {
        let index_range = match culling_mode {
            CullingMode::Underground => 0..alt_indices.underground_end as u32,
            CullingMode::Surface => alt_indices.deep_end as u32..model.len() as u32,
            CullingMode::None => 0..model.len() as u32,
        };

        // Don't render anything if there's nothing to render!
        if index_range.is_empty() {
            return;
        }

        let submodel = model.submodel(index_range);

        if self.atlas_textures
            // Check if we are still using the same atlas texture as the previous drawn
            // chunk
            .filter(|current_atlas_textures| Arc::ptr_eq(current_atlas_textures, atlas_textures))
            .is_none()
        {
            self.render_pass
                .set_bind_group(2, &atlas_textures.bind_group, &[]);
            self.atlas_textures = Some(atlas_textures);
        };

        self.render_pass.set_bind_group(3, &locals.bind_group, &[]);

        self.render_pass.set_vertex_buffer(0, submodel.buf());
        self.render_pass
            .draw_indexed(0..submodel.len() / 4 * 6, 0, 0..1);
    }
}

#[must_use]
pub struct ParticleDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Scope<'pass_ref, wgpu::RenderPass<'pass>>,
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
            .draw_indexed(0..model.len() as u32 / 4 * 6, 0, 0..instances.count() as u32);
    }
}

#[must_use]
pub struct RopeDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Scope<'pass_ref, wgpu::RenderPass<'pass>>,
}

impl<'pass_ref, 'pass: 'pass_ref> RopeDrawer<'pass_ref, 'pass> {
    // Note: if we ever need to draw less than the whole model, these APIs can be
    // changed
    pub fn draw<'data: 'pass>(
        &mut self,
        model: &'data Model<rope::Vertex>,
        locals: &'data rope::BoundLocals,
    ) {
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..));
        self.render_pass.set_bind_group(2, &locals.bind_group, &[]);
        // TODO: since we cast to u32 maybe this should returned by the len/count
        // functions?
        self.render_pass
            .draw_indexed(0..model.len() as u32 / 4 * 6, 0, 0..1);
    }
}

#[must_use]
pub struct SpriteDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Scope<'pass_ref, wgpu::RenderPass<'pass>>,
    globals: &'pass GlobalsBindGroup,
}

impl<'pass_ref, 'pass: 'pass_ref> SpriteDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass, T>(
        &mut self,
        terrain_locals: &'data Bound<T>,
        instances: &'data Instances<sprite::Instance>,
        alt_indices: &'data AltIndices,
        culling_mode: CullingMode,
    ) {
        let instance_range = match culling_mode {
            CullingMode::Underground => 0..alt_indices.underground_end as u32,
            CullingMode::Surface => alt_indices.deep_end as u32..instances.count() as u32,
            CullingMode::None => 0..instances.count() as u32,
        };

        // Don't render anything if there's nothing to render!
        if instance_range.is_empty() {
            return;
        }

        self.render_pass
            .set_bind_group(3, &terrain_locals.bind_group, &[]);

        let subinstances = instances.subinstances(instance_range);

        self.render_pass.set_vertex_buffer(0, subinstances.buf());
        self.render_pass.draw_indexed(
            0..sprite::VERT_PAGE_SIZE / 4 * 6,
            0,
            0..subinstances.count(),
        );
    }
}

impl<'pass_ref, 'pass: 'pass_ref> Drop for SpriteDrawer<'pass_ref, 'pass> {
    fn drop(&mut self) {
        // Reset to regular globals
        self.render_pass
            .set_bind_group(0, &self.globals.bind_group, &[]);
    }
}

#[must_use]
pub struct LodObjectDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Scope<'pass_ref, wgpu::RenderPass<'pass>>,
}

impl<'pass_ref, 'pass: 'pass_ref> LodObjectDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass>(
        &mut self,
        model: &'data Model<lod_object::Vertex>,
        instances: &'data Instances<lod_object::Instance>,
    ) {
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..));
        self.render_pass
            .set_vertex_buffer(1, instances.buf().slice(..));
        self.render_pass
            .draw(0..model.len() as u32, 0..instances.count() as u32);
    }
}

#[must_use]
pub struct FluidDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Scope<'pass_ref, wgpu::RenderPass<'pass>>,
}

impl<'pass_ref, 'pass: 'pass_ref> FluidDrawer<'pass_ref, 'pass> {
    pub fn draw<'data: 'pass>(
        &mut self,
        model: &'data Model<fluid::Vertex>,
        locals: &'data terrain::BoundLocals,
    ) {
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..));
        self.render_pass.set_bind_group(2, &locals.bind_group, &[]);
        self.render_pass
            .draw_indexed(0..model.len() as u32 / 4 * 6, 0, 0..1);
    }
}

// Second pass: volumetrics
#[must_use]
pub struct VolumetricPassDrawer<'pass> {
    render_pass: OwningScope<'pass, wgpu::RenderPass<'pass>>,
    borrow: &'pass RendererBorrow<'pass>,
    clouds_pipeline: &'pass clouds::CloudsPipeline,
}

impl<'pass> VolumetricPassDrawer<'pass> {
    pub fn draw_clouds(&mut self) {
        self.render_pass
            .set_pipeline(&self.clouds_pipeline.pipeline);
        self.render_pass
            .set_bind_group(2, &self.borrow.locals.clouds_bind.bind_group, &[]);
        self.render_pass.draw(0..3, 0..1);
    }
}

// Third pass: transparents
#[must_use]
pub struct TransparentPassDrawer<'pass> {
    render_pass: OwningScope<'pass, wgpu::RenderPass<'pass>>,
    borrow: &'pass RendererBorrow<'pass>,
    trail_pipeline: &'pass trail::TrailPipeline,
}

impl<'pass> TransparentPassDrawer<'pass> {
    pub fn draw_trails(&mut self) -> Option<TrailDrawer<'_, 'pass>> {
        let shadow = &self.borrow.shadow?;

        let mut render_pass = self.render_pass.scope("trails", self.borrow.device);

        render_pass.set_pipeline(&self.trail_pipeline.pipeline);
        set_quad_index_buffer::<trail::Vertex>(&mut render_pass, self.borrow);

        render_pass.set_bind_group(1, &shadow.bind.bind_group, &[]);

        Some(TrailDrawer { render_pass })
    }
}

#[must_use]
pub struct TrailDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Scope<'pass_ref, wgpu::RenderPass<'pass>>,
}

impl<'pass_ref, 'pass: 'pass_ref> TrailDrawer<'pass_ref, 'pass> {
    pub fn draw(&mut self, submodel: SubModel<'pass, trail::Vertex>) {
        self.render_pass.set_vertex_buffer(0, submodel.buf());
        self.render_pass
            .draw_indexed(0..submodel.len() / 4 * 6, 0, 0..1);
    }
}

/// Third pass: postprocess + ui
#[must_use]
pub struct ThirdPassDrawer<'pass> {
    render_pass: OwningScope<'pass, wgpu::RenderPass<'pass>>,
    borrow: &'pass RendererBorrow<'pass>,
}

impl<'pass> ThirdPassDrawer<'pass> {
    /// Does nothing if the postprocess pipeline is not available
    pub fn draw_postprocess(&mut self) {
        let postprocess = match self.borrow.pipelines.all() {
            Some(p) => &p.postprocess,
            None => return,
        };

        let mut render_pass = self.render_pass.scope("postprocess", self.borrow.device);
        render_pass.set_pipeline(&postprocess.pipeline);
        render_pass.set_bind_group(1, &self.borrow.locals.postprocess_bind.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    /// Returns None if the UI pipeline is not available (note: this should
    /// never be the case for now)
    pub fn draw_ui(&mut self) -> Option<UiDrawer<'_, 'pass>> {
        let ui = self.borrow.pipelines.ui()?;

        let mut render_pass = self.render_pass.scope("ui", self.borrow.device);
        render_pass.set_pipeline(&ui.pipeline);
        set_quad_index_buffer::<ui::Vertex>(&mut render_pass, self.borrow);

        Some(UiDrawer { render_pass })
    }
}

#[must_use]
pub struct UiDrawer<'pass_ref, 'pass: 'pass_ref> {
    render_pass: Scope<'pass_ref, wgpu::RenderPass<'pass>>,
}

#[must_use]
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
        buf: &'data DynamicModel<ui::Vertex>,
        scissor: Aabr<u16>,
    ) -> PreparedUiDrawer<'_, 'pass> {
        // Note: not actually prepared yet
        // we do this to avoid having to write extra code for the set functions
        let mut prepared = PreparedUiDrawer {
            render_pass: &mut self.render_pass,
        };
        // Prepare
        prepared.set_locals(locals);
        prepared.set_model(buf);
        prepared.set_scissor(scissor);

        prepared
    }
}

impl<'pass_ref, 'pass: 'pass_ref> PreparedUiDrawer<'pass_ref, 'pass> {
    pub fn set_locals<'data: 'pass>(&mut self, locals: &'data ui::BoundLocals) {
        self.render_pass.set_bind_group(1, &locals.bind_group, &[]);
    }

    pub fn set_model<'data: 'pass>(&mut self, model: &'data DynamicModel<ui::Vertex>) {
        self.render_pass.set_vertex_buffer(0, model.buf().slice(..))
    }

    pub fn set_scissor(&mut self, scissor: Aabr<u16>) {
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

fn set_quad_index_buffer<'a, V: super::Vertex>(
    pass: &mut wgpu::RenderPass<'a>,
    borrow: &RendererBorrow<'a>,
) {
    if let Some(format) = V::QUADS_INDEX {
        let slice = match format {
            wgpu::IndexFormat::Uint16 => borrow.quad_index_buffer_u16.buf.slice(..),
            wgpu::IndexFormat::Uint32 => borrow.quad_index_buffer_u32.buf.slice(..),
        };

        pass.set_index_buffer(slice, format);
    }
}
