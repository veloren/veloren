use super::{client_init::ClientInit, DEFAULT_PORT};
use crate::{
    menu::char_selection::CharSelectionState, singleplayer::Singleplayer, Direction, GlobalState,
    PlayState, PlayStateResult,
};
use common::comp;
use client::Client;
use log::warn;
use std::net::SocketAddr;

pub struct StartSingleplayerState {
    singleplayer: Singleplayer,
    sock: SocketAddr,
}

impl StartSingleplayerState {
    /// Create a new `MainMenuState`.
    pub fn new() -> Self {
        let (singleplayer, sock) = Singleplayer::new(None); // TODO: Make client and server use the same thread pool

        Self { singleplayer, sock }
    }
}

impl PlayState for StartSingleplayerState {
    fn play(&mut self, direction: Direction, global_state: &mut GlobalState) -> PlayStateResult {
        match direction {
            Direction::Forwards => {
                let username = "singleplayer".to_owned();
                let server_address = self.sock.ip().to_string();

                let client_init = ClientInit::new(
                    (server_address.clone(), self.sock.port(), true),
                    comp::Player::new(
                        username.clone(),
                        Some(global_state.settings.graphics.view_distance),
                    ),
                    true,
                );

                // Create the client.
                let client = loop {
                    match client_init.poll() {
                        Some(Ok(client)) => break client,
                        Some(Err(err)) => {
                            warn!("Failed to start single-player server: {:?}", err);
                            return PlayStateResult::Pop;
                        }
                        _ => {}
                    }
                };

                PlayStateResult::Push(Box::new(CharSelectionState::new(
                    &mut global_state.window,
                    std::rc::Rc::new(std::cell::RefCell::new(client)),
                )))
            }
            Direction::Backwards => PlayStateResult::Pop,
        }
    }

    fn name(&self) -> &'static str {
        "Starting Single-Player"
    }
}
