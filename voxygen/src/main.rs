mod menu;
mod window;

// Standard
use std::mem;

// Crate
use crate::{
    menu::title::TitleState,
    window::Window,
};

// A type used to store state that is shared between all play states
pub struct GlobalState {
    window: Window,
}

// States can either close (and revert to a previous state), push a new state on top of themselves,
// or switch to a totally different state
pub enum StateResult {
    Close,
    Push(Box<dyn PlayState>),
    Switch(Box<dyn PlayState>),
}

pub trait PlayState {
    fn play(&mut self, global_state: &mut GlobalState) -> StateResult;
}

fn main() {
    let mut states: Vec<Box<dyn PlayState>> = vec![Box::new(TitleState::new())];

    let mut global_state = GlobalState {
        window: Window::new(),
    };

    loop {
        // Implement state transfer logic
        match states.last_mut().map(|last| last.play(&mut global_state)) {
            Some(StateResult::Close) => { states.pop(); },
            Some(StateResult::Push(new_state)) => { states.push(new_state); },
            Some(StateResult::Switch(mut new_state)) => if let Some(old_state) = states.last_mut() {
                mem::swap(old_state, &mut new_state);
            } else {
                break;
            },
            None => break,
        }
    }
}
