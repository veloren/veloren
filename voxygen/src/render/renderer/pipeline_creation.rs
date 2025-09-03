use crate::render::{
    pipelines::rain_occlusion,
    renderer::compiler::{ShaderCCompiler, ShaderStage, WgpuCompiler},
};

use super::{
    super::{
        AaMode, BloomMode, CloudMode, ExperimentalShader, FluidMode, LightingMode, PipelineModes,
        ReflectionMode, RenderError, ShadowMode,
        pipelines::{
            blit, bloom, clouds, debug, figure, fluid, lod_object, lod_terrain, particle,
            postprocess, rope, shadow, skybox, sprite, terrain, trail, ui,
        },
    },
    ImmutableLayouts, Layouts,
    shaders::Shaders,
};
use common_base::{prof_span, prof_span_alloc};
use std::sync::Arc;

/// All the pipelines
pub struct Pipelines {
    pub debug: debug::DebugPipeline,
    pub figure: figure::FigurePipeline,
    pub fluid: fluid::FluidPipeline,
    pub lod_terrain: lod_terrain::LodTerrainPipeline,
    pub particle: particle::ParticlePipeline,
    pub rope: rope::RopePipeline,
    pub trail: trail::TrailPipeline,
    pub clouds: clouds::CloudsPipeline,
    pub bloom: Option<bloom::BloomPipelines>,
    pub postprocess: postprocess::PostProcessPipeline,
    // Consider reenabling at some time
    // player_shadow: figure::FigurePipeline,
    pub skybox: skybox::SkyboxPipeline,
    pub sprite: sprite::SpritePipeline,
    pub lod_object: lod_object::LodObjectPipeline,
    pub terrain: terrain::TerrainPipeline,
    pub ui: ui::UiPipeline,
    pub premultiply_alpha: ui::PremultiplyAlphaPipeline,
    pub blit: blit::BlitPipeline,
}

/// Pipelines that are needed to render 3D stuff in-game
/// Use to decouple interface pipeline creation when initializing the renderer
pub struct IngamePipelines {
    debug: debug::DebugPipeline,
    figure: figure::FigurePipeline,
    fluid: fluid::FluidPipeline,
    lod_terrain: lod_terrain::LodTerrainPipeline,
    particle: particle::ParticlePipeline,
    rope: rope::RopePipeline,
    trail: trail::TrailPipeline,
    clouds: clouds::CloudsPipeline,
    pub bloom: Option<bloom::BloomPipelines>,
    postprocess: postprocess::PostProcessPipeline,
    // Consider reenabling at some time
    // player_shadow: figure::FigurePipeline,
    skybox: skybox::SkyboxPipeline,
    sprite: sprite::SpritePipeline,
    lod_object: lod_object::LodObjectPipeline,
    terrain: terrain::TerrainPipeline,
}

pub struct ShadowPipelines {
    pub point: Option<shadow::PointShadowPipeline>,
    pub directed: Option<shadow::ShadowPipeline>,
    pub figure: Option<shadow::ShadowFigurePipeline>,
    pub debug: Option<shadow::ShadowDebugPipeline>,
}

pub struct RainOcclusionPipelines {
    pub terrain: Option<rain_occlusion::RainOcclusionPipeline>,
    pub figure: Option<rain_occlusion::RainOcclusionFigurePipeline>,
}

// TODO: Find a better name for this?
pub struct IngameAndShadowPipelines {
    pub ingame: IngamePipelines,
    pub shadow: ShadowPipelines,
    pub rain_occlusion: RainOcclusionPipelines,
}

/// Pipelines neccesary to display the UI and take screenshots
/// Use to decouple interface pipeline creation when initializing the renderer
pub struct InterfacePipelines {
    pub ui: ui::UiPipeline,
    pub premultiply_alpha: ui::PremultiplyAlphaPipeline,
    pub blit: blit::BlitPipeline,
}

impl Pipelines {
    pub fn consolidate(interface: InterfacePipelines, ingame: IngamePipelines) -> Self {
        Self {
            debug: ingame.debug,
            figure: ingame.figure,
            fluid: ingame.fluid,
            lod_terrain: ingame.lod_terrain,
            particle: ingame.particle,
            rope: ingame.rope,
            trail: ingame.trail,
            clouds: ingame.clouds,
            bloom: ingame.bloom,
            postprocess: ingame.postprocess,
            //player_shadow: ingame.player_shadow,
            skybox: ingame.skybox,
            sprite: ingame.sprite,
            lod_object: ingame.lod_object,
            terrain: ingame.terrain,
            ui: interface.ui,
            premultiply_alpha: interface.premultiply_alpha,
            blit: interface.blit,
        }
    }
}

