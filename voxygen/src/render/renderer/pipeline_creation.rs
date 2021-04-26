use super::{
    super::{
        pipelines::{
            blit, clouds, figure, fluid, lod_terrain, particle, postprocess, shadow, skybox,
            sprite, terrain, ui,
        },
        AaMode, CloudMode, FluidMode, LightingMode, RenderError, RenderMode, ShadowMode,
    },
    shaders::Shaders,
    Layouts,
};
use common_base::prof_span;
use std::sync::Arc;

/// All the pipelines
pub struct Pipelines {
    pub figure: figure::FigurePipeline,
    pub fluid: fluid::FluidPipeline,
    pub lod_terrain: lod_terrain::LodTerrainPipeline,
    pub particle: particle::ParticlePipeline,
    pub clouds: clouds::CloudsPipeline,
    pub postprocess: postprocess::PostProcessPipeline,
    // Consider reenabling at some time
    // player_shadow: figure::FigurePipeline,
    pub skybox: skybox::SkyboxPipeline,
    pub sprite: sprite::SpritePipeline,
    pub terrain: terrain::TerrainPipeline,
    pub ui: ui::UiPipeline,
    pub blit: blit::BlitPipeline,
}

/// Pipelines that are needed to render 3D stuff in-game
/// Use to decouple interface pipeline creation when initializing the renderer
pub struct IngamePipelines {
    figure: figure::FigurePipeline,
    fluid: fluid::FluidPipeline,
    lod_terrain: lod_terrain::LodTerrainPipeline,
    particle: particle::ParticlePipeline,
    clouds: clouds::CloudsPipeline,
    postprocess: postprocess::PostProcessPipeline,
    // Consider reenabling at some time
    // player_shadow: figure::FigurePipeline,
    skybox: skybox::SkyboxPipeline,
    sprite: sprite::SpritePipeline,
    terrain: terrain::TerrainPipeline,
}

pub struct ShadowPipelines {
    pub point: Option<shadow::PointShadowPipeline>,
    pub directed: Option<shadow::ShadowPipeline>,
    pub figure: Option<shadow::ShadowFigurePipeline>,
}

pub struct IngameAndShadowPipelines {
    pub ingame: IngamePipelines,
    pub shadow: ShadowPipelines,
}

/// Pipelines neccesary to display the UI and take screenshots
/// Use to decouple interface pipeline creation when initializing the renderer
pub struct InterfacePipelines {
    pub ui: ui::UiPipeline,
    pub blit: blit::BlitPipeline,
}

impl Pipelines {
    pub fn consolidate(interface: InterfacePipelines, ingame: IngamePipelines) -> Self {
        Self {
            figure: ingame.figure,
            fluid: ingame.fluid,
            lod_terrain: ingame.lod_terrain,
            particle: ingame.particle,
            clouds: ingame.clouds,
            postprocess: ingame.postprocess,
            //player_shadow: ingame.player_shadow,
            skybox: ingame.skybox,
            sprite: ingame.sprite,
            terrain: ingame.terrain,
            ui: interface.ui,
            blit: interface.blit,
        }
    }
}

/// Processed shaders ready for use in pipeline creation
struct ShaderModules {
    skybox_vert: wgpu::ShaderModule,
    skybox_frag: wgpu::ShaderModule,
    figure_vert: wgpu::ShaderModule,
    figure_frag: wgpu::ShaderModule,
    terrain_vert: wgpu::ShaderModule,
    terrain_frag: wgpu::ShaderModule,
    fluid_vert: wgpu::ShaderModule,
    fluid_frag: wgpu::ShaderModule,
    sprite_vert: wgpu::ShaderModule,
    sprite_frag: wgpu::ShaderModule,
    particle_vert: wgpu::ShaderModule,
    particle_frag: wgpu::ShaderModule,
    ui_vert: wgpu::ShaderModule,
    ui_frag: wgpu::ShaderModule,
    lod_terrain_vert: wgpu::ShaderModule,
    lod_terrain_frag: wgpu::ShaderModule,
    clouds_vert: wgpu::ShaderModule,
    clouds_frag: wgpu::ShaderModule,
    postprocess_vert: wgpu::ShaderModule,
    postprocess_frag: wgpu::ShaderModule,
    blit_vert: wgpu::ShaderModule,
    blit_frag: wgpu::ShaderModule,
    point_light_shadows_vert: wgpu::ShaderModule,
    light_shadows_directed_vert: wgpu::ShaderModule,
    light_shadows_figure_vert: wgpu::ShaderModule,
}

