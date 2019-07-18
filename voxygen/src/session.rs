use crate::{
    hud::{DebugInfo, Event as HudEvent, Hud},
    key_state::KeyState,
    render::Renderer,
    scene::Scene,
    settings::Settings,
    window::{Event, GameInput, Window},
    Direction, Error, GlobalState, PlayState, PlayStateResult,
};
use client::{self, Client};
use common::{
    clock::Clock, comp, comp::Pos, msg::ClientState, terrain::Block, vol::ReadVol, ChatType,
};
use log::{error, warn};
use std::{cell::RefCell, rc::Rc, time::Duration};
use vek::*;

pub struct SessionState {
    scene: Scene,
    client: Rc<RefCell<Client>>,
    hud: Hud,
    key_state: KeyState,
    controller: comp::Controller,
    selected_block: Block,
}

/// Represents an active game session (i.e., the one being played).
impl SessionState {
    /// Create a new `SessionState`.
    pub fn new(window: &mut Window, client: Rc<RefCell<Client>>, _settings: Settings) -> Self {
        // Create a scene for this session. The scene handles visible elements of the game world.
        let scene = Scene::new(window.renderer_mut());
        Self {
            scene,
            client,
            key_state: KeyState::new(),
            controller: comp::Controller::default(),
            hud: Hud::new(window),
            selected_block: Block::new(1, Rgb::broadcast(255)),
        }
    }
}

impl SessionState {
    /// Tick the session (and the client attached to it).
    fn tick(&mut self, dt: Duration) -> Result<(), Error> {
        for event in self.client.borrow_mut().tick(self.controller.clone(), dt)? {
            match event {
                client::Event::Chat { chat_type, message } => {
                    match chat_type {
                        ChatType::Ping => {
                            // TODO: Play ping message here
                        }
                        _ => {}
                    }
                    self.hud
                        .new_message(client::Event::Chat { chat_type, message });
                }
                client::Event::Disconnect => {} // TODO
            }
        }

        Ok(())
    }

    /// Clean up the session (and the client attached to it) after a tick.
    pub fn cleanup(&mut self) {
        self.client.borrow_mut().cleanup();
    }

    /// Render the session to the screen.
    ///
    /// This method should be called once per frame.
    pub fn render(&mut self, renderer: &mut Renderer) {
        // Clear the screen
        renderer.clear();

        // Render the screen using the global renderer
        self.scene.render(renderer, &mut self.client.borrow_mut());
        // Draw the UI to the screen
        self.hud.render(renderer, self.scene.globals());

        // Finish the frame
        renderer.flush();
    }
}

