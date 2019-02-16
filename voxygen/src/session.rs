// Standard
use std::time::Duration;

// Library
use vek::*;

// Project
use common::clock::Clock;
use client::{
    self,
    Client,
};

// Crate
use crate::{
    Error,
    PlayState,
    PlayStateResult,
    GlobalState,
    key_state::KeyState,
    window::{Event, Key, Window},
    render::Renderer,
    scene::Scene,
    ui::test::TestUi,
};

const FPS: u64 = 60;

pub struct SessionState {
    scene: Scene,
    client: Client,
    key_state: KeyState,
    // TODO: remove this
    test_ui: TestUi,
}

/// Represents an active game session (i.e: one that is being played)
impl SessionState {
    /// Create a new `SessionState`
    pub fn new(window: &mut Window) -> Result<Self, Error> {
        let client = Client::new(([127, 0, 0, 1], 59003))?.with_test_state(); // <--- TODO: Remove this
        Ok(Self {
            // Create a scene for this session. The scene handles visible elements of the game world
            scene: Scene::new(window.renderer_mut(), &client),
            client,
            key_state: KeyState::new(),
            test_ui: TestUi::new(window),
        })
    }
}

// The background colour
const BG_COLOR: Rgba<f32> = Rgba { r: 0.0, g: 0.3, b: 1.0, a: 1.0 };

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

        self.client.tick(client::Input { move_dir }, dt)?;
        Ok(())
    }

    /// Clean up the session (and the client attached to it) after a tick
    pub fn cleanup(&mut self) {
        self.client.cleanup();
    }

    /// Render the session to the screen.
    ///
    /// This method should be called once per frame.
    pub fn render(&mut self, renderer: &mut Renderer) {
        // Clear the screen
        renderer.clear(BG_COLOR);

        // Render the screen using the global renderer
        self.scene.render_to(renderer);
        // Draw the UI to the screen
        self.test_ui.render(renderer);

        // Finish the frame
        renderer.flush();
    }
}

impl PlayState for SessionState {
    fn play(&mut self, global_state: &mut GlobalState) -> PlayStateResult {
        // Trap the cursor
        global_state.window.grab_cursor(true);

        // Set up an fps clock
        let mut clock = Clock::new();

        // Load a few chunks TODO: Remove this
        for x in -6..7 {
            for y in -6..7 {
                for z in -1..2 {
                    self.client.load_chunk(Vec3::new(x, y, z));
                }
            }
        }

        // Game loop
        loop {
            // Handle window events
            for event in global_state.window.fetch_events() {
                let _handled = match event {
                    Event::Close => return PlayStateResult::Shutdown,
                    // When 'q' is pressed, exit the session
                    Event::Char('q') => return PlayStateResult::Pop,
                    // Toggle cursor grabbing
                    Event::KeyDown(Key::ToggleCursor) => {
                        global_state.window.grab_cursor(!global_state.window.is_cursor_grabbed());
                    },
                    // Movement Key Pressed
                    Event::KeyDown(Key::MoveForward) => self.key_state.up = true,
                    Event::KeyDown(Key::MoveBack) => self.key_state.down = true,
                    Event::KeyDown(Key::MoveLeft) => self.key_state.left = true,
                    Event::KeyDown(Key::MoveRight) => self.key_state.right = true,
                    // Movement Key Released
                    Event::KeyUp(Key::MoveForward) => self.key_state.up = false,
                    Event::KeyUp(Key::MoveBack) => self.key_state.down = false,
                    Event::KeyUp(Key::MoveLeft) => self.key_state.left = false,
                    Event::KeyUp(Key::MoveRight) => self.key_state.right = false,
                    // Pass events to ui
                    Event::UiEvent(input) => {
                        self.test_ui.handle_event(input);
                    }
                    // Pass all other events to the scene
                    event => { self.scene.handle_input_event(event); },
                };
                // TODO: Do something if the event wasn't handled?
            }

            // Perform an in-game tick
            self.tick(clock.get_last_delta())
                .expect("Failed to tick the scene");

            // Maintain the scene
            self.scene.maintain(global_state.window.renderer_mut(), &self.client);
            // Maintain the UI
            self.test_ui.maintain(global_state.window.renderer_mut());

            // Render the session
            self.render(global_state.window.renderer_mut());

            // Display the frame on the window
            global_state.window
                .swap_buffers()
                .expect("Failed to swap window buffers");

            // Wait for the next tick
            clock.tick(Duration::from_millis(1000 / FPS));

            // Clean things up after the tick
            self.cleanup();
        }
    }

    fn name(&self) -> &'static str { "Session" }
}
