//! Module containing controller-specific abstractions allowing complex
//! keybindings

use crate::{game_input::GameInput, window::MenuInput};
use gilrs::{Axis as GilAxis, Button as GilButton, ev::Code as GilCode};
use hashbrown::{HashMap, HashSet};
use i18n::Localization;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

#[derive(Serialize, Deserialize)]
struct ControllerSettingsSerde {
    // save as a delta against defaults for efficiency
    game_button_map: HashMap<GameInput, Option<Button>>,
    menu_button_map: HashMap<MenuInput, Option<Button>>,
    game_analog_button_map: HashMap<AnalogButtonGameAction, AnalogButton>,
    menu_analog_button_map: HashMap<AnalogButtonMenuAction, AnalogButton>,
    game_axis_map: HashMap<AxisGameAction, Axis>,
    menu_axis_map: HashMap<AxisMenuAction, Axis>,
    layer_button_map: HashMap<GameInput, Option<LayerEntry>>,
    modifier_buttons: Vec<Button>,

    pan_sensitivity: u32,
    pan_invert_y: bool,
    axis_deadzones: HashMap<Axis, f32>,
    button_deadzones: HashMap<AnalogButton, f32>,
    mouse_emulation_sensitivity: u32,
    inverted_axes: Vec<Axis>,
}

impl From<ControllerSettings> for ControllerSettingsSerde {
    fn from(controller_settings: ControllerSettings) -> Self {
        // Do a delta between default() ControllerSettings and the argument,
        // let buttons be only the custom keybindings chosen by the user
        //
        // check game buttons
        let mut button_bindings: HashMap<GameInput, Option<Button>> = HashMap::new();
        for (k, v) in controller_settings.game_button_map {
            if ControllerSettings::default_button_binding(k) != v {
                button_bindings.insert(k, v);
            }
        }

        // check game layers
        let mut layer_bindings: HashMap<GameInput, Option<LayerEntry>> = HashMap::new();
        for (k, v) in controller_settings.layer_button_map {
            if ControllerSettings::default_layer_binding(k) != v {
                layer_bindings.insert(k, v);
            }
        }

        // none hashmap values
        let modifier_buttons = controller_settings.modifier_buttons;
        let pan_sensitivity = controller_settings.pan_sensitivity;
        let pan_invert_y = controller_settings.pan_invert_y;
        let axis_deadzones = controller_settings.axis_deadzones;

        let mouse_emulation_sensitivity = controller_settings.mouse_emulation_sensitivity;
        let inverted_axes = controller_settings.inverted_axes;

        ControllerSettingsSerde {
            game_button_map: button_bindings,
            menu_button_map: HashMap::new(),
            game_analog_button_map: HashMap::new(),
            menu_analog_button_map: HashMap::new(),
            game_axis_map: HashMap::new(),
            menu_axis_map: HashMap::new(),
            layer_button_map: layer_bindings,

            modifier_buttons,
            pan_sensitivity,
            pan_invert_y,
            axis_deadzones,

            button_deadzones: HashMap::new(),
            mouse_emulation_sensitivity,
            inverted_axes,
        }
    }
}

/// Contains all controller related settings and keymaps
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(from = "ControllerSettingsSerde", into = "ControllerSettingsSerde")]
pub struct ControllerSettings {
    pub game_button_map: HashMap<GameInput, Option<Button>>,
    pub inverse_game_button_map: HashMap<Button, HashSet<GameInput>>,
    pub menu_button_map: HashMap<MenuInput, Option<Button>>,
    pub inverse_menu_button_map: HashMap<Button, HashSet<MenuInput>>,
    pub game_analog_button_map: HashMap<AnalogButtonGameAction, AnalogButton>,
    pub inverse_game_analog_button_map: HashMap<AnalogButton, HashSet<AnalogButtonGameAction>>,
    pub menu_analog_button_map: HashMap<AnalogButtonMenuAction, AnalogButton>,
    pub inverse_menu_analog_button_map: HashMap<AnalogButton, HashSet<AnalogButtonMenuAction>>,
    pub game_axis_map: HashMap<AxisGameAction, Axis>,
    pub inverse_game_axis_map: HashMap<Axis, HashSet<AxisGameAction>>,
    pub menu_axis_map: HashMap<AxisMenuAction, Axis>,
    pub inverse_menu_axis_map: HashMap<Axis, HashSet<AxisMenuAction>>,
    pub layer_button_map: HashMap<GameInput, Option<LayerEntry>>,
    pub inverse_layer_button_map: HashMap<LayerEntry, HashSet<GameInput>>,

