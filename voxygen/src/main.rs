#![deny(unsafe_code)]
#![recursion_limit = "2048"]

mod cli;

#[cfg(all(
    target_os = "windows",
    not(feature = "tracy-memory"),
    not(feature = "hot-egui"),
    not(feature = "hot-anim"),
))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// Allow profiling allocations with Tracy
#[cfg_attr(feature = "tracy-memory", global_allocator)]
#[cfg(feature = "tracy-memory")]
static GLOBAL: common_base::tracy_client::ProfiledAllocator<std::alloc::System> =
    common_base::tracy_client::ProfiledAllocator::new(std::alloc::System, 128);

use i18n::{self, LocalizationHandle};
#[cfg(feature = "singleplayer")]
use veloren_voxygen::singleplayer::SingleplayerState;
use veloren_voxygen::{
    audio::AudioFrontend,
    panic_handler,
    profile::Profile,
    run,
    scene::terrain::SpriteRenderContext,
    settings::{get_fps, AudioOutput, Settings},
    window::Window,
    GlobalState,
};

use chrono::Utc;
#[cfg(feature = "hot-reloading")]
use common::assets;
use common::clock::Clock;
use std::{panic, path::PathBuf};
use tracing::{info, warn};
#[cfg(feature = "egui-ui")]
use veloren_voxygen::ui::egui::EguiState;