/// Processed shaders ready for use in pipeline creation
struct ShaderModules {
    skybox_vert: wgpu::ShaderModule,
    skybox_frag: wgpu::ShaderModule,
    debug_vert: wgpu::ShaderModule,
    debug_frag: wgpu::ShaderModule,
    figure_vert: wgpu::ShaderModule,
    figure_frag: wgpu::ShaderModule,
    terrain_vert: wgpu::ShaderModule,
    terrain_frag: wgpu::ShaderModule,
    fluid_vert: wgpu::ShaderModule,
    fluid_frag: wgpu::ShaderModule,
    sprite_vert: wgpu::ShaderModule,
    sprite_frag: wgpu::ShaderModule,
    lod_object_vert: wgpu::ShaderModule,
    lod_object_frag: wgpu::ShaderModule,
    particle_vert: wgpu::ShaderModule,
    particle_frag: wgpu::ShaderModule,
    rope_vert: wgpu::ShaderModule,
    rope_frag: wgpu::ShaderModule,
    trail_vert: wgpu::ShaderModule,
    trail_frag: wgpu::ShaderModule,
    ui_vert: wgpu::ShaderModule,
    ui_frag: wgpu::ShaderModule,
    premultiply_alpha_vert: wgpu::ShaderModule,
    premultiply_alpha_frag: wgpu::ShaderModule,
    lod_terrain_vert: wgpu::ShaderModule,
    lod_terrain_frag: wgpu::ShaderModule,
    clouds_vert: wgpu::ShaderModule,
    clouds_frag: wgpu::ShaderModule,
    dual_downsample_filtered_frag: wgpu::ShaderModule,
    dual_downsample_frag: wgpu::ShaderModule,
    dual_upsample_frag: wgpu::ShaderModule,
    postprocess_vert: wgpu::ShaderModule,
    postprocess_frag: wgpu::ShaderModule,
    blit_vert: wgpu::ShaderModule,
    blit_frag: wgpu::ShaderModule,
    point_light_shadows_vert: wgpu::ShaderModule,
    light_shadows_directed_vert: wgpu::ShaderModule,
    light_shadows_figure_vert: wgpu::ShaderModule,
    light_shadows_debug_vert: wgpu::ShaderModule,
    rain_occlusion_directed_vert: wgpu::ShaderModule,
    rain_occlusion_figure_vert: wgpu::ShaderModule,
}

