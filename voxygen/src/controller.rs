//! Module containing controller-specific abstractions allowing complex
//! keybindings

use crate::{
    game_input::GameInput, settings::gamepad::con_settings::LayerEntry, window::MenuInput,
};
use gilrs::{ev::Code as GilCode, Axis as GilAxis, Button as GilButton};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

/// Contains all controller related settings and keymaps
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ControllerSettings {
    pub game_button_map: HashMap<Button, Vec<GameInput>>,
    pub menu_button_map: HashMap<Button, Vec<MenuInput>>,
    pub game_analog_button_map: HashMap<AnalogButton, Vec<AnalogButtonGameAction>>,
    pub menu_analog_button_map: HashMap<AnalogButton, Vec<AnalogButtonMenuAction>>,
    pub game_axis_map: HashMap<Axis, Vec<AxisGameAction>>,
    pub menu_axis_map: HashMap<Axis, Vec<AxisMenuAction>>,
    pub layer_button_map: HashMap<LayerEntry, Vec<GameInput>>,
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
}

impl From<&crate::settings::GamepadSettings> for ControllerSettings {
    fn from(settings: &crate::settings::GamepadSettings) -> Self {
        Self {
            game_button_map: {
                let mut map: HashMap<_, Vec<_>> = HashMap::new();
                map.entry(settings.game_buttons.primary)
                    .or_default()
                    .push(GameInput::Primary);
                map.entry(settings.game_buttons.secondary)
                    .or_default()
                    .push(GameInput::Secondary);
                map.entry(settings.game_buttons.block)
                    .or_default()
                    .push(GameInput::Block);
                map.entry(settings.game_buttons.slot1)
                    .or_default()
                    .push(GameInput::Slot1);
                map.entry(settings.game_buttons.slot2)
                    .or_default()
                    .push(GameInput::Slot2);
                map.entry(settings.game_buttons.slot3)
                    .or_default()
                    .push(GameInput::Slot3);
                map.entry(settings.game_buttons.slot4)
                    .or_default()
                    .push(GameInput::Slot4);
                map.entry(settings.game_buttons.slot5)
                    .or_default()
                    .push(GameInput::Slot5);
                map.entry(settings.game_buttons.slot6)
                    .or_default()
                    .push(GameInput::Slot6);
                map.entry(settings.game_buttons.slot7)
                    .or_default()
                    .push(GameInput::Slot7);
                map.entry(settings.game_buttons.slot8)
                    .or_default()
                    .push(GameInput::Slot8);
                map.entry(settings.game_buttons.slot9)
                    .or_default()
                    .push(GameInput::Slot9);
                map.entry(settings.game_buttons.slot10)
                    .or_default()
                    .push(GameInput::Slot10);
                map.entry(settings.game_buttons.slot11)
                    .or_default()
                    .push(GameInput::Slot11);
                map.entry(settings.game_buttons.slot12)
                    .or_default()
                    .push(GameInput::Slot12);
                map.entry(settings.game_buttons.toggle_cursor)
                    .or_default()
                    .push(GameInput::ToggleCursor);
                map.entry(settings.game_buttons.escape)
                    .or_default()
                    .push(GameInput::Escape);
                map.entry(settings.game_buttons.enter)
                    .or_default()
                    .push(GameInput::Chat);
                map.entry(settings.game_buttons.command)
                    .or_default()
                    .push(GameInput::Command);
                map.entry(settings.game_buttons.move_forward)
                    .or_default()
                    .push(GameInput::MoveForward);
                map.entry(settings.game_buttons.move_left)
                    .or_default()
                    .push(GameInput::MoveLeft);
                map.entry(settings.game_buttons.move_back)
                    .or_default()
                    .push(GameInput::MoveBack);
                map.entry(settings.game_buttons.move_right)
                    .or_default()
                    .push(GameInput::MoveRight);
                map.entry(settings.game_buttons.jump)
                    .or_default()
                    .push(GameInput::Jump);
                map.entry(settings.game_buttons.sit)
                    .or_default()
                    .push(GameInput::Sit);
                map.entry(settings.game_buttons.dance)
                    .or_default()
                    .push(GameInput::Dance);
                map.entry(settings.game_buttons.glide)
                    .or_default()
                    .push(GameInput::Glide);
                map.entry(settings.game_buttons.climb)
                    .or_default()
                    .push(GameInput::Climb);
                map.entry(settings.game_buttons.climb_down)
                    .or_default()
                    .push(GameInput::ClimbDown);
                map.entry(settings.game_buttons.swimup)
                    .or_default()
                    .push(GameInput::SwimUp);
                map.entry(settings.game_buttons.swimdown)
                    .or_default()
                    .push(GameInput::SwimDown);
                map.entry(settings.game_buttons.sneak)
                    .or_default()
                    .push(GameInput::Sneak);
                map.entry(settings.game_buttons.toggle_lantern)
                    .or_default()
                    .push(GameInput::ToggleLantern);
                map.entry(settings.game_buttons.mount)
                    .or_default()
                    .push(GameInput::Mount);
                map.entry(settings.game_buttons.map)
                    .or_default()
                    .push(GameInput::Map);
                map.entry(settings.game_buttons.bag)
                    .or_default()
                    .push(GameInput::Bag);
                map.entry(settings.game_buttons.social)
                    .or_default()
                    .push(GameInput::Social);
                map.entry(settings.game_buttons.crafting)
                    .or_default()
                    .push(GameInput::Crafting);
                map.entry(settings.game_buttons.diary)
                    .or_default()
                    .push(GameInput::Diary);
                map.entry(settings.game_buttons.settings)
                    .or_default()
                    .push(GameInput::Settings);
                map.entry(settings.game_buttons.controls)
                    .or_default()
                    .push(GameInput::Controls);
                map.entry(settings.game_buttons.toggle_interface)
                    .or_default()
                    .push(GameInput::ToggleInterface);
                map.entry(settings.game_buttons.toggle_debug)
                    .or_default()
                    .push(GameInput::ToggleDebug);
                #[cfg(feature = "egui-ui")]
                map.entry(settings.game_buttons.toggle_debug)
                    .or_default()
                    .push(GameInput::ToggleEguiDebug);
                map.entry(settings.game_buttons.toggle_chat)
                    .or_default()
                    .push(GameInput::ToggleChat);
                map.entry(settings.game_buttons.fullscreen)
                    .or_default()
                    .push(GameInput::Fullscreen);
                map.entry(settings.game_buttons.screenshot)
                    .or_default()
                    .push(GameInput::Screenshot);
                map.entry(settings.game_buttons.toggle_ingame_ui)
                    .or_default()
                    .push(GameInput::ToggleIngameUi);
                map.entry(settings.game_buttons.roll)
                    .or_default()
                    .push(GameInput::Roll);
                map.entry(settings.game_buttons.respawn)
                    .or_default()
                    .push(GameInput::Respawn);
                map.entry(settings.game_buttons.interact)
                    .or_default()
                    .push(GameInput::Interact);
                map.entry(settings.game_buttons.toggle_wield)
                    .or_default()
                    .push(GameInput::ToggleWield);
                map.entry(settings.game_buttons.swap_loadout)
                    .or_default()
                    .push(GameInput::SwapLoadout);
                map
            },
            menu_button_map: {
                let mut map: HashMap<_, Vec<_>> = HashMap::new();
                map.entry(settings.menu_buttons.up)
                    .or_default()
                    .push(MenuInput::Up);
                map.entry(settings.menu_buttons.down)
                    .or_default()
                    .push(MenuInput::Down);
                map.entry(settings.menu_buttons.left)
                    .or_default()
                    .push(MenuInput::Left);
                map.entry(settings.menu_buttons.right)
                    .or_default()
                    .push(MenuInput::Right);
                map.entry(settings.menu_buttons.scroll_up)
                    .or_default()
                    .push(MenuInput::ScrollUp);
                map.entry(settings.menu_buttons.scroll_down)
                    .or_default()
                    .push(MenuInput::ScrollDown);
                map.entry(settings.menu_buttons.scroll_left)
                    .or_default()
                    .push(MenuInput::ScrollLeft);
                map.entry(settings.menu_buttons.scroll_right)
                    .or_default()
                    .push(MenuInput::ScrollRight);
                map.entry(settings.menu_buttons.home)
                    .or_default()
                    .push(MenuInput::Home);
                map.entry(settings.menu_buttons.end)
                    .or_default()
                    .push(MenuInput::End);
                map.entry(settings.menu_buttons.apply)
                    .or_default()
                    .push(MenuInput::Apply);
                map.entry(settings.menu_buttons.back)
                    .or_default()
                    .push(MenuInput::Back);
                map.entry(settings.menu_buttons.exit)
                    .or_default()
                    .push(MenuInput::Exit);
                map
            },
            game_analog_button_map: HashMap::new(),
            menu_analog_button_map: HashMap::new(),
            game_axis_map: {
                let mut map: HashMap<_, Vec<_>> = HashMap::new();
                map.entry(settings.game_axis.movement_x)
                    .or_default()
                    .push(AxisGameAction::MovementX);
                map.entry(settings.game_axis.movement_y)
                    .or_default()
                    .push(AxisGameAction::MovementY);
                map.entry(settings.game_axis.camera_x)
                    .or_default()
                    .push(AxisGameAction::CameraX);
                map.entry(settings.game_axis.camera_y)
                    .or_default()
                    .push(AxisGameAction::CameraY);
                map
            },
            menu_axis_map: {
                let mut map: HashMap<_, Vec<_>> = HashMap::new();
                map.entry(settings.menu_axis.move_x)
                    .or_default()
                    .push(AxisMenuAction::MoveX);
                map.entry(settings.menu_axis.move_y)
                    .or_default()
                    .push(AxisMenuAction::MoveY);
                map.entry(settings.menu_axis.scroll_x)
                    .or_default()
                    .push(AxisMenuAction::ScrollX);
                map.entry(settings.menu_axis.scroll_y)
                    .or_default()
                    .push(AxisMenuAction::ScrollY);
                map
            },
            layer_button_map: {
                let mut map: HashMap<_, Vec<_>> = HashMap::new();
                map.entry(settings.game_layer_buttons.primary)
                    .or_default()
                    .push(GameInput::Primary);
                map.entry(settings.game_layer_buttons.secondary)
                    .or_default()
                    .push(GameInput::Secondary);
                map.entry(settings.game_layer_buttons.block)
                    .or_default()
                    .push(GameInput::Block);
                map.entry(settings.game_layer_buttons.slot1)
                    .or_default()
                    .push(GameInput::Slot1);
                map.entry(settings.game_layer_buttons.slot2)
                    .or_default()
                    .push(GameInput::Slot2);
                map.entry(settings.game_layer_buttons.slot3)
                    .or_default()
                    .push(GameInput::Slot3);
                map.entry(settings.game_layer_buttons.slot4)
                    .or_default()
                    .push(GameInput::Slot4);
                map.entry(settings.game_layer_buttons.slot5)
                    .or_default()
                    .push(GameInput::Slot5);
                map.entry(settings.game_layer_buttons.slot6)
                    .or_default()
                    .push(GameInput::Slot6);
                map.entry(settings.game_layer_buttons.slot7)
                    .or_default()
                    .push(GameInput::Slot7);
                map.entry(settings.game_layer_buttons.slot8)
                    .or_default()
                    .push(GameInput::Slot8);
                map.entry(settings.game_layer_buttons.slot9)
                    .or_default()
                    .push(GameInput::Slot9);
                map.entry(settings.game_layer_buttons.slot10)
                    .or_default()
                    .push(GameInput::Slot10);
                map.entry(settings.game_layer_buttons.slot11)
                    .or_default()
                    .push(GameInput::Slot11);
                map.entry(settings.game_layer_buttons.slot12)
                    .or_default()
                    .push(GameInput::Slot12);
                map.entry(settings.game_layer_buttons.toggle_cursor)
                    .or_default()
                    .push(GameInput::ToggleCursor);
                map.entry(settings.game_layer_buttons.escape)
                    .or_default()
                    .push(GameInput::Escape);
                map.entry(settings.game_layer_buttons.enter)
                    .or_default()
                    .push(GameInput::Chat);
                map.entry(settings.game_layer_buttons.command)
                    .or_default()
                    .push(GameInput::Command);
                map.entry(settings.game_layer_buttons.move_forward)
                    .or_default()
                    .push(GameInput::MoveForward);
                map.entry(settings.game_layer_buttons.move_left)
                    .or_default()
                    .push(GameInput::MoveLeft);
                map.entry(settings.game_layer_buttons.move_back)
                    .or_default()
                    .push(GameInput::MoveBack);
                map.entry(settings.game_layer_buttons.move_right)
                    .or_default()
                    .push(GameInput::MoveRight);
                map.entry(settings.game_layer_buttons.jump)
                    .or_default()
                    .push(GameInput::Jump);
                map.entry(settings.game_layer_buttons.sit)
                    .or_default()
                    .push(GameInput::Sit);
                map.entry(settings.game_layer_buttons.dance)
                    .or_default()
                    .push(GameInput::Dance);
                map.entry(settings.game_layer_buttons.glide)
                    .or_default()
                    .push(GameInput::Glide);
                map.entry(settings.game_layer_buttons.climb)
                    .or_default()
                    .push(GameInput::Climb);
                map.entry(settings.game_layer_buttons.climb_down)
                    .or_default()
                    .push(GameInput::ClimbDown);
                map.entry(settings.game_layer_buttons.swimup)
                    .or_default()
                    .push(GameInput::SwimUp);
                map.entry(settings.game_layer_buttons.swimdown)
                    .or_default()
                    .push(GameInput::SwimDown);
                map.entry(settings.game_layer_buttons.sneak)
                    .or_default()
                    .push(GameInput::Sneak);
                map.entry(settings.game_layer_buttons.toggle_lantern)
                    .or_default()
                    .push(GameInput::ToggleLantern);
                map.entry(settings.game_layer_buttons.mount)
                    .or_default()
                    .push(GameInput::Mount);
                map.entry(settings.game_layer_buttons.map)
                    .or_default()
                    .push(GameInput::Map);
                map.entry(settings.game_layer_buttons.bag)
                    .or_default()
                    .push(GameInput::Bag);
                map.entry(settings.game_layer_buttons.social)
                    .or_default()
                    .push(GameInput::Social);
                map.entry(settings.game_layer_buttons.crafting)
                    .or_default()
                    .push(GameInput::Crafting);
                map.entry(settings.game_layer_buttons.diary)
                    .or_default()
                    .push(GameInput::Diary);
                map.entry(settings.game_layer_buttons.settings)
                    .or_default()
                    .push(GameInput::Settings);
                map.entry(settings.game_layer_buttons.controls)
                    .or_default()
                    .push(GameInput::Controls);
                map.entry(settings.game_layer_buttons.toggle_interface)
                    .or_default()
                    .push(GameInput::ToggleInterface);
                map.entry(settings.game_layer_buttons.toggle_debug)
                    .or_default()
                    .push(GameInput::ToggleDebug);
                #[cfg(feature = "egui-ui")]
                map.entry(settings.game_layer_buttons.toggle_debug)
                    .or_default()
                    .push(GameInput::ToggleEguiDebug);
                map.entry(settings.game_layer_buttons.toggle_chat)
                    .or_default()
                    .push(GameInput::ToggleChat);
                map.entry(settings.game_layer_buttons.fullscreen)
                    .or_default()
                    .push(GameInput::Fullscreen);
                map.entry(settings.game_layer_buttons.screenshot)
                    .or_default()
                    .push(GameInput::Screenshot);
                map.entry(settings.game_layer_buttons.toggle_ingame_ui)
                    .or_default()
                    .push(GameInput::ToggleIngameUi);
                map.entry(settings.game_layer_buttons.roll)
                    .or_default()
                    .push(GameInput::Roll);
                map.entry(settings.game_layer_buttons.respawn)
                    .or_default()
                    .push(GameInput::Respawn);
                map.entry(settings.game_layer_buttons.interact)
                    .or_default()
                    .push(GameInput::Interact);
                map.entry(settings.game_layer_buttons.toggle_wield)
                    .or_default()
                    .push(GameInput::ToggleWield);
                map.entry(settings.game_layer_buttons.swap_loadout)
                    .or_default()
                    .push(GameInput::SwapLoadout);
                map
            },
            modifier_buttons: {
                let vec: Vec<Button> = vec![
                    Button::Simple(GilButton::RightTrigger),
                    Button::Simple(GilButton::LeftTrigger),
                ];
                vec
            },
            pan_sensitivity: settings.pan_sensitivity,
            pan_invert_y: settings.pan_invert_y,
            axis_deadzones: settings.axis_deadzones.clone(),
            button_deadzones: settings.button_deadzones.clone(),
            mouse_emulation_sensitivity: settings.mouse_emulation_sensitivity,
            inverted_axes: settings.inverted_axes.clone(),
        }
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
