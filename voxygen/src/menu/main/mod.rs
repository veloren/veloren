mod client_init;
mod ui;

use super::char_selection::CharSelectionState;
#[cfg(feature = "singleplayer")]
use crate::singleplayer::Singleplayer;
use crate::{
    i18n::{Localization, LocalizationHandle},
    render::Renderer,
    settings::Settings,
    window::Event,
    Direction, GlobalState, PlayState, PlayStateResult,
};
#[cfg(feature = "singleplayer")]
use client::addr::ConnectionArgs;
use client::{
    error::{InitProtocolError, NetworkConnectError, NetworkError},
    ServerInfo,
};
use client_init::{ClientConnArgs, ClientInit, Error as InitError, Msg as InitMsg};
use common::comp;
use common_base::span;
use std::{fmt::Debug, sync::Arc};
use tokio::runtime;
use tracing::error;
use ui::{Event as MainMenuEvent, MainMenuUi};

pub struct MainMenuState {
    main_menu_ui: MainMenuUi,
    // Used for client creation.
    client_init: Option<ClientInit>,
}

impl MainMenuState {
    /// Create a new `MainMenuState`.
    pub fn new(global_state: &mut GlobalState) -> Self {
        Self {
            main_menu_ui: MainMenuUi::new(global_state),
            client_init: None,
        }
    }
}

impl PlayState for MainMenuState {
    fn enter(&mut self, global_state: &mut GlobalState, _: Direction) {
        // Kick off title music
        if global_state.settings.audio.output.is_enabled() && global_state.audio.music_enabled() {
            global_state.audio.play_title_music();
        }

        // Reset singleplayer server if it was running already
        #[cfg(feature = "singleplayer")]
        {
            global_state.singleplayer = None;
        }

        // Updated localization in case the selected language was changed
        self.main_menu_ui
            .update_language(global_state.i18n, &global_state.settings);
        // Set scale mode in case it was change
        self.main_menu_ui
            .set_scale_mode(global_state.settings.interface.ui_scale);
    }