impl ShaderModules {
    pub fn new(
        device: &wgpu::Device,
        shaders: &Shaders,
        pipeline_modes: &PipelineModes,
        has_shadow_views: bool,
    ) -> Result<Self, RenderError> {
        prof_span!(_guard, "ShaderModules::new");

        let constants = shaders.get("include.constants").unwrap();
        let globals = shaders.get("include.globals").unwrap();
        let sky = shaders.get("include.sky").unwrap();
        let light = shaders.get("include.light").unwrap();
        let srgb = shaders.get("include.srgb").unwrap();
        let random = shaders.get("include.random").unwrap();
        let lod = shaders.get("include.lod").unwrap();
        let shadows = shaders.get("include.shadows").unwrap();
        let rain_occlusion = shaders.get("include.rain_occlusion").unwrap();
        let point_glow = shaders.get("include.point_glow").unwrap();
        let fxaa = shaders.get("include.fxaa").unwrap();

        // We dynamically add extra configuration settings to the constants file.
        let mut constants = format!(
            r#"
{}

#define VOXYGEN_COMPUTATION_PREFERENCE {}
#define FLUID_MODE {}
#define CLOUD_MODE {}
#define REFLECTION_MODE {}
#define LIGHTING_ALGORITHM {}
#define SHADOW_MODE {}

"#,
            constants.0.as_str(),
            // TODO: Configurable vertex/fragment shader preference.
            "VOXYGEN_COMPUTATION_PREFERENCE_FRAGMENT",
            match pipeline_modes.fluid {
                FluidMode::Low => "FLUID_MODE_LOW",
                FluidMode::Medium => "FLUID_MODE_MEDIUM",
                FluidMode::High => "FLUID_MODE_HIGH",
            },
            match pipeline_modes.cloud {
                CloudMode::None => "CLOUD_MODE_NONE",
                CloudMode::Minimal => "CLOUD_MODE_MINIMAL",
                CloudMode::Low => "CLOUD_MODE_LOW",
                CloudMode::Medium => "CLOUD_MODE_MEDIUM",
                CloudMode::High => "CLOUD_MODE_HIGH",
                CloudMode::Ultra => "CLOUD_MODE_ULTRA",
            },
            match pipeline_modes.reflection {
                ReflectionMode::Low => "REFLECTION_MODE_LOW",
                ReflectionMode::Medium => "REFLECTION_MODE_MEDIUM",
                ReflectionMode::High => "REFLECTION_MODE_HIGH",
            },
            match pipeline_modes.lighting {
                LightingMode::Ashikhmin => "LIGHTING_ALGORITHM_ASHIKHMIN",
                LightingMode::BlinnPhong => "LIGHTING_ALGORITHM_BLINN_PHONG",
                LightingMode::Lambertian => "LIGHTING_ALGORITHM_LAMBERTIAN",
            },
            match pipeline_modes.shadow {
                ShadowMode::None => "SHADOW_MODE_NONE",
                ShadowMode::Map(_) if has_shadow_views => "SHADOW_MODE_MAP",
                ShadowMode::Cheap | ShadowMode::Map(_) => "SHADOW_MODE_CHEAP",
            },
        );

        if pipeline_modes.point_glow > f32::EPSILON {
            constants += &format!(
                "\n#define POINT_GLOW_FACTOR {}\n",
                pipeline_modes.point_glow
            );
        }

        if pipeline_modes.flashing_lights_enabled {
            constants += "#define FLASHING_LIGHTS_ENABLED\n";
        }

        if pipeline_modes.rain_enabled {
            constants += "#define RAIN_ENABLED\n";
        }

        for shader in pipeline_modes.experimental_shaders.iter() {
            constants += &format!(
                "#define EXPERIMENTAL_{}\n",
                format!("{:?}", shader).to_uppercase()
            );
        }

        let constants = match pipeline_modes.bloom {
            BloomMode::Off => constants,
            BloomMode::On(config) => {
                format!(
                    r#"
{}

#define BLOOM_FACTOR {}
#define BLOOM_UNIFORM_BLUR {}

"#,
                    constants,
                    config.factor.fraction(),
                    config.uniform_blur,
                )
            },
        };

        let anti_alias = shaders
            .get(match pipeline_modes.aa {
                AaMode::None => "antialias.none",
                AaMode::Bilinear => "antialias.bilinear",
                AaMode::Fxaa => "antialias.fxaa",
                AaMode::MsaaX4 => "antialias.msaa-x4",
                AaMode::MsaaX8 => "antialias.msaa-x8",
                AaMode::MsaaX16 => "antialias.msaa-x16",
                AaMode::Hqx => "antialias.hqx",
                AaMode::FxUpscale => "antialias.fxupscale",
            })
            .unwrap();

        let cloud = shaders
            .get(match pipeline_modes.cloud {
                CloudMode::None => "include.cloud.none",
                _ => "include.cloud.regular",
            })
            .unwrap();

        let shaderc_opts = !pipeline_modes
            .experimental_shaders
            .contains(&ExperimentalShader::DisableShadercOptimization);
        let fetch_include = move |name: &str, shader_name: &str| -> Result<String, String> {
            Ok(match name {
                "constants.glsl" => constants.clone(),
                "globals.glsl" => globals.0.to_owned(),
                "shadows.glsl" => shadows.0.to_owned(),
                "rain_occlusion.glsl" => rain_occlusion.0.to_owned(),
                "sky.glsl" => sky.0.to_owned(),
                "light.glsl" => light.0.to_owned(),
                "srgb.glsl" => srgb.0.to_owned(),
                "random.glsl" => random.0.to_owned(),
                "lod.glsl" => lod.0.to_owned(),
                "anti-aliasing.glsl" => anti_alias.0.to_owned(),
                "cloud.glsl" => cloud.0.to_owned(),
                "point_glow.glsl" => point_glow.0.to_owned(),
                "fxaa.glsl" => fxaa.0.to_owned(),
                other => {
                    return Err(format!(
                        "Include {} in {} is not defined",
                        other, shader_name
                    ));
                },
            })
        };

        const SHADERC: bool = true;

        let mut compiler: Box<dyn super::compiler::Compiler> = if SHADERC {
            Box::new(ShaderCCompiler::new(shaderc_opts, fetch_include)?)
        } else {
            Box::new(WgpuCompiler::new(fetch_include)?)
        };

        let mut create_shader = move |name, stage| {
            tracing::info!("Compiling {name}");
            let glsl = &shaders
                .get(name)
                .unwrap_or_else(|| panic!("Can't retrieve shader: {}", name))
                .0;
            compiler.create_shader_module(device, glsl, stage, name)
        };

        let selected_fluid_shader = ["fluid-frag.", match pipeline_modes.fluid {
            FluidMode::Low => "cheap",
            _ => "shiny",
        }]
        .concat();

        Ok(Self {
            skybox_vert: create_shader("skybox-vert", ShaderStage::Vertex)?,
            skybox_frag: create_shader("skybox-frag", ShaderStage::Fragment)?,
            debug_vert: create_shader("debug-vert", ShaderStage::Vertex)?,
            debug_frag: create_shader("debug-frag", ShaderStage::Fragment)?,
            figure_vert: create_shader("figure-vert", ShaderStage::Vertex)?,
            figure_frag: create_shader("figure-frag", ShaderStage::Fragment)?,
            terrain_vert: create_shader("terrain-vert", ShaderStage::Vertex)?,
            terrain_frag: create_shader("terrain-frag", ShaderStage::Fragment)?,
            fluid_vert: create_shader("fluid-vert", ShaderStage::Vertex)?,
            fluid_frag: create_shader(&selected_fluid_shader, ShaderStage::Fragment)?,
            sprite_vert: create_shader("sprite-vert", ShaderStage::Vertex)?,
            sprite_frag: create_shader("sprite-frag", ShaderStage::Fragment)?,
            lod_object_vert: create_shader("lod-object-vert", ShaderStage::Vertex)?,
            lod_object_frag: create_shader("lod-object-frag", ShaderStage::Fragment)?,
            particle_vert: create_shader("particle-vert", ShaderStage::Vertex)?,
            particle_frag: create_shader("particle-frag", ShaderStage::Fragment)?,
            rope_vert: create_shader("rope-vert", ShaderStage::Vertex)?,
            rope_frag: create_shader("rope-frag", ShaderStage::Fragment)?,
            trail_vert: create_shader("trail-vert", ShaderStage::Vertex)?,
            trail_frag: create_shader("trail-frag", ShaderStage::Fragment)?,
            ui_vert: create_shader("ui-vert", ShaderStage::Vertex)?,
            ui_frag: create_shader("ui-frag", ShaderStage::Fragment)?,
            premultiply_alpha_vert: create_shader("premultiply-alpha-vert", ShaderStage::Vertex)?,
            premultiply_alpha_frag: create_shader("premultiply-alpha-frag", ShaderStage::Fragment)?,
            lod_terrain_vert: create_shader("lod-terrain-vert", ShaderStage::Vertex)?,
            lod_terrain_frag: create_shader("lod-terrain-frag", ShaderStage::Fragment)?,
            clouds_vert: create_shader("clouds-vert", ShaderStage::Vertex)?,
            clouds_frag: create_shader("clouds-frag", ShaderStage::Fragment)?,
            dual_downsample_filtered_frag: create_shader(
                "dual-downsample-filtered-frag",
                ShaderStage::Fragment,
            )?,
            dual_downsample_frag: create_shader("dual-downsample-frag", ShaderStage::Fragment)?,
            dual_upsample_frag: create_shader("dual-upsample-frag", ShaderStage::Fragment)?,
            postprocess_vert: create_shader("postprocess-vert", ShaderStage::Vertex)?,
            postprocess_frag: create_shader("postprocess-frag", ShaderStage::Fragment)?,
            blit_vert: create_shader("blit-vert", ShaderStage::Vertex)?,
            blit_frag: create_shader("blit-frag", ShaderStage::Fragment)?,
            point_light_shadows_vert: create_shader(
                "point-light-shadows-vert",
                ShaderStage::Vertex,
            )?,
            light_shadows_directed_vert: create_shader(
                "light-shadows-directed-vert",
                ShaderStage::Vertex,
            )?,
            light_shadows_figure_vert: create_shader(
                "light-shadows-figure-vert",
                ShaderStage::Vertex,
            )?,
            light_shadows_debug_vert: create_shader(
                "light-shadows-debug-vert",
                ShaderStage::Vertex,
            )?,
            rain_occlusion_directed_vert: create_shader(
                "rain-occlusion-directed-vert",
                ShaderStage::Vertex,
            )?,
            rain_occlusion_figure_vert: create_shader(
                "rain-occlusion-figure-vert",
                ShaderStage::Vertex,
            )?,
        })
    }
}