    pub modifier_buttons: Vec<Button>,
    pub pan_sensitivity: u32,
    pub pan_invert_y: bool,
    pub axis_deadzones: HashMap<Axis, f32>,
    pub button_deadzones: HashMap<AnalogButton, f32>,
    pub mouse_emulation_sensitivity: u32,
    pub inverted_axes: Vec<Axis>,
}

impl From<ControllerSettingsSerde> for ControllerSettings {
    fn from(controller_serde: ControllerSettingsSerde) -> Self {
        let button_bindings = controller_serde.game_button_map;
        let layer_bindings = controller_serde.layer_button_map;
        let mut controller_settings = ControllerSettings::default();
        // update button bindings
        for (k, maybe_v) in button_bindings {
            match maybe_v {
                Some(v) => controller_settings.modify_button_binding(k, v),
                None => controller_settings.remove_button_binding(k),
            }
        }
        // update layer bindings
        for (k, maybe_v) in layer_bindings {
            match maybe_v {
                Some(v) => controller_settings.modify_layer_binding(k, v),
                None => controller_settings.remove_layer_binding(k),
            }
        }
        controller_settings
    }
}

impl ControllerSettings {
    pub fn apply_axis_deadzone(&self, k: &Axis, input: f32) -> f32 {
        let threshold = *self.axis_deadzones.get(k).unwrap_or(&0.2);

        // This could be one comparison per handled event faster if threshold was
        // guaranteed to fall into <0, 1) range
        let input_abs = input.abs();
        if input_abs <= threshold || threshold >= 1.0 {
            0.0
        } else if threshold <= 0.0 {
            input
        } else {
            (input_abs - threshold) / (1.0 - threshold) * input.signum()
        }
    }

    pub fn apply_button_deadzone(&self, k: &AnalogButton, input: f32) -> f32 {
        let threshold = *self.button_deadzones.get(k).unwrap_or(&0.2);

        // This could be one comparison per handled event faster if threshold was
        // guaranteed to fall into <0, 1) range
        if input <= threshold || threshold >= 1.0 {
            0.0
        } else if threshold <= 0.0 {
            input
        } else {
            (input - threshold) / (1.0 - threshold)
        }
    }

    pub fn remove_button_binding(&mut self, game_input: GameInput) {
        if let Some(inverse) = self
            .game_button_map
            .insert(game_input, None)
            .flatten()
            .and_then(|button| self.inverse_game_button_map.get_mut(&button))
        {
            inverse.remove(&game_input);
        }
    }

    pub fn remove_layer_binding(&mut self, layer_input: GameInput) {
        if let Some(inverse) = self
            .layer_button_map
            .insert(layer_input, None)
            .flatten()
            .and_then(|button| self.inverse_layer_button_map.get_mut(&button))
        {
            inverse.remove(&layer_input);
        }
    }

    pub fn remove_menu_binding(&mut self, menu_input: MenuInput) {
        if let Some(inverse) = self
            .menu_button_map
            .insert(menu_input, None)
            .flatten()
            .and_then(|button| self.inverse_menu_button_map.get_mut(&button))
        {
            inverse.remove(&menu_input);
        }
    }

    pub fn get_game_button_binding(&self, input: GameInput) -> Option<Button> {
        self.game_button_map.get(&input).cloned().flatten()
    }

    pub fn get_associated_game_button_inputs(
        &self,
        button: &Button,
    ) -> Option<&HashSet<GameInput>> {
        self.inverse_game_button_map.get(button)
    }

    pub fn get_associated_game_layer_inputs(
        &self,
        layers: &LayerEntry,
    ) -> Option<&HashSet<GameInput>> {
        self.inverse_layer_button_map.get(layers)
    }

    pub fn get_menu_button_binding(&self, input: MenuInput) -> Option<Button> {
        self.menu_button_map.get(&input).cloned().flatten()
    }

