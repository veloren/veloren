mod client_init;
mod start_singleplayer;
mod ui;

use super::char_selection::CharSelectionState;
use crate::{window::Event, Direction, GlobalState, PlayState, PlayStateResult};
use client_init::{ClientInit, Error as InitError};
use common::{clock::Clock, comp};
use start_singleplayer::StartSingleplayerState;
use std::time::Duration;
use ui::{Event as MainMenuEvent, MainMenuUi};
use vek::*;

const FPS: u64 = 60;

pub struct MainMenuState {
    main_menu_ui: MainMenuUi,
}

impl MainMenuState {
    /// Create a new `MainMenuState`.
    pub fn new(global_state: &mut GlobalState) -> Self {
        Self {
            main_menu_ui: MainMenuUi::new(global_state),
        }
    }
}

const DEFAULT_PORT: u16 = 59003;

// Background colour
const BG_COLOR: Rgba<f32> = Rgba {
    r: 0.0,
    g: 0.3,
    b: 1.0,
    a: 1.0,
};

impl PlayState for MainMenuState {
    fn play(&mut self, _: Direction, global_state: &mut GlobalState) -> PlayStateResult {
        // Set up an fps clock.
        let mut clock = Clock::new();

        // Used for client creation.
        let mut client_init: Option<ClientInit> = None;

        loop {
            // Handle window events.
            for event in global_state.window.fetch_events() {
                match event {
                    Event::Close => return PlayStateResult::Shutdown,
                    // Pass events to ui.
                    Event::Ui(event) => {
                        self.main_menu_ui.handle_event(event);
                    }
                    // Ignore all other events.
                    _ => {}
                }
            }

            global_state.window.renderer_mut().clear(BG_COLOR);

            // Poll client creation.
            match client_init.as_ref().and_then(|init| init.poll()) {
                Some(Ok(client)) => {
                    self.main_menu_ui.connected();
                    return PlayStateResult::Push(Box::new(CharSelectionState::new(
                        &mut global_state.window,
                        std::rc::Rc::new(std::cell::RefCell::new(client)),
                    )));
                }
                Some(Err(err)) => {
                    client_init = None;
                    self.main_menu_ui.login_error(
                        match err {
                            InitError::BadAddress(_) | InitError::NoAddress => "Server not found",
                            InitError::ConnectionFailed(_) => "Connection failed",
                            InitError::ClientCrashed => "Client crashed",
                        }
                        .to_string(),
                    );
                }
                None => {}
            }

            // Maintain global_state
            global_state.maintain();

            // Maintain the UI.
            for event in self.main_menu_ui.maintain(global_state) {
                match event {
                    MainMenuEvent::LoginAttempt {
                        username,
                        server_address,
                    } => {
                        let mut net_settings = &mut global_state.settings.networking;
                        net_settings.username = username.clone();
                        if !net_settings.servers.contains(&server_address) {
                            net_settings.servers.push(server_address.clone());
                        }
                        // TODO: Handle this result.
                        global_state
                            .settings
                            .save_to_file()
                            .expect("Failed to save settings!");
                        // Don't try to connect if there is already a connection in progress.
                        client_init = client_init.or(Some(ClientInit::new(
                            (server_address, DEFAULT_PORT, false),
                            comp::Player::new(
                                username.clone(),
                                Some(global_state.settings.graphics.view_distance),
                            ),
                            false,
                        )));
                    }
                    MainMenuEvent::StartSingleplayer => {
                        return PlayStateResult::Push(Box::new(StartSingleplayerState::new()));
                    }
                    MainMenuEvent::Quit => return PlayStateResult::Shutdown,
                    MainMenuEvent::DisclaimerClosed => {
                        global_state.settings.show_disclaimer = false
                    }
                }
            }

            // Draw the UI to the screen.
            self.main_menu_ui.render(global_state.window.renderer_mut());

            // Finish the frame.
            global_state.window.renderer_mut().flush();
            global_state
                .window
                .swap_buffers()
                .expect("Failed to swap window buffers!");

            // Wait for the next tick
            clock.tick(Duration::from_millis(1000 / FPS));
        }
    }

    fn name(&self) -> &'static str {
        "Title"
    }
}