/// Things needed to create a pipeline
#[derive(Clone, Copy)]
struct PipelineNeeds<'a> {
    device: &'a wgpu::Device,
    layouts: &'a Layouts,
    shaders: &'a ShaderModules,
    pipeline_modes: &'a PipelineModes,
    surface_config: &'a wgpu::SurfaceConfiguration,
}

/// Creates InterfacePipelines in parallel
fn create_interface_pipelines(
    needs: PipelineNeeds,
    pool: &rayon::ThreadPool,
    tasks: [Task; 3],
) -> InterfacePipelines {
    prof_span!(_guard, "create_interface_pipelines");

    let [ui_task, premultiply_alpha_task, blit_task] = tasks;
    // Construct a pipeline for rendering UI elements
    let create_ui = || {
        ui_task.run(
            || {
                ui::UiPipeline::new(
                    needs.device,
                    &needs.shaders.ui_vert,
                    &needs.shaders.ui_frag,
                    needs.surface_config,
                    &needs.layouts.global,
                    &needs.layouts.ui,
                )
            },
            "ui pipeline creation",
        )
    };

    let create_premultiply_alpha = || {
        premultiply_alpha_task.run(
            || {
                ui::PremultiplyAlphaPipeline::new(
                    needs.device,
                    &needs.shaders.premultiply_alpha_vert,
                    &needs.shaders.premultiply_alpha_frag,
                    &needs.layouts.premultiply_alpha,
                )
            },
            "premultiply alpha pipeline creation",
        )
    };

    // Construct a pipeline for blitting, used during screenshotting
    let create_blit = || {
        blit_task.run(
            || {
                blit::BlitPipeline::new(
                    needs.device,
                    &needs.shaders.blit_vert,
                    &needs.shaders.blit_frag,
                    needs.surface_config,
                    &needs.layouts.blit,
                )
            },
            "blit pipeline creation",
        )
    };

    let (ui, (premultiply_alpha, blit)) = pool.join(create_ui, || {
        pool.join(create_premultiply_alpha, create_blit)
    });

    InterfacePipelines {
        ui,
        premultiply_alpha,
        blit,
    }
}

