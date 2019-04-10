mod ui;

use super::char_selection::CharSelectionState;
use crate::{
    window::{Event, Window},
    GlobalState, PlayState, PlayStateResult,
};
use client::{self, Client};
use common::{
    comp,
    clock::Clock,
};
use std::time::Duration;
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

            // Maintain the UI (TODO: Maybe clean this up a little to avoid rightward drift?)
            for event in self.main_menu_ui.maintain(global_state.window.renderer_mut()) {
                match event {
                    MainMenuEvent::LoginAttempt{ username, server_address } => {
                        use std::net::ToSocketAddrs;
                        const DEFAULT_PORT: u16 = 59003;
                        // Parses ip address or resolves hostname
                        // Note: if you use an ipv6 address the number after the last colon will be used as the port unless you use [] around the address
                        match server_address.to_socket_addrs().or((server_address.as_str(), DEFAULT_PORT).to_socket_addrs()) {
                            Ok(mut socket_adders) => {
                                while let Some(socket_addr) = socket_adders.next() {
                                    // TODO: handle error
                                    match Client::new(socket_addr, comp::Player::new(username.clone()), Some(comp::Character::test()), 300) {
                                        Ok(client) => {
                                            return PlayStateResult::Push(
                                                Box::new(CharSelectionState::new(
                                                    &mut global_state.window,
                                                    std::rc::Rc::new(std::cell::RefCell::new(client)) // <--- TODO: Remove this
                                                ))
                                            );
                                        }
                                        Err(client::Error::Network(_)) => {} // assume connection failed and try next address
                                        Err(err) => {
                                             panic!("Unexpected non Network error when creating client: {:?}", err);
                                        }
                                    }
                                }
                                // Parsing/host name resolution successful but no connection succeeded
                                self.main_menu_ui.login_error("Could not connect to address".to_string());
                            }
                            Err(err) => {
                                // Error parsing input string or error resolving host name
                                self.main_menu_ui.login_error("No such host is known".to_string());
                            }
                        }
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
