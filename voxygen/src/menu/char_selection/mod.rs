mod ui;

use crate::{
    i18n::{i18n_asset_key, Localization},
    render::Renderer,
    scene::simple::{self as scene, Scene},
    session::SessionState,
    settings::Settings,
    window::Event as WinEvent,
    Direction, GlobalState, PlayState, PlayStateResult,
};
use client::{self, Client};
use common::{assets::Asset, comp, span, state::DeltaTime};
use specs::WorldExt;
use std::{cell::RefCell, rc::Rc};
use tracing::error;
use ui::CharSelectionUi;

pub struct CharSelectionState {
    char_selection_ui: CharSelectionUi,
    client: Rc<RefCell<Client>>,
    scene: Scene,
}

impl CharSelectionState {
    /// Create a new `CharSelectionState`.
    pub fn new(global_state: &mut GlobalState, client: Rc<RefCell<Client>>) -> Self {
        let scene = Scene::new(
            global_state.window.renderer_mut(),
            Some("fixture.selection_bg"),
            &*client.borrow(),
        );
        Self {
            char_selection_ui: CharSelectionUi::new(global_state),
            client,
            scene,
        }
    }

    fn get_humanoid_body(&self) -> Option<comp::humanoid::Body> {
        self.char_selection_ui
            .selected_character()
            .and_then(|character| match character.body {
                comp::Body::Humanoid(body) => Some(body),
                _ => None,
            })
    }
}

impl PlayState for CharSelectionState {
    fn enter(&mut self, _: &mut GlobalState, _: Direction) {
        // Load the player's character list
        self.client.borrow_mut().load_character_list();
    }

    fn tick(&mut self, global_state: &mut GlobalState, events: Vec<WinEvent>) -> PlayStateResult {
        span!(_guard, "tick", "<CharSelectionState as PlayState>::tick");
        let (client_presence, client_registered) = {
            let client = self.client.borrow();
            (client.presence(), client.registered())
        };
        if client_presence.is_none() && client_registered {
            // Handle window events
            for event in events {
                if self.char_selection_ui.handle_event(event.clone()) {
                    continue;
                }
                match event {
                    WinEvent::Close => {
                        return PlayStateResult::Shutdown;
                    },
                    // Pass all other events to the scene
                    event => {
                        self.scene.handle_input_event(event);
                    }, // TODO: Do something if the event wasn't handled?
                }
            }

            // Maintain the UI.
            let events = self
                .char_selection_ui
                .maintain(global_state, &mut self.client.borrow_mut());

            for event in events {
                match event {
                    ui::Event::Logout => {
                        return PlayStateResult::Pop;
                    },
                    ui::Event::AddCharacter { alias, tool, body } => {
                        self.client.borrow_mut().create_character(alias, tool, body);
                    },
                    ui::Event::DeleteCharacter(character_id) => {
                        self.client.borrow_mut().delete_character(character_id);
                    },
                    ui::Event::Play(character) => {
                        // TODO: eliminate option in character id
                        if let Some(character_id) = character.character.id {
                            self.client.borrow_mut().request_character(character_id);
                        }

                        return PlayStateResult::Switch(Box::new(SessionState::new(
                            global_state,
                            Rc::clone(&self.client),
                        )));
                    },
                }
            }

            let humanoid_body = self.get_humanoid_body();
            let loadout = self.char_selection_ui.get_loadout();

            // Maintain the scene.
            {
                let client = self.client.borrow();
                let scene_data = scene::SceneData {
                    time: client.state().get_time(),
                    delta_time: client.state().ecs().read_resource::<DeltaTime>().0,
                    tick: client.get_tick(),
                    thread_pool: client.thread_pool(),
                    body: humanoid_body,
                    gamma: global_state.settings.graphics.gamma,
                    ambiance: global_state.settings.graphics.ambiance,
                    mouse_smoothing: global_state.settings.gameplay.smooth_pan_enable,
                    figure_lod_render_distance: global_state
                        .settings
                        .graphics
                        .figure_lod_render_distance
                        as f32,
                };
                self.scene.maintain(
                    global_state.window.renderer_mut(),
                    scene_data,
                    loadout.as_ref(),
                );
            }

            // Tick the client (currently only to keep the connection alive).
            let localized_strings = Localization::load_expect(&i18n_asset_key(
                &global_state.settings.language.selected_language,
            ));

            match self.client.borrow_mut().tick(
                comp::ControllerInputs::default(),
                global_state.clock.dt(),
                |_| {},
            ) {
                Ok(events) => {
                    for event in events {
                        match event {
                            client::Event::SetViewDistance(vd) => {
                                global_state.settings.graphics.view_distance = vd;
                                global_state.settings.save_to_file_warn();
                            },
                            client::Event::Disconnect => {
                                global_state.info_message = Some(
                                    localized_strings
                                        .get("main.login.server_shut_down")
                                        .to_owned(),
                                );
                                return PlayStateResult::Pop;
                            },
                            _ => {},
                        }
                    }
                },
                Err(err) => {
                    global_state.info_message =
                        Some(localized_strings.get("common.connection_lost").to_owned());
                    error!(?err, "[char_selection] Failed to tick the client");
                    return PlayStateResult::Pop;
                },
            }

            // TODO: make sure rendering is not relying on cleaned up stuff
            self.client.borrow_mut().cleanup();

            PlayStateResult::Continue
        } else {
            error!("Client not in pending or registered state. Popping char selection play state");
            // TODO set global_state.info_message
            PlayStateResult::Pop
        }
    }

    fn name(&self) -> &'static str { "Title" }

    fn render(&mut self, renderer: &mut Renderer, _: &Settings) {
        let humanoid_body = self.get_humanoid_body();
        let loadout = self.char_selection_ui.get_loadout();

        // Render the scene.
        self.scene.render(
            renderer,
            self.client.borrow().get_tick(),
            humanoid_body,
            loadout.as_ref(),
        );

        // Draw the UI to the screen.
        self.char_selection_ui.render(renderer);
    }
}
