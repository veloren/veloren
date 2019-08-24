use crate::{
    hud::{DebugInfo, Event as HudEvent, Hud},
    key_state::KeyState,
    render::Renderer,
    scene::{camera::Camera, Scene},
    window::{Event, GameInput},
    Direction, Error, GlobalState, PlayState, PlayStateResult,
};
use client::{self, Client};
use common::{
    clock::Clock,
    comp,
    comp::Pos,
    comp::Vel,
    msg::ClientState,
    terrain::{Block, BlockKind},
    vol::ReadVol,
};
use log::error;
use specs::Join;
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
    pub fn new(global_state: &mut GlobalState, client: Rc<RefCell<Client>>) -> Self {
        // Create a scene for this session. The scene handles visible elements of the game world.
        let mut scene = Scene::new(global_state.window.renderer_mut());
        scene
            .camera_mut()
            .set_fov_deg(global_state.settings.graphics.fov);
        Self {
            scene,
            client,
            key_state: KeyState::new(),
            controller: comp::Controller::default(),
            hud: Hud::new(global_state),
            selected_block: Block::new(BlockKind::Normal, Rgb::broadcast(255)),
        }
    }
}

impl SessionState {
    /// Tick the session (and the client attached to it).
    fn tick(&mut self, dt: Duration) -> Result<(), Error> {
        for event in self.client.borrow_mut().tick(self.controller.clone(), dt)? {
            match event {
                client::Event::Chat {
                    chat_type: _,
                    message: _,
                } => {
                    self.hud.new_message(event);
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

        // Send startup commands to the server
        if global_state.settings.send_logon_commands {
            for cmd in &global_state.settings.logon_commands {
                self.client.borrow_mut().send_chat(cmd.to_string());
            }
        }

        // Game loop
        let mut current_client_state = self.client.borrow().get_client_state();
        while let ClientState::Pending | ClientState::Character | ClientState::Dead =
            current_client_state
        {
            // Compute camera data
            let (view_mat, _, cam_pos) = self
                .scene
                .camera()
                .compute_dependents(&self.client.borrow());
            let cam_dir: Vec3<f32> = Vec3::from(view_mat.inverted() * -Vec4::unit_z());

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
                            let (d, b) = {
                                let terrain = client.state().terrain();
                                let ray = terrain.ray(cam_pos, cam_pos + cam_dir * 100.0).cast();
                                (ray.0, if let Ok(Some(_)) = ray.1 { true } else { false })
                            };

                            if b {
                                let pos =
                                    (cam_pos + cam_dir * (d - 0.01)).map(|e| e.floor() as i32);
                                client.place_block(pos, self.selected_block);
                            }
                        } else {
                            self.controller.attack = state
                        }
                    }

                    Event::InputUpdate(GameInput::Block, state) => {
                        let mut client = self.client.borrow_mut();
                        if state
                            && client
                                .state()
                                .read_storage::<comp::CanBuild>()
                                .get(client.entity())
                                .is_some()
                        {
                            let (d, b) = {
                                let terrain = client.state().terrain();
                                let ray = terrain.ray(cam_pos, cam_pos + cam_dir * 100.0).cast();
                                (ray.0, if let Ok(Some(_)) = ray.1 { true } else { false })
                            };

                            if b {
                                let pos = (cam_pos + cam_dir * d).map(|e| e.floor() as i32);
                                client.remove_block(pos);
                            }
                        } else {
                            self.controller.block = state;
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
                    Event::InputUpdate(GameInput::Interact, state) => {
                        let mut client = self.client.borrow_mut();

                        let player_pos = client
                            .state()
                            .read_storage::<comp::Pos>()
                            .get(client.entity())
                            .copied();

                        if let (Some(player_pos), true) = (player_pos, state) {
                            let entity = (
                                &client.state().ecs().entities(),
                                &client.state().ecs().read_storage::<comp::Pos>(),
                                &client.state().ecs().read_storage::<comp::Item>(),
                            )
                                .join()
                                .filter(|(_, pos, _)| {
                                    pos.0.distance_squared(player_pos.0) < 3.0 * 3.0
                                })
                                .min_by_key(|(_, pos, _)| {
                                    (pos.0.distance_squared(player_pos.0) * 1000.0) as i32
                                })
                                .map(|(entity, _, _)| entity);

                            if let Some(entity) = entity {
                                client.pick_up(entity);
                            }
                        }
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

            self.controller.look_dir = cam_dir;

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
                    velocity: self
                        .client
                        .borrow()
                        .state()
                        .ecs()
                        .read_storage::<Vel>()
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
                        global_state.settings.save_to_file_warn();
                    }
                    HudEvent::AdjustMouseZoom(sensitivity) => {
                        global_state.window.zoom_sensitivity = sensitivity;
                        global_state.settings.gameplay.zoom_sensitivity = sensitivity;
                        global_state.settings.save_to_file_warn();
                    }
                    HudEvent::AdjustViewDistance(view_distance) => {
                        self.client.borrow_mut().set_view_distance(view_distance);

                        global_state.settings.graphics.view_distance = view_distance;
                        global_state.settings.save_to_file_warn();
                    }
                    HudEvent::CrosshairTransp(crosshair_transp) => {
                        global_state.settings.gameplay.crosshair_transp = crosshair_transp;
                        global_state.settings.save_to_file_warn();
                    }
                    HudEvent::CrosshairType(crosshair_type) => {
                        global_state.settings.gameplay.crosshair_type = crosshair_type;
                        global_state.settings.save_to_file_warn();
                    }
                    HudEvent::ToggleXpBar(xp_bar) => {
                        global_state.settings.gameplay.xp_bar = xp_bar;
                        global_state.settings.save_to_file_warn();
                    }
                    HudEvent::ToggleEnBars(en_bars) => {
                        global_state.settings.gameplay.en_bars = en_bars;
                        global_state.settings.save_to_file_warn();
                    }
                    HudEvent::ToggleBarNumbers(bar_numbers) => {
                        global_state.settings.gameplay.bar_numbers = bar_numbers;
                        global_state.settings.save_to_file_warn();
                    }
                    HudEvent::ToggleShortcutNumbers(shortcut_numbers) => {
                        global_state.settings.gameplay.shortcut_numbers = shortcut_numbers;
                        global_state.settings.save_to_file_warn();
                    }
                    HudEvent::UiScale(scale_change) => {
                        global_state.settings.gameplay.ui_scale =
                            self.hud.scale_change(scale_change);
                        global_state.settings.save_to_file_warn();
                    }

                    HudEvent::AdjustVolume(volume) => {
                        global_state.audio.model.player.set_volume(volume);

                        global_state.settings.audio.music_volume = volume;
                        global_state.settings.save_to_file_warn();
                    }
                    HudEvent::ChangeAudioDevice(name) => {
                        global_state.audio.model.player.set_device(&name.clone());

                        global_state.settings.audio.audio_device = Some(name);
                        global_state.settings.save_to_file_warn();
                    }
                    HudEvent::ChangeMaxFPS(fps) => {
                        global_state.settings.graphics.max_fps = fps;
                        global_state.settings.save_to_file_warn();
                    }
                    HudEvent::SwapInventorySlots(a, b) => {
                        self.client.borrow_mut().swap_inventory_slots(a, b)
                    }
                    HudEvent::DropInventorySlot(x) => {
                        self.client.borrow_mut().drop_inventory_slot(x)
                    }
                    HudEvent::ChangeFOV(new_fov) => {
                        global_state.settings.graphics.fov = new_fov;
                        global_state.settings.save_to_file_warn();
                        &self.scene.camera_mut().set_fov_deg(new_fov);
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
