//! Module containing controller-specific abstractions allowing complex
//! keybindings

use crate::{
    game_input::GameInput, settings::gamepad::con_settings::LayerEntry, window::MenuInput,
};
use gilrs::{Axis as GilAxis, Button as GilButton, ev::Code as GilCode};
use hashbrown::{HashMap, HashSet};
use i18n::Localization;
use serde::{Deserialize, Serialize};

/// Contains all controller related settings and keymaps
#[derive(Clone, Debug, Default)]
pub struct ControllerSettings {
    pub game_button_map: HashMap<GameInput, Button>,
    pub inverse_game_button_map: HashMap<Button, HashSet<GameInput>>,
    pub menu_button_map: HashMap<MenuInput, Button>,
    pub inverse_menu_button_map: HashMap<Button, HashSet<MenuInput>>,
    pub game_analog_button_map: HashMap<AnalogButtonGameAction, AnalogButton>,
    pub inverse_game_analog_button_map: HashMap<AnalogButton, HashSet<AnalogButtonGameAction>>,
    pub menu_analog_button_map: HashMap<AnalogButtonMenuAction, AnalogButton>,
    pub inverse_menu_analog_button_map: HashMap<AnalogButton, HashSet<AnalogButtonMenuAction>>,
    pub game_axis_map: HashMap<AxisGameAction, Axis>,
    pub inverse_game_axis_map: HashMap<Axis, HashSet<AxisGameAction>>,
    pub menu_axis_map: HashMap<AxisMenuAction, Axis>,
    pub inverse_menu_axis_map: HashMap<Axis, HashSet<AxisMenuAction>>,
    pub layer_button_map: HashMap<GameInput, LayerEntry>,
    pub inverse_layer_button_map: HashMap<LayerEntry, HashSet<GameInput>>,
    pub modifier_buttons: Vec<Button>,
    pub pan_sensitivity: u32,
    pub pan_invert_y: bool,
    pub axis_deadzones: HashMap<Axis, f32>,
    pub button_deadzones: HashMap<AnalogButton, f32>,
    pub mouse_emulation_sensitivity: u32,
    pub inverted_axes: Vec<Axis>,
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

    pub fn get_game_button_binding(&self, input: GameInput) -> Option<Button> {
        self.game_button_map.get(&input).copied()
    }

    pub fn get_menu_button_binding(&self, input: MenuInput) -> Option<Button> {
        self.menu_button_map.get(&input).copied()
    }

    pub fn get_layer_button_binding(&self, input: GameInput) -> Option<LayerEntry> {
        self.layer_button_map.get(&input).copied()
    }

    pub fn insert_game_button_binding(&mut self, game_input: GameInput, game_button: Button) {
        if game_button != Button::default() {
            self.game_button_map.insert(game_input, game_button);
            self.inverse_game_button_map
                .entry(game_button)
                .or_default()
                .insert(game_input);
        }
    }