impl ShaderModules {
    pub fn new(
        device: &wgpu::Device,
        shaders: &Shaders,
        mode: &RenderMode,
        has_shadow_views: bool,
    ) -> Result<Self, RenderError> {
        prof_span!(_guard, "ShaderModules::new");
        use shaderc::{CompileOptions, Compiler, OptimizationLevel, ResolvedInclude, ShaderKind};

        let constants = shaders.get("include.constants").unwrap();
        let globals = shaders.get("include.globals").unwrap();
        let sky = shaders.get("include.sky").unwrap();
        let light = shaders.get("include.light").unwrap();
        let srgb = shaders.get("include.srgb").unwrap();
        let random = shaders.get("include.random").unwrap();
        let lod = shaders.get("include.lod").unwrap();
        let shadows = shaders.get("include.shadows").unwrap();

        // We dynamically add extra configuration settings to the constants file.
        let constants = format!(
            r#"
{}

#define VOXYGEN_COMPUTATION_PREFERENCE {}
#define FLUID_MODE {}
#define CLOUD_MODE {}
#define LIGHTING_ALGORITHM {}
#define SHADOW_MODE {}

"#,
            &constants.0,
            // TODO: Configurable vertex/fragment shader preference.
            "VOXYGEN_COMPUTATION_PREFERENCE_FRAGMENT",
            match mode.fluid {
                FluidMode::Cheap => "FLUID_MODE_CHEAP",
                FluidMode::Shiny => "FLUID_MODE_SHINY",
            },
            match mode.cloud {
                CloudMode::None => "CLOUD_MODE_NONE",
                CloudMode::Minimal => "CLOUD_MODE_MINIMAL",
                CloudMode::Low => "CLOUD_MODE_LOW",
                CloudMode::Medium => "CLOUD_MODE_MEDIUM",
                CloudMode::High => "CLOUD_MODE_HIGH",
                CloudMode::Ultra => "CLOUD_MODE_ULTRA",
            },
            match mode.lighting {
                LightingMode::Ashikhmin => "LIGHTING_ALGORITHM_ASHIKHMIN",
                LightingMode::BlinnPhong => "LIGHTING_ALGORITHM_BLINN_PHONG",
                LightingMode::Lambertian => "LIGHTING_ALGORITHM_LAMBERTIAN",
            },
            match mode.shadow {
                ShadowMode::None => "SHADOW_MODE_NONE",
                ShadowMode::Map(_) if has_shadow_views => "SHADOW_MODE_MAP",
                ShadowMode::Cheap | ShadowMode::Map(_) => "SHADOW_MODE_CHEAP",
            },
        );

        let anti_alias = shaders
            .get(match mode.aa {
                AaMode::None => "antialias.none",
                AaMode::Fxaa => "antialias.fxaa",
                AaMode::MsaaX4 => "antialias.msaa-x4",
                AaMode::MsaaX8 => "antialias.msaa-x8",
                AaMode::MsaaX16 => "antialias.msaa-x16",
            })
            .unwrap();

        let cloud = shaders
            .get(match mode.cloud {
                CloudMode::None => "include.cloud.none",
                _ => "include.cloud.regular",
            })
            .unwrap();

        let mut compiler = Compiler::new().ok_or(RenderError::ErrorInitializingCompiler)?;
        let mut options = CompileOptions::new().ok_or(RenderError::ErrorInitializingCompiler)?;
        options.set_optimization_level(OptimizationLevel::Performance);
        options.set_forced_version_profile(430, shaderc::GlslProfile::Core);
        options.set_include_callback(move |name, _, shader_name, _| {
            Ok(ResolvedInclude {
                resolved_name: name.to_string(),
                content: match name {
                    "constants.glsl" => constants.clone(),
                    "globals.glsl" => globals.0.to_owned(),
                    "shadows.glsl" => shadows.0.to_owned(),
                    "sky.glsl" => sky.0.to_owned(),
                    "light.glsl" => light.0.to_owned(),
                    "srgb.glsl" => srgb.0.to_owned(),
                    "random.glsl" => random.0.to_owned(),
                    "lod.glsl" => lod.0.to_owned(),
                    "anti-aliasing.glsl" => anti_alias.0.to_owned(),
                    "cloud.glsl" => cloud.0.to_owned(),
                    other => {
                        return Err(format!(
                            "Include {} in {} is not defined",
                            other, shader_name
                        ));
                    },
                },
            })
        });

        let mut create_shader = |name, kind| {
            let glsl = &shaders
                .get(name)
                .unwrap_or_else(|| panic!("Can't retrieve shader: {}", name))
                .0;
            let file_name = format!("{}.glsl", name);
            create_shader_module(device, &mut compiler, glsl, kind, &file_name, &options)
        };

        let selected_fluid_shader = ["fluid-frag.", match mode.fluid {
            FluidMode::Cheap => "cheap",
            FluidMode::Shiny => "shiny",
        }]
        .concat();

        Ok(Self {
            skybox_vert: create_shader("skybox-vert", ShaderKind::Vertex)?,
            skybox_frag: create_shader("skybox-frag", ShaderKind::Fragment)?,
            figure_vert: create_shader("figure-vert", ShaderKind::Vertex)?,
            figure_frag: create_shader("figure-frag", ShaderKind::Fragment)?,
            terrain_vert: create_shader("terrain-vert", ShaderKind::Vertex)?,
            terrain_frag: create_shader("terrain-frag", ShaderKind::Fragment)?,
            fluid_vert: create_shader("fluid-vert", ShaderKind::Vertex)?,
            fluid_frag: create_shader(&selected_fluid_shader, ShaderKind::Fragment)?,
            sprite_vert: create_shader("sprite-vert", ShaderKind::Vertex)?,
            sprite_frag: create_shader("sprite-frag", ShaderKind::Fragment)?,
            particle_vert: create_shader("particle-vert", ShaderKind::Vertex)?,
            particle_frag: create_shader("particle-frag", ShaderKind::Fragment)?,
            ui_vert: create_shader("ui-vert", ShaderKind::Vertex)?,
            ui_frag: create_shader("ui-frag", ShaderKind::Fragment)?,
            lod_terrain_vert: create_shader("lod-terrain-vert", ShaderKind::Vertex)?,
            lod_terrain_frag: create_shader("lod-terrain-frag", ShaderKind::Fragment)?,
            clouds_vert: create_shader("clouds-vert", ShaderKind::Vertex)?,
            clouds_frag: create_shader("clouds-frag", ShaderKind::Fragment)?,
            postprocess_vert: create_shader("postprocess-vert", ShaderKind::Vertex)?,
            postprocess_frag: create_shader("postprocess-frag", ShaderKind::Fragment)?,
            blit_vert: create_shader("blit-vert", ShaderKind::Vertex)?,
            blit_frag: create_shader("blit-frag", ShaderKind::Fragment)?,
            point_light_shadows_vert: create_shader(
                "point-light-shadows-vert",
                ShaderKind::Vertex,
            )?,
            light_shadows_directed_vert: create_shader(
                "light-shadows-directed-vert",
                ShaderKind::Vertex,
            )?,
            light_shadows_figure_vert: create_shader(
                "light-shadows-figure-vert",
                ShaderKind::Vertex,
            )?,
        })
    }
}

