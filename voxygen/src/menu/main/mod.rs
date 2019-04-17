mod client_init;
mod ui;

use super::char_selection::CharSelectionState;
use crate::{
    window::{Event, Window},
    GlobalState, PlayState, PlayStateResult,
    singleplayer::Singleplayer,
};
use client_init::{ClientInit, Error as InitError};
use common::{clock::Clock, comp};
use std::time::Duration;
use std::thread;
use ui::{Event as MainMenuEvent, MainMenuUi};
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

        // Used for client creation
        let mut client_init: Option<ClientInit> = None;

        loop {
            // Handle window events
            for event in global_state.window.fetch_events() {
                match event {
                    Event::Close => return PlayStateResult::Shutdown,
                    // Pass events to ui
                    Event::Ui(event) => {
                        self.main_menu_ui.handle_event(event);
                    }
                    // Ignore all other events
                    _ => {}
                }
            }

            global_state.window.renderer_mut().clear(BG_COLOR);

            // Poll client creation
            match client_init.as_ref().and_then(|init| init.poll())  {
                Some(Ok(client)) => {
                    self.main_menu_ui.connected();
                    return PlayStateResult::Push(Box::new(CharSelectionState::new(
                        &mut global_state.window,
                        std::rc::Rc::new(std::cell::RefCell::new(client)),
                    )));
                }
                Some(Err(err)) => {
                    client_init = None;
                    self.main_menu_ui.login_error(match err {
                        InitError::BadAddress(_) | InitError::NoAddress => "No such host is known",
                        InitError::ConnectionFailed(_) => "Could not connect to address",
                    }.to_string());
                },
                None => {}
            }

            // Maintain the UI
            for event in self
                .main_menu_ui
                .maintain(global_state.window.renderer_mut())
            {
                match event {
                    MainMenuEvent::LoginAttempt {
                        username,
                        server_address,
                    } => {
                        const DEFAULT_PORT: u16 = 59003;
                        // Don't try to connect if there is already a connection in progress
                        client_init = client_init.or(Some(ClientInit::new(
                            (server_address, DEFAULT_PORT, false),
                            (
                                comp::Player::new(username.clone()),
                                Some(comp::Character::test()),
                                Some(comp::Animation::Idle),
                                300,
                            ),
                        )));
                    }
                    MainMenuEvent::StartSingleplayer => {
                        global_state.singleplayer = Some(Singleplayer::new());
                    }
                    MainMenuEvent::Quit => return PlayStateResult::Shutdown,
                }
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