impl PlayState for SessionState {
    fn play(&mut self, _: Direction, global_state: &mut GlobalState) -> PlayStateResult {
        // Trap the cursor.
        global_state.window.grab_cursor(true);

        // Set up an fps clock.
        let mut clock = Clock::start();
        self.client.borrow_mut().clear_terrain();

        // Game loop
        let mut current_client_state = self.client.borrow().get_client_state();
        while let ClientState::Pending | ClientState::Character | ClientState::Dead =
            current_client_state
        {
            // Handle window events.
            for event in global_state.window.fetch_events() {
                // Pass all events to the ui first.
                if self.hud.handle_event(event.clone(), global_state) {
                    continue;
                }

                match event {
                    Event::Close => {
                        return PlayStateResult::Shutdown;
                    }
                    Event::InputUpdate(GameInput::Attack, state) => {
                        self.controller.respawn = state; // TODO: Move this into separate GameInput

                        // Check the existence of CanBuild component. If it's here, use LMB to
                        // place blocks, if not, use it to attack
                        let mut client = self.client.borrow_mut();
                        if state
                            && client
                                .state()
                                .read_storage::<comp::CanBuild>()
                                .get(client.entity())
                                .is_some()
                        {
                            let cam_pos = self.scene.camera().compute_dependents(&client).2;
                            let cam_dir =
                                (self.scene.camera().get_focus_pos() - cam_pos).normalized();

                            let (d, b) = {
                                let terrain = client.state().terrain();
                                let ray = terrain.ray(cam_pos, cam_pos + cam_dir * 100.0).cast();
                                (ray.0, if let Ok(Some(_)) = ray.1 { true } else { false })
                            };

                            if b {
                                let pos =
                                    (cam_pos + cam_dir * (d - 0.01)).map(|e| e.floor() as i32);
                                client.place_block(pos, self.selected_block); // TODO: Handle block color with a command
                            }
                        } else {
                            self.controller.attack = state
                        }
                    }

                    Event::InputUpdate(GameInput::SecondAttack, state) => {
                        if state {
                            let mut client = self.client.borrow_mut();
                            if client
                                .state()
                                .read_storage::<comp::CanBuild>()
                                .get(client.entity())
                                .is_some()
                            {
                                let cam_pos = self.scene.camera().compute_dependents(&client).2;
                                let cam_dir =
                                    (self.scene.camera().get_focus_pos() - cam_pos).normalized();

                                let (d, b) = {
                                    let terrain = client.state().terrain();
                                    let ray =
                                        terrain.ray(cam_pos, cam_pos + cam_dir * 100.0).cast();
                                    (ray.0, if let Ok(Some(_)) = ray.1 { true } else { false })
                                };

                                if b {
                                    let pos = (cam_pos + cam_dir * d).map(|e| e.floor() as i32);
                                    client.remove_block(pos);
                                }
                            } else {
                                // TODO: Handle secondary attack
                            }
                        }
                    }
                    Event::InputUpdate(GameInput::Roll, state) => {
                        let client = self.client.borrow();
                        if client
                            .state()
                            .read_storage::<comp::CanBuild>()
                            .get(client.entity())
                            .is_some()
                        {
                            if state {
                                let cam_pos = self.scene.camera().compute_dependents(&client).2;
                                let cam_dir =
                                    (self.scene.camera().get_focus_pos() - cam_pos).normalized();

                                if let Ok(Some(block)) = client
                                    .state()
                                    .terrain()
                                    .ray(cam_pos, cam_pos + cam_dir * 100.0)
                                    .cast()
                                    .1
                                {
                                    self.selected_block = *block;
                                }
                            }
                        } else {
                            self.controller.roll = state;
                        }
                    }
                    Event::InputUpdate(GameInput::Jump, state) => {
                        self.controller.jump = state;
                    }
                    Event::InputUpdate(GameInput::MoveForward, state) => self.key_state.up = state,
                    Event::InputUpdate(GameInput::MoveBack, state) => self.key_state.down = state,
                    Event::InputUpdate(GameInput::MoveLeft, state) => self.key_state.left = state,
                    Event::InputUpdate(GameInput::MoveRight, state) => self.key_state.right = state,
                    Event::InputUpdate(GameInput::Glide, state) => {
                        self.controller.glide = state;
                    }

                    // Pass all other events to the scene
                    event => {
                        self.scene.handle_input_event(event);
                    } // TODO: Do something if the event wasn't handled?
                }
            }

            // Calculate the movement input vector of the player from the current key presses
            // and the camera direction.
            let ori = self.scene.camera().get_orientation();
            let unit_vecs = (
                Vec2::new(ori[0].cos(), -ori[0].sin()),
                Vec2::new(ori[0].sin(), ori[0].cos()),
            );
            let dir_vec = self.key_state.dir_vec();
            self.controller.move_dir = unit_vecs.0 * dir_vec[0] + unit_vecs.1 * dir_vec[1];

            // Perform an in-game tick.
            if let Err(err) = self.tick(clock.get_avg_delta()) {
                error!("Failed to tick the scene: {:?}", err);
                return PlayStateResult::Pop;
            }

            // Maintain global state.
            global_state.maintain();

            // Extract HUD events ensuring the client borrow gets dropped.
            let hud_events = self.hud.maintain(
                &self.client.borrow(),
                global_state,
                DebugInfo {
                    tps: clock.get_tps(),
                    ping_ms: self.client.borrow().get_ping_ms(),
                    coordinates: self
                        .client
                        .borrow()
                        .state()
                        .ecs()
                        .read_storage::<Pos>()
                        .get(self.client.borrow().entity())
                        .cloned(),
                },
                &self.scene.camera(),
            );

            // Maintain the UI.
            for event in hud_events {
                match event {
                    HudEvent::SendMessage(msg) => {
                        // TODO: Handle result
                        self.client.borrow_mut().send_chat(msg);
                    }
                    HudEvent::CharacterSelection => {
                        self.client.borrow_mut().request_remove_character()
                    }
                    HudEvent::Logout => self.client.borrow_mut().request_logout(),
                    HudEvent::Quit => {
                        return PlayStateResult::Shutdown;
                    }
                    HudEvent::AdjustMousePan(sensitivity) => {
                        global_state.window.pan_sensitivity = sensitivity;
                        global_state.settings.gameplay.pan_sensitivity = sensitivity;
                        if let Err(err) = global_state.settings.save_to_file() {
                            warn!("Failed to save settings: {:?}", err);
                        }
                    }
                    HudEvent::AdjustMouseZoom(sensitivity) => {
                        global_state.window.zoom_sensitivity = sensitivity;
                        global_state.settings.gameplay.zoom_sensitivity = sensitivity;
                        if let Err(err) = global_state.settings.save_to_file() {
                            warn!("Failed to save settings: {:?}", err);
                        }
                    }
                    HudEvent::AdjustViewDistance(view_distance) => {
                        self.client.borrow_mut().set_view_distance(view_distance);

                        global_state.settings.graphics.view_distance = view_distance;
                        if let Err(err) = global_state.settings.save_to_file() {
                            warn!("Failed to save settings: {:?}", err);
                        }
                    }
                    HudEvent::CrosshairTransp(crosshair_transp) => {
                        global_state.settings.gameplay.crosshair_transp = crosshair_transp;
                        global_state.settings.gameplay.crosshair_transp = crosshair_transp;
                        if let Err(err) = global_state.settings.save_to_file() {
                            warn!("Failed to save settings: {:?}", err);
                        }
                    }
                    HudEvent::AdjustVolume(volume) => {
                        global_state.audio.model.player.set_volume(volume);

                        global_state.settings.audio.music_volume = volume;
                        if let Err(err) = global_state.settings.save_to_file() {
                            warn!("Failed to save settings: {:?}", err);
                        }
                    }
                    HudEvent::ChangeAudioDevice(name) => {
                        global_state.audio.model.player.set_device(&name.clone());

                        global_state.settings.audio.audio_device = Some(name);
                        if let Err(err) = global_state.settings.save_to_file() {
                            warn!("Failed to save settings!\n{:?}", err);
                        }
                    }
                    HudEvent::ChangeMaxFPS(fps) => {
                        global_state.settings.graphics.max_fps = fps;
                        if let Err(err) = global_state.settings.save_to_file() {
                            warn!("Failed to save settings!\n{:?}", err);
                        }
                    }
                }
            }

            // Maintain the scene.
            self.scene
                .maintain(global_state.window.renderer_mut(), &self.client.borrow());

            // Render the session.
            self.render(global_state.window.renderer_mut());

            // Display the frame on the window.
            global_state
                .window
                .swap_buffers()
                .expect("Failed to swap window buffers!");

            // Wait for the next tick.
            clock.tick(Duration::from_millis(
                1000 / global_state.settings.graphics.max_fps as u64,
            ));

            // Clean things up after the tick.
            self.cleanup();

            current_client_state = self.client.borrow().get_client_state();
        }

        PlayStateResult::Pop
    }

    fn name(&self) -> &'static str {
        "Session"
    }
}
