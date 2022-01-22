use crate::{EguiAction, EguiActions};
use egui::{CtxRef, Vec2, Window};

pub fn draw_experimental_shaders_window(
    ctx: &CtxRef,
    open: &mut bool,
    egui_actions: &mut EguiActions,
    experimental_shaders: &[(String, bool)],
) {
    Window::new("Experimental Shaders")
        .open(open)
        .default_width(250.0)
        .default_height(600.0)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
            experimental_shaders.iter().for_each(|(shader, enabled)| {
                let mut enabled_mut = *enabled;

                ui.checkbox(&mut enabled_mut, shader);

                if enabled_mut != *enabled {
                    egui_actions.actions.push(EguiAction::SetExperimentalShader(
                        shader.into(),
                        enabled_mut,
                    ));
                }
            })
        });
}
