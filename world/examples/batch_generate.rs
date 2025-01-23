use std::{
    fs::{self, File, create_dir_all},
    io::Write,
    ops::RangeInclusive,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use clap::{Parser, Subcommand};
use common::{
    resources::MapKind,
    terrain::{
        map::{MapConfig, MapSample},
        uniform_idx_as_vec2,
    },
};
use image::{DynamicImage, GenericImage, ImageEncoder, codecs::png::PngEncoder};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rand::{Rng, thread_rng};
use rayon::ThreadPool;
use serde::{Deserialize, Serialize};
use tracing::{Level, Span, debug, error, info, info_span};
use tracing_subscriber::EnvFilter;
use vek::{Aabr, Rgb, Vec2};
use veloren_world::{
    CONFIG, IndexOwned, World, WorldGenerateStage,
    sim::{FileOpts, GenOpts, WorldOpts, WorldSimStage, get_horizon_map, sample_pos, sample_wpos},
};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    subcommand: Action,
    /// Whether .bin files should be saved for maps
    #[arg(short, long)]
    save_bin: bool,
    /// Hide progress bars
    #[arg(short, long)]
    no_progress: bool,
    /// Path to where maps are saved
    #[arg(long)]
    maps_path: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Action {
    /// Generate maps in a loop using the provided configuration
    Batch {
        /// Configuration to use for map generation
        config: String,
        /// How many maps will be generated in parallel
        #[arg(short, long)]
        threads: Option<usize>,
    },
    /// Generate a map from the .ron file emitted by the batch command
    Regenerate {
        config: String,
        /// Override erosion quality
        #[arg(long)]
        erosion_quality: Option<f32>,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct BatchGenerateConfig {
    scale: RangeInclusive<f64>,
    size: (u32, u32),
    kind: MapKind,
    erosion_quality: RangeInclusive<f32>,
}

impl BatchGenerateConfig {
    fn gen_rand(&self) -> GenOpts {
        GenOpts {
            x_lg: self.size.0,
            y_lg: self.size.1,
            scale: thread_rng().gen_range(self.scale.clone()),
            map_kind: self.kind,
            erosion_quality: thread_rng().gen_range(self.erosion_quality.clone()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct MapGenConfig {
    seed: u32,
    gen_opts: GenOpts,
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::WARN)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let command = Cli::parse();

    let maps_path = command.maps_path.unwrap_or(PathBuf::from("maps"));

    match command.subcommand {
        Action::Batch { config, threads } => do_batch_generate(
            config,
            command.save_bin,
            threads,
            command.no_progress,
            maps_path,
        ),
        Action::Regenerate {
            config,
            erosion_quality,
        } => do_regenerate(
            config,
            maps_path,
            erosion_quality,
            command.no_progress,
            command.save_bin,
        ),
    }
}

fn generate_one(
    seed: u32,
    base_path: &Path,
    gen_opts: GenOpts,
    (save_bin, save_image, save_metadata): (bool, bool, bool),
    span: &Span,
    threadpool: &ThreadPool,
    progress: Option<ProgressBar>,
) -> (World, IndexOwned) {
    if let Some(progress) = &progress {
        progress.set_message(seed.to_string());
    }

    let (world, index) = World::generate(
        seed,
        WorldOpts {
            seed_elements: false,
            world_file: if save_bin {
                FileOpts::Save(base_path.with_extension("bin"), gen_opts.clone())
            } else {
                FileOpts::Generate(gen_opts.clone())
            },
            calendar: None,
        },
        threadpool,
        &|stage| {
            if let WorldGenerateStage::WorldSimGenerate(WorldSimStage::Erosion(percentage)) = stage
            {
                if let Some(progress) = &progress {
                    progress.set_position(percentage as u64);
                }

                span.in_scope(|| {
                    info!("Erosion progress: {percentage:02.0}%");
                })
            }
        },
    );

    if save_image {
        let index_ref = index.as_index_ref();
        let sampler = world.sim();
        let map_size_lg = sampler.map_size_lg();

        let horizons = get_horizon_map(
            map_size_lg,
            Aabr {
                min: Vec2::zero(),
                max: map_size_lg.chunks().map(|e| e as i32),
            },
            CONFIG.sea_level,
            CONFIG.sea_level + sampler.max_height,
            |posi| {
                let sample = sampler.get(uniform_idx_as_vec2(map_size_lg, posi)).unwrap();

                sample.basement.max(sample.water_alt)
            },
            |a| a,
            |h| h,
        )
        .ok();

        let mut map_config = MapConfig::orthographic(map_size_lg, 0.0..=sampler.max_height);
        map_config.horizons = horizons.as_ref();
        map_config.is_shaded = true;
        map_config.is_stylized_topo = true;
        let map = sampler.get_map(index_ref, None);

        let mut image = DynamicImage::new(
            map_size_lg.chunks().x as u32,
            map_size_lg.chunks().y as u32,
            image::ColorType::Rgba8,
        );

        map_config.generate(
            |pos| {
                let default_sample = sample_pos(&map_config, sampler, index_ref, None, pos);
                let [r, g, b, _a] = map.rgba[pos].to_le_bytes();

                MapSample {
                    rgb: Rgb::new(r, g, b),
                    ..default_sample
                }
            },
            |wpos| sample_wpos(&map_config, sampler, wpos),
            |pos, (r, g, b, a)| {
                image.put_pixel(
                    pos.x as u32,
                    map_size_lg.chunks().y as u32 - pos.y as u32 - 1,
                    [r, g, b, a].into(),
                )
            },
        );

        let mut image_file =
            File::create_new(base_path.with_extension("png")).expect("Could not create map file");

        if let Err(error) = PngEncoder::new(&mut image_file).write_image(
            image.as_bytes(),
            map_size_lg.chunks().x as u32,
            map_size_lg.chunks().y as u32,
            image::ExtendedColorType::Rgba8,
        ) {
            error!(?error, "Could not write image data");
        }

        let _ = image_file.flush();
    }

    if save_metadata {
        // Write config
        if let Err(error) = fs::write(
            base_path.with_extension("ron"),
            ron::ser::to_string_pretty(&MapGenConfig { seed, gen_opts }, Default::default())
                .unwrap(),
        ) {
            error!(?error, "Colud not write map configuration file");
        }
    }

    info!("Finished writing map to: {}", base_path.display());
    if let Some(progress) = progress {
        progress.finish()
    }

    (world, index)
}

fn do_regenerate(
    config: String,
    maps_path: PathBuf,
    erosion_quality: Option<f32>,
    no_progress: bool,
    save_bin: bool,
) {
    let mut config: MapGenConfig =
        ron::from_str(&fs::read_to_string(config).expect("Failed to read generation file"))
            .expect("Could not parse generation file");

    let base_path = if let Some(erosion_quality) = erosion_quality {
        config.gen_opts.erosion_quality = erosion_quality;
        maps_path.join(format!("{}_{:03}", config.seed, erosion_quality * 100.0))
    } else {
        maps_path.join(config.seed.to_string())
    };

    let span = info_span!("Generating map", map = ?config);
    let pool = rayon::ThreadPoolBuilder::new().build().unwrap();

    generate_one(
        config.seed,
        &base_path,
        config.gen_opts,
        (save_bin, true, true),
        &span,
        &pool,
        (!no_progress).then(progress_bar),
    );
}

fn do_batch_generate(
    file: String,
    save_bin: bool,
    threads: Option<usize>,
    no_progress: bool,
    maps_path: PathBuf,
) {
    let config: BatchGenerateConfig =
        ron::from_str(&fs::read_to_string(file).expect("Failed to read generator config file"))
            .expect("Could not parse generator config");

    #[cfg(debug_assertions)]
    tracing::warn!("For best performance, run this in release mode");

    let threads = threads.unwrap_or(1);

    let mut handles = vec![];

    let map_i = Arc::new(AtomicUsize::new(0));
    let shutdown_started = Arc::new(std::sync::atomic::AtomicBool::new(false));

    debug!("Registering shutdown signal");
    use signal_hook::consts::signal::*;
    let _ = signal_hook::flag::register_conditional_default(SIGINT, Arc::clone(&shutdown_started));
    let _ = signal_hook::flag::register(SIGINT, Arc::clone(&shutdown_started));

    create_dir_all(&maps_path).unwrap();

    let progress_bars = (!no_progress).then(MultiProgress::new);

    for thread_id in 0..threads {
        info!(?thread_id, "Starting thread");
        let config = config.clone();
        let map_i = Arc::clone(&map_i);
        let shutdown_started = Arc::clone(&shutdown_started);
        let maps_path = maps_path.clone();
        let progress_bars = progress_bars.clone();

        let h = std::thread::spawn::<_, ()>(move || {
            loop {
                let progress = progress_bars.as_ref().map(|bars| {
                    let progress = progress_bar();
                    bars.add(progress.clone());
                    progress
                });

                if shutdown_started.load(Ordering::Relaxed) {
                    info!(?thread_id, "Shutting down thread");
                    break;
                }

                let map_i = map_i.fetch_add(1, Ordering::SeqCst);

                if let Some(progress) = &progress {
                    progress.set_prefix(format!("Map {}", map_i));
                }

                let seed = thread_rng().gen::<u32>();
                let span = info_span!("generate", map_i, thread_id);
                let _guard = span.enter();
                let gen_opts = config.gen_rand();
                let base_path = maps_path.join(seed.to_string());

                let threadpool = rayon::ThreadPoolBuilder::new().build().unwrap();

                info!("Starting world generation");
                generate_one(
                    seed,
                    &base_path,
                    gen_opts,
                    (save_bin, true, true),
                    &span,
                    &threadpool,
                    progress,
                );
            }
        });

        handles.push(h);
    }

    for handle in handles {
        let _ = handle.join();
    }
}

fn progress_bar() -> ProgressBar {
    ProgressBar::new(100).with_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{eta:6}] {prefix:8} {msg:15} [{wide_bar:.red/cyan}] {percent:3}%",
        )
        .unwrap()
        .progress_chars("#>~"),
    )
}