fn create_shader_module(
    device: &wgpu::Device,
    compiler: &mut shaderc::Compiler,
    source: &str,
    kind: shaderc::ShaderKind,
    file_name: &str,
    options: &shaderc::CompileOptions,
) -> Result<wgpu::ShaderModule, RenderError> {
    prof_span!(_guard, "create_shader_modules");
    use std::borrow::Cow;

    let spv = compiler
        .compile_into_spirv(source, kind, file_name, "main", Some(options))
        .map_err(|e| (file_name, e))?;

    Ok(device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: Some(source),
        source: wgpu::ShaderSource::SpirV(Cow::Borrowed(spv.as_binary())),
        flags: wgpu::ShaderFlags::empty(), // TODO: renable wgpu::ShaderFlags::VALIDATION,
    }))
}

/// Things needed to create a pipeline
#[derive(Clone, Copy)]
struct PipelineNeeds<'a> {
    device: &'a wgpu::Device,
    layouts: &'a Layouts,
    shaders: &'a ShaderModules,
    mode: &'a RenderMode,
    sc_desc: &'a wgpu::SwapChainDescriptor,
}

/// Creates InterfacePipelines in parallel
fn create_interface_pipelines(
    needs: PipelineNeeds,
    pool: &rayon::ThreadPool,
    tasks: [Task; 2],
) -> InterfacePipelines {
    prof_span!(_guard, "create_interface_pipelines");

    let [ui_task, blit_task] = tasks;
    // Construct a pipeline for rendering UI elements
    let create_ui = || {
        ui_task.run(
            || {
                ui::UiPipeline::new(
                    needs.device,
                    &needs.shaders.ui_vert,
                    &needs.shaders.ui_frag,
                    needs.sc_desc,
                    &needs.layouts.global,
                    &needs.layouts.ui,
                )
            },
            "ui pipeline creation",
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
                    needs.sc_desc,
                    &needs.layouts.blit,
                )
            },
            "blit pipeline creation",
        )
    };

    let (ui, blit) = pool.join(create_ui, create_blit);

    InterfacePipelines { ui, blit }
}

