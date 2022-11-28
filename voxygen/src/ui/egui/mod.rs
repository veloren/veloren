use crate::{
    scene::{DebugShape, DebugShapeId, Scene},
    session::settings_change::{Graphics, SettingsChange},
    settings::Settings,
    window::Window,
};
use client::Client;
use egui::FontDefinitions;
use egui_winit_platform::{Platform, PlatformDescriptor};
use voxygen_egui::{EguiAction, EguiDebugInfo, EguiDebugShapeAction, EguiInnerState};

pub struct EguiState {
    pub platform: Platform,
    egui_inner_state: EguiInnerState,
    new_debug_shape_id: Option<u64>,
}

impl EguiState {
    pub fn new(window: &Window) -> Self {
        let platform = Platform::new(PlatformDescriptor {
            physical_width: window.window().inner_size().width,
            physical_height: window.window().inner_size().height,
            scale_factor: window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });

        Self {
            platform,
            egui_inner_state: EguiInnerState::default(),
            new_debug_shape_id: None,
        }
    }

    pub fn maintain(
        &mut self,
        client: &mut Client,
        scene: &mut Scene,
        debug_info: Option<EguiDebugInfo>,
        settings: &Settings,
    ) -> Option<SettingsChange> {
        use crate::render::ExperimentalShader;
        use strum::IntoEnumIterator;
        let experimental_shaders = ExperimentalShader::iter()
            .map(|s| {
                (
                    s.to_string(),
                    settings
                        .graphics
                        .render_mode
                        .experimental_shaders
                        .contains(&s),
                )
            })
            .collect();

        let egui_actions = voxygen_egui::maintain(
            &mut self.platform,
            &mut self.egui_inner_state,
            client,
            debug_info,
            self.new_debug_shape_id.take(),
            experimental_shaders,
        );

        let mut new_render_mode = None;

        egui_actions
            .actions
            .into_iter()
            .for_each(|action| match action {
                EguiAction::ChatCommand { cmd, args } => {
                    client.send_command(cmd.keyword().into(), args);
                },
                EguiAction::DebugShape(debug_shape_action) => match debug_shape_action {
                    EguiDebugShapeAction::AddCylinder { height, radius } => {
                        let shape_id = scene
                            .debug
                            .add_shape(DebugShape::Cylinder { height, radius });
                        self.new_debug_shape_id = Some(shape_id.0);
                    },
                    EguiDebugShapeAction::RemoveShape(debug_shape_id) => {
                        scene.debug.remove_shape(DebugShapeId(debug_shape_id));
                    },
                    EguiDebugShapeAction::SetPosAndColor { id, pos, color } => {
                        let identity_ori = [0.0, 0.0, 0.0, 1.0];
                        scene
                            .debug
                            .set_context(DebugShapeId(id), pos, color, identity_ori);
                    },
                },
                EguiAction::SetExperimentalShader(shader, enabled) => {
                    if let Ok(shader) = ExperimentalShader::try_from(shader.as_str()) {
                        let shaders = &mut new_render_mode
                            .get_or_insert_with(|| settings.graphics.render_mode.clone())
                            .experimental_shaders;

                        if enabled {
                            shaders.insert(shader);
                        } else {
                            shaders.remove(&shader);
                        }
                    }
                },
            });

        new_render_mode.map(|rm| SettingsChange::Graphics(Graphics::ChangeRenderMode(Box::new(rm))))
    }
}
