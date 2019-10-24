mod scene;
mod ui;

use crate::{
    session::SessionState, window::Event as WinEvent, Direction, GlobalState, PlayState,
    PlayStateResult,
};
use client::{self, Client};
use common::{clock::Clock, comp, msg::ClientState};
use log::error;
use scene::Scene;
use std::{cell::RefCell, rc::Rc, time::Duration};
use ui::CharSelectionUi;

pub struct CharSelectionState {
    char_selection_ui: CharSelectionUi,
    client: Rc<RefCell<Client>>,
    scene: Scene,
}

impl CharSelectionState {
    /// Create a new `CharSelectionState`.
    pub fn new(global_state: &mut GlobalState, client: Rc<RefCell<Client>>) -> Self {
        Self {
            char_selection_ui: CharSelectionUi::new(global_state),
            client,
            scene: Scene::new(global_state.window.renderer_mut()),
        }
    }
}

impl PlayState for CharSelectionState {
    fn play(&mut self, _: Direction, global_state: &mut GlobalState) -> PlayStateResult {
        // Set up an fps clock.
        let mut clock = Clock::start();

        let mut current_client_state = self.client.borrow().get_client_state();
        while let ClientState::Pending | ClientState::Registered = current_client_state {
            // Handle window events
            for event in global_state.window.fetch_events() {
                if self.char_selection_ui.handle_event(event.clone()) {
                    continue;
                }
                match event {
                    WinEvent::Close => {
                        return PlayStateResult::Shutdown;
                    }
                    // Pass all other events to the scene
                    event => {
                        self.scene.handle_input_event(event);
                    } // TODO: Do something if the event wasn't handled?
                }
            }

            global_state.window.renderer_mut().clear();

            // Maintain the UI.
            let events = self
                .char_selection_ui
                .maintain(global_state.window.renderer_mut(), &self.client.borrow());
            for event in events {
                match event {
                    ui::Event::Logout => {
                        return PlayStateResult::Pop;
                    }
                    ui::Event::Play => {
                        self.client.borrow_mut().request_character(
                            self.char_selection_ui.character_name.clone(),
                            comp::Body::Humanoid(self.char_selection_ui.character_body),
                            self.char_selection_ui.character_tool,
                        );
                        return PlayStateResult::Push(Box::new(SessionState::new(
                            global_state,
                            self.client.clone(),
                        )));
                    }
                }
            }

            // Maintain global state.
            global_state.maintain(clock.get_last_delta().as_secs_f32());

            // Maintain the scene.
            self.scene.maintain(
                global_state.window.renderer_mut(),
                &self.client.borrow(),
                self.char_selection_ui.character_body,
            );

            // Render the scene.
            self.scene.render(
                global_state.window.renderer_mut(),
                &self.client.borrow(),
                self.char_selection_ui.character_body,
                &comp::Equipment {
                    main: if let Some(kind) = self.char_selection_ui.character_tool {
                        Some(comp::Item::Tool {
                            kind: kind,
                            power: 10,
                            stamina: 0,
                            strength: 0,
                            dexterity: 0,
                            intelligence: 0,
                        })
                    } else {
                        None
                    },
                    alt: None,
                },
            );

            // Draw the UI to the screen.
            self.char_selection_ui
                .render(global_state.window.renderer_mut(), self.scene.globals());

            // Tick the client (currently only to keep the connection alive).
            if let Err(err) = self
                .client
                .borrow_mut()
                .tick(comp::ControllerInputs::default(), clock.get_last_delta())
            {
                error!("Failed to tick the scene: {:?}", err);
                return PlayStateResult::Pop;
            }
            self.client.borrow_mut().cleanup();

            // Finish the frame.
            global_state.window.renderer_mut().flush();
            global_state
                .window
                .swap_buffers()
                .expect("Failed to swap window buffers");

            // Wait for the next tick.
            clock.tick(Duration::from_millis(
                1000 / (global_state.settings.graphics.max_fps as u64),
            ));

            current_client_state = self.client.borrow().get_client_state();
        }

        PlayStateResult::Pop
    }

    fn name(&self) -> &'static str {
        "Title"
    }
}