/// Create IngamePipelines and shadow pipelines in parallel
fn create_ingame_and_shadow_pipelines(
    needs: PipelineNeeds,
    pool: &rayon::ThreadPool,
    tasks: [Task; 12],
) -> IngameAndShadowPipelines {
    prof_span!(_guard, "create_ingame_and_shadow_pipelines");

    let PipelineNeeds {
        device,
        layouts,
        shaders,
        mode,
        sc_desc,
    } = needs;

    let [
        skybox_task,
        figure_task,
        terrain_task,
        fluid_task,
        sprite_task,
        particle_task,
        lod_terrain_task,
        clouds_task,
        postprocess_task,
        // TODO: if these are ever actually optionally done, counting them 
        // as tasks to do beforehand seems kind of iffy since they will just 
        // be skipped
        point_shadow_task,
        terrain_directed_shadow_task,
        figure_directed_shadow_task,
    ] = tasks;

    // TODO: pass in format of target color buffer

    // Pipeline for rendering skyboxes
    let create_skybox = || {
        skybox_task.run(
            || {
                skybox::SkyboxPipeline::new(
                    device,
                    &shaders.skybox_vert,
                    &shaders.skybox_frag,
                    &layouts.global,
                    mode.aa,
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
                    mode.aa,
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
                    mode.aa,
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
                    mode.aa,
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
                    mode.aa,
                )
            },
            "sprite pipeline creation",
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
                    mode.aa,
                )
            },
            "particle pipeline creation",
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
                    mode.aa,
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
                    mode.aa,
                )
            },
            "clouds pipeline creation",
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
                    mode.aa,
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
                    mode.aa,
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
                    mode.aa,
                )
            },
            "figure directed shadow pipeline creation",
        )
    };

    let j1 = || pool.join(create_skybox, create_figure);
    let j2 = || pool.join(create_terrain, create_fluid);
    let j3 = || pool.join(create_sprite, create_particle);
    let j4 = || pool.join(create_lod_terrain, create_clouds);
    let j5 = || pool.join(create_postprocess, create_point_shadow);
    let j6 = || {
        pool.join(
            create_terrain_directed_shadow,
            create_figure_directed_shadow,
        )
    };

    // Ignore this
    let (
        (((skybox, figure), (terrain, fluid)), ((sprite, particle), (lod_terrain, clouds))),
        ((postprocess, point_shadow), (terrain_directed_shadow, figure_directed_shadow)),
    ) = pool.join(
        || pool.join(|| pool.join(j1, j2), || pool.join(j3, j4)),
        || pool.join(j5, j6),
    );

    IngameAndShadowPipelines {
        ingame: IngamePipelines {
            skybox,
            figure,
            terrain,
            fluid,
            sprite,
            particle,
            lod_terrain,
            clouds,
            postprocess,
            // player_shadow_pipeline,
        },
        shadow: ShadowPipelines {
            point: Some(point_shadow),
            directed: Some(terrain_directed_shadow),
            figure: Some(figure_directed_shadow),
        },
    }
}

