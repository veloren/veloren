#![deny(unsafe_code)]
#![allow(clippy::option_map_unit_fn)]
#![feature(drain_filter, bool_to_option)]
#![recursion_limit = "2048"]

#[macro_use]
pub mod ui;
pub mod audio;
pub mod controller;
mod ecs;
pub mod error;
pub mod hud;
pub mod i18n;
pub mod key_state;
pub mod logging;
pub mod menu;
pub mod mesh;
pub mod profile;
pub mod render;
pub mod run;
pub mod scene;
pub mod session;
pub mod settings;
#[cfg(feature = "singleplayer")]
pub mod singleplayer;
pub mod window;

// Reexports
pub use crate::error::Error;

#[cfg(feature = "singleplayer")]
use crate::singleplayer::Singleplayer;
use crate::{
    audio::AudioFrontend,
    profile::Profile,
    render::Renderer,
    settings::Settings,
    window::{Event, Window},
};
use common::{assets::watch, clock::Clock};

/// A type used to store state that is shared between all play states.
pub struct GlobalState {
    pub settings: Settings,
    pub profile: Profile,
    pub window: Window,
    pub audio: AudioFrontend,
    pub info_message: Option<String>,
    pub clock: Clock,
    #[cfg(feature = "singleplayer")]
    pub singleplayer: Option<Singleplayer>,
    // TODO: redo this so that the watcher doesn't have to exist for reloading to occur
    pub localization_watcher: watch::ReloadIndicator,
}

impl GlobalState {
    /// Called after a change in play state has occurred (usually used to
    /// reverse any temporary effects a state may have made).
    pub fn on_play_state_changed(&mut self) {
        self.window.grab_cursor(false);
        self.window.needs_refresh_resize();
    }

    pub fn maintain(&mut self, dt: f32) { self.audio.maintain(dt); }

    #[cfg(feature = "singleplayer")]
    pub fn paused(&self) -> bool {
        self.singleplayer
            .as_ref()
            .map_or(false, Singleplayer::is_paused)
    }

    #[cfg(not(feature = "singleplayer"))]
    pub fn paused(&self) -> bool { false }

    pub fn unpause(&self) {
        #[cfg(feature = "singleplayer")]
        {
            self.singleplayer.as_ref().map(|s| s.pause(false));
        }
    }
}

// TODO: appears to be currently unused by playstates
pub enum Direction {
    Forwards,
    Backwards,
}

/// States can either close (and revert to a previous state), push a new state
/// on top of themselves, or switch to a totally different state.
pub enum PlayStateResult {
    /// Keep running this play state.
    Continue,
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
    /// Called when entering this play state from another
    fn enter(&mut self, global_state: &mut GlobalState, direction: Direction);

    /// Tick the play state
    fn tick(&mut self, global_state: &mut GlobalState, events: Vec<Event>) -> PlayStateResult;

    /// Get a descriptive name for this state type.
    fn name(&self) -> &'static str;

    /// Draw the play state.
    fn render(&mut self, renderer: &mut Renderer, settings: &Settings);
}
