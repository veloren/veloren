use super::{client_init::ClientInit, DEFAULT_PORT};
use crate::{
    menu::char_selection::CharSelectionState, singleplayer::Singleplayer, Direction, GlobalState,
    PlayState, PlayStateResult,
};
use common::comp;

pub struct StartSingleplayerState {
    singleplayer: Singleplayer,
}

impl StartSingleplayerState {
    /// Create a new `MainMenuState`
    pub fn new() -> Self {
        Self {
            singleplayer: Singleplayer::new(),
        }
    }
}

impl PlayState for StartSingleplayerState {
    fn play(&mut self, direction: Direction, global_state: &mut GlobalState) -> PlayStateResult {
        match direction {
            Direction::Forwards => {
                let username = "singleplayer".to_owned();
                let server_address = "localhost".to_owned();

                let client_init = ClientInit::new(
                    (server_address.clone(), DEFAULT_PORT, false),
                    (comp::Player::new(username.clone()), 300),
                );

                // Client creation
                let client = loop {
                    match client_init.poll() {
                        Some(Ok(client)) => break client,
                        // Should always work
                        Some(Err(err)) => {},
                        _ => {}
                    }
                };

                let mut net_settings = &mut global_state.settings.networking;
                net_settings.username = username.clone();
                if !net_settings.servers.contains(&server_address) {
                    net_settings.servers.push(server_address.clone());
                }
                global_state.settings.save_to_file();

                PlayStateResult::Push(Box::new(CharSelectionState::new(
                    &mut global_state.window,
                    std::rc::Rc::new(std::cell::RefCell::new(client)),
                )))
            }
            Direction::Backwards => PlayStateResult::Pop,
        }
    }

    fn name(&self) -> &'static str {
        "Starting Singleplayer"
    }
}
