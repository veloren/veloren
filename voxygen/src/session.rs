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
    window::{Event, Key},
    render::Renderer,
    scene::Scene,
};

const FPS: u64 = 60;

pub struct SessionState {
    scene: Scene,
    client: Client,
}

/// Represents an active game session (i.e: one that is being played)
impl SessionState {
    /// Create a new `SessionState`
    pub fn new(renderer: &mut Renderer) -> Self {
        let client = Client::new().with_test_state(); // <--- TODO: Remove this
        Self {
            // Create a scene for this session. The scene handles visible elements of the game world
            scene: Scene::new(renderer, &client),
            client,
        }
    }
}

// The background colour
const BG_COLOR: Rgba<f32> = Rgba { r: 0.0, g: 0.3, b: 1.0, a: 1.0 };

impl SessionState {
    /// Tick the session (and the client attached to it)
    pub fn tick(&mut self, dt: Duration) -> Result<(), Error> {
        self.client.tick(client::Input {}, dt)?;
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
        for x in -4..5 {
            for y in -4..5 {
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