    pub fn get_layer_button_binding(&self, input: GameInput) -> Option<LayerEntry> {
        self.layer_button_map.get(&input).cloned().flatten()
    }

    pub fn insert_game_button_binding(&mut self, game_input: GameInput, game_button: Button) {
        if game_button != Button::default() {
            self.game_button_map.insert(game_input, Some(game_button));
            self.inverse_game_button_map
                .entry(game_button)
                .or_default()
                .insert(game_input);
        }
    }

    pub fn insert_menu_button_binding(&mut self, menu_input: MenuInput, button: Button) {
        if button != Button::default() {
            self.menu_button_map.insert(menu_input, Some(button));
            self.inverse_menu_button_map
                .entry(button)
                .or_default()
                .insert(menu_input);
        }
    }

    pub fn insert_game_axis_binding(&mut self, input: AxisGameAction, axis: Axis) {
        if axis != Axis::default() {
            self.game_axis_map.insert(input, axis);
            self.inverse_game_axis_map
                .entry(axis)
                .or_default()
                .insert(input);
        }
    }

    pub fn insert_menu_axis_binding(&mut self, input: AxisMenuAction, axis: Axis) {
        if axis != Axis::default() {
            self.menu_axis_map.insert(input, axis);
            self.inverse_menu_axis_map
                .entry(axis)
                .or_default()
                .insert(input);
        }
    }

    pub fn insert_layer_button_binding(&mut self, input: GameInput, layer_entry: LayerEntry) {
        if layer_entry != LayerEntry::default() {
            self.layer_button_map.insert(input, Some(layer_entry));
            self.inverse_layer_button_map
                .entry(layer_entry)
                .or_default()
                .insert(input);
        }
    }

    pub fn modify_button_binding(&mut self, game_input: GameInput, button: Button) {
        // for the Button->GameInput hashmap, we first need to remove the GameInput from
        // the old binding
        if let Some(old_binding) = self.get_game_button_binding(game_input) {
            self.inverse_game_button_map
                .entry(old_binding)
                .or_default()
                .remove(&game_input);
        }
        // then we add the GameInput to the proper key
        self.inverse_game_button_map
            .entry(button)
            .or_default()
            .insert(game_input);
        // for the GameInput->button hashmap, just overwrite the value
        self.game_button_map.insert(game_input, Some(button));
    }

    pub fn modify_layer_binding(&mut self, game_input: GameInput, layers: LayerEntry) {
        // for the LayerEntry->GameInput hashmap, we first need to remove the GameInput
        // from the old binding
        if let Some(old_binding) = self.get_layer_button_binding(game_input) {
            self.inverse_layer_button_map
                .entry(old_binding)
                .or_default()
                .remove(&game_input);
        }
        // then we add the GameInput to the proper key
        self.inverse_layer_button_map
            .entry(layers)
            .or_default()
            .insert(game_input);
        // for the GameInput->layer hashmap, just overwrite the value
        self.layer_button_map.insert(game_input, Some(layers));
    }

    pub fn modify_menu_binding(&mut self, menu_input: MenuInput, button: Button) {
        // for the Button->GameInput hashmap, we first need to remove the GameInput from
        // the old binding
        if let Some(old_binding) = self.get_menu_button_binding(menu_input) {
            self.inverse_menu_button_map
                .entry(old_binding)
                .or_default()
                .remove(&menu_input);
        }
        // then we add the GameInput to the proper key
        self.inverse_menu_button_map
            .entry(button)
            .or_default()
            .insert(menu_input);
        // for the MenuInput->button hashmap, just overwrite the value
        self.menu_button_map.insert(menu_input, Some(button));
    }

