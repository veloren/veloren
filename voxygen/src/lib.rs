#![deny(unsafe_code)]
#![feature(drain_filter)]
#![recursion_limit = "2048"]

#[macro_use]
pub mod ui;
pub mod anim;
pub mod audio;
mod ecs;
pub mod error;
pub mod hud;
pub mod i18n;
pub mod key_state;
pub mod logging;
pub mod menu;
pub mod mesh;
pub mod meta;
pub mod render;
pub mod scene;
pub mod session;
pub mod settings;
#[cfg(feature = "singleplayer")]
pub mod singleplayer;
pub mod window;

// Reexports
pub use crate::error::Error;

use crate::{
    audio::AudioFrontend, meta::Meta, settings::Settings, singleplayer::Singleplayer,
    window::Window,
};

/// A type used to store state that is shared between all play states.
pub struct GlobalState {
    pub settings: Settings,
    pub meta: Meta,
    pub window: Window,
    pub audio: AudioFrontend,
    pub info_message: Option<String>,
    pub singleplayer: Option<Singleplayer>,
}

impl GlobalState {
    /// Called after a change in play state has occurred (usually used to
    /// reverse any temporary effects a state may have made).
    pub fn on_play_state_changed(&mut self) {
        self.window.grab_cursor(false);
        self.window.needs_refresh_resize();
    }

    pub fn maintain(&mut self, dt: f32) { self.audio.maintain(dt); }
}

pub enum Direction {
    Forwards,
    Backwards,
}

/// States can either close (and revert to a previous state), push a new state
/// on top of themselves, or switch to a totally different state.
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

/// A trait representing a playable game state. This may be a menu, a game
/// session, the title screen, etc.
pub trait PlayState {
    /// Play the state until some change of state is required (i.e: a menu is
    /// opened or the game is closed).
    fn play(&mut self, direction: Direction, global_state: &mut GlobalState) -> PlayStateResult;

    /// Get a descriptive name for this state type.
    fn name(&self) -> &'static str;
}
