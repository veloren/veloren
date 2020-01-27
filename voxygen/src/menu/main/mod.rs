mod client_init;
#[cfg(feature = "singleplayer")]
mod ui;

use super::char_selection::CharSelectionState;
use crate::{
    i18n::{i18n_asset_key, VoxygenLocalization},
    singleplayer::Singleplayer,
    window::Event,
    Direction, GlobalState, PlayState, PlayStateResult,
};
use argon2::{self, Config};
use client_init::{ClientInit, Error as InitError};
use common::{assets::load_expect, clock::Clock, comp};
use log::warn;
#[cfg(feature = "singleplayer")]
use std::time::Duration;
use ui::{Event as MainMenuEvent, MainMenuUi};

pub struct MainMenuState {
    main_menu_ui: MainMenuUi,
    title_music_channel: Option<usize>,
    singleplayer: Option<Singleplayer>,
}

impl MainMenuState {
    /// Create a new `MainMenuState`.
    pub fn new(global_state: &mut GlobalState) -> Self {
        Self {
            main_menu_ui: MainMenuUi::new(global_state),
            title_music_channel: None,
            singleplayer: None,
        }
    }
}

const DEFAULT_PORT: u16 = 14004;

impl PlayState for MainMenuState {
    fn play(&mut self, _: Direction, global_state: &mut GlobalState) -> PlayStateResult {
        // Set up an fps clock.
        let mut clock = Clock::start();

        // Used for client creation.
        let mut client_init: Option<ClientInit> = None;

        // Kick off title music
        if self.title_music_channel.is_none() && global_state.settings.audio.audio_on {
            self.title_music_channel = global_state
                .audio
                .play_music("voxygen.audio.soundtrack.veloren_title_tune");
        }

        // Reset singleplayer server if it was running already
        self.singleplayer = None;

        loop {
            // Handle window events.
            for event in global_state.window.fetch_events(&mut global_state.settings) {
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

            global_state.window.renderer_mut().clear();

            // Poll client creation.
            match client_init.as_ref().and_then(|init| init.poll()) {
                Some(Ok(mut client)) => {
                    self.main_menu_ui.connected();
                    // Register voxygen components / resources
                    crate::ecs::init(client.state_mut().ecs_mut());
                    return PlayStateResult::Push(Box::new(CharSelectionState::new(
                        global_state,
                        std::rc::Rc::new(std::cell::RefCell::new(client)),
                    )));
                }
                Some(Err(err)) => {
                    client_init = None;
                    global_state.info_message = Some(
                        match err {
                            InitError::BadAddress(_) | InitError::NoAddress => "Server not found",
                            InitError::InvalidAuth => "Invalid credentials",
                            InitError::ServerIsFull => "Server is full",
                            InitError::ConnectionFailed(_) => "Connection failed",
                            InitError::ClientCrashed => "Client crashed",
                        }
                        .to_string(),
                    );
                }
                None => {}
            }

            // Maintain global_state
            global_state.maintain(clock.get_last_delta().as_secs_f32());

            // Maintain the UI.
            for event in self
                .main_menu_ui
                .maintain(global_state, clock.get_last_delta())
            {
                match event {
                    MainMenuEvent::LoginAttempt {
                        username,
                        password,
                        server_address,
                    } => {
                        attempt_login(
                            global_state,
                            username,
                            password,
                            server_address,
                            DEFAULT_PORT,
                            &mut client_init,
                        );
                    }
                    MainMenuEvent::CancelLoginAttempt => {
                        // client_init contains Some(ClientInit), which spawns a thread which contains a TcpStream::connect() call
                        // This call is blocking
                        // TODO fix when the network rework happens
                        self.singleplayer = None;
                        client_init = None;
                        self.main_menu_ui.cancel_connection();
                    }
                    #[cfg(feature = "singleplayer")]
                    MainMenuEvent::StartSingleplayer => {
                        let (singleplayer, server_settings) = Singleplayer::new(None); // TODO: Make client and server use the same thread pool

                        self.singleplayer = Some(singleplayer);

                        attempt_login(
                            global_state,
                            "singleplayer".to_owned(),
                            "".to_owned(),
                            server_settings.gameserver_address.ip().to_string(),
                            server_settings.gameserver_address.port(),
                            &mut client_init,
                        );
                    }
                    MainMenuEvent::Settings => {} // TODO
                    MainMenuEvent::Quit => return PlayStateResult::Shutdown,
                    MainMenuEvent::DisclaimerClosed => {
                        global_state.settings.show_disclaimer = false
                    }
                }
            }
            let localized_strings = load_expect::<VoxygenLocalization>(&i18n_asset_key(
                &global_state.settings.language.selected_language,
            ));

            if let Some(info) = global_state.info_message.take() {
                self.main_menu_ui
                    .show_info(info, localized_strings.get("common.okay").to_owned());
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
            clock.tick(Duration::from_millis(
                1000 / (global_state.settings.graphics.max_fps as u64),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "Title"
    }
}

fn attempt_login(
    global_state: &mut GlobalState,
    username: String,
    password: String,
    server_address: String,
    server_port: u16,
    client_init: &mut Option<ClientInit>,
) {
    let mut net_settings = &mut global_state.settings.networking;
    net_settings.username = username.clone();
    net_settings.password = password.clone();
    if !net_settings.servers.contains(&server_address) {
        net_settings.servers.push(server_address.clone());
    }
    if let Err(err) = global_state.settings.save_to_file() {
        warn!("Failed to save settings: {:?}", err);
    }

    let player = comp::Player::new(
        username.clone(),
        Some(global_state.settings.graphics.view_distance),
    );

    if player.is_valid() {
        // Don't try to connect if there is already a connection in progress.
        if client_init.is_none() {
            *client_init = Some(ClientInit::new(
                (server_address, server_port, false),
                player,
                {
                    let salt = b"staticsalt_zTuGkGvybZIjZbNUDtw15";
                    let config = Config::default();
                    argon2::hash_encoded(password.as_bytes(), salt, &config).unwrap()
                },
            ));
        }
    } else {
        global_state.info_message = Some("Invalid username or password".to_string());
    }
}