/// Create IngamePipelines and shadow pipelines in parallel
fn create_ingame_and_shadow_pipelines(
    needs: PipelineNeeds,
    pool: &rayon::ThreadPool,
    // TODO: Reduce the boilerplate in this file
    tasks: [Task; 20],
    format: wgpu::TextureFormat,
) -> IngameAndShadowPipelines {
    prof_span!(_guard, "create_ingame_and_shadow_pipelines");

    let PipelineNeeds {
        device,
        layouts,
        shaders,
        pipeline_modes,
        surface_config: sc_desc,
    } = needs;

    let [
        debug_task,
        skybox_task,
        figure_task,
        terrain_task,
        fluid_task,
        sprite_task,
        lod_object_task,
        particle_task,
        rope_task,
        trail_task,
        lod_terrain_task,
        clouds_task,
        bloom_task,
        postprocess_task,
        // TODO: if these are ever actually optionally done, counting them
        // as tasks to do beforehand seems kind of iffy since they will just
        // be skipped
        point_shadow_task,
        terrain_directed_shadow_task,
        figure_directed_shadow_task,
        debug_directed_shadow_task,
        terrain_directed_rain_occlusion_task,
        figure_directed_rain_occlusion_task,
    ] = tasks;

    // TODO: pass in format of target color buffer

    // Pipeline for rendering debug shapes
    let create_debug = || {
        debug_task.run(
            || {
                debug::DebugPipeline::new(
                    device,
                    &shaders.debug_vert,
                    &shaders.debug_frag,
                    &layouts.global,
                    &layouts.debug,
                    pipeline_modes.aa,
                    format,
                )
            },
            "debug pipeline creation",
        )
    };
    // Pipeline for rendering skyboxes
    let create_skybox = || {
        skybox_task.run(
            || {
                skybox::SkyboxPipeline::new(
                    device,
                    &shaders.skybox_vert,
                    &shaders.skybox_frag,
                    &layouts.global,
                    pipeline_modes.aa,
                    format,
                )
            },
            "skybox pipeline creation",
        )
    };
    // Pipeline for rendering figures
    let create_figure = || {
        figure_task.run(
            || {
                figure::FigurePipeline::new(
                    device,
                    &shaders.figure_vert,
                    &shaders.figure_frag,
                    &layouts.global,
                    &layouts.figure,
                    pipeline_modes.aa,
                    format,
                )
            },
            "figure pipeline creation",
        )
    };
    // Pipeline for rendering terrain
    let create_terrain = || {
        terrain_task.run(
            || {
                terrain::TerrainPipeline::new(
                    device,
                    &shaders.terrain_vert,
                    &shaders.terrain_frag,
                    &layouts.global,
                    &layouts.terrain,
                    pipeline_modes.aa,
                    format,
                )
            },
            "terrain pipeline creation",
        )
    };
    // Pipeline for rendering fluids
    let create_fluid = || {
        fluid_task.run(
            || {
                fluid::FluidPipeline::new(
                    device,
                    &shaders.fluid_vert,
                    &shaders.fluid_frag,
                    &layouts.global,
                    &layouts.terrain,
                    pipeline_modes.aa,
                    format,
                )
            },
            "fluid pipeline creation",
        )
    };
    // Pipeline for rendering sprites
    let create_sprite = || {
        sprite_task.run(
            || {
                sprite::SpritePipeline::new(
                    device,
                    &shaders.sprite_vert,
                    &shaders.sprite_frag,
                    &layouts.global,
                    &layouts.sprite,
                    &layouts.terrain,
                    pipeline_modes.aa,
                    format,
                )
            },
            "sprite pipeline creation",
        )
    };
    // Pipeline for rendering lod objects
    let create_lod_object = || {
        lod_object_task.run(
            || {
                lod_object::LodObjectPipeline::new(
                    device,
                    &shaders.lod_object_vert,
                    &shaders.lod_object_frag,
                    &layouts.global,
                    pipeline_modes.aa,
                    format,
                )
            },
            "lod object pipeline creation",
        )
    };
    // Pipeline for rendering particles
    let create_particle = || {
        particle_task.run(
            || {
                particle::ParticlePipeline::new(
                    device,
                    &shaders.particle_vert,
                    &shaders.particle_frag,
                    &layouts.global,
                    pipeline_modes.aa,
                    format,
                )
            },
            "particle pipeline creation",
        )
    };
    // Pipeline for rendering ropes
    let create_rope = || {
        rope_task.run(
            || {
                rope::RopePipeline::new(
                    device,
                    &shaders.rope_vert,
                    &shaders.rope_frag,
                    &layouts.global,
                    &layouts.rope,
                    pipeline_modes.aa,
                    format,
                )
            },
            "rope pipeline creation",
        )
    };
    // Pipeline for rendering weapon trails
    let create_trail = || {
        trail_task.run(
            || {
                trail::TrailPipeline::new(
                    device,
                    &shaders.trail_vert,
                    &shaders.trail_frag,
                    &layouts.global,
                    pipeline_modes.aa,
                    format,
                )
            },
            "trail pipeline creation",
        )
    };
    // Pipeline for rendering terrain
    let create_lod_terrain = || {
        lod_terrain_task.run(
            || {
                lod_terrain::LodTerrainPipeline::new(
                    device,
                    &shaders.lod_terrain_vert,
                    &shaders.lod_terrain_frag,
                    &layouts.global,
                    pipeline_modes.aa,
                    format,
                )
            },
            "lod terrain pipeline creation",
        )
    };
    // Pipeline for rendering our clouds (a kind of post-processing)
    let create_clouds = || {
        clouds_task.run(
            || {
                clouds::CloudsPipeline::new(
                    device,
                    &shaders.clouds_vert,
                    &shaders.clouds_frag,
                    &layouts.global,
                    &layouts.clouds,
                    pipeline_modes.aa,
                    format,
                )
            },
            "clouds pipeline creation",
        )
    };
    // Pipelines for rendering our bloom
    let create_bloom = || {
        bloom_task.run(
            || {
                match &pipeline_modes.bloom {
                    BloomMode::Off => None,
                    BloomMode::On(config) => Some(config),
                }
                .map(|bloom_config| {
                    bloom::BloomPipelines::new(
                        device,
                        &shaders.blit_vert,
                        &shaders.dual_downsample_filtered_frag,
                        &shaders.dual_downsample_frag,
                        &shaders.dual_upsample_frag,
                        format,
                        &layouts.bloom,
                        bloom_config,
                    )
                })
            },
            "bloom pipelines creation",
        )
    };
    // Pipeline for rendering our post-processing
    let create_postprocess = || {
        postprocess_task.run(
            || {
                postprocess::PostProcessPipeline::new(
                    device,
                    &shaders.postprocess_vert,
                    &shaders.postprocess_frag,
                    sc_desc,
                    &layouts.global,
                    &layouts.postprocess,
                )
            },
            "postprocess pipeline creation",
        )
    };

    //
    // // Pipeline for rendering the player silhouette
    // let player_shadow_pipeline = create_pipeline(
    //     factory,
    //     figure::pipe::Init {
    //         tgt_depth: (gfx::preset::depth::PASS_TEST/*,
    //         Stencil::new(
    //             Comparison::Equal,
    //             0xff,
    //             (StencilOp::Keep, StencilOp::Keep, StencilOp::Keep),
    //         ),*/),
    //         ..figure::pipe::new()
    //     },
    //     &figure_vert,
    //     &Glsl::load_watched(
    //         "voxygen.shaders.player-shadow-frag",
    //         shader_reload_indicator,
    //     )
    //     .unwrap(),
    //     &include_ctx,
    //     gfx::state::CullFace::Back,
    // )?;

    // Pipeline for rendering point light terrain shadow maps.
    let create_point_shadow = || {
        point_shadow_task.run(
            || {
                shadow::PointShadowPipeline::new(
                    device,
                    &shaders.point_light_shadows_vert,
                    &layouts.global,
                    &layouts.terrain,
                    pipeline_modes.aa,
                )
            },
            "point shadow pipeline creation",
        )
    };
    // Pipeline for rendering directional light terrain shadow maps.
    let create_terrain_directed_shadow = || {
        terrain_directed_shadow_task.run(
            || {
                shadow::ShadowPipeline::new(
                    device,
                    &shaders.light_shadows_directed_vert,
                    &layouts.global,
                    &layouts.terrain,
                    pipeline_modes.aa,
                )
            },
            "terrain directed shadow pipeline creation",
        )
    };
    // Pipeline for rendering directional light figure shadow maps.
    let create_figure_directed_shadow = || {
        figure_directed_shadow_task.run(
            || {
                shadow::ShadowFigurePipeline::new(
                    device,
                    &shaders.light_shadows_figure_vert,
                    &layouts.global,
                    &layouts.figure,
                    pipeline_modes.aa,
                )
            },
            "figure directed shadow pipeline creation",
        )
    };
    // Pipeline for rendering directional light debug shadow maps.
    let create_debug_directed_shadow = || {
        debug_directed_shadow_task.run(
            || {
                shadow::ShadowDebugPipeline::new(
                    device,
                    &shaders.light_shadows_debug_vert,
                    &layouts.global,
                    &layouts.debug,
                    pipeline_modes.aa,
                )
            },
            "figure directed shadow pipeline creation",
        )
    };
    // Pipeline for rendering directional light terrain rain occlusion maps.
    let create_terrain_directed_rain_occlusion = || {
        terrain_directed_rain_occlusion_task.run(
            || {
                rain_occlusion::RainOcclusionPipeline::new(
                    device,
                    &shaders.rain_occlusion_directed_vert,
                    &layouts.global,
                    &layouts.terrain,
                    pipeline_modes.aa,
                )
            },
            "terrain directed rain occlusion pipeline creation",
        )
    };
    // Pipeline for rendering directional light figure rain occlusion maps.
    let create_figure_directed_rain_occlusion = || {
        figure_directed_rain_occlusion_task.run(
            || {
                rain_occlusion::RainOcclusionFigurePipeline::new(
                    device,
                    &shaders.rain_occlusion_figure_vert,
                    &layouts.global,
                    &layouts.figure,
                    pipeline_modes.aa,
                )
            },
            "figure directed rain occlusion pipeline creation",
        )
    };

    let j1 = || pool.join(create_debug, || pool.join(create_skybox, create_figure));
    let j2 = || pool.join(create_terrain, || pool.join(create_fluid, create_bloom));
    let j3 = || pool.join(create_sprite, create_particle);
    let j4 = || {
        pool.join(create_lod_terrain, || {
            pool.join(create_clouds, create_trail)
        })
    };
    let j5 = || pool.join(create_postprocess, create_point_shadow);
    let j6 = || {
        pool.join(create_terrain_directed_shadow, || {
            pool.join(create_figure_directed_shadow, create_debug_directed_shadow)
        })
    };
    let j7 = || {
        pool.join(create_lod_object, || {
            pool.join(
                create_terrain_directed_rain_occlusion,
                create_figure_directed_rain_occlusion,
            )
        })
    };
    let j8 = create_rope;

    // Ignore this
    let (
        (
            ((debug, (skybox, figure)), (terrain, (fluid, bloom))),
            ((sprite, particle), (lod_terrain, (clouds, trail))),
        ),
        (
            (
                (postprocess, point_shadow),
                (terrain_directed_shadow, (figure_directed_shadow, debug_directed_shadow)),
            ),
            ((lod_object, (terrain_directed_rain_occlusion, figure_directed_rain_occlusion)), rope),
        ),
    ) = pool.join(
        || pool.join(|| pool.join(j1, j2), || pool.join(j3, j4)),
        || pool.join(|| pool.join(j5, j6), || pool.join(j7, j8)),
    );

    IngameAndShadowPipelines {
        ingame: IngamePipelines {
            debug,
            figure,
            fluid,
            lod_terrain,
            particle,
            rope,
            trail,
            clouds,
            bloom,
            postprocess,
            skybox,
            sprite,
            lod_object,
            terrain,
            // player_shadow_pipeline,
        },
        // TODO: skip creating these if the shadow map setting is not enabled
        shadow: ShadowPipelines {
            point: Some(point_shadow),
            directed: Some(terrain_directed_shadow),
            figure: Some(figure_directed_shadow),
            debug: Some(debug_directed_shadow),
        },
        rain_occlusion: RainOcclusionPipelines {
            terrain: Some(terrain_directed_rain_occlusion),
            figure: Some(figure_directed_rain_occlusion),
        },
    }
}

