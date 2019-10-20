use super::client_init::ClientInit;
use crate::{
    menu::char_selection::CharSelectionState, singleplayer::Singleplayer, Direction, GlobalState,
    PlayState, PlayStateResult,
};
use common::comp;
use log::warn;
use server::settings::ServerSettings;

pub struct StartSingleplayerState {
    // Necessary to keep singleplayer working
    _singleplayer: Singleplayer,
    server_settings: ServerSettings,
}

impl StartSingleplayerState {
    /// Create a new `MainMenuState`.
    pub fn new() -> Self {
        let (_singleplayer, server_settings) = Singleplayer::new(None); // TODO: Make client and server use the same thread pool

        Self {
            _singleplayer,
            server_settings,
        }
    }
}

impl PlayState for StartSingleplayerState {
    fn play(&mut self, direction: Direction, global_state: &mut GlobalState) -> PlayStateResult {
        match direction {
            Direction::Forwards => {
                let username = "singleplayer".to_owned();

                let client_init = ClientInit::new(
                    //TODO: check why we are converting out IP:Port to String instead of parsing it directly as SockAddr
                    (
                        self.server_settings.gameserver_address.ip().to_string(),
                        self.server_settings.gameserver_address.port(),
                        true,
                    ),
                    comp::Player::new(
                        username.clone(),
                        Some(global_state.settings.graphics.view_distance),
                    ),
                    String::default(),
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

                // Print the metrics port
                println!(
                    "Metrics port: {}",
                    self.server_settings.metrics_address.port()
                );

                PlayStateResult::Push(Box::new(CharSelectionState::new(
                    global_state,
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
