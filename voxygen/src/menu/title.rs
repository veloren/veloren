// Library
use vek::*;
use image;
use conrod_core::Ui;
use conrod_core::UiBuilder;
use conrod_core::widget::image::Image as ImageWidget;
use conrod_core::image::Map as ImageMap;
use conrod_core::{Positionable, Sizeable};
use conrod_core::Widget;

// Crate
use crate::{
    PlayState,
    PlayStateResult,
    GlobalState,
    window::Event,
    session::SessionState,
    render::{
        Consts,
        UiLocals,
        Renderer,
        Texture,
        UiPipeline,
        create_ui_quad_mesh,
    },
};

pub struct TitleState {
    ui: Ui,
    image_map: ImageMap<Texture<UiPipeline>>,
}

impl TitleState {
    /// Create a new `TitleState`
    pub fn new(renderer: &mut Renderer) -> Self {
        let mut ui = UiBuilder::new([500.0, 500.0]).build();
        let widget_id = ui.widget_id_generator().next();
        let mut image_map = ImageMap::new();
        let img = image::open(concat!(env!("CARGO_MANIFEST_DIR"), "/test_assets/test.png")).unwrap();
        let img = renderer.create_texture(&img).unwrap();
        let img_id = image_map.insert(img);
        ImageWidget::new(img_id)
            .x_y(0.0, 0.0)
            .w_h(500.0,500.0)
            .set(widget_id, &mut ui.set_widgets());
        Self {
            ui,
            image_map
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


            // Maintain the UI
            //self.ui.maintain(global_state.window.renderer_mut());

            // Draw the UI to the screen
            //self.ui.render(global_state.window.renderer_mut());
            if let Some(mut primitives) = Some(self.ui.draw()){//_if_changed() {
                // Clear the screen
                global_state.window.renderer_mut().clear(BG_COLOR);

                //render the primatives one at a time
                while let Some(prim) = primitives.next() {
                    let mut renderer = global_state.window.renderer_mut();
                    use conrod_core::render::{Primitive, PrimitiveKind};
                    let Primitive {kind, scizzor, rect, ..} = prim;
                    match kind {
                        PrimitiveKind::Image { image_id, color, source_rect } => {

                            let mut locals = renderer.create_consts(&[UiLocals::default()]).unwrap();
                            renderer.update_consts(&mut locals, &[UiLocals::new(
                                    [0.0, 0.0, 1.0, 1.0],
                                )]);
                            let model = renderer.create_model(&create_ui_quad_mesh()).unwrap();
                            global_state.window.renderer_mut().render_ui_element(&model, &locals, self.image_map.get(&image_id).unwrap())
                        }
                        _ => {println!("should not reach here");}
                    }
                }


                // Finish the frame
                global_state.window.renderer_mut().flush();
                global_state.window
                    .swap_buffers()
                    .expect("Failed to swap window buffers");
            }



        }
    }

    fn name(&self) -> &'static str { "Title" }
}
