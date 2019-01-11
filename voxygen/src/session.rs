// Library
use vek::*;

// Crate
use crate::{
    PlayState,
    PlayStateResult,
    GlobalState,
    window::Event,
    render::Renderer,
    scene::Scene,
};

pub struct SessionState {
    scene: Scene,
}

/// Represents an active game session (i.e: one that is being played)
impl SessionState {
    /// Create a new `SessionState`
    pub fn from_renderer(renderer: &mut Renderer) -> Self {
        Self {
            // Create a scene for this session. The scene handles visible elements of the game world
            scene: Scene::new(renderer),
        }
    }
}

// The background colour
const BG_COLOR: Rgba<f32> = Rgba { r: 0.0, g: 0.3, b: 1.0, a: 1.0 };

impl PlayState for SessionState {
    fn play(&mut self, global_state: &mut GlobalState) -> PlayStateResult {
        // Game loop
        loop {
            // Handle window events
            for event in global_state.window.fetch_events() {
                match event {
                    Event::Close => return PlayStateResult::Shutdown,
                    // When 'q' is pressed, exit the session
                    Event::Char('q') => return PlayStateResult::Pop,
                    // Ignore all other events
                    _ => {},
                }
            }

            // Clear the screen
            global_state.window.renderer_mut().clear(BG_COLOR);

            // Render the screen using the global renderer
            self.scene.render_to(global_state.window.renderer_mut());

            // Finish the frame
            global_state.window.renderer_mut().flush();
            global_state.window.display()
                .expect("Failed to display window");
        }
    }

    fn name(&self) -> &'static str { "Session" }
}
