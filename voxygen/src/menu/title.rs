// Library
use vek::*;

// Crate
use crate::{
    PlayState,
    PlayStateResult,
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
    fn play(&mut self, global_state: &mut GlobalState) -> PlayStateResult {
        'eventloop: loop {
            // Process window events
            for event in global_state.window.fetch_events() {
                match event {
                    Event::Close => break 'eventloop PlayStateResult::Shutdown,
                }
            }

            global_state.window.renderer_mut().clear(Rgba::new(
                0.0,
                0.3,
                1.0,
                1.0,
            ));
            global_state.window.renderer_mut().flush();
            global_state.window.display()
                .expect("Failed to display window");
        }
    }
}
