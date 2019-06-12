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
use common::{clock::Clock, comp, comp::phys::Pos, msg::ClientState};
use log::{error, warn};
use std::{cell::RefCell, rc::Rc, time::Duration};
use vek::*;

pub struct SessionState {
    scene: Scene,
    client: Rc<RefCell<Client>>,
    key_state: KeyState,
    hud: Hud,
}

/// Represents an active game session (i.e., the one being played).
impl SessionState {
    /// Create a new `SessionState`.
    pub fn new(window: &mut Window, client: Rc<RefCell<Client>>, settings: Settings) -> Self {
        // Create a scene for this session. The scene handles visible elements of the game world.
        let scene = Scene::new(window.renderer_mut(), &client.borrow());
        Self {
            scene,
            client,
            key_state: KeyState::new(),
            hud: Hud::new(window),
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

impl SessionState {
    /// Tick the session (and the client attached to it).
    pub fn tick(&mut self, dt: Duration) -> Result<(), Error> {
        // Calculate the movement input vector of the player from the current key presses
        // and the camera direction.
        let ori = self.scene.camera().get_orientation();
        let unit_vecs = (
            Vec2::new(ori[0].cos(), -ori[0].sin()),
            Vec2::new(ori[0].sin(), ori[0].cos()),
        );
        let dir_vec = self.key_state.dir_vec();
        let move_dir = unit_vecs.0 * dir_vec[0] + unit_vecs.1 * dir_vec[1];

        for event in self
            .client
            .borrow_mut()
            .tick(comp::Control { move_dir }, dt)?
        {
            match event {
                client::Event::Chat(msg) => {
                    self.hud.new_message(msg);
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
        renderer.clear(BG_COLOR);

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
        let mut clock = Clock::new();
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
                    Event::InputUpdate(GameInput::Attack, true) => {
                        self.client.borrow_mut().attack();
                        self.client.borrow_mut().respawn();
                    }
                    Event::InputUpdate(GameInput::Jump, true) => {
                        self.client.borrow_mut().jump();
                    }
                    Event::InputUpdate(GameInput::MoveForward, state) => self.key_state.up = state,
                    Event::InputUpdate(GameInput::MoveBack, state) => self.key_state.down = state,
                    Event::InputUpdate(GameInput::MoveLeft, state) => self.key_state.left = state,
                    Event::InputUpdate(GameInput::MoveRight, state) => self.key_state.right = state,
                    Event::InputUpdate(GameInput::Glide, state) => {
                        self.client.borrow_mut().glide(state)
                    }

                    // Pass all other events to the scene
                    event => {
                        self.scene.handle_input_event(event);
                    } // TODO: Do something if the event wasn't handled?
                }
            }

            // Perform an in-game tick.
            if let Err(err) = self.tick(clock.get_last_delta()) {
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
                    HudEvent::AdjustVolume(volume) => {
                        global_state.audio.set_volume(volume);

                        global_state.settings.audio.music_volume = volume;
                        if let Err(err) = global_state.settings.save_to_file() {
                            warn!("Failed to save settings: {:?}", err);
                        }
                    }
                    HudEvent::ChangeAudioDevice(name) => {
                        global_state.audio.set_device(name.clone());

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