    #[allow(clippy::single_match)] // TODO: remove when event match has multiple arms
    fn tick(&mut self, global_state: &mut GlobalState, events: Vec<Event>) -> PlayStateResult {
        span!(_guard, "tick", "<MainMenuState as PlayState>::tick");

        // Poll server creation
        #[cfg(feature = "singleplayer")]
        {
            if let Some(singleplayer) = &global_state.singleplayer {
                match singleplayer.receiver.try_recv() {
                    Ok(Ok(runtime)) => {
                        // Attempt login after the server is finished initializing
                        attempt_login(
                            &mut global_state.settings,
                            &mut global_state.info_message,
                            "singleplayer".to_owned(),
                            "".to_owned(),
                            ClientConnArgs::Resolved(ConnectionArgs::Mpsc(14004)),
                            &mut self.client_init,
                            Some(runtime),
                        );
                    },
                    Ok(Err(e)) => {
                        error!(?e, "Could not start server");
                        global_state.singleplayer = None;
                        self.client_init = None;
                        self.main_menu_ui.cancel_connection();
                        self.main_menu_ui.show_info(format!("Error: {:?}", e));
                    },
                    Err(_) => (),
                }
            }
        }

        // Handle window events.
        for event in events {
            // Pass all events to the ui first.
            if self.main_menu_ui.handle_event(event.clone()) {
                continue;
            }

            match event {
                Event::Close => return PlayStateResult::Shutdown,
                // Ignore all other events.
                _ => {},
            }
        }
        // Poll client creation.
        match self.client_init.as_ref().and_then(|init| init.poll()) {
            Some(InitMsg::Done(Ok(mut client))) => {
                self.client_init = None;
                self.main_menu_ui.connected();
                // Register voxygen components / resources
                crate::ecs::init(client.state_mut().ecs_mut());
                return PlayStateResult::Push(Box::new(CharSelectionState::new(
                    global_state,
                    std::rc::Rc::new(std::cell::RefCell::new(client)),
                )));
            },
            Some(InitMsg::Done(Err(err))) => {
                let localized_strings = global_state.i18n.read();
                self.client_init = None;
                global_state.info_message = Some({
                    let err = match err {
                        InitError::NoAddress => {
                            localized_strings.get("main.login.server_not_found").into()
                        },
                        InitError::ClientError {
                            error,
                            mismatched_server_info,
                        } => match error {
                            client::Error::SpecsErr(e) => format!(
                                "{}: {}",
                                localized_strings.get("main.login.internal_error"),
                                e
                            ),
                            client::Error::AuthErr(e) => format!(
                                "{}: {}",
                                localized_strings.get("main.login.authentication_error"),
                                e
                            ),
                            client::Error::Kicked(e) => e,
                            client::Error::TooManyPlayers => {
                                localized_strings.get("main.login.server_full").into()
                            },
                            client::Error::AuthServerNotTrusted => localized_strings
                                .get("main.login.untrusted_auth_server")
                                .into(),
                            client::Error::ServerWentMad => localized_strings
                                .get("main.login.outdated_client_or_server")
                                .into(),
                            client::Error::ServerTimeout => {
                                localized_strings.get("main.login.timeout").into()
                            },
                            client::Error::ServerShutdown => {
                                localized_strings.get("main.login.server_shut_down").into()
                            },
                            client::Error::NotOnWhitelist => {
                                localized_strings.get("main.login.not_on_whitelist").into()
                            },
                            client::Error::Banned(reason) => format!(
                                "{}: {}",
                                localized_strings.get("main.login.banned"),
                                reason
                            ),
                            client::Error::InvalidCharacter => {
                                localized_strings.get("main.login.invalid_character").into()
                            },
                            client::Error::NetworkErr(NetworkError::ConnectFailed(
                                NetworkConnectError::Handshake(InitProtocolError::WrongVersion(_)),
                            )) => get_network_error_text(
                                &localized_strings,
                                localized_strings.get("main.login.network_wrong_version"),
                                mismatched_server_info,
                            ),
                            client::Error::NetworkErr(e) => get_network_error_text(
                                &localized_strings,
                                e,
                                mismatched_server_info,
                            ),
                            client::Error::ParticipantErr(e) => get_network_error_text(
                                &localized_strings,
                                e,
                                mismatched_server_info,
                            ),
                            client::Error::StreamErr(e) => get_network_error_text(
                                &localized_strings,
                                e,
                                mismatched_server_info,
                            ),
                            client::Error::Other(e) => {
                                format!("{}: {}", localized_strings.get("common.error"), e)
                            },
                            client::Error::AuthClientError(e) => match e {
                                // TODO: remove parentheses
                                client::AuthClientError::RequestError(e) => format!(
                                    "{}: {}",
                                    localized_strings.get("main.login.failed_sending_request"),
                                    e
                                ),
                                client::AuthClientError::JsonError(e) => format!(
                                    "{}: {}",
                                    localized_strings.get("main.login.failed_sending_request"),
                                    e
                                ),
                                client::AuthClientError::InsecureSchema => localized_strings
                                    .get("main.login.insecure_auth_scheme")
                                    .into(),
                                client::AuthClientError::ServerError(_, e) => {
                                    String::from_utf8_lossy(&e).to_string()
                                },
                            },
                            client::Error::AuthServerUrlInvalid(e) => {
                                format!(
                                    "{}: https://{}",
                                    localized_strings
                                        .get("main.login.failed_auth_server_url_invalid"),
                                    e
                                )
                            },
                        },
                        InitError::ClientCrashed => {
                            localized_strings.get("main.login.client_crashed").into()
                        },
                    };
                    // Log error for possible additional use later or incase that the error
                    // displayed is cut of.
                    error!("{}", err);
                    err
                });
            },
            Some(InitMsg::IsAuthTrusted(auth_server)) => {
                if global_state
                    .settings
                    .networking
                    .trusted_auth_servers
                    .contains(&auth_server)
                {
                    // Can't fail since we just polled it, it must be Some
                    self.client_init
                        .as_ref()
                        .unwrap()
                        .auth_trust(auth_server, true);
                } else {
                    // Show warning that auth server is not trusted and prompt for approval
                    self.main_menu_ui.auth_trust_prompt(auth_server);
                }
            },
            None => {},
        }

        // Maintain the UI.
        for event in self
            .main_menu_ui
            .maintain(global_state, global_state.clock.dt())
        {
            match event {
                MainMenuEvent::LoginAttempt {
                    username,
                    password,
                    server_address,
                } => {
                    let mut net_settings = &mut global_state.settings.networking;
                    net_settings.username = username.clone();
                    net_settings.default_server = server_address.clone();
                    if !net_settings.servers.contains(&server_address) {
                        net_settings.servers.push(server_address.clone());
                    }
                    global_state.settings.save_to_file_warn();

                    attempt_login(
                        &mut global_state.settings,
                        &mut global_state.info_message,
                        username,
                        password,
                        ClientConnArgs::Host(server_address),
                        &mut self.client_init,
                        None,
                    );
                },
                MainMenuEvent::CancelLoginAttempt => {
                    // client_init contains Some(ClientInit), which spawns a thread which contains a
                    // TcpStream::connect() call This call is blocking
                    // TODO fix when the network rework happens
                    #[cfg(feature = "singleplayer")]
                    {
                        global_state.singleplayer = None;
                    }
                    self.client_init = None;
                    self.main_menu_ui.cancel_connection();
                },
                MainMenuEvent::ChangeLanguage(new_language) => {
                    global_state.settings.language.selected_language =
                        new_language.language_identifier;
                    global_state.i18n = LocalizationHandle::load_expect(
                        &global_state.settings.language.selected_language,
                    );
                    global_state.i18n.read().log_missing_entries();
                    global_state
                        .i18n
                        .set_english_fallback(global_state.settings.language.use_english_fallback);
                    self.main_menu_ui
                        .update_language(global_state.i18n, &global_state.settings);
                },
                #[cfg(feature = "singleplayer")]
                MainMenuEvent::StartSingleplayer => {
                    let singleplayer = Singleplayer::new();

                    global_state.singleplayer = Some(singleplayer);
                },
                MainMenuEvent::Quit => return PlayStateResult::Shutdown,
                // Note: Keeping in case we re-add the disclaimer
                /*MainMenuEvent::DisclaimerAccepted => {
                    global_state.settings.show_disclaimer = false
                },*/
                MainMenuEvent::AuthServerTrust(auth_server, trust) => {
                    if trust {
                        global_state
                            .settings
                            .networking
                            .trusted_auth_servers
                            .insert(auth_server.clone());
                        global_state.settings.save_to_file_warn();
                    }
                    self.client_init
                        .as_ref()
                        .map(|init| init.auth_trust(auth_server, trust));
                },
            }
        }

        if let Some(info) = global_state.info_message.take() {
            self.main_menu_ui.show_info(info);
        }

        PlayStateResult::Continue
    }