fn main() {
    // Process CLI arguments
    use clap::Parser;
    let args = cli::Args::parse();

    if let Some(command) = args.command {
        match command {
            cli::Commands::ListWgpuBackends => {
                #[cfg(target_os = "windows")]
                let backends = &["opengl", "dx12", "vulkan"];
                #[cfg(target_os = "linux")]
                let backends = &["opengl", "vulkan"];
                #[cfg(target_os = "macos")]
                let backends = &["metal"];

                for backend in backends {
                    println!("{backend}");
                }
                return;
            },
        }
    }

    #[cfg(feature = "tracy")]
    common_base::tracy_client::Client::start();

    let userdata_dir = common_base::userdata_dir_workspace!();

    // Determine where Voxygen's logs should go
    // Choose a path to store the logs by the following order:
    //  - The VOXYGEN_LOGS environment variable
    //  - The <userdata>/voxygen/logs
    let logs_dir = std::env::var_os("VOXYGEN_LOGS")
        .map(PathBuf::from)
        .unwrap_or_else(|| userdata_dir.join("voxygen").join("logs"));

    // Init logging and hold the guards.
    let now = Utc::now();
    let log_filename = format!("{}_voxygen.log", now.format("%Y-%m-%d"));
    let _guards = common_frontend::init_stdout(Some((&logs_dir, &log_filename)));

    // Re-run userdata selection so any warnings will be logged
    common_base::userdata_dir_workspace!();

    info!("Using userdata dir at: {}", userdata_dir.display());

    // Determine Voxygen's config directory either by env var or placed in veloren's
    // userdata folder
    let config_dir = std::env::var_os("VOXYGEN_CONFIG")
        .map(PathBuf::from)
        .and_then(|path| {
            if path.exists() {
                Some(path)
            } else {
                warn!(?path, "VOXYGEN_CONFIG points to invalid path.");
                None
            }
        })
        .unwrap_or_else(|| userdata_dir.join("voxygen"));
    info!("Using config dir at: {}", config_dir.display());

    // Load the settings
    // Note: This won't log anything due to it being called before
    // `logging::init`. The issue is we need to read a setting to decide
    // whether we create a log file or not.
    let mut settings = Settings::load(&config_dir);
    settings.display_warnings();
    // Save settings to add new fields or create the file if it is not already there
    if let Err(err) = settings.save_to_file(&config_dir) {
        panic!("Failed to save settings: {:?}", err);
    }

    panic_handler::set_panic_hook(log_filename, logs_dir);

    // Setup tokio runtime
    use common::consts::MIN_RECOMMENDED_TOKIO_THREADS;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::runtime::Builder;

    // TODO: evaluate std::thread::available_concurrency as a num_cpus replacement
    let cores = num_cpus::get();
    let tokio_runtime = Arc::new(
        Builder::new_multi_thread()
            .enable_all()
            .worker_threads((cores / 4).max(MIN_RECOMMENDED_TOKIO_THREADS))
            .thread_name_fn(|| {
                static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
                format!("tokio-voxygen-{}", id)
            })
            .build()
            .unwrap(),
    );

    #[cfg(feature = "hot-reloading")]
    assets::start_hot_reloading();

    // Initialise watcher for animation hot-reloading
    #[cfg(feature = "hot-anim")]
    {
        anim::init();
    }

    // Initialise watcher for egui hot-reloading
    #[cfg(feature = "hot-egui")]
    {
        voxygen_egui::init();
    }

    // Setup audio
    let mut audio = match settings.audio.output {
        AudioOutput::Off => AudioFrontend::no_audio(),
        AudioOutput::Automatic => AudioFrontend::new(
            settings.audio.num_sfx_channels,
            settings.audio.num_ui_channels,
            settings.audio.subtitles,
            settings.audio.combat_music_enabled,
        ),
        //    AudioOutput::Device(ref dev) => Some(dev.clone()),
    };

    audio.set_master_volume(settings.audio.master_volume.get_checked());
    audio.set_music_volume(settings.audio.music_volume.get_checked());
    audio.set_sfx_volume(settings.audio.sfx_volume.get_checked());
    audio.set_ambience_volume(settings.audio.ambience_volume.get_checked());
    audio.set_music_spacing(settings.audio.music_spacing);

    // Load the profile.
    let profile = Profile::load(&config_dir);

    let mut i18n =
        LocalizationHandle::load(&settings.language.selected_language).unwrap_or_else(|error| {
            let selected_language = &settings.language.selected_language;
            warn!(
                ?error,
                ?selected_language,
                "Impossible to load language: change to the default language (English) instead.",
            );
            settings.language.selected_language = i18n::REFERENCE_LANG.to_owned();
            LocalizationHandle::load_expect(&settings.language.selected_language)
        });
    i18n.set_english_fallback(settings.language.use_english_fallback);

    // Create window
    use veloren_voxygen::{error::Error, render::RenderError};
    let (mut window, event_loop) = match Window::new(&settings, &tokio_runtime) {
        Ok(ok) => ok,
        // Custom panic message when a graphics backend could not be found
        Err(Error::RenderError(RenderError::CouldNotFindAdapter)) => {
            #[cfg(target_os = "windows")]
            const POTENTIAL_FIX: &str =
                " Updating the graphics drivers on this system may resolve this issue.";
            #[cfg(target_os = "macos")]
            const POTENTIAL_FIX: &str = "";
            #[cfg(not(any(target_os = "windows", target_os = "macos")))]
            const POTENTIAL_FIX: &str =
                " Installing or updating vulkan drivers may resolve this issue.";

            panic!(
                "Failed to select a rendering backend! No compatible backends were found. We \
                 currently support vulkan, metal, dx12, and dx11.{} If the issue persists, please \
                 include the operating system and GPU details in your bug report to help us \
                 identify the cause.",
                POTENTIAL_FIX
            );
        },
        Err(error) => panic!("Failed to create window!: {:?}", error),
    };

    let clipboard = iced_winit::Clipboard::connect(window.window());

    let lazy_init = SpriteRenderContext::new(window.renderer_mut());

    #[cfg(feature = "egui-ui")]
    let egui_state = EguiState::new(&window);

    #[cfg(feature = "discord")]
    let discord = if settings.networking.enable_discord_integration {
        veloren_voxygen::discord::Discord::start(&tokio_runtime)
    } else {
        veloren_voxygen::discord::Discord::Inactive
    };

    let global_state = GlobalState {
        userdata_dir,
        config_dir,
        audio,
        profile,
        window,
        tokio_runtime,
        #[cfg(feature = "egui-ui")]
        egui_state,
        lazy_init,
        clock: Clock::new(std::time::Duration::from_secs_f64(
            1.0 / get_fps(settings.graphics.max_fps) as f64,
        )),
        settings,
        info_message: None,
        #[cfg(feature = "singleplayer")]
        singleplayer: SingleplayerState::None,
        i18n,
        clipboard,
        clear_shadows_next_frame: false,
        #[cfg(feature = "discord")]
        discord,
    };

    run::run(global_state, event_loop, args.server);
}