    pub fn insert_menu_button_binding(&mut self, menu_input: MenuInput, button: Button) {
        if button != Button::default() {
            self.menu_button_map.insert(menu_input, button);
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
            self.layer_button_map.insert(input, layer_entry);
            self.inverse_layer_button_map
                .entry(layer_entry)
                .or_default()
                .insert(input);
        }
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
            {
                if let Some(game_inputs) = self.inverse_game_button_map.get(&layer_entry.button) {
                    for a in layer_inputs.iter() {
                        for b in game_inputs.iter() {
                            if !GameInput::can_share_bindings(*a, *b) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }
}

impl From<&crate::settings::GamepadSettings> for ControllerSettings {
    fn from(settings: &crate::settings::GamepadSettings) -> Self {
        let mut controller_settings: ControllerSettings = ControllerSettings::default();

        controller_settings
            .insert_game_button_binding(GameInput::Primary, settings.game_buttons.primary);
        controller_settings
            .insert_game_button_binding(GameInput::Secondary, settings.game_buttons.secondary);
        controller_settings
            .insert_game_button_binding(GameInput::Block, settings.game_buttons.block);
        controller_settings
            .insert_game_button_binding(GameInput::Slot1, settings.game_buttons.slot1);
        controller_settings
            .insert_game_button_binding(GameInput::Slot2, settings.game_buttons.slot2);
        controller_settings
            .insert_game_button_binding(GameInput::Slot3, settings.game_buttons.slot3);
        controller_settings
            .insert_game_button_binding(GameInput::Slot4, settings.game_buttons.slot4);
        controller_settings
            .insert_game_button_binding(GameInput::Slot5, settings.game_buttons.slot5);
        controller_settings
            .insert_game_button_binding(GameInput::Slot6, settings.game_buttons.slot6);
        controller_settings
            .insert_game_button_binding(GameInput::Slot7, settings.game_buttons.slot7);
        controller_settings
            .insert_game_button_binding(GameInput::Slot8, settings.game_buttons.slot8);
        controller_settings
            .insert_game_button_binding(GameInput::Slot9, settings.game_buttons.slot9);
        controller_settings
            .insert_game_button_binding(GameInput::Slot10, settings.game_buttons.slot10);
        controller_settings.insert_game_button_binding(
            GameInput::ToggleCursor,
            settings.game_buttons.toggle_cursor,
        );
        controller_settings
            .insert_game_button_binding(GameInput::Escape, settings.game_buttons.escape);
        controller_settings
            .insert_game_button_binding(GameInput::Chat, settings.game_buttons.enter);
        controller_settings
            .insert_game_button_binding(GameInput::Command, settings.game_buttons.command);
        controller_settings
            .insert_game_button_binding(GameInput::MoveForward, settings.game_buttons.move_forward);
        controller_settings
            .insert_game_button_binding(GameInput::MoveLeft, settings.game_buttons.move_left);
        controller_settings
            .insert_game_button_binding(GameInput::MoveBack, settings.game_buttons.move_back);
        controller_settings
            .insert_game_button_binding(GameInput::MoveRight, settings.game_buttons.move_right);
        controller_settings.insert_game_button_binding(GameInput::Jump, settings.game_buttons.jump);
        controller_settings.insert_game_button_binding(GameInput::Sit, settings.game_buttons.sit);
        controller_settings
            .insert_game_button_binding(GameInput::Dance, settings.game_buttons.dance);
        controller_settings
            .insert_game_button_binding(GameInput::Glide, settings.game_buttons.glide);
        controller_settings
            .insert_game_button_binding(GameInput::SwimUp, settings.game_buttons.swimup);
        controller_settings
            .insert_game_button_binding(GameInput::SwimDown, settings.game_buttons.swimdown);
        controller_settings
            .insert_game_button_binding(GameInput::Sneak, settings.game_buttons.sneak);
        controller_settings.insert_game_button_binding(
            GameInput::ToggleLantern,
            settings.game_buttons.toggle_lantern,
        );
        controller_settings
            .insert_game_button_binding(GameInput::Mount, settings.game_buttons.mount);
        controller_settings.insert_game_button_binding(GameInput::Map, settings.game_buttons.map);
        controller_settings
            .insert_game_button_binding(GameInput::Inventory, settings.game_buttons.bag);
        controller_settings
            .insert_game_button_binding(GameInput::Social, settings.game_buttons.social);
        controller_settings
            .insert_game_button_binding(GameInput::Crafting, settings.game_buttons.crafting);
        controller_settings
            .insert_game_button_binding(GameInput::Diary, settings.game_buttons.diary);
        controller_settings
            .insert_game_button_binding(GameInput::Settings, settings.game_buttons.settings);
        controller_settings
            .insert_game_button_binding(GameInput::Controls, settings.game_buttons.controls);
        controller_settings.insert_game_button_binding(
            GameInput::ToggleInterface,
            settings.game_buttons.toggle_interface,
        );
        controller_settings
            .insert_game_button_binding(GameInput::ToggleDebug, settings.game_buttons.toggle_debug);
        #[cfg(feature = "egui-ui")]
        controller_settings.insert_game_button_binding(
            GameInput::ToggleEguiDebug,
            settings.game_buttons.toggle_debug,
        );
        controller_settings
            .insert_game_button_binding(GameInput::ToggleChat, settings.game_buttons.toggle_chat);
        controller_settings
            .insert_game_button_binding(GameInput::Fullscreen, settings.game_buttons.fullscreen);
        controller_settings
            .insert_game_button_binding(GameInput::Screenshot, settings.game_buttons.screenshot);
        controller_settings.insert_game_button_binding(
            GameInput::ToggleIngameUi,
            settings.game_buttons.toggle_ingame_ui,
        );
        controller_settings.insert_game_button_binding(GameInput::Roll, settings.game_buttons.roll);
        controller_settings
            .insert_game_button_binding(GameInput::Respawn, settings.game_buttons.respawn);
        controller_settings
            .insert_game_button_binding(GameInput::Interact, settings.game_buttons.interact);
        controller_settings
            .insert_game_button_binding(GameInput::ToggleWield, settings.game_buttons.toggle_wield);
        controller_settings
            .insert_game_button_binding(GameInput::SwapLoadout, settings.game_buttons.swap_loadout);

        controller_settings.insert_menu_button_binding(MenuInput::Up, settings.menu_buttons.up);
        controller_settings.insert_menu_button_binding(MenuInput::Down, settings.menu_buttons.down);
        controller_settings.insert_menu_button_binding(MenuInput::Left, settings.menu_buttons.left);
        controller_settings
            .insert_menu_button_binding(MenuInput::Right, settings.menu_buttons.right);
        controller_settings
            .insert_menu_button_binding(MenuInput::ScrollUp, settings.menu_buttons.scroll_up);
        controller_settings
            .insert_menu_button_binding(MenuInput::ScrollDown, settings.menu_buttons.scroll_down);
        controller_settings
            .insert_menu_button_binding(MenuInput::ScrollLeft, settings.menu_buttons.scroll_left);
        controller_settings
            .insert_menu_button_binding(MenuInput::ScrollRight, settings.menu_buttons.scroll_right);
        controller_settings.insert_menu_button_binding(MenuInput::Home, settings.menu_buttons.home);
        controller_settings.insert_menu_button_binding(MenuInput::End, settings.menu_buttons.end);
        controller_settings
            .insert_menu_button_binding(MenuInput::Apply, settings.menu_buttons.apply);
        controller_settings.insert_menu_button_binding(MenuInput::Back, settings.menu_buttons.back);
        controller_settings.insert_menu_button_binding(MenuInput::Exit, settings.menu_buttons.exit);

        controller_settings
            .insert_game_axis_binding(AxisGameAction::MovementX, settings.game_axis.movement_x);
        controller_settings
            .insert_game_axis_binding(AxisGameAction::MovementY, settings.game_axis.movement_y);
        controller_settings
            .insert_game_axis_binding(AxisGameAction::CameraX, settings.game_axis.camera_x);
        controller_settings
            .insert_game_axis_binding(AxisGameAction::CameraY, settings.game_axis.camera_y);

        controller_settings
            .insert_menu_axis_binding(AxisMenuAction::MoveX, settings.menu_axis.move_x);
        controller_settings
            .insert_menu_axis_binding(AxisMenuAction::MoveY, settings.menu_axis.move_y);
        controller_settings
            .insert_menu_axis_binding(AxisMenuAction::ScrollX, settings.menu_axis.scroll_x);
        controller_settings
            .insert_menu_axis_binding(AxisMenuAction::ScrollY, settings.menu_axis.scroll_y);

        controller_settings.insert_layer_button_binding(
            GameInput::Secondary,
            settings.game_layer_buttons.secondary,
        );
        controller_settings
            .insert_layer_button_binding(GameInput::Primary, settings.game_layer_buttons.primary);
        controller_settings
            .insert_layer_button_binding(GameInput::Block, settings.game_layer_buttons.block);
        controller_settings
            .insert_layer_button_binding(GameInput::Slot1, settings.game_layer_buttons.slot1);
        controller_settings
            .insert_layer_button_binding(GameInput::Slot2, settings.game_layer_buttons.slot2);
        controller_settings
            .insert_layer_button_binding(GameInput::Slot3, settings.game_layer_buttons.slot3);
        controller_settings
            .insert_layer_button_binding(GameInput::Slot4, settings.game_layer_buttons.slot4);
        controller_settings
            .insert_layer_button_binding(GameInput::Slot5, settings.game_layer_buttons.slot5);
        controller_settings
            .insert_layer_button_binding(GameInput::Slot6, settings.game_layer_buttons.slot6);
        controller_settings
            .insert_layer_button_binding(GameInput::Slot7, settings.game_layer_buttons.slot7);
        controller_settings
            .insert_layer_button_binding(GameInput::Slot8, settings.game_layer_buttons.slot8);
        controller_settings
            .insert_layer_button_binding(GameInput::Slot9, settings.game_layer_buttons.slot9);
        controller_settings
            .insert_layer_button_binding(GameInput::Slot10, settings.game_layer_buttons.slot10);
        controller_settings.insert_layer_button_binding(
            GameInput::ToggleCursor,
            settings.game_layer_buttons.toggle_cursor,
        );
        controller_settings
            .insert_layer_button_binding(GameInput::Escape, settings.game_layer_buttons.escape);
        controller_settings
            .insert_layer_button_binding(GameInput::Chat, settings.game_layer_buttons.enter);
        controller_settings
            .insert_layer_button_binding(GameInput::Command, settings.game_layer_buttons.command);
        controller_settings.insert_layer_button_binding(
            GameInput::MoveForward,
            settings.game_layer_buttons.move_forward,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::MoveLeft,
            settings.game_layer_buttons.move_left,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::MoveBack,
            settings.game_layer_buttons.move_back,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::MoveRight,
            settings.game_layer_buttons.move_right,
        );
        controller_settings
            .insert_layer_button_binding(GameInput::Jump, settings.game_layer_buttons.jump);
        controller_settings
            .insert_layer_button_binding(GameInput::Sit, settings.game_layer_buttons.sit);
        controller_settings
            .insert_layer_button_binding(GameInput::Dance, settings.game_layer_buttons.dance);
        controller_settings
            .insert_layer_button_binding(GameInput::Glide, settings.game_layer_buttons.glide);
        controller_settings
            .insert_layer_button_binding(GameInput::SwimUp, settings.game_layer_buttons.swimup);
        controller_settings
            .insert_layer_button_binding(GameInput::SwimDown, settings.game_layer_buttons.swimdown);
        controller_settings
            .insert_layer_button_binding(GameInput::Sneak, settings.game_layer_buttons.sneak);
        controller_settings.insert_layer_button_binding(
            GameInput::ToggleLantern,
            settings.game_layer_buttons.toggle_lantern,
        );
        controller_settings
            .insert_layer_button_binding(GameInput::Mount, settings.game_layer_buttons.mount);
        controller_settings
            .insert_layer_button_binding(GameInput::Map, settings.game_layer_buttons.map);
        controller_settings
            .insert_layer_button_binding(GameInput::Inventory, settings.game_layer_buttons.bag);
        controller_settings
            .insert_layer_button_binding(GameInput::Social, settings.game_layer_buttons.social);
        controller_settings
            .insert_layer_button_binding(GameInput::Crafting, settings.game_layer_buttons.crafting);
        controller_settings
            .insert_layer_button_binding(GameInput::Diary, settings.game_layer_buttons.diary);
        controller_settings
            .insert_layer_button_binding(GameInput::Settings, settings.game_layer_buttons.settings);
        controller_settings
            .insert_layer_button_binding(GameInput::Controls, settings.game_layer_buttons.controls);
        controller_settings.insert_layer_button_binding(
            GameInput::ToggleInterface,
            settings.game_layer_buttons.toggle_interface,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::ToggleDebug,
            settings.game_layer_buttons.toggle_debug,
        );
        #[cfg(feature = "egui-ui")]
        controller_settings.insert_layer_button_binding(
            GameInput::ToggleEguiDebug,
            settings.game_layer_buttons.toggle_debug,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::ToggleChat,
            settings.game_layer_buttons.toggle_chat,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::Fullscreen,
            settings.game_layer_buttons.fullscreen,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::Screenshot,
            settings.game_layer_buttons.screenshot,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::ToggleIngameUi,
            settings.game_layer_buttons.toggle_ingame_ui,
        );
        controller_settings
            .insert_layer_button_binding(GameInput::Roll, settings.game_layer_buttons.roll);
        controller_settings
            .insert_layer_button_binding(GameInput::Respawn, settings.game_layer_buttons.respawn);
        controller_settings
            .insert_layer_button_binding(GameInput::Interact, settings.game_layer_buttons.interact);
        controller_settings.insert_layer_button_binding(
            GameInput::ToggleWield,
            settings.game_layer_buttons.toggle_wield,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::SwapLoadout,
            settings.game_layer_buttons.swap_loadout,
        );

        controller_settings.modifier_buttons = vec![
            Button::Simple(GilButton::RightTrigger),
            Button::Simple(GilButton::LeftTrigger),
        ];
        controller_settings.pan_sensitivity = settings.pan_sensitivity;
        controller_settings.pan_invert_y = settings.pan_invert_y;
        controller_settings
            .axis_deadzones
            .clone_from(&settings.axis_deadzones);
        controller_settings
            .button_deadzones
            .clone_from(&settings.button_deadzones);
        controller_settings.mouse_emulation_sensitivity = settings.mouse_emulation_sensitivity;
        controller_settings
            .inverted_axes
            .clone_from(&settings.inverted_axes);

        controller_settings
    }
}

/// All the menu actions you can bind to an Axis
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum AxisMenuAction {
    MoveX,
    MoveY,
    ScrollX,
    ScrollY,
}

/// All the game actions you can bind to an Axis
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
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
