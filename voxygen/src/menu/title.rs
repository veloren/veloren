// Library
use vek::*;

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

            global_state.window.render_ctx_mut().clear(Rgba::new(
                0.0,
                0.3,
                1.0,
                1.0,
            ));
            global_state.window.render_ctx_mut().flush_and_cleanup();
            global_state.window.swap_buffers();
        }

        StateResult::Close
    }
}
