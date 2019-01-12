// Library
use vek::*;

// Crate
use crate::{
    PlayState,
    PlayStateResult,
    GlobalState,
    window::Event,
    session::SessionState,
};

pub struct TitleState;

impl TitleState {
    /// Create a new `TitleState`
    pub fn new() -> Self {
        Self
    }
}

// The background colour
const BG_COLOR: Rgba<f32> = Rgba { r: 0.0, g: 0.3, b: 1.0, a: 1.0 };

impl PlayState for TitleState {
    fn play(&mut self, global_state: &mut GlobalState) -> PlayStateResult {
        loop {
            // Handle window events
            for event in global_state.window.fetch_events() {
                match event {
                    Event::Close => return PlayStateResult::Shutdown,
                    // When space is pressed, start a session
                    Event::Char(' ') => return PlayStateResult::Push(
                        Box::new(SessionState::from_renderer(global_state.window.renderer_mut())),
                    ),
                    // Ignore all other events
                    _ => {},
                }
            }

            // Clear the screen
            global_state.window.renderer_mut().clear(BG_COLOR);

            // Finish the frame
            global_state.window.renderer_mut().flush();
            global_state.window
                .swap_buffers()
                .expect("Failed to swap window buffers");
        }
    }

    fn name(&self) -> &'static str { "Title" }
}
