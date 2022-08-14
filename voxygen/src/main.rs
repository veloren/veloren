#![deny(unsafe_code)]
#![feature(bool_to_option)]
#![recursion_limit = "2048"]

#[cfg(all(
    target_os = "windows",
    not(feature = "tracy-memory"),
    not(feature = "hot-egui")
))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// Allow profiling allocations with Tracy
#[cfg_attr(feature = "tracy-memory", global_allocator)]
#[cfg(feature = "tracy-memory")]
static GLOBAL: common_base::tracy_client::ProfiledAllocator<std::alloc::System> =
    common_base::tracy_client::ProfiledAllocator::new(std::alloc::System, 128);

use i18n::{self, LocalizationHandle};
use veloren_voxygen::{
    audio::AudioFrontend,
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
use tracing::{error, info, warn};
#[cfg(feature = "egui-ui")]
use veloren_voxygen::ui::egui::EguiState;

fn main() {
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

    // Set up panic handler to relay swish panic messages to the user
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let panic_info_payload = panic_info.payload();
        let payload_string = panic_info_payload.downcast_ref::<String>();
        let reason = match payload_string {
            Some(s) => s,
            None => {
                let payload_str = panic_info_payload.downcast_ref::<&str>();
                match payload_str {
                    Some(st) => st,
                    None => "Payload is not a string",
                }
            },
        };
        let msg = format!(
            "A critical error has occurred and Voxygen has been forced to \
            terminate in an unusual manner. Details about the error can be \
            found below.\n\
            \n\
            > What should I do?\n\
            \n\
            We need your help to fix this! You can help by contacting us and \
            reporting this problem. To do this, open an issue on the Veloren \
            issue tracker:\n\
            \n\
            https://www.gitlab.com/veloren/veloren/issues/new\n\
            \n\
            If you're on the Veloren community Discord server, we'd be \
            grateful if you could also post a message in the #support channel.
            \n\
            > What should I include?\n\
            \n\
            The error information below will be useful in finding and fixing \
            the problem. Please include as much information about your setup \
            and the events that led up to the panic as possible.
            \n\
            Voxygen has logged information about the problem (including this \
            message) to the file {}. Please include the contents of this \
            file in your bug report.
            \n\
            > Error information\n\
            \n\
            The information below is intended for developers and testers.\n\
            \n\
            Panic Payload: {:?}\n\
            PanicInfo: {}\n\
            Game version: {} [{}]",
            logs_dir.join(&log_filename).display(),
            reason,
            panic_info,
            *common::util::GIT_HASH,
            *common::util::GIT_DATE
        );

        error!(
            "VOXYGEN HAS PANICKED\n\n{}\n\nBacktrace:\n{:?}",
            msg,
            backtrace::Backtrace::new(),
        );

        #[cfg(feature = "native-dialog")]
        {
            use native_dialog::{MessageDialog, MessageType};

            let mbox = move || {
                MessageDialog::new()
                    .set_title("Voxygen has panicked")
                    //somehow `<` and `>` are invalid characters and cause the msg to get replaced
                    // by some generic text thus i replace them
                    .set_text(&msg.replace('<', "[").replace('>', "]"))
                    .set_type(MessageType::Error)
                    .show_alert()
                    .unwrap()
            };

            // On windows we need to spawn a thread as the msg doesn't work otherwise
            #[cfg(target_os = "windows")]
            {
                let builder = std::thread::Builder::new().name("shutdown".into());
                builder
                    .spawn(move || {
                        mbox();
                    })
                    .unwrap()
                    .join()
                    .unwrap();
            }

            #[cfg(not(target_os = "windows"))]
            mbox();
        }

        default_hook(panic_info);
    }));

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
        ),
        //    AudioOutput::Device(ref dev) => Some(dev.clone()),
    };

    audio.set_master_volume(settings.audio.master_volume);
    audio.set_music_volume(settings.audio.music_volume);
    audio.set_sfx_volume(settings.audio.sfx_volume);
    audio.set_ambience_volume(settings.audio.ambience_volume);
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
        singleplayer: None,
        i18n,
        clipboard,
        client_error: None,
        clear_shadows_next_frame: false,
    };

    run::run(global_state, event_loop);
}
