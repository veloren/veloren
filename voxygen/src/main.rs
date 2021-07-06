#![deny(unsafe_code)]
#![allow(clippy::option_map_unit_fn)]
#![feature(bool_to_option)]
#![recursion_limit = "2048"]

use veloren_voxygen::{
    audio::AudioFrontend,
    i18n::{self, LocalizationHandle},
    profile::Profile,
    run,
    scene::terrain::SpriteRenderContext,
    settings::{get_fps, AudioOutput, Settings},
    window::Window,
    GlobalState,
};

#[cfg(feature = "hot-reloading")]
use common::assets;
use common::clock::Clock;
use std::panic;
use tracing::{error, info, warn};
#[cfg(feature = "egui-ui")]
use veloren_voxygen::ui::egui::EguiState;

#[allow(clippy::manual_unwrap_or)]
fn main() {
    // Load the settings
    // Note: This won't log anything due to it being called before
    // `logging::init`. The issue is we need to read a setting to decide
    // whether we create a log file or not.
    let mut settings = Settings::load();
    // Save settings to add new fields or create the file if it is not already there
    if let Err(err) = settings.save_to_file() {
        panic!("Failed to save settings: {:?}", err);
    }

    // Init logging and hold the guards.
    const LOG_FILENAME: &str = "voxygen.log";
    let _guards = common_frontend::init_stdout(Some((&settings.log.logs_path, LOG_FILENAME)));

    if let Some(path) = veloren_voxygen::settings::voxygen_data_dir().parent() {
        info!("Using userdata dir at: {}", path.display());
    } else {
        error!("Can't log userdata dir, voxygen data dir has no parent!");
    }

    // Set up panic handler to relay swish panic messages to the user
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let panic_info_payload = panic_info.payload();
        let payload_string = panic_info_payload.downcast_ref::<String>();
        let reason = match payload_string {
            Some(s) => &s,
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
            Settings::load()
                .log
                .logs_path
                .join("voxygen-<date>.log")
                .display(),
            reason,
            panic_info,
            common::util::GIT_HASH.to_string(),
            common::util::GIT_DATE.to_string()
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
        AudioOutput::Automatic => AudioFrontend::new(settings.audio.max_sfx_channels),
        //    AudioOutput::Device(ref dev) => Some(dev.clone()),
    };

    audio.set_master_volume(settings.audio.master_volume);
    audio.set_music_volume(settings.audio.music_volume);
    audio.set_sfx_volume(settings.audio.sfx_volume);

    // Load the profile.
    let profile = Profile::load();

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
    i18n.read().log_missing_entries();
    i18n.set_english_fallback(settings.language.use_english_fallback);

    // Create window
    let (mut window, event_loop) = Window::new(&settings).expect("Failed to create window!");

    let clipboard = iced_winit::Clipboard::connect(window.window());

    let lazy_init = SpriteRenderContext::new(window.renderer_mut());

    #[cfg(feature = "egui-ui")]
    let egui_state = EguiState::new(&window);

    let global_state = GlobalState {
        audio,
        profile,
        window,
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
