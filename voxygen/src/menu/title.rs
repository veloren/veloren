// Library
use vek::*;

// Crate
use crate::{
    PlayState,
    PlayStateResult,
    GlobalState,
    window::Event,
    render,
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
const BG_COLOR: Rgba<f32> = Rgba { r: 0.8, g: 1.0, b: 0.8, a: 1.0 };

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
            global_state.window.display()
                .expect("Failed to display window");
        }
    }

    fn name(&self) -> &'static str { "Title" }
}
