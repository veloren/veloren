#![deny(unsafe_code)]
#![allow(incomplete_features)]
#![allow(
    clippy::identity_op,
    clippy::option_map_unit_fn,
    clippy::needless_pass_by_ref_mut //until we find a better way for specs
)]
#![deny(clippy::clone_on_ref_ptr)]
#![feature(
    extract_if,
    trait_alias,
    option_get_or_insert_default,
    map_try_insert,
    slice_as_chunks,
    let_chains,
    generic_const_exprs,
    maybe_uninit_uninit_array,
    maybe_uninit_array_assume_init,
    closure_lifetime_binder
)]
#![recursion_limit = "2048"]

#[macro_use]
pub mod ui;
pub mod audio;
pub mod cmd;
pub mod controller;
mod credits;
#[cfg(feature = "discord")] pub mod discord;
mod ecs;
pub mod error;
pub mod game_input;
pub mod hud;
pub mod key_state;
pub mod menu;
pub mod mesh;
pub mod panic_handler;
pub mod profile;
pub mod render;
pub mod run;
pub mod scene;
pub mod session;
pub mod settings;
#[cfg(feature = "singleplayer")]
pub mod singleplayer;
pub mod window;

#[cfg(feature = "singleplayer")]
use crate::singleplayer::Singleplayer;
#[cfg(feature = "singleplayer")]
use crate::singleplayer::SingleplayerState;
#[cfg(feature = "egui-ui")]
use crate::ui::egui::EguiState;
use crate::{
    audio::AudioFrontend,
    profile::Profile,
    render::{Drawer, GlobalsBindGroup},
    settings::Settings,
    window::{Event, Window},
};
use common::clock::Clock;
use common_base::span;
use i18n::LocalizationHandle;
use std::path::PathBuf;

use std::sync::Arc;
use tokio::runtime::Runtime;

/// A type used to store state that is shared between all play states.
pub struct GlobalState {
    pub userdata_dir: PathBuf,
    pub config_dir: PathBuf,
    pub settings: Settings,
    pub profile: Profile,
    pub window: Window,
    pub tokio_runtime: Arc<Runtime>,
    #[cfg(feature = "egui-ui")]
    pub egui_state: EguiState,
    pub lazy_init: scene::terrain::SpriteRenderContextLazy,
    pub audio: AudioFrontend,
    pub info_message: Option<String>,
    pub clock: Clock,
    #[cfg(feature = "singleplayer")]
    pub singleplayer: SingleplayerState,
    // TODO: redo this so that the watcher doesn't have to exist for reloading to occur
    pub i18n: LocalizationHandle,
    pub clipboard: iced_winit::Clipboard,
    // Used to clear the shadow textures when entering a PlayState that doesn't utilise shadows
    pub clear_shadows_next_frame: bool,
    /// A channel that sends Discord activity updates to a background task
    #[cfg(feature = "discord")]
    pub discord: crate::discord::Discord,
}

impl GlobalState {
    /// Called after a change in play state has occurred (usually used to
    /// reverse any temporary effects a state may have made).
    pub fn on_play_state_changed(&mut self) {
        self.window.grab_cursor(false);
        self.window.needs_refresh_resize();
    }

    pub fn maintain(&mut self, dt: std::time::Duration) {
        span!(_guard, "maintain", "GlobalState::maintain");
        self.audio.maintain(dt);
        self.window.renderer().maintain()
    }

    #[cfg(feature = "singleplayer")]
    pub fn paused(&self) -> bool {
        self.singleplayer
            .as_running()
            .map_or(false, Singleplayer::is_paused)
    }

    #[cfg(not(feature = "singleplayer"))]
    pub fn paused(&self) -> bool { false }

    #[cfg(feature = "singleplayer")]
    pub fn unpause(&self) { self.singleplayer.as_running().map(|s| s.pause(false)); }

    #[cfg(feature = "singleplayer")]
    pub fn pause(&self) { self.singleplayer.as_running().map(|s| s.pause(true)); }
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

    /// Determines whether the play state should have an enforced FPS cap
    fn capped_fps(&self) -> bool;

    fn globals_bind_group(&self) -> &GlobalsBindGroup;

    /// Draw the play state.
    fn render(&self, drawer: &mut Drawer<'_>, settings: &Settings);

    /// Determines whether egui will be rendered for this play state
    fn egui_enabled(&self) -> bool;
}
