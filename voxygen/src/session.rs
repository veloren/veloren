use crate::{
    hud::{Event as HudEvent, Hud},
    key_state::KeyState,
    render::Renderer,
    scene::Scene,
    settings::Settings,
    window::{Event, Key, Window},
    Direction, Error, GlobalState, PlayState, PlayStateResult,
};
use client::{self, Client, Input, InputEvent};
use common::clock::Clock;
use std::{cell::RefCell, mem, rc::Rc, time::Duration};
use vek::*;

const FPS: u64 = 60;

pub struct SessionState {
    scene: Scene,
    client: Rc<RefCell<Client>>,
    key_state: KeyState,
    input_events: Vec<InputEvent>,
    hud: Hud,
}

/// Represents an active game session (i.e: one that is being played)
impl SessionState {
    /// Create a new `SessionState`
    pub fn new(window: &mut Window, client: Rc<RefCell<Client>>, settings: Settings) -> Self {
        // Create a scene for this session. The scene handles visible elements of the game world
        let scene = Scene::new(window.renderer_mut(), &client.borrow());
        Self {
            scene,
            client,
            key_state: KeyState::new(),
            hud: Hud::new(window, settings),
            input_events: Vec::new(),
        }
    }
}

// The background colour
const BG_COLOR: Rgba<f32> = Rgba {
    r: 0.0,
    g: 0.3,
    b: 1.0,
    a: 1.0,
};

impl SessionState {
    /// Tick the session (and the client attached to it)
    pub fn tick(&mut self, dt: Duration) -> Result<(), Error> {
        // Calculate the movement input vector of the player from the current key presses and the camera direction
        let ori = self.scene.camera().get_orientation();
        let unit_vecs = (
            Vec2::new(ori[0].cos(), -ori[0].sin()),
            Vec2::new(ori[0].sin(), ori[0].cos()),
        );
        let dir_vec = self.key_state.dir_vec();
        let move_dir = unit_vecs.0 * dir_vec[0] + unit_vecs.1 * dir_vec[1];

        // Take the input events
        let mut input_events = Vec::new();
        mem::swap(&mut self.input_events, &mut input_events);

        for event in self.client.borrow_mut().tick(
            Input {
                move_dir,
                jumping: self.key_state.jump,
                events: input_events,
            },
            dt,
        )? {
            match event {
                client::Event::Chat(msg) => {
                    self.hud.new_message(msg);
                }
                client::Event::Disconnect => {
                    // TODO
                }
            }
        }

        Ok(())
    }

    /// Clean up the session (and the client attached to it) after a tick
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
        self.hud.render(renderer);

        // Finish the frame
        renderer.flush();
    }
}

impl PlayState for SessionState {
    fn play(&mut self, _: Direction, global_state: &mut GlobalState) -> PlayStateResult {
        // Trap the cursor
        global_state.window.grab_cursor(true);

        // Set up an fps clock
        let mut clock = Clock::new();

        // Load a few chunks TODO: Remove this
        /*
        for x in -6..7 {
            for y in -6..7 {
                for z in -1..2 {
                    self.client.borrow_mut().load_chunk(Vec3::new(x, y, z));
                }
            }
        }
        */

        // Game loop
        loop {
            // Handle window events
            for event in global_state.window.fetch_events() {
                // Pass all events to the ui first
                if self.hud.handle_event(event.clone(), global_state) {
                    continue;
                }
                let _handled = match event {
                    Event::Close => {
                        return PlayStateResult::Shutdown;
                    }
                    // Movement Key Pressed
                    Event::KeyDown(Key::MoveForward) => self.key_state.up = true,
                    Event::KeyDown(Key::MoveBack) => self.key_state.down = true,
                    Event::KeyDown(Key::MoveLeft) => self.key_state.left = true,
                    Event::KeyDown(Key::MoveRight) => self.key_state.right = true,
                    Event::KeyDown(Key::Jump) => {
                        self.input_events.push(InputEvent::Jump);
                        self.key_state.jump = true;
                    }
                    // Movement Key Released
                    Event::KeyUp(Key::MoveForward) => self.key_state.up = false,
                    Event::KeyUp(Key::MoveBack) => self.key_state.down = false,
                    Event::KeyUp(Key::MoveLeft) => self.key_state.left = false,
                    Event::KeyUp(Key::MoveRight) => self.key_state.right = false,
                    Event::KeyUp(Key::Jump) => self.key_state.jump = false,
                    // Pass all other events to the scene
                    event => {
                        self.scene.handle_input_event(event);
                    }
                };
                // TODO: Do something if the event wasn't handled?
            }

            // Perform an in-game tick
            self.tick(clock.get_last_delta())
                .expect("Failed to tick the scene");

            // Maintain the scene
            self.scene.maintain(
                global_state.window.renderer_mut(),
                &mut self.client.borrow_mut(),
            );
            // Maintain the UI
            for event in self
                .hud
                .maintain(global_state.window.renderer_mut(), clock.get_tps())
            {
                match event {
                    HudEvent::SendMessage(msg) => {
                        // TODO: Handle result
                        self.client.borrow_mut().send_chat(msg);
                    }
                    HudEvent::Logout => return PlayStateResult::Pop,
                    HudEvent::Quit => {
                        return PlayStateResult::Shutdown;
                    }
                }
            }

            // Render the session
            self.render(global_state.window.renderer_mut());

            // Display the frame on the window
            global_state
                .window
                .swap_buffers()
                .expect("Failed to swap window buffers");

            // Wait for the next tick
            clock.tick(Duration::from_millis(1000 / FPS));

            // Clean things up after the tick
            self.cleanup();
        }
    }

    fn name(&self) -> &'static str {
        "Session"
    }
}
