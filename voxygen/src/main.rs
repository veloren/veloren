#![deny(unsafe_code)]
#![feature(drain_filter)]
#![recursion_limit = "2048"]

#[cfg(feature = "discord")]
#[macro_use]
extern crate lazy_static;

#[cfg(feature = "discord")]
pub mod discord;

#[cfg(feature = "discord")]
use std::sync::Mutex;

#[macro_use]
pub mod ui;
pub mod anim;
pub mod audio;
pub mod error;
pub mod hud;
pub mod key_state;
mod logging;
pub mod menu;
pub mod mesh;
pub mod render;
pub mod scene;
pub mod session;
pub mod settings;
#[cfg(feature = "singleplayer")]
pub mod singleplayer;
pub mod window;

// Reexports
pub use crate::error::Error;

use crate::{audio::AudioFrontend, menu::main::MainMenuState, settings::Settings, window::Window};
use log::{debug, error};
use std::{mem, panic, str::FromStr};

/// A type used to store state that is shared between all play states.
pub struct GlobalState {
    settings: Settings,
    window: Window,
    audio: AudioFrontend,
    info_message: Option<String>,
}

impl GlobalState {
    /// Called after a change in play state has occurred (usually used to reverse any temporary
    /// effects a state may have made).
    pub fn on_play_state_changed(&mut self) {
        self.window.grab_cursor(false);
        self.window.needs_refresh_resize();
    }

    pub fn maintain(&mut self, dt: f32) {
        self.audio.maintain(dt);
    }
}

pub enum Direction {
    Forwards,
    Backwards,
}

/// States can either close (and revert to a previous state), push a new state on top of themselves,
/// or switch to a totally different state.
pub enum PlayStateResult {
    /// Pop all play states in reverse order and shut down the program.
    Shutdown,
    /// Close the current play state and pop it from the play state stack.
    Pop,
    /// Push a new play state onto the play state stack.
    Push(Box<dyn PlayState>),
    /// Switch the current play state with a new play state.
    Switch(Box<dyn PlayState>),
}

/// A trait representing a playable game state. This may be a menu, a game session, the title
/// screen, etc.
pub trait PlayState {
    /// Play the state until some change of state is required (i.e: a menu is opened or the game
    /// is closed).
    fn play(&mut self, direction: Direction, global_state: &mut GlobalState) -> PlayStateResult;

    /// Get a descriptive name for this state type.
    fn name(&self) -> &'static str;
}

#[cfg(feature = "discord")]
lazy_static! {
    //Set up discord rich presence
    static ref DISCORD_INSTANCE: Mutex<discord::DiscordState> = {
        discord::run()
    };
}