/// Creates all the pipelines used to render.
/// Use this for the initial creation.
/// It blocks the main thread to create the interface pipelines while moving the
/// creation of other pipelines into the background
/// NOTE: this tries to use all the CPU cores to complete as soon as possible
pub(super) fn initial_create_pipelines(
    device: Arc<wgpu::Device>,
    layouts: Arc<Layouts>,
    shaders: Shaders,
    mode: RenderMode,
    sc_desc: wgpu::SwapChainDescriptor,
    has_shadow_views: bool,
) -> Result<
    (
        InterfacePipelines,
        PipelineCreation<IngameAndShadowPipelines>,
    ),
    RenderError,
> {
    prof_span!(_guard, "initial_create_pipelines");

    // Process shaders into modules
    let shader_modules = ShaderModules::new(&device, &shaders, &mode, has_shadow_views)?;

    // Create threadpool for parallel portion
    let pool = rayon::ThreadPoolBuilder::new()
        .thread_name(|n| format!("pipeline-creation-{}", n))
        .build()
        .unwrap();

    let needs = PipelineNeeds {
        device: &device,
        layouts: &layouts,
        shaders: &shader_modules,
        mode: &mode,
        sc_desc: &sc_desc,
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
            mode: &mode,
            sc_desc: &sc_desc,
        };

        let pipelines = create_ingame_and_shadow_pipelines(needs, &pool, progress.create_tasks());

        pipeline_send.send(pipelines).expect("Channel disconnected");
    });

    Ok((interface_pipelines, pipeline_creation))
}

/// Creates all the pipelines used to render.
/// Use this to recreate all the pipelines in the background.
/// TODO: report progress
/// NOTE: this tries to use all the CPU cores to complete as soon as possible
pub(super) fn recreate_pipelines(
    device: Arc<wgpu::Device>,
    layouts: Arc<Layouts>,
    shaders: Shaders,
    mode: RenderMode,
    sc_desc: wgpu::SwapChainDescriptor,
    has_shadow_views: bool,
) -> PipelineCreation<Result<(Pipelines, ShadowPipelines), RenderError>> {
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
        let shader_modules = match ShaderModules::new(&device, &shaders, &mode, has_shadow_views) {
            Ok(modules) => modules,
            Err(err) => {
                result_send.send(Err(err)).expect("Channel disconnected");
                return;
            },
        };
        drop(guard);

        let needs = PipelineNeeds {
            device: &device,
            layouts: &layouts,
            shaders: &shader_modules,
            mode: &mode,
            sc_desc: &sc_desc,
        };

        // Create interface pipelines
        let interface = create_interface_pipelines(needs, &pool, interface_tasks);

        // Create the rest of the pipelines
        let IngameAndShadowPipelines { ingame, shadow } =
            create_ingame_and_shadow_pipelines(needs, &pool, ingame_and_shadow_tasks);

        // Send them
        result_send
            .send(Ok((Pipelines::consolidate(interface, ingame), shadow)))
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
        Task { progress: &self }
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
                prof_span!(guard, _name);
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