    fn name(&self) -> &'static str { "Title" }

    fn render(&mut self, renderer: &mut Renderer, _: &Settings) {
        // Draw the UI to the screen.
        self.main_menu_ui.render(renderer);
    }
}

/// When a network error is received and there is a mismatch between the client
/// and server version it is almost definitely due to this mismatch rather than
/// a true networking error.
fn get_network_error_text(
    localization: &Localization,
    error: impl Debug,
    mismatched_server_info: Option<ServerInfo>,
) -> String {
    if let Some(server_info) = mismatched_server_info {
        format!(
            "{} {}: {} {}: {}",
            localization.get("main.login.network_wrong_version"),
            localization.get("main.login.client_version"),
            common::util::GIT_HASH.to_string(),
            localization.get("main.login.server_version"),
            server_info.git_hash
        )
    } else {
        format!(
            "{}: {:?}",
            localization.get("main.login.network_error"),
            error
        )
    }
}
fn attempt_login(
    settings: &mut Settings,
    info_message: &mut Option<String>,
    username: String,
    password: String,
    connection_args: ClientConnArgs,
    client_init: &mut Option<ClientInit>,
    runtime: Option<Arc<runtime::Runtime>>,
) {
    if let Err(err) = comp::Player::alias_validate(&username) {
        *info_message = Some(err.to_string());
        return;
    }

    // Don't try to connect if there is already a connection in progress.
    if client_init.is_none() {
        *client_init = Some(ClientInit::new(
            connection_args,
            username,
            Some(settings.graphics.view_distance),
            password,
            runtime,
        ));
    }
}