    /// Return true if this button is used for multiple GameInputs that aren't
    /// expected to be safe to have bound to the same button at the same time
    pub fn game_button_has_conflicting_bindings(&self, game_button: Button) -> bool {
        if let Some(game_inputs) = self.inverse_game_button_map.get(&game_button) {
            for a in game_inputs.iter() {
                for b in game_inputs.iter() {
                    if !GameInput::can_share_bindings(*a, *b) {
                        return true;
                    }
                }
            }

            let layer_entry = LayerEntry {
                button: game_button,
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            };
            if let Some(layer_inputs) = self.inverse_layer_button_map.get(&layer_entry) {
                for a in game_inputs.iter() {
                    for b in layer_inputs.iter() {
                        if !GameInput::can_share_bindings(*a, *b) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    pub fn menu_button_has_conflicting_bindings(&self, menu_button: Button) -> bool {
        self.inverse_menu_button_map
            .get(&menu_button)
            .is_some_and(|menu_inputs| menu_inputs.len() > 1)
    }

    /// Return true if this key is used for multiple GameInputs that aren't
    /// expected to be safe to have bound to the same key at the same time
    pub fn layer_entry_has_conflicting_bindings(&self, layer_entry: LayerEntry) -> bool {
        if let Some(layer_inputs) = self.inverse_layer_button_map.get(&layer_entry) {
            for a in layer_inputs.iter() {
                for b in layer_inputs.iter() {
                    if !GameInput::can_share_bindings(*a, *b) {
                        return true;
                    }
                }
            }

            if layer_entry.mod1 == Button::Simple(GilButton::Unknown)
                && layer_entry.mod2 == Button::Simple(GilButton::Unknown)
                && let Some(game_inputs) = self.inverse_game_button_map.get(&layer_entry.button)
            {
                for a in layer_inputs.iter() {
                    for b in game_inputs.iter() {
                        if !GameInput::can_share_bindings(*a, *b) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    pub fn default_game_axis(game_axis: AxisGameAction) -> Option<Axis> {
        match game_axis {
            AxisGameAction::MovementX => Some(Axis::Simple(GilAxis::LeftStickX)),
            AxisGameAction::MovementY => Some(Axis::Simple(GilAxis::LeftStickY)),
            AxisGameAction::CameraX => Some(Axis::Simple(GilAxis::RightStickX)),
            AxisGameAction::CameraY => Some(Axis::Simple(GilAxis::RightStickY)),
        }
    }

    pub fn default_menu_axis(menu_axis: AxisMenuAction) -> Option<Axis> {
        match menu_axis {
            AxisMenuAction::MoveX => Some(Axis::Simple(GilAxis::LeftStickX)),
            AxisMenuAction::MoveY => Some(Axis::Simple(GilAxis::LeftStickY)),
            AxisMenuAction::ScrollX => Some(Axis::Simple(GilAxis::RightStickX)),
            AxisMenuAction::ScrollY => Some(Axis::Simple(GilAxis::RightStickY)),
        }
    }

    pub fn default_button_binding(game_input: GameInput) -> Option<Button> {
        match game_input {
            GameInput::Primary => Some(Button::Simple(GilButton::RightTrigger2)),
            GameInput::Secondary => Some(Button::Simple(GilButton::LeftTrigger2)),
            GameInput::Block => Some(Button::Simple(GilButton::LeftTrigger)),
            GameInput::Slot1 => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Slot2 => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Slot3 => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Slot4 => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Slot5 => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Slot6 => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Slot7 => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Slot8 => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Slot9 => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Slot10 => Some(Button::Simple(GilButton::Unknown)),
            GameInput::ToggleCursor => Some(Button::Simple(GilButton::Unknown)),
            GameInput::MoveForward => Some(Button::Simple(GilButton::Unknown)),
            GameInput::MoveBack => Some(Button::Simple(GilButton::Unknown)),
            GameInput::MoveLeft => Some(Button::Simple(GilButton::Unknown)),
            GameInput::MoveRight => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Jump => Some(Button::Simple(GilButton::South)),
            GameInput::WallJump => Some(Button::Simple(GilButton::South)),
            GameInput::Sit => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Crawl => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Dance => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Greet => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Glide => Some(Button::Simple(GilButton::DPadUp)),
            GameInput::SwimUp => Some(Button::Simple(GilButton::South)),
            GameInput::SwimDown => Some(Button::Simple(GilButton::West)),
            GameInput::Fly => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Sneak => Some(Button::Simple(GilButton::LeftThumb)),
            GameInput::CancelClimb => Some(Button::Simple(GilButton::East)),
            GameInput::ToggleLantern => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Mount => Some(Button::Simple(GilButton::West)),
            GameInput::StayFollow => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Chat => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Command => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Escape => Some(Button::Simple(GilButton::Start)),
            GameInput::Map => Some(Button::Simple(GilButton::Select)),
            GameInput::Inventory => Some(Button::Simple(GilButton::DPadRight)),
            GameInput::Trade => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Social => Some(Button::Simple(GilButton::DPadLeft)),
            GameInput::Crafting => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Diary => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Settings => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Controls => Some(Button::Simple(GilButton::Unknown)),
            GameInput::ToggleInterface => Some(Button::Simple(GilButton::Unknown)),
            GameInput::ToggleDebug => Some(Button::Simple(GilButton::Unknown)),
            #[cfg(feature = "egui-ui")]
            GameInput::ToggleEguiDebug => Some(Button::Simple(GilButton::Unknown)),
            GameInput::ToggleChat => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Fullscreen => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Screenshot => Some(Button::Simple(GilButton::Unknown)),
            GameInput::ToggleIngameUi => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Roll => Some(Button::Simple(GilButton::RightThumb)),
            GameInput::GiveUp => Some(Button::Simple(GilButton::South)),
            GameInput::Respawn => Some(Button::Simple(GilButton::South)),
            GameInput::Interact => Some(Button::Simple(GilButton::West)),
            GameInput::ToggleWield => Some(Button::Simple(GilButton::East)),
            GameInput::SwapLoadout => Some(Button::Simple(GilButton::DPadDown)),
            GameInput::FreeLook => Some(Button::Simple(GilButton::Unknown)),
            GameInput::AutoWalk => Some(Button::Simple(GilButton::Unknown)),
            GameInput::ZoomIn => Some(Button::Simple(GilButton::Unknown)),
            GameInput::ZoomOut => Some(Button::Simple(GilButton::Unknown)),
            GameInput::ZoomLock => Some(Button::Simple(GilButton::Unknown)),
            GameInput::CameraClamp => Some(Button::Simple(GilButton::Unknown)),
            GameInput::CycleCamera => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Select => Some(Button::Simple(GilButton::Unknown)),
            GameInput::AcceptGroupInvite => Some(Button::Simple(GilButton::Unknown)),
            GameInput::DeclineGroupInvite => Some(Button::Simple(GilButton::Unknown)),
            GameInput::MapZoomIn => Some(Button::Simple(GilButton::Unknown)),
            GameInput::MapZoomOut => Some(Button::Simple(GilButton::Unknown)),
            GameInput::MapSetMarker => Some(Button::Simple(GilButton::Unknown)),
            GameInput::SpectateSpeedBoost => Some(Button::Simple(GilButton::Unknown)),
            GameInput::SpectateViewpoint => Some(Button::Simple(GilButton::Unknown)),
            GameInput::MuteMaster => Some(Button::Simple(GilButton::Unknown)),
            GameInput::MuteInactiveMaster => Some(Button::Simple(GilButton::Unknown)),
            GameInput::MuteMusic => Some(Button::Simple(GilButton::Unknown)),
            GameInput::MuteSfx => Some(Button::Simple(GilButton::Unknown)),
            GameInput::MuteAmbience => Some(Button::Simple(GilButton::Unknown)),
            GameInput::ToggleWalk => Some(Button::Simple(GilButton::Unknown)),
        }
    }

    pub fn default_layer_binding(layer_input: GameInput) -> Option<LayerEntry> {
        match layer_input {
            GameInput::Primary => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Secondary => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Block => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Slot1 => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Slot2 => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Slot3 => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Slot4 => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Slot5 => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Slot6 => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Slot7 => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Slot8 => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Slot9 => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Slot10 => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::ToggleCursor => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::MoveForward => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::MoveBack => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::MoveLeft => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::MoveRight => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Jump => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::WallJump => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Sit => Some(LayerEntry {
                button: Button::Simple(GilButton::DPadDown),
                mod1: Button::Simple(GilButton::RightTrigger),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Crawl => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Dance => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Greet => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Glide => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::SwimUp => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::SwimDown => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Fly => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Sneak => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::CancelClimb => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::ToggleLantern => Some(LayerEntry {
                button: Button::Simple(GilButton::DPadUp),
                mod1: Button::Simple(GilButton::RightTrigger),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Mount => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::StayFollow => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Chat => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Command => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Escape => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Map => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Inventory => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Trade => Some(LayerEntry {
                button: Button::Simple(GilButton::North),
                mod1: Button::Simple(GilButton::RightTrigger),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Social => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Crafting => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Diary => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Settings => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Controls => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::ToggleInterface => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::ToggleDebug => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            #[cfg(feature = "egui-ui")]
            GameInput::ToggleEguiDebug => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::ToggleChat => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Fullscreen => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Screenshot => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::ToggleIngameUi => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Roll => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::GiveUp => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Respawn => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Interact => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::ToggleWield => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::SwapLoadout => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::FreeLook => Some(LayerEntry {
                button: Button::Simple(GilButton::South),
                mod1: Button::Simple(GilButton::RightTrigger),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::AutoWalk => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::ZoomIn => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::ZoomOut => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::ZoomLock => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::CameraClamp => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::CycleCamera => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::Select => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::AcceptGroupInvite => Some(LayerEntry {
                button: Button::Simple(GilButton::DPadLeft),
                mod1: Button::Simple(GilButton::RightTrigger),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::DeclineGroupInvite => Some(LayerEntry {
                button: Button::Simple(GilButton::DPadRight),
                mod1: Button::Simple(GilButton::RightTrigger),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::MapZoomIn => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::MapZoomOut => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::MapSetMarker => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::SpectateSpeedBoost => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::SpectateViewpoint => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::MuteMaster => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::MuteInactiveMaster => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::MuteMusic => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::MuteSfx => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::MuteAmbience => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
            GameInput::ToggleWalk => Some(LayerEntry {
                button: Button::Simple(GilButton::Unknown),
                mod1: Button::Simple(GilButton::Unknown),
                mod2: Button::Simple(GilButton::Unknown),
            }),
        }
    }

    pub fn default_menu_button_binding(menu_input: MenuInput) -> Option<Button> {
        match menu_input {
            MenuInput::Up => Some(Button::Simple(GilButton::DPadUp)),
            MenuInput::Down => Some(Button::Simple(GilButton::DPadDown)),
            MenuInput::Left => Some(Button::Simple(GilButton::DPadLeft)),
            MenuInput::Right => Some(Button::Simple(GilButton::DPadRight)),
            MenuInput::ScrollUp => Some(Button::Simple(GilButton::Unknown)),
            MenuInput::ScrollDown => Some(Button::Simple(GilButton::Unknown)),
            MenuInput::ScrollLeft => Some(Button::Simple(GilButton::Unknown)),
            MenuInput::ScrollRight => Some(Button::Simple(GilButton::Unknown)),
            MenuInput::Home => Some(Button::Simple(GilButton::Unknown)),
            MenuInput::End => Some(Button::Simple(GilButton::Unknown)),
            MenuInput::Apply => Some(Button::Simple(GilButton::South)),
            MenuInput::Back => Some(Button::Simple(GilButton::East)),
            MenuInput::Exit => Some(Button::Simple(GilButton::Mode)),
        }
    }
}

impl Default for ControllerSettings {
    fn default() -> Self {
        let mut controller_settings = Self {
            game_button_map: HashMap::new(),
            inverse_game_button_map: HashMap::new(),
            menu_button_map: HashMap::new(),
            inverse_menu_button_map: HashMap::new(),
            game_analog_button_map: HashMap::new(),
            inverse_game_analog_button_map: HashMap::new(),
            menu_analog_button_map: HashMap::new(),
            inverse_menu_analog_button_map: HashMap::new(),
            game_axis_map: HashMap::new(),
            inverse_game_axis_map: HashMap::new(),
            menu_axis_map: HashMap::new(),
            inverse_menu_axis_map: HashMap::new(),
            layer_button_map: HashMap::new(),
            inverse_layer_button_map: HashMap::new(),

            modifier_buttons: vec![
                Button::Simple(GilButton::RightTrigger),
                Button::Simple(GilButton::LeftTrigger),
            ],
            pan_sensitivity: 10,
            pan_invert_y: false,
            axis_deadzones: HashMap::new(),
            button_deadzones: HashMap::new(),
            mouse_emulation_sensitivity: 12,
            inverted_axes: Vec::new(),
        };
        // sets the button bindings for game button inputs
        for button_input in GameInput::iter() {
            match ControllerSettings::default_button_binding(button_input) {
                None => {},
                Some(default) => {
                    controller_settings.insert_game_button_binding(button_input, default)
                },
            };
        }
        // sets the layer bindings for game layer inputs
        for layer_input in GameInput::iter() {
            if let Some(default) = ControllerSettings::default_layer_binding(layer_input) {
                controller_settings.insert_layer_button_binding(layer_input, default);
            };
        }
        // sets the menu button bindings for game menu button inputs
        for button_input in MenuInput::iter() {
            if let Some(default) = ControllerSettings::default_menu_button_binding(button_input) {
                controller_settings.insert_menu_button_binding(button_input, default)
            };
        }
        // sets the axis bindings for game axis inputs
        for axis_input in AxisGameAction::iter() {
            if let Some(default) = ControllerSettings::default_game_axis(axis_input) {
                controller_settings.insert_game_axis_binding(axis_input, default)
            };
        }
        // sets the axis bindings for menu axis inputs
        for axis_input in AxisMenuAction::iter() {
            if let Some(default) = ControllerSettings::default_menu_axis(axis_input) {
                controller_settings.insert_menu_axis_binding(axis_input, default)
            };
        }
        controller_settings
    }
}

/// All the menu actions you can bind to an Axis
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize, EnumIter)]
pub enum AxisMenuAction {
    MoveX,
    MoveY,
    ScrollX,
    ScrollY,
}

/// All the game actions you can bind to an Axis
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize, EnumIter)]
pub enum AxisGameAction {
    MovementX,
    MovementY,
    CameraX,
    CameraY,
}

/// All the menu actions you can bind to an analog button
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum AnalogButtonMenuAction {}

/// All the game actions you can bind to an analog button
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum AnalogButtonGameAction {}

/// Button::Simple(GilButton::Unknown) is invalid and equal to mapping an action
/// to nothing
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Button {
    Simple(GilButton),
    EventCode(u32),
}

impl Default for Button {
    fn default() -> Self { Button::Simple(GilButton::Unknown) }
}

impl Button {
    // Returns button description (e.g Left Trigger)
    pub fn display_string(&self, localized_strings: &Localization) -> String {
        use self::Button::*;
        // This exists here to keep the string in scope after the match
        let button_string = match self {
            Simple(GilButton::South) => localized_strings.get_msg("gamepad-south").to_string(),
            Simple(GilButton::East) => localized_strings.get_msg("gamepad-east").to_string(),
            Simple(GilButton::North) => localized_strings.get_msg("gamepad-north").to_string(),
            Simple(GilButton::West) => localized_strings.get_msg("gamepad-west").to_string(),
            Simple(GilButton::C) => localized_strings.get_msg("gamepad-c").to_string(),
            Simple(GilButton::Z) => localized_strings.get_msg("gamepad-z").to_string(),
            Simple(GilButton::LeftTrigger) => localized_strings
                .get_msg("gamepad-left_trigger")
                .to_string(),
            Simple(GilButton::LeftTrigger2) => localized_strings
                .get_msg("gamepad-left_trigger_2")
                .to_string(),
            Simple(GilButton::RightTrigger) => localized_strings
                .get_msg("gamepad-right_trigger")
                .to_string(),
            Simple(GilButton::RightTrigger2) => localized_strings
                .get_msg("gamepad-right_trigger_2")
                .to_string(),
            Simple(GilButton::Select) => localized_strings.get_msg("gamepad-select").to_string(),
            Simple(GilButton::Start) => localized_strings.get_msg("gamepad-start").to_string(),
            Simple(GilButton::Mode) => localized_strings.get_msg("gamepad-mode").to_string(),
            Simple(GilButton::LeftThumb) => {
                localized_strings.get_msg("gamepad-left_thumb").to_string()
            },
            Simple(GilButton::RightThumb) => {
                localized_strings.get_msg("gamepad-right_thumb").to_string()
            },
            Simple(GilButton::DPadUp) => localized_strings.get_msg("gamepad-dpad_up").to_string(),
            Simple(GilButton::DPadDown) => {
                localized_strings.get_msg("gamepad-dpad_down").to_string()
            },
            Simple(GilButton::DPadLeft) => {
                localized_strings.get_msg("gamepad-dpad_left").to_string()
            },
            Simple(GilButton::DPadRight) => {
                localized_strings.get_msg("gamepad-dpad_right").to_string()
            },
            Simple(GilButton::Unknown) => localized_strings.get_msg("gamepad-unknown").to_string(),
            EventCode(code) => code.to_string(),
        };

        button_string.to_owned()
    }

    // If it exists, returns the shortened version of a button name
    // (e.g. Left Trigger -> LT)
    pub fn try_shortened(&self) -> Option<String> {
        use self::Button::*;
        let button_string = match self {
            Simple(GilButton::South) => "A",
            Simple(GilButton::East) => "B",
            Simple(GilButton::North) => "Y",
            Simple(GilButton::West) => "X",
            Simple(GilButton::LeftTrigger) => "LB",
            Simple(GilButton::LeftTrigger2) => "LT",
            Simple(GilButton::RightTrigger) => "RB",
            Simple(GilButton::RightTrigger2) => "RT",
            Simple(GilButton::LeftThumb) => "L3",
            Simple(GilButton::RightThumb) => "R3",
            _ => return None,
        };

        Some(button_string.to_owned())
    }
}

// represents a controller button to fire a GameInput on
// includes two modifier buttons to determine what layer is active
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct LayerEntry {
    pub button: Button,
    pub mod1: Button,
    pub mod2: Button,
}

impl Default for LayerEntry {
    fn default() -> Self {
        // binding to unknown = getting skipped from processing
        Self {
            button: Button::Simple(GilButton::Unknown),
            mod1: Button::Simple(GilButton::Unknown),
            mod2: Button::Simple(GilButton::Unknown),
        }
    }
}

impl LayerEntry {
    pub fn display_string(&self, localized_strings: &Localization) -> String {
        use self::Button::*;

        let mod1: Option<String> = match self.mod1 {
            Simple(GilButton::Unknown) => None,
            _ => self
                .mod1
                .try_shortened()
                .map_or(Some(self.mod1.display_string(localized_strings)), Some),
        };
        let mod2: Option<String> = match self.mod2 {
            Simple(GilButton::Unknown) => None,
            _ => self
                .mod2
                .try_shortened()
                .map_or(Some(self.mod2.display_string(localized_strings)), Some),
        };

        format!(
            "{}{}{} {}",
            mod1.map_or("".to_owned(), |m1| format!("{} + ", m1)),
            mod2.map_or("".to_owned(), |m2| format!("{} + ", m2)),
            self.button.display_string(localized_strings),
            self.button
                .try_shortened()
                .map_or("".to_owned(), |short| format!("({})", short))
        )
    }
}

/// AnalogButton::Simple(GilButton::Unknown) is invalid and equal to mapping an
/// action to nothing
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum AnalogButton {
    Simple(GilButton),
    EventCode(u32),
}

impl Default for AnalogButton {
    fn default() -> Self { AnalogButton::Simple(GilButton::Unknown) }
}

/// Axis::Simple(GilAxis::Unknown) is invalid and equal to mapping an action to
/// nothing
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Axis {
    Simple(GilAxis),
    EventCode(u32),
}

impl Default for Axis {
    fn default() -> Self { Axis::Simple(GilAxis::Unknown) }
}

impl From<(GilAxis, GilCode)> for Axis {
    fn from((axis, code): (GilAxis, GilCode)) -> Self {
        match axis {
            GilAxis::Unknown => Self::EventCode(code.into_u32()),
            _ => Self::Simple(axis),
        }
    }
}

impl From<(GilButton, GilCode)> for Button {
    fn from((button, code): (GilButton, GilCode)) -> Self {
        match button {
            GilButton::Unknown => Self::EventCode(code.into_u32()),
            _ => Self::Simple(button),
        }
    }
}

impl From<(GilButton, GilCode)> for AnalogButton {
    fn from((button, code): (GilButton, GilCode)) -> Self {
        match button {
            GilButton::Unknown => Self::EventCode(code.into_u32()),
            _ => Self::Simple(button),
        }
    }
}
