mod menu;
mod render;
mod window;

// Standard
use std::{
    any,
    mem,
};

// Library
use glutin;
use failure;

// Crate
use crate::{
    menu::title::TitleState,
    window::Window,
    render::RenderErr,
};

#[derive(Debug)]
pub enum VoxygenErr {
    BackendErr(Box<any::Any>),
    RenderErr(RenderErr),
    Other(failure::Error),
}

impl From<RenderErr> for VoxygenErr {
    fn from(err: RenderErr) -> Self {
        VoxygenErr::RenderErr(err)
    }
}

// A type used to store state that is shared between all play states
pub struct GlobalState {
    window: Window,
}

// States can either close (and revert to a previous state), push a new state on top of themselves,
// or switch to a totally different state
pub enum PlayStateResult {
    /// Pop all play states in reverse order and shut down the program
    Shutdown,
    /// Close the current play state
    Close,
    /// Push a new play state onto the play state stack
    Push(Box<dyn PlayState>),
    /// Switch the current play state with a new play state
    Switch(Box<dyn PlayState>),
}

pub trait PlayState {
    fn play(&mut self, global_state: &mut GlobalState) -> PlayStateResult;
}

fn main() {
    let mut states: Vec<Box<dyn PlayState>> = vec![Box::new(TitleState::new())];

    let mut global_state = GlobalState {
        window: Window::new()
            .expect("Failed to create window"),
    };

    while let Some(state_result) = states.last_mut().map(|last| last.play(&mut global_state)){
        // Implement state transfer logic
        match state_result {
            PlayStateResult::Shutdown => while states.last().is_some() {
                states.pop();
            },
            PlayStateResult::Close => {
                states.pop();
            },
            PlayStateResult::Push(new_state) => {
                states.push(new_state);
            },
            PlayStateResult::Switch(mut new_state) => {
                states.last_mut().map(|old_state| mem::swap(old_state, &mut new_state));
            },
        }
    }
}
