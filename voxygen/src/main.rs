#![deny(unsafe_code)]
#![feature(duration_float, drain_filter)]
#![recursion_limit = "2048"]

#[cfg(feature = "discord")]
#[macro_use]
extern crate lazy_static;

#[cfg(feature = "discord")]
pub mod discord;

#[cfg(feature = "discord")]
use parking_lot::Mutex;

#[macro_use]
pub mod ui;
pub mod anim;
pub mod audio;
pub mod error;
pub mod hud;
pub mod key_state;
pub mod menu;
pub mod mesh;
pub mod render;
pub mod scene;
pub mod session;
pub mod settings;
pub mod singleplayer;
pub mod window;

// Reexports
pub use crate::error::Error;

use crate::{
    audio::base::Genre, audio::AudioFrontend, menu::main::MainMenuState, settings::Settings,
    window::Window,
};
use heaptrack::track_mem;
use log::{self, debug, error, info, warn};

use simplelog::{CombinedLogger, Config, TermLogger, TerminalMode, WriteLogger};
use std::{fs::File, mem, panic, str::FromStr};

track_mem!();

/// A type used to store state that is shared between all play states.
pub struct GlobalState {
    settings: Settings,
    window: Window,
    audio: AudioFrontend,
}

impl GlobalState {
    /// Called after a change in play state has occurred (usually used to reverse any temporary
    /// effects a state may have made).
    pub fn on_play_state_changed(&mut self) {
        self.window.grab_cursor(false);
        self.window.needs_refresh_resize();
    }

    pub fn maintain(&mut self) {
        // TODO: Maintain both `Bgm` and `Sfx` audio threads.
        self.audio.play();
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
    // Load the settings
    let settings = Settings::load();
    // Save settings to add new fields or create the file if it is not already there
    if let Err(err) = settings.save_to_file() {
        panic!("Failed to save settings: {:?}", err);
    }

    // Initialize logging.
    let term_log_level = std::env::var_os("VOXYGEN_LOG")
        .and_then(|env| env.to_str().map(|s| s.to_owned()))
        .and_then(|s| log::LevelFilter::from_str(&s).ok())
        .unwrap_or(log::LevelFilter::Warn);
    CombinedLogger::init(vec![
        TermLogger::new(term_log_level, Config::default(), TerminalMode::Mixed).unwrap(),
        WriteLogger::new(
            log::LevelFilter::Info,
            Config::default(),
            File::create(&settings.log.file).unwrap(),
        ),
    ])
    .unwrap();

    // Set up panic handler to relay swish panic messages to the user
    let settings_clone = settings.clone();
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
            settings_clone.log.file, reason, panic_info,
        );

        error!(
            "VOXYGEN HAS PANICKED\n\n{}\n\nBacktrace:\n{:?}",
            msg,
            backtrace::Backtrace::new(),
        );

        msgbox::create("Voxygen has panicked", &msg, msgbox::IconType::ERROR);

        default_hook(panic_info);
    }));

    // Set up the global state.
    let audio = if settings.audio.audio_on {
        AudioFrontend::new()
    } else {
        AudioFrontend::no_audio()
    };

    let mut global_state = GlobalState {
        audio,
        window: Window::new(&settings).expect("Failed to create window!"),
        settings,
    };

    // Initialize discord. (lazy_static initalise lazily...)
    #[cfg(feature = "discord")]
    {
        match DISCORD_INSTANCE.lock() {
            Ok(_disc) => {
                //great
            }
            Err(e) => log::error!("Couldn't init discord: {}", e),
        }
    }

    match global_state.audio.model.get_genre() {
        Genre::Bgm => {
            global_state.settings.audio.audio_device =
                Some(crate::audio::base::get_default_device())
        }
        Genre::Sfx => unimplemented!(),
        Genre::None => global_state.settings.audio.audio_device = None,
    }

    // Set up the initial play state.
    let mut states: Vec<Box<dyn PlayState>> = vec![Box::new(MainMenuState::new(&mut global_state))];
    states
        .last()
        .map(|current_state| info!("Started game with state '{}'", current_state.name()));

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
                info!("Shutting down all states...");
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