fn main() {
    // Initialize logging.
    let term_log_level = std::env::var_os("VOXYGEN_LOG")
        .and_then(|env| env.to_str().map(|s| s.to_owned()))
        .and_then(|s| log::LevelFilter::from_str(&s).ok())
        .unwrap_or(log::LevelFilter::Warn);

    let file_log_level = std::env::var_os("VOXYGEN_FILE_LOG")
        .and_then(|env| env.to_str().map(|s| s.to_owned()))
        .and_then(|s| log::LevelFilter::from_str(&s).ok())
        .unwrap_or(log::LevelFilter::Debug);

    // Load the settings
    // Note: This won't log anything due to it being called before ``logging::init``.
    //       The issue is we need to read a setting to decide whether we create a log file or not.
    let settings = Settings::load();

    logging::init(&settings, term_log_level, file_log_level);

    // Save settings to add new fields or create the file if it is not already there
    if let Err(err) = settings.save_to_file() {
        panic!("Failed to save settings: {:?}", err);
    }
    let audio_device = match &settings.audio.audio_device {
        Some(d) => d.to_string(),
        None => audio::get_default_device(),
    };

    let mut audio = if settings.audio.audio_on {
        AudioFrontend::new(audio_device, 16)
    } else {
        AudioFrontend::no_audio()
    };

    audio.set_music_volume(settings.audio.music_volume);
    audio.set_sfx_volume(settings.audio.sfx_volume);

    let mut global_state = GlobalState {
        audio,
        window: Window::new(&settings).expect("Failed to create window!"),
        settings,
        info_message: None,
    };

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
            }
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
            message) to the file {:#?}. Please include the contents of this \
            file in your bug report.
            \n\
            > Error information\n\
            \n\
            The information below is intended for developers and testers.\n\
            \n\
            Panic Payload: {:?}\n\
            PanicInfo: {}",
            // TODO: Verify that this works
            Settings::get_settings_path()
                .join("voxygen-<date>.log")
                .display(),
            reason,
            panic_info,
        );

        error!(
            "VOXYGEN HAS PANICKED\n\n{}\n\nBacktrace:\n{:?}",
            msg,
            backtrace::Backtrace::new(),
        );

        #[cfg(feature = "msgbox")]
        {
            #[cfg(target_os = "macos")]
            dispatch::Queue::main()
                .sync(|| msgbox::create("Voxygen has panicked", &msg, msgbox::IconType::Error));
            #[cfg(not(target_os = "macos"))]
            msgbox::create("Voxygen has panicked", &msg, msgbox::IconType::Error);
        }

        default_hook(panic_info);
    }));

    // Initialise Discord
    #[cfg(feature = "discord")]
    {
        use discord::DiscordUpdate;
        discord::send_all(vec![
            DiscordUpdate::Details("Menu".into()),
            DiscordUpdate::State("Idling".into()),
            DiscordUpdate::LargeImg("bg_main".into()),
        ]);
    }

    // Set up the initial play state.
    let mut states: Vec<Box<dyn PlayState>> = vec![Box::new(MainMenuState::new(&mut global_state))];
    states
        .last()
        .map(|current_state| debug!("Started game with state '{}'", current_state.name()));

    // What's going on here?
    // ---------------------
    // The state system used by Voxygen allows for the easy development of stack-based menus.
    // For example, you may want a "title" state that can push a "main menu" state on top of it,
    // which can in turn push a "settings" state or a "game session" state on top of it.
    // The code below manages the state transfer logic automatically so that we don't have to
    // re-engineer it for each menu we decide to add to the game.
    let mut direction = Direction::Forwards;
    while let Some(state_result) = states
        .last_mut()
        .map(|last| last.play(direction, &mut global_state))
    {
        // Implement state transfer logic.
        match state_result {
            PlayStateResult::Shutdown => {
                direction = Direction::Backwards;
                debug!("Shutting down all states...");
                while states.last().is_some() {
                    states.pop().map(|old_state| {
                        debug!("Popped state '{}'.", old_state.name());
                        global_state.on_play_state_changed();
                    });
                }
            }
            PlayStateResult::Pop => {
                direction = Direction::Backwards;
                states.pop().map(|old_state| {
                    debug!("Popped state '{}'.", old_state.name());
                    global_state.on_play_state_changed();
                });
            }
            PlayStateResult::Push(new_state) => {
                direction = Direction::Forwards;
                debug!("Pushed state '{}'.", new_state.name());
                states.push(new_state);
                global_state.on_play_state_changed();
            }
            PlayStateResult::Switch(mut new_state) => {
                direction = Direction::Forwards;
                states.last_mut().map(|old_state| {
                    debug!(
                        "Switching to state '{}' from state '{}'.",
                        new_state.name(),
                        old_state.name()
                    );
                    mem::swap(old_state, &mut new_state);
                    global_state.on_play_state_changed();
                });
            }
        }
    }

    //Properly shutdown discord thread
    #[cfg(feature = "discord")]
    {
        match DISCORD_INSTANCE.lock() {
            Ok(mut disc) => {
                let _ = disc.tx.send(discord::DiscordUpdate::Shutdown);
                match disc.thread.take() {
                    Some(th) => {
                        let _ = th.join();
                    }
                    None => {
                        error!("couldn't gracefully shutdown discord thread");
                    }
                }
            }
            Err(e) => error!("couldn't gracefully shutdown discord thread: {}", e),
        }
    }

    // Save any unsaved changes to settings
    global_state.settings.save_to_file_warn();
}
