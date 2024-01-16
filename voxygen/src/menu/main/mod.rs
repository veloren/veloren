mod client_init;
mod ui;

use super::{char_selection::CharSelectionState, dummy_scene::Scene, server_info::ServerInfoState};
#[cfg(feature = "singleplayer")]
use crate::singleplayer::SingleplayerState;
use crate::{
    render::{Drawer, GlobalsBindGroup},
    settings::Settings,
    window::Event,
    Direction, GlobalState, PlayState, PlayStateResult,
};
use client::{
    addr::ConnectionArgs,
    error::{InitProtocolError, NetworkConnectError, NetworkError},
    Client, ClientInitStage, ServerInfo,
};
use client_init::{ClientInit, Error as InitError, Msg as InitMsg};
use common::comp;
use common_base::span;
use i18n::LocalizationHandle;
#[cfg(feature = "singleplayer")]
use server::ServerInitStage;
use std::sync::Arc;
use tokio::runtime;
use tracing::error;
use ui::{Event as MainMenuEvent, MainMenuUi};

pub use ui::rand_bg_image_spec;

#[derive(Debug)]
pub enum DetailedInitializationStage {
    #[cfg(feature = "singleplayer")]
    Singleplayer,
    #[cfg(feature = "singleplayer")]
    SingleplayerServer(ServerInitStage),
    StartingMultiplayer,
    Client(ClientInitStage),
    CreatingRenderPipeline(usize, usize),
}

enum InitState {
    None,
    // Waiting on the client initialization
    Client(ClientInit),
    // Client initialized but still waiting on Renderer pipeline creation
    Pipeline(Box<Client>),
}

impl InitState {
    fn client(&self) -> Option<&ClientInit> {
        if let Self::Client(client_init) = &self {
            Some(client_init)
        } else {
            None
        }
    }
}

pub struct MainMenuState {
    main_menu_ui: MainMenuUi,
    init: InitState,
    scene: Scene,
}

impl MainMenuState {
    /// Create a new `MainMenuState`.
    pub fn new(global_state: &mut GlobalState, server: Option<String>) -> Self {
        Self {
            main_menu_ui: MainMenuUi::new(global_state, server),
            init: InitState::None,
            scene: Scene::new(global_state.window.renderer_mut()),
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
            global_state.singleplayer = SingleplayerState::None;
        }

        // Updated localization in case the selected language was changed
        self.main_menu_ui
            .update_language(global_state.i18n, &global_state.settings);
        // Set scale mode in case it was change
        self.main_menu_ui
            .set_scale_mode(global_state.settings.interface.ui_scale);

        #[cfg(feature = "discord")]
        global_state.discord.enter_main_menu();
    }

