#![feature(drain_filter)]
#![recursion_limit="2048"]

pub mod anim;
pub mod error;
pub mod hud;
pub mod key_state;
pub mod menu;
pub mod mesh;
pub mod render;
pub mod scene;
pub mod session;
pub mod ui;
pub mod window;

// Reexports
pub use crate::error::Error;

// Standard
use std::mem;

// Library
use log;
use pretty_env_logger;

// Crate
use crate::{
    menu::main::MainMenuState,
    window::Window,
};

/// A type used to store state that is shared between all play states
pub struct GlobalState {
    window: Window,
}

impl GlobalState {
    /// Called after a change in play state has occured (usually used to reverse any temporary
    /// effects a state may have made).
    pub fn on_play_state_changed(&mut self) {
        self.window.grab_cursor(false);
        self.window.needs_refresh_resize();
    }
}

// States can either close (and revert to a previous state), push a new state on top of themselves,
// or switch to a totally different state
pub enum PlayStateResult {
    /// Pop all play states in reverse order and shut down the program
    Shutdown,
    /// Close the current play state and pop it from the play state stack
    Pop,
    /// Push a new play state onto the play state stack
    Push(Box<dyn PlayState>),
    /// Switch the current play state with a new play state
    Switch(Box<dyn PlayState>),
}

/// A trait representing a playable game state. This may be a menu, a game session, the title
/// screen, etc.
pub trait PlayState {
    /// Play the state until some change of state is required (i.e: a menu is opened or the game
    /// is closed).
    fn play(&mut self, global_state: &mut GlobalState) -> PlayStateResult;

    /// Get a descriptive name for this state type
    fn name(&self) -> &'static str;
}

fn main() {
    // Init logging
    pretty_env_logger::init();

    // Set up the global state
    let mut global_state = GlobalState {
        window: Window::new()
            .expect("Failed to create window"),
    };

    // Set up the initial play state
    let mut states: Vec<Box<dyn PlayState>> = vec![Box::new(MainMenuState::new(
        &mut global_state.window,
    ))];
    states.last().map(|current_state| {
        log::info!("Started game with state '{}'", current_state.name())
    });

    // What's going on here?
    // ---------------------
    // The state system used by Voxygen allows for the easy development of stack-based menus.
    // For example, you may want a "title" state that can push a "main menu" state on top of it,
    // which can in turn push a "settings" state or a "game session" state on top of it.
    // The code below manages the state transfer logic automatically so that we don't have to
    // re-engineer it for each menu we decide to add to the game.
    while let Some(state_result) = states.last_mut().map(|last| last.play(&mut global_state)){
        // Implement state transfer logic
        match state_result {
            PlayStateResult::Shutdown => {
                log::info!("Shutting down all states...");
                while states.last().is_some() {
                    states.pop().map(|old_state| {
                        log::info!("Popped state '{}'", old_state.name());
                        global_state.on_play_state_changed();
                    });
                }
            },
            PlayStateResult::Pop => {
                states.pop().map(|old_state| {
                    log::info!("Popped state '{}'", old_state.name());
                    global_state.on_play_state_changed();
                });
            },
            PlayStateResult::Push(new_state) => {
                log::info!("Pushed state '{}'", new_state.name());
                states.push(new_state);
                global_state.on_play_state_changed();
            },
            PlayStateResult::Switch(mut new_state) => {
                states.last_mut().map(|old_state| {
                    log::info!("Switching to state '{}' from state '{}'", new_state.name(), old_state.name());
                    mem::swap(old_state, &mut new_state);
                    global_state.on_play_state_changed();
                });
            },
        }
    }
}