/// Creates all the pipelines used to render.
/// Use this for the initial creation.
/// It blocks the main thread to create the interface pipelines while moving the
/// creation of other pipelines into the background
/// NOTE: this tries to use all the CPU cores to complete as soon as possible
pub(super) fn initial_create_pipelines(
    device: wgpu::Device,
    layouts: Layouts,
    shaders: Shaders,
    pipeline_modes: PipelineModes,
    surface_config: wgpu::SurfaceConfiguration,
    has_shadow_views: bool,
    intermediate_format: wgpu::TextureFormat,
) -> Result<
    (
        InterfacePipelines,
        PipelineCreation<IngameAndShadowPipelines>,
    ),
    RenderError,
> {
    prof_span!(_guard, "initial_create_pipelines");

    // Process shaders into modules
    let shader_modules = ShaderModules::new(&device, &shaders, &pipeline_modes, has_shadow_views)?;

    // Create threadpool for parallel portion
    let pool = rayon::ThreadPoolBuilder::new()
        .thread_name(|n| format!("pipeline-creation-{}", n))
        .build()
        .unwrap();

    let needs = PipelineNeeds {
        device: &device,
        layouts: &layouts,
        shaders: &shader_modules,
        pipeline_modes: &pipeline_modes,
        surface_config: &surface_config,
    };

    // Create interface pipelines while blocking the main thread
    // Note: we use a throwaway Progress tracker here since we don't need to track
    // the progress
    let interface_pipelines =
        create_interface_pipelines(needs, &pool, Progress::new().create_tasks());

    let pool = Arc::new(pool);
    let send_pool = Arc::clone(&pool);
    // Track pipeline creation progress
    let progress = Arc::new(Progress::new());
    let (pipeline_send, pipeline_recv) = crossbeam_channel::bounded(0);
    let pipeline_creation = PipelineCreation {
        progress: Arc::clone(&progress),
        recv: pipeline_recv,
    };
    // Start background compilation
    pool.spawn(move || {
        let pool = &*send_pool;

        let needs = PipelineNeeds {
            device: &device,
            layouts: &layouts,
            shaders: &shader_modules,
            pipeline_modes: &pipeline_modes,
            surface_config: &surface_config,
        };

        let pipelines = create_ingame_and_shadow_pipelines(
            needs,
            pool,
            progress.create_tasks(),
            intermediate_format,
        );

        pipeline_send.send(pipelines).expect("Channel disconnected");
    });

    Ok((interface_pipelines, pipeline_creation))
}

