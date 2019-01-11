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

const BG_COLOR: Rgba<f32> = Rgba { r: 0.0, g: 0.3, b: 1.0, a: 1.0 };

impl PlayState for TitleState {
    fn play(&mut self, global_state: &mut GlobalState) -> PlayStateResult {
        'eventloop: loop {
            // Process window events
            for event in global_state.window.fetch_events() {
                match event {
                    Event::Close => break 'eventloop PlayStateResult::Shutdown,
                }
            }

            global_state.window.renderer_mut().clear(BG_COLOR);
            global_state.window.renderer_mut().flush();
            global_state.window.display()
                .expect("Failed to display window");
        }
    }
}
