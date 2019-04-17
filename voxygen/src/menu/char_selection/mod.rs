mod ui;

use crate::{
    window::{Event, Window},
    session::SessionState,
    GlobalState, PlayState, PlayStateResult,
};
use client::{self, Client};
use common::clock::Clock;
use std::{cell::RefCell, rc::Rc, time::Duration};
use ui::CharSelectionUi;
use vek::*;

const FPS: u64 = 60;

pub struct CharSelectionState {
    char_selection_ui: CharSelectionUi,
    client: Rc<RefCell<Client>>,
}

impl CharSelectionState {
    /// Create a new `CharSelectionState`
    pub fn new(window: &mut Window, client: Rc<RefCell<Client>>) -> Self {
        Self {
            char_selection_ui: CharSelectionUi::new(window),
            client,
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

impl PlayState for CharSelectionState {
    fn play(&mut self, global_state: &mut GlobalState) -> PlayStateResult {
        // Set up an fps clock
        let mut clock = Clock::new();

        loop {
            // Handle window events
            for event in global_state.window.fetch_events() {
                match event {
                    Event::Close => {
                        global_state.singleplayer = None;
                        return PlayStateResult::Shutdown;
                    },
                    // Pass events to ui
                    Event::Ui(event) => {
                        self.char_selection_ui.handle_event(event);
                    }
                    // Ignore all other events
                    _ => {}
                }
            }

            global_state.window.renderer_mut().clear(BG_COLOR);

            // Maintain the UI
            for event in self.char_selection_ui.maintain(global_state.window.renderer_mut()) {
                match event {
                    ui::Event::Logout => {
                        global_state.singleplayer = None;
                        return PlayStateResult::Pop;
                    },
                    ui::Event::Play => return PlayStateResult::Push(
                        Box::new(SessionState::new(&mut global_state.window, self.client.clone()))
                    ),
                }
            }

            // Draw the UI to the screen
            self.char_selection_ui.render(global_state.window.renderer_mut());

            // Tick the client (currently only to keep the connection alive)
            self.client.borrow_mut().tick(client::Input::default(), clock.get_last_delta())
                .expect("Failed to tick the client");
            self.client.borrow_mut().cleanup();

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
