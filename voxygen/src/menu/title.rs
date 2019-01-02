// Crate
use crate::{
    PlayState,
    StateResult,
    GlobalState,
    window::Event,
};

pub struct TitleState;

impl TitleState {
    pub fn new() -> Self {
        Self
    }
}

impl PlayState for TitleState {
    fn play(&mut self, global_state: &mut GlobalState) -> StateResult {
        let mut running = true;
        while running {
            global_state.window.poll_events(|event| match event {
                Event::Close => running = false,
            });

            global_state.window.swap_buffers();
        }

        StateResult::Close
    }
}