/// Creates all the pipelines used to render.
/// Use this to recreate all the pipelines in the background.
/// TODO: report progress
/// NOTE: this tries to use all the CPU cores to complete as soon as possible
pub(super) fn recreate_pipelines(
    device: wgpu::Device,
    immutable_layouts: Arc<ImmutableLayouts>,
    shaders: Shaders,
    pipeline_modes: PipelineModes,
    surface_config: wgpu::SurfaceConfiguration,
    has_shadow_views: bool,
    intermediate_format: wgpu::TextureFormat,
) -> PipelineCreation<
    Result<
        (
            Pipelines,
            ShadowPipelines,
            RainOcclusionPipelines,
            Arc<postprocess::PostProcessLayout>,
        ),
        RenderError,
    >,
> {
    prof_span!(_guard, "recreate_pipelines");

    // Create threadpool for parallel portion
    let pool = rayon::ThreadPoolBuilder::new()
        .thread_name(|n| format!("pipeline-recreation-{}", n))
        .build()
        .unwrap();
    let pool = Arc::new(pool);
    let send_pool = Arc::clone(&pool);
    // Track pipeline creation progress
    let progress = Arc::new(Progress::new());
    let (result_send, result_recv) = crossbeam_channel::bounded(0);
    let pipeline_creation = PipelineCreation {
        progress: Arc::clone(&progress),
        recv: result_recv,
    };
    // Start background compilation
    pool.spawn(move || {
        let pool = &*send_pool;

        // Create tasks upfront so the total counter will be accurate
        let shader_task = progress.create_task();
        let interface_tasks = progress.create_tasks();
        let ingame_and_shadow_tasks = progress.create_tasks();

        // Process shaders into modules
        let guard = shader_task.start("process shaders");
        let shader_modules =
            match ShaderModules::new(&device, &shaders, &pipeline_modes, has_shadow_views) {
                Ok(modules) => modules,
                Err(err) => {
                    result_send.send(Err(err)).expect("Channel disconnected");
                    return;
                },
            };
        drop(guard);

        // Create new postprocess layouts
        let postprocess_layouts = Arc::new(postprocess::PostProcessLayout::new(
            &device,
            &pipeline_modes,
        ));

        let layouts = Layouts {
            immutable: immutable_layouts,
            postprocess: postprocess_layouts,
        };

        let needs = PipelineNeeds {
            device: &device,
            layouts: &layouts,
            shaders: &shader_modules,
            pipeline_modes: &pipeline_modes,
            surface_config: &surface_config,
        };

        // Create interface pipelines
        let interface = create_interface_pipelines(needs, pool, interface_tasks);

        // Create the rest of the pipelines
        let IngameAndShadowPipelines {
            ingame,
            shadow,
            rain_occlusion,
        } = create_ingame_and_shadow_pipelines(
            needs,
            pool,
            ingame_and_shadow_tasks,
            intermediate_format,
        );

        // Send them
        result_send
            .send(Ok((
                Pipelines::consolidate(interface, ingame),
                shadow,
                rain_occlusion,
                layouts.postprocess,
            )))
            .expect("Channel disconnected");
    });

    pipeline_creation
}