    #[allow(clippy::single_match)] // TODO: remove when event match has multiple arms
    fn tick(&mut self, global_state: &mut GlobalState, events: Vec<Event>) -> PlayStateResult {
        span!(_guard, "tick", "<MainMenuState as PlayState>::tick");

        // Pull in localizations
        let localized_strings = &global_state.i18n.read();

        // Poll server creation
        #[cfg(feature = "singleplayer")]
        {
            if let Some(singleplayer) = global_state.singleplayer.as_running() {
                if let Ok(stage_update) = singleplayer.init_stage_receiver.try_recv() {
                    self.main_menu_ui.update_stage(
                        DetailedInitializationStage::SingleplayerServer(stage_update),
                    );
                }

                match singleplayer.receiver.try_recv() {
                    Ok(Ok(())) => {
                        // Attempt login after the server is finished initializing
                        attempt_login(
                            &mut global_state.info_message,
                            "singleplayer".to_owned(),
                            "".to_owned(),
                            ConnectionArgs::Mpsc(14004),
                            &mut self.init,
                            &global_state.tokio_runtime,
                            global_state.settings.language.send_to_server.then_some(
                                global_state.settings.language.selected_language.clone(),
                            ),
                            &global_state.i18n,
                        );
                    },
                    Ok(Err(e)) => {
                        error!(?e, "Could not start server");
                        global_state.singleplayer = SingleplayerState::None;
                        self.init = InitState::None;
                        self.main_menu_ui.cancel_connection();
                        let server_err = match e {
                            server::Error::NetworkErr(e) => localized_strings
                                .get_msg_ctx("main-servers-network_error", &i18n::fluent_args! {
                                    "raw_error" => e.to_string()
                                })
                                .into_owned(),
                            server::Error::ParticipantErr(e) => localized_strings
                                .get_msg_ctx(
                                    "main-servers-participant_error",
                                    &i18n::fluent_args! {
                                        "raw_error" => e.to_string()
                                    },
                                )
                                .into_owned(),
                            server::Error::StreamErr(e) => localized_strings
                                .get_msg_ctx("main-servers-stream_error", &i18n::fluent_args! {
                                    "raw_error" => e.to_string()
                                })
                                .into_owned(),
                            server::Error::DatabaseErr(e) => localized_strings
                                .get_msg_ctx("main-servers-database_error", &i18n::fluent_args! {
                                    "raw_error" => e.to_string()
                                })
                                .into_owned(),
                            server::Error::PersistenceErr(e) => localized_strings
                                .get_msg_ctx(
                                    "main-servers-persistence_error",
                                    &i18n::fluent_args! {
                                        "raw_error" => e.to_string()
                                    },
                                )
                                .into_owned(),
                            server::Error::RtsimError(e) => localized_strings
                                .get_msg_ctx("main-servers-rtsim_error", &i18n::fluent_args! {
                                    "raw_error" => e.to_string(),
                                })
                                .into_owned(),
                            server::Error::Other(e) => localized_strings
                                .get_msg_ctx("main-servers-other_error", &i18n::fluent_args! {
                                    "raw_error" => e,
                                })
                                .into_owned(),
                        };
                        global_state.info_message = Some(
                            localized_strings
                                .get_msg_ctx(
                                    "main-servers-singleplayer_error",
                                    &i18n::fluent_args! {
                                        "sp_error" => server_err
                                    },
                                )
                                .into_owned(),
                        );
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

        if let Some(client_stage_update) = self.init.client().and_then(|init| init.stage_update()) {
            self.main_menu_ui
                .update_stage(DetailedInitializationStage::Client(client_stage_update));
        }

        // Poll client creation.
        match self.init.client().and_then(|init| init.poll()) {
            Some(InitMsg::Done(Ok(mut client))) => {
                // Register voxygen components / resources
                crate::ecs::init(client.state_mut().ecs_mut());
                self.init = InitState::Pipeline(Box::new(client));
            },
            Some(InitMsg::Done(Err(e))) => {
                self.init = InitState::None;
                error!(?e, "Client Init failed raw error");
                let e = get_client_msg_error(e, &global_state.i18n);
                // Log error for possible additional use later or in case that the error
                // displayed is cut of.
                error!(?e, "Client Init failed");
                global_state.info_message = Some(
                    localized_strings
                        .get_msg_ctx("main-login-client_init_failed", &i18n::fluent_args! {
                            "init_fail_reason" => e
                        })
                        .into_owned(),
                );
            },
            Some(InitMsg::IsAuthTrusted(auth_server)) => {
                if global_state
                    .settings
                    .networking
                    .trusted_auth_servers
                    .contains(&auth_server)
                {
                    // Can't fail since we just polled it, it must be Some
                    self.init.client().unwrap().auth_trust(auth_server, true);
                } else {
                    // Show warning that auth server is not trusted and prompt for approval
                    self.main_menu_ui.auth_trust_prompt(auth_server);
                }
            },
            None => {},
        }

        // Tick the client to keep the connection alive if we are waiting on pipelines
        if let InitState::Pipeline(client) = &mut self.init {
            match client.tick(comp::ControllerInputs::default(), global_state.clock.dt()) {
                Ok(events) => {
                    for event in events {
                        match event {
                            client::Event::SetViewDistance(_vd) => {},
                            client::Event::Disconnect => {
                                global_state.info_message = Some(
                                    localized_strings
                                        .get_msg("main-login-server_shut_down")
                                        .into_owned(),
                                );
                                self.init = InitState::None;
                            },
                            _ => {},
                        }
                    }
                },
                Err(err) => {
                    global_state.info_message = Some(
                        localized_strings
                            .get_msg("common-connection_lost")
                            .into_owned(),
                    );
                    error!(?err, "[main menu] Failed to tick the client");
                    self.init = InitState::None;
                },
            }
        }

        // Poll renderer pipeline creation
        if let InitState::Pipeline(..) = &self.init {
            if let Some((done, total)) = &global_state.window.renderer().pipeline_creation_status()
            {
                self.main_menu_ui.update_stage(
                    DetailedInitializationStage::CreatingRenderPipeline(*done, *total),
                );
            // If complete go to char select screen
            } else {
                // Always succeeds since we check above
                if let InitState::Pipeline(client) =
                    core::mem::replace(&mut self.init, InitState::None)
                {
                    self.main_menu_ui.connected();

                    let server_info = client.server_info().clone();
                    let server_description = client.server_description().clone();

                    let char_select = CharSelectionState::new(
                        global_state,
                        std::rc::Rc::new(std::cell::RefCell::new(*client)),
                    );

                    let new_state = ServerInfoState::try_from_server_info(
                        global_state,
                        self.main_menu_ui.bg_img_spec(),
                        char_select,
                        server_info,
                        server_description,
                        false,
                    )
                    .map(|s| Box::new(s) as _)
                    .unwrap_or_else(|s| Box::new(s) as _);

                    return PlayStateResult::Push(new_state);
                }
            }
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
                    let net_settings = &mut global_state.settings.networking;
                    let use_quic = net_settings.use_quic;
                    net_settings.username = username.clone();
                    net_settings.default_server = server_address.clone();
                    if !net_settings.servers.contains(&server_address) {
                        net_settings.servers.push(server_address.clone());
                    }
                    global_state
                        .settings
                        .save_to_file_warn(&global_state.config_dir);

                    let connection_args = if use_quic {
                        ConnectionArgs::Quic {
                            hostname: server_address,
                            prefer_ipv6: false,
                        }
                    } else {
                        ConnectionArgs::Tcp {
                            hostname: server_address,
                            prefer_ipv6: false,
                        }
                    };
                    attempt_login(
                        &mut global_state.info_message,
                        username,
                        password,
                        connection_args,
                        &mut self.init,
                        &global_state.tokio_runtime,
                        global_state
                            .settings
                            .language
                            .send_to_server
                            .then_some(global_state.settings.language.selected_language.clone()),
                        &global_state.i18n,
                    );
                },
                MainMenuEvent::CancelLoginAttempt => {
                    // init contains InitState::Client(ClientInit), which spawns a thread which
                    // contains a TcpStream::connect() call This call is
                    // blocking TODO fix when the network rework happens
                    #[cfg(feature = "singleplayer")]
                    {
                        global_state.singleplayer = SingleplayerState::None;
                    }
                    self.init = InitState::None;
                    self.main_menu_ui.cancel_connection();
                },
                MainMenuEvent::ChangeLanguage(new_language) => {
                    global_state.settings.language.selected_language =
                        new_language.language_identifier;
                    global_state.i18n = LocalizationHandle::load_expect(
                        &global_state.settings.language.selected_language,
                    );
                    global_state
                        .i18n
                        .set_english_fallback(global_state.settings.language.use_english_fallback);
                    self.main_menu_ui
                        .update_language(global_state.i18n, &global_state.settings);
                },
                #[cfg(feature = "singleplayer")]
                MainMenuEvent::StartSingleplayer => {
                    global_state.singleplayer.run(&global_state.tokio_runtime);
                },
                #[cfg(feature = "singleplayer")]
                MainMenuEvent::InitSingleplayer => {
                    global_state.singleplayer = SingleplayerState::init();
                },
                #[cfg(feature = "singleplayer")]
                MainMenuEvent::SinglePlayerChange(change) => {
                    if let SingleplayerState::Init(ref mut init) = global_state.singleplayer {
                        match change {
                            ui::WorldsChange::SetActive(world) => init.current = world,
                            ui::WorldsChange::Delete(world) => init.remove(world),
                            ui::WorldsChange::Regenerate(world) => init.delete_map_file(world),
                            ui::WorldsChange::AddNew => init.new_world(),
                            ui::WorldsChange::CurrentWorldChange(change) => {
                                if let Some(world) = init
                                    .current
                                    .map(|i| &mut init.worlds[i])
                                    .filter(|map| !map.is_generated)
                                {
                                    change.apply(world);
                                    init.save_current_meta();
                                }
                            },
                        }
                    }
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
                        global_state
                            .settings
                            .save_to_file_warn(&global_state.config_dir);
                    }
                    self.init
                        .client()
                        .map(|init| init.auth_trust(auth_server, trust));
                },
                MainMenuEvent::DeleteServer { server_index } => {
                    let net_settings = &mut global_state.settings.networking;
                    net_settings.servers.remove(server_index);

                    global_state
                        .settings
                        .save_to_file_warn(&global_state.config_dir);
                },
            }
        }

        if let Some(info) = global_state.info_message.take() {
            self.main_menu_ui.show_info(info);
        }

        PlayStateResult::Continue
    }

    fn name(&self) -> &'static str { "Title" }

    fn capped_fps(&self) -> bool { true }

    fn globals_bind_group(&self) -> &GlobalsBindGroup { self.scene.global_bind_group() }

    fn render(&self, drawer: &mut Drawer<'_>, _: &Settings) {
        // Draw the UI to the screen.
        let mut third_pass = drawer.third_pass();
        if let Some(mut ui_drawer) = third_pass.draw_ui() {
            self.main_menu_ui.render(&mut ui_drawer);
        };
    }

    fn egui_enabled(&self) -> bool { false }
}

fn get_client_msg_error(
    error: client_init::Error,
    localized_strings: &LocalizationHandle,
) -> String {
    let localization = localized_strings.read();

    // When a network error is received and there is a mismatch between the client
    // and server version it is almost definitely due to this mismatch rather than
    // a true networking error.
    let net_error = |error: String, mismatched_server_info: Option<ServerInfo>| -> String {
        if let Some(server_info) =
            mismatched_server_info.filter(|info| info.git_hash != *common::util::GIT_HASH)
        {
            format!(
                "{} {}: {} ({}) {}: {} ({})",
                localization.get_msg("main-login-network_wrong_version"),
                localization.get_msg("main-login-client_version"),
                &*common::util::GIT_HASH,
                &*common::util::GIT_DATE,
                localization.get_msg("main-login-server_version"),
                server_info.git_hash,
                server_info.git_date,
            )
        } else {
            format!(
                "{}: {}",
                localization.get_msg("main-login-network_error"),
                error
            )
        }
    };

    use client::Error;
    match error {
        InitError::ClientError {
            error,
            mismatched_server_info,
        } => match error {
            Error::SpecsErr(e) => {
                format!(
                    "{}: {}",
                    localization.get_msg("main-login-internal_error"),
                    e
                )
            },
            Error::AuthErr(e) => format!(
                "{}: {}",
                localization.get_msg("main-login-authentication_error"),
                e
            ),
            Error::Kicked(e) => e,
            Error::TooManyPlayers => localization.get_msg("main-login-server_full").into(),
            Error::AuthServerNotTrusted => localization
                .get_msg("main-login-untrusted_auth_server")
                .into(),
            Error::ServerTimeout => localization.get_msg("main-login-timeout").into(),
            Error::ServerShutdown => localization.get_msg("main-login-server_shut_down").into(),
            Error::NotOnWhitelist => localization.get_msg("main-login-not_on_whitelist").into(),
            Error::Banned(reason) => {
                format!("{}: {}", localization.get_msg("main-login-banned"), reason)
            },
            Error::InvalidCharacter => localization.get_msg("main-login-invalid_character").into(),
            Error::NetworkErr(NetworkError::ConnectFailed(NetworkConnectError::Handshake(
                InitProtocolError::WrongVersion(_),
            ))) => net_error(
                localization
                    .get_msg("main-login-network_wrong_version")
                    .into_owned(),
                mismatched_server_info,
            ),
            Error::NetworkErr(e) => net_error(e.to_string(), mismatched_server_info),
            Error::ParticipantErr(e) => net_error(e.to_string(), mismatched_server_info),
            Error::StreamErr(e) => net_error(e.to_string(), mismatched_server_info),
            Error::HostnameLookupFailed(e) => {
                format!(
                    "{}: {}",
                    localization.get_msg("main-login-server_not_found"),
                    e
                )
            },
            Error::Other(e) => {
                format!("{}: {}", localization.get_msg("common-error"), e)
            },
            Error::AuthClientError(e) => match e {
                // TODO: remove parentheses
                client::AuthClientError::RequestError(e) => format!(
                    "{}: {}",
                    localization.get_msg("main-login-failed_sending_request"),
                    e
                ),
                client::AuthClientError::JsonError(e) => format!(
                    "{}: {}",
                    localization.get_msg("main-login-failed_sending_request"),
                    e
                ),
                client::AuthClientError::InsecureSchema => localization
                    .get_msg("main-login-insecure_auth_scheme")
                    .into(),
                client::AuthClientError::ServerError(_, e) => String::from_utf8_lossy(&e).into(),
            },
            Error::AuthServerUrlInvalid(e) => {
                format!(
                    "{}: https://{}",
                    localization.get_msg("main-login-failed_auth_server_url_invalid"),
                    e
                )
            },
        },
        InitError::ClientCrashed => localization.get_msg("main-login-client_crashed").into(),
        InitError::ServerNotFound => localization.get_msg("main-login-server_not_found").into(),
    }
}

fn attempt_login(
    info_message: &mut Option<String>,
    username: String,
    password: String,
    connection_args: ConnectionArgs,
    init: &mut InitState,
    runtime: &Arc<runtime::Runtime>,
    locale: Option<String>,
    localized_strings: &LocalizationHandle,
) {
    let localization = localized_strings.read();
    if let Err(err) = comp::Player::alias_validate(&username) {
        match err {
            comp::AliasError::ForbiddenCharacters => {
                *info_message = Some(
                    localization
                        .get_msg("main-login-username_bad_characters")
                        .into_owned(),
                );
            },
            comp::AliasError::TooLong => {
                *info_message = Some(
                    localization
                        .get_msg_ctx("main-login-username_too_long", &i18n::fluent_args! {
                            "max_len" => comp::MAX_ALIAS_LEN
                        })
                        .into_owned(),
                );
            },
        }
        return;
    }

    // Don't try to connect if there is already a connection in progress.
    if let InitState::None = init {
        *init = InitState::Client(ClientInit::new(
            connection_args,
            username,
            password,
            Arc::clone(runtime),
            locale,
        ));
    }
}
