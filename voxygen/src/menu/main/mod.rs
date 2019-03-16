mod ui;

use crate::{
    session::SessionState,
    window::{Event, Window},
    GlobalState, PlayState, PlayStateResult,
};
use common::clock::Clock;
use std::time::Duration;
use ui::MainMenuUi;
use vek::*;

const FPS: u64 = 60;

pub struct MainMenuState {
    main_menu_ui: MainMenuUi,
}

impl MainMenuState {
    /// Create a new `MainMenuState`
    pub fn new(window: &mut Window) -> Self {
        Self {
            main_menu_ui: MainMenuUi::new(window),
        }
    }
}

// Background colour
const BG_COLOR: Rgba<f32> = Rgba {
    r: 0.0,
    g: 0.3,
    b: 1.0,
    a: 1.0,
};

impl PlayState for MainMenuState {
    fn play(&mut self, global_state: &mut GlobalState) -> PlayStateResult {
        // Set up an fps clock
        let mut clock = Clock::new();

        loop {
            // Handle window events
            for event in global_state.window.fetch_events() {
                match event {
                    Event::Close => return PlayStateResult::Shutdown,
                    // Pass events to ui
                    Event::UiEvent(input) => {
                        self.main_menu_ui.handle_event(input);
                    }
                    // Ignore all other events
                    _ => {}
                }
            }

            global_state.window.renderer_mut().clear(BG_COLOR);

            // Maintain the UI
            self.main_menu_ui
                .maintain(global_state.window.renderer_mut());
            // Check if there should be a login attempt
            if let Some((username, address)) = self.main_menu_ui.login_attempt() {
                // For now just start a new session
                return PlayStateResult::Push(
                    Box::new(SessionState::new(&mut global_state.window).unwrap()) // TODO: Handle this error
                );
            }

            // Draw the UI to the screen
            self.main_menu_ui.render(global_state.window.renderer_mut());

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