use core::sync::atomic::{AtomicUsize, Ordering};

/// Represents future task that has not been started
/// Dropping this will mark the task as complete though
struct Task<'a> {
    progress: &'a Progress,
}

/// Represents in-progress task, drop when complete
// NOTE: fields are unused because they are only used for their Drop impls
struct StartedTask<'a> {
    _span: common_base::ProfSpan,
    _task: Task<'a>,
}

#[derive(Default)]
struct Progress {
    total: AtomicUsize,
    complete: AtomicUsize,
    // Note: could easily add a "started counter" if that would be useful
}

impl Progress {
    pub fn new() -> Self { Self::default() }

    /// Creates a task incrementing the total number of tasks
    /// NOTE: all tasks should be created as upfront as possible so that the
    /// total reflects the amount of tasks that will need to be completed
    pub fn create_task(&self) -> Task {
        self.total.fetch_add(1, Ordering::Relaxed);
        Task { progress: self }
    }

    /// Helper method for creating tasks to do in bulk
    pub fn create_tasks<const N: usize>(&self) -> [Task; N] { [(); N].map(|()| self.create_task()) }
}

impl<'a> Task<'a> {
    /// Start a task.
    /// The name is used for profiling.
    fn start(self, _name: &str) -> StartedTask<'a> {
        // _name only used when tracy feature is activated
        StartedTask {
            _span: {
                prof_span_alloc!(guard, _name);
                guard
            },
            _task: self,
        }
    }

    /// Convenience function to run the provided closure as the task
    /// Completing the task when this function returns
    fn run<T>(self, task: impl FnOnce() -> T, name: &str) -> T {
        let _guard = self.start(name);
        task()
    }
}

impl Drop for Task<'_> {
    fn drop(&mut self) { self.progress.complete.fetch_add(1, Ordering::Relaxed); }
}

pub struct PipelineCreation<T> {
    progress: Arc<Progress>,
    recv: crossbeam_channel::Receiver<T>,
}

impl<T> PipelineCreation<T> {
    /// Returns the number of pipelines being built and completed
    /// (total, complete)
    /// NOTE: there is no guarantee that `total >= complete` due to relaxed
    /// atomics but this property should hold most of the time
    pub fn status(&self) -> (usize, usize) {
        let progress = &*self.progress;
        (
            progress.total.load(Ordering::Relaxed),
            progress.complete.load(Ordering::Relaxed),
        )
    }

    /// Checks if the pipelines were completed and returns the result if they
    /// were
    pub fn try_complete(self) -> Result<T, Self> {
        use crossbeam_channel::TryRecvError;
        match self.recv.try_recv() {
            // Yay!
            Ok(t) => Ok(t),
            // Normal error, we have not gotten anything yet
            Err(TryRecvError::Empty) => Err(self),
            // How rude!
            Err(TryRecvError::Disconnected) => {
                panic!(
                    "Background thread panicked or dropped the sender without sending anything!"
                );
            },
        }
    }
}
