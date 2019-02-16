// Library
use vek::*;
use image;


// Crate
use crate::{
    PlayState,
    PlayStateResult,
    GlobalState,
    window::{
        Event,
        Window,
    },
    session::SessionState,
    ui::title::TitleUi,
};


pub struct TitleState {
    title_ui: TitleUi,
}

impl TitleState {
    /// Create a new `TitleState`
    pub fn new(window: &mut Window) -> Self {
        Self {
            title_ui: TitleUi::new(window)
        }
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
                        Box::new(SessionState::new(&mut global_state.window).unwrap()), // TODO: Handle this error
                    ),
                    // Pass events to ui
                    Event::UiEvent(input) => {
                        self.title_ui.handle_event(input);
                    }
                    // Ignore all other events
                    _ => {},
                }
            }

            global_state.window.renderer_mut().clear(BG_COLOR);

            // Maintain the UI
            self.title_ui.maintain(global_state.window.renderer_mut());

            // Draw the UI to the screen
            self.title_ui.render(global_state.window.renderer_mut());

            // Finish the frame
            global_state.window.renderer_mut().flush();
            global_state.window
            .swap_buffers()
            .expect("Failed to swap window buffers");

        }
    }

    fn name(&self) -> &'static str { "Title" }
}
