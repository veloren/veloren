// Library
use vek::*;
use image;

// Crate
use crate::{
    PlayState,
    PlayStateResult,
    GlobalState,
    window::Event,
    session::SessionState,
    render::Renderer,
    ui::{
        Ui,
        element::{
            Widget,
            image::Image,
        },
    },
};

pub struct TitleState {
    ui: Ui,
}

impl TitleState {
    /// Create a new `TitleState`
    pub fn new(renderer: &mut Renderer) -> Self {
        let img = Image::new(renderer, &image::open(concat!(env!("CARGO_MANIFEST_DIR"), "/test_assets/test.png")).unwrap()).unwrap();
        let widget = Widget::new(renderer, img).unwrap();
        Self {
            ui: Ui::new(renderer, widget).unwrap(),
        }
    }
}

// The background colour
const BG_COLOR: Rgba<f32> = Rgba { r: 0.0, g: 0.3, b: 1.0, a: 1.0 };

impl PlayState for TitleState {
    fn play(&mut self, global_state: &mut GlobalState) -> PlayStateResult {
        loop {
            // Handle window events
            for event in global_state.window.fetch_events() {
                match event {
                    Event::Close => return PlayStateResult::Shutdown,
                    // When space is pressed, start a session
                    Event::Char(' ') => return PlayStateResult::Push(
                        Box::new(SessionState::new(global_state.window.renderer_mut()).unwrap()), // TODO: Handle this error
                    ),
                    // Ignore all other events
                    _ => {},
                }
            }

            // Clear the screen
            global_state.window.renderer_mut().clear(BG_COLOR);

            // Maintain the UI
            self.ui.maintain(global_state.window.renderer_mut());

            // Draw the UI to the screen
            self.ui.render(global_state.window.renderer_mut());

            // Finish the frame
            global_state.window.renderer_mut().flush();
            global_state.window
                .swap_buffers()
                .expect("Failed to swap window buffers");
        }
    }

    fn name(&self) -> &'static str { "Title" }
}
