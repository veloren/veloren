mod ui;

use super::main::MainMenuState;
use crate::{
    window::{Event, Window},
    GlobalState, PlayState, PlayStateResult,
};
use common::clock::Clock;
use std::time::Duration;
use ui::TitleUi;
use vek::*;

const FPS: u64 = 60;

pub struct TitleState {
    title_ui: TitleUi,
}

impl TitleState {
    /// Create a new `TitleState`
    pub fn new(window: &mut Window) -> Self {
        Self {
            title_ui: TitleUi::new(window),
        }
    }
}

// The background colour
const BG_COLOR: Rgba<f32> = Rgba {
    r: 0.0,
    g: 0.3,
    b: 1.0,
    a: 1.0,
};

impl PlayState for TitleState {
    fn play(&mut self, global_state: &mut GlobalState) -> PlayStateResult {
        // Set up an fps clock
        let mut clock = Clock::new();

        loop {
            // Handle window events
            for event in global_state.window.fetch_events() {
                match event {
                    Event::Close => return PlayStateResult::Shutdown,
                    // When any key is pressed, go to the main menu
                    Event::Char(_) => {
                        return PlayStateResult::Push(Box::new(MainMenuState::new(
                            &mut global_state.window,
                        )));
                    }
                    // Pass events to ui
                    Event::UiEvent(input) => {
                        self.title_ui.handle_event(input);
                    }
                    // Ignore all other events
                    _ => {}
                }
            }

            global_state.window.renderer_mut().clear(BG_COLOR);

            // Maintain the UI
            self.title_ui.maintain(global_state.window.renderer_mut());

            // Draw the UI to the screen
            self.title_ui.render(global_state.window.renderer_mut());

            // Finish the frame
            global_state.window.renderer_mut().flush();
            global_state
                .window
                .swap_buffers()
                .expect("Failed to swap window buffers");

            // Wait for the next tick
            clock.tick(Duration::from_millis(1000 / FPS));
        }
    }

    fn name(&self) -> &'static str {
        "Title"
    }
}
