use crate::{
    scene::{DebugShape, DebugShapeId, Scene},
    session::settings_change::{Graphics, SettingsChange},
    settings::Settings,
    window::Window,
};
use client::Client;
use egui::{Context, ViewportId};
use egui_winit::State as WinitState;
use voxygen_egui::{EguiAction, EguiDebugInfo, EguiDebugShapeAction, EguiInnerState};

pub struct EguiState {
    pub winit_state: WinitState,
    egui_inner_state: EguiInnerState,
    new_debug_shape_id: Option<u64>,
}

impl EguiState {
    pub fn new(window: &Window) -> Self {
        let egui_ctx = Context::default();
        let winit_state = WinitState::new(
            egui_ctx,
            ViewportId::ROOT,
            window.window(),
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        Self {
            winit_state,
            egui_inner_state: EguiInnerState::default(),
            new_debug_shape_id: None,
        }
    }

    pub fn maintain(
        &mut self,
        client: &mut Client,
        scene: &mut Scene,
        window: &winit::window::Window,
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
            &mut self.winit_state,
            &mut self.egui_inner_state,
            client,
            window,
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
                EguiAction::SetShowDebugVector(enabled) => {
                    scene.debug_vectors_enabled = enabled;
                },
            });

        new_render_mode.map(|rm| SettingsChange::Graphics(Graphics::ChangeRenderMode(Box::new(rm))))
    }
}
