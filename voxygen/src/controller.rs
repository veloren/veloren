//! Module containing controller-specific abstractions allowing complex
//! keybindings

use crate::{
    game_input::GameInput, settings::gamepad::con_settings::LayerEntry, window::MenuInput,
};
use gilrs::{Axis as GilAxis, Button as GilButton, ev::Code as GilCode};
use hashbrown::{HashMap, HashSet};
use i18n::Localization;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

#[derive(Serialize, Deserialize)]
struct ControllerSettingsSerde {
    // save as a delta against defaults for efficiency
    game_button_map: HashMap<GameInput, Option<Button>>,
    menu_button_map: HashMap<MenuInput, Button>,
    game_analog_button_map: HashMap<AnalogButtonGameAction, AnalogButton>,
    menu_analog_button_map: HashMap<AnalogButtonMenuAction, AnalogButton>,
    game_axis_map: HashMap<AxisGameAction, Axis>,
    menu_axis_map: HashMap<AxisMenuAction, Axis>,
    layer_button_map: HashMap<GameInput, LayerEntry>,
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
        let mut button_bindings: HashMap<GameInput, Option<Button>> = HashMap::new();
        // Do a delta between default() ControllerSettings and the argument,
        // let buttons be only the custom keybindings chosen by the user
        for (k, v) in controller_settings.game_button_map {
            if ControllerSettings::default_button_binding(k) != v {
                button_bindings.insert(k, v);
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
            layer_button_map: HashMap::new(),

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
#[derive(Clone, Debug)]
pub struct ControllerSettings {
    pub game_button_map: HashMap<GameInput, Option<Button>>,
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

impl From<ControllerSettingsSerde> for ControllerSettings {
    fn from(controller_serde: ControllerSettingsSerde) -> Self {
        let button_bindings = controller_serde.game_button_map;
        let mut controller_settings = ControllerSettings::default();
        for (k, maybe_v) in button_bindings {
            match maybe_v {
                Some(v) => controller_settings.modify_button_binding(k, v),
                None => controller_settings.remove_button_binding(k),
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

    pub fn get_game_button_binding(&self, input: GameInput) -> Option<Button> {
        self.game_button_map.get(&input).cloned().flatten()
    }

    pub fn get_menu_button_binding(&self, input: MenuInput) -> Option<Button> {
        self.menu_button_map.get(&input).copied()
    }

    pub fn get_layer_button_binding(&self, input: GameInput) -> Option<LayerEntry> {
        self.layer_button_map.get(&input).copied()
    }

    pub fn insert_game_button_binding(&mut self, game_input: GameInput, game_button: Button) {
        if game_button != Button::default() {
            self.game_button_map
                .insert(game_input, Some(game_button.clone()));
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
            .entry(button.clone())
            .or_default()
            .insert(game_input);
        // for the GameInput->button hashmap, just overwrite the value
        self.game_button_map.insert(game_input, Some(button));
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

    pub fn default_button_binding(game_input: GameInput) -> Option<Button> {
        match game_input {
            GameInput::Primary => Some(Button::Simple(GilButton::RightTrigger2)),
            GameInput::Secondary => Some(Button::Simple(GilButton::LeftTrigger2)),
            GameInput::Block => Some(Button::Simple(GilButton::North)),
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
            GameInput::Glide => Some(Button::Simple(GilButton::Unknown)),
            GameInput::SwimUp => Some(Button::Simple(GilButton::South)),
            GameInput::SwimDown => Some(Button::Simple(GilButton::West)),
            GameInput::Fly => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Sneak => Some(Button::Simple(GilButton::Unknown)),
            GameInput::CancelClimb => Some(Button::Simple(GilButton::Unknown)),
            GameInput::ToggleLantern => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Mount => Some(Button::Simple(GilButton::Unknown)),
            GameInput::StayFollow => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Chat => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Command => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Escape => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Map => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Inventory => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Trade => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Social => Some(Button::Simple(GilButton::Unknown)),
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
            GameInput::Roll => Some(Button::Simple(GilButton::Unknown)),
            GameInput::GiveUp => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Respawn => Some(Button::Simple(GilButton::Unknown)),
            GameInput::Interact => Some(Button::Simple(GilButton::Unknown)),
            GameInput::ToggleWield => Some(Button::Simple(GilButton::Unknown)),
            GameInput::SwapLoadout => Some(Button::Simple(GilButton::Unknown)),
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

            modifier_buttons: Vec::new(),
            pan_sensitivity: 10,
            pan_invert_y: false,
            axis_deadzones: HashMap::new(),
            button_deadzones: HashMap::new(),
            mouse_emulation_sensitivity: 12,
            inverted_axes: Vec::new(),
        };
        // sets the button bindings for those game button inputs
        for button_input in GameInput::iter() {
            match ControllerSettings::default_button_binding(button_input) {
                None => {},
                Some(default) => {
                    controller_settings.insert_game_button_binding(button_input, default)
                },
            };
        }
        controller_settings
    }
}

impl From<&crate::settings::GamepadSettings> for ControllerSettings {
    fn from(settings: &crate::settings::GamepadSettings) -> Self {
        let mut controller_settings: ControllerSettings = ControllerSettings::default();

        // controller_settings
        //     .insert_game_button_binding(GameInput::Primary,
        // settings.game_buttons.primary); controller_settings
        //     .insert_game_button_binding(GameInput::Secondary,
        // settings.game_buttons.secondary); controller_settings
        //     .insert_game_button_binding(GameInput::Block,
        // settings.game_buttons.block); controller_settings
        //     .insert_game_button_binding(GameInput::Slot1,
        // settings.game_buttons.slot1); controller_settings
        //     .insert_game_button_binding(GameInput::Slot2,
        // settings.game_buttons.slot2); controller_settings
        //     .insert_game_button_binding(GameInput::Slot3,
        // settings.game_buttons.slot3); controller_settings
        //     .insert_game_button_binding(GameInput::Slot4,
        // settings.game_buttons.slot4); controller_settings
        //     .insert_game_button_binding(GameInput::Slot5,
        // settings.game_buttons.slot5); controller_settings
        //     .insert_game_button_binding(GameInput::Slot6,
        // settings.game_buttons.slot6); controller_settings
        //     .insert_game_button_binding(GameInput::Slot7,
        // settings.game_buttons.slot7); controller_settings
        //     .insert_game_button_binding(GameInput::Slot8,
        // settings.game_buttons.slot8); controller_settings
        //     .insert_game_button_binding(GameInput::Slot9,
        // settings.game_buttons.slot9); controller_settings
        //     .insert_game_button_binding(GameInput::Slot10,
        // settings.game_buttons.slot10); controller_settings.
        // insert_game_button_binding(     GameInput::ToggleCursor,
        //     settings.game_buttons.toggle_cursor,
        // );
        // controller_settings
        //     .insert_game_button_binding(GameInput::Escape,
        // settings.game_buttons.escape); controller_settings
        //     .insert_game_button_binding(GameInput::Chat,
        // settings.game_buttons.enter); controller_settings
        //     .insert_game_button_binding(GameInput::Command,
        // settings.game_buttons.command); controller_settings
        //     .insert_game_button_binding(GameInput::MoveForward,
        // settings.game_buttons.move_forward); controller_settings
        //     .insert_game_button_binding(GameInput::MoveLeft,
        // settings.game_buttons.move_left); controller_settings
        //     .insert_game_button_binding(GameInput::MoveBack,
        // settings.game_buttons.move_back); controller_settings
        //     .insert_game_button_binding(GameInput::MoveRight,
        // settings.game_buttons.move_right); controller_settings.
        // insert_game_button_binding(GameInput::Jump, settings.game_buttons.jump);
        // controller_settings.insert_game_button_binding(GameInput::Sit,
        // settings.game_buttons.sit); controller_settings
        //     .insert_game_button_binding(GameInput::Dance,
        // settings.game_buttons.dance); controller_settings
        //     .insert_game_button_binding(GameInput::Glide,
        // settings.game_buttons.glide); controller_settings
        //     .insert_game_button_binding(GameInput::SwimUp,
        // settings.game_buttons.swimup); controller_settings
        //     .insert_game_button_binding(GameInput::SwimDown,
        // settings.game_buttons.swimdown); controller_settings
        //     .insert_game_button_binding(GameInput::Sneak,
        // settings.game_buttons.sneak); controller_settings.
        // insert_game_button_binding(     GameInput::ToggleLantern,
        //     settings.game_buttons.toggle_lantern,
        // );
        // controller_settings
        //     .insert_game_button_binding(GameInput::Mount,
        // settings.game_buttons.mount); controller_settings.
        // insert_game_button_binding(GameInput::Map, settings.game_buttons.map);
        // controller_settings
        //     .insert_game_button_binding(GameInput::Inventory,
        // settings.game_buttons.bag); controller_settings
        //     .insert_game_button_binding(GameInput::Social,
        // settings.game_buttons.social); controller_settings
        //     .insert_game_button_binding(GameInput::Crafting,
        // settings.game_buttons.crafting); controller_settings
        //     .insert_game_button_binding(GameInput::Diary,
        // settings.game_buttons.diary); controller_settings
        //     .insert_game_button_binding(GameInput::Settings,
        // settings.game_buttons.settings); controller_settings
        //     .insert_game_button_binding(GameInput::Controls,
        // settings.game_buttons.controls); controller_settings.
        // insert_game_button_binding(     GameInput::ToggleInterface,
        //     settings.game_buttons.toggle_interface,
        // );
        // controller_settings
        //     .insert_game_button_binding(GameInput::ToggleDebug,
        // settings.game_buttons.toggle_debug); #[cfg(feature = "egui-ui")]
        // controller_settings.insert_game_button_binding(
        //     GameInput::ToggleEguiDebug,
        //     settings.game_buttons.toggle_debug,
        // );
        // controller_settings
        //     .insert_game_button_binding(GameInput::ToggleChat,
        // settings.game_buttons.toggle_chat); controller_settings
        //     .insert_game_button_binding(GameInput::Fullscreen,
        // settings.game_buttons.fullscreen); controller_settings
        //     .insert_game_button_binding(GameInput::Screenshot,
        // settings.game_buttons.screenshot); controller_settings.
        // insert_game_button_binding(     GameInput::ToggleIngameUi,
        //     settings.game_buttons.toggle_ingame_ui,
        // );
        // controller_settings.insert_game_button_binding(GameInput::Roll,
        // settings.game_buttons.roll); controller_settings
        //     .insert_game_button_binding(GameInput::Respawn,
        // settings.game_buttons.respawn); controller_settings
        //     .insert_game_button_binding(GameInput::Interact,
        // settings.game_buttons.interact); controller_settings
        //     .insert_game_button_binding(GameInput::ToggleWield,
        // settings.game_buttons.toggle_wield); controller_settings
        //     .insert_game_button_binding(GameInput::SwapLoadout,
        // settings.game_buttons.swap_loadout);

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
        controller_settings.insert_layer_button_binding(
            GameInput::WallJump,
            settings.game_layer_buttons.wall_jump,
        );
        controller_settings
            .insert_layer_button_binding(GameInput::Sit, settings.game_layer_buttons.sit);
        controller_settings
            .insert_layer_button_binding(GameInput::Crawl, settings.game_layer_buttons.crawl);
        controller_settings
            .insert_layer_button_binding(GameInput::Dance, settings.game_layer_buttons.dance);
        controller_settings
            .insert_layer_button_binding(GameInput::Greet, settings.game_layer_buttons.greet);
        controller_settings
            .insert_layer_button_binding(GameInput::Glide, settings.game_layer_buttons.glide);
        controller_settings
            .insert_layer_button_binding(GameInput::SwimUp, settings.game_layer_buttons.swimup);
        controller_settings
            .insert_layer_button_binding(GameInput::SwimDown, settings.game_layer_buttons.swimdown);
        controller_settings
            .insert_layer_button_binding(GameInput::Fly, settings.game_layer_buttons.fly);
        controller_settings
            .insert_layer_button_binding(GameInput::Sneak, settings.game_layer_buttons.sneak);
        controller_settings.insert_layer_button_binding(
            GameInput::CancelClimb,
            settings.game_layer_buttons.cancel_climb,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::ToggleLantern,
            settings.game_layer_buttons.toggle_lantern,
        );
        controller_settings
            .insert_layer_button_binding(GameInput::Mount, settings.game_layer_buttons.mount);
        controller_settings.insert_layer_button_binding(
            GameInput::StayFollow,
            settings.game_layer_buttons.stayfollow,
        );
        controller_settings
            .insert_layer_button_binding(GameInput::Chat, settings.game_layer_buttons.chat);
        controller_settings
            .insert_layer_button_binding(GameInput::Map, settings.game_layer_buttons.map);
        controller_settings.insert_layer_button_binding(
            GameInput::Inventory,
            settings.game_layer_buttons.inventory,
        );
        controller_settings
            .insert_layer_button_binding(GameInput::Trade, settings.game_layer_buttons.trade);
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
            .insert_layer_button_binding(GameInput::GiveUp, settings.game_layer_buttons.give_up);
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
        controller_settings.insert_layer_button_binding(
            GameInput::FreeLook,
            settings.game_layer_buttons.free_look,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::AutoWalk,
            settings.game_layer_buttons.auto_walk,
        );
        controller_settings
            .insert_layer_button_binding(GameInput::ZoomIn, settings.game_layer_buttons.zoom_in);
        controller_settings
            .insert_layer_button_binding(GameInput::ZoomOut, settings.game_layer_buttons.zoom_out);
        controller_settings.insert_layer_button_binding(
            GameInput::ZoomLock,
            settings.game_layer_buttons.zoom_lock,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::CameraClamp,
            settings.game_layer_buttons.camera_clamp,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::CycleCamera,
            settings.game_layer_buttons.cycle_camera,
        );
        controller_settings
            .insert_layer_button_binding(GameInput::Select, settings.game_layer_buttons.select);
        controller_settings.insert_layer_button_binding(
            GameInput::AcceptGroupInvite,
            settings.game_layer_buttons.accept_group_invite,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::DeclineGroupInvite,
            settings.game_layer_buttons.decline_group_invite,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::MapZoomIn,
            settings.game_layer_buttons.map_zoom_in,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::MapZoomOut,
            settings.game_layer_buttons.map_zoom_out,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::MapSetMarker,
            settings.game_layer_buttons.map_set_marker,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::SpectateSpeedBoost,
            settings.game_layer_buttons.spectate_speed_boost,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::SpectateViewpoint,
            settings.game_layer_buttons.spectate_viewpoint,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::MuteMaster,
            settings.game_layer_buttons.mute_master,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::MuteInactiveMaster,
            settings.game_layer_buttons.mute_inactive_master,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::MuteMusic,
            settings.game_layer_buttons.mute_music,
        );
        controller_settings
            .insert_layer_button_binding(GameInput::MuteSfx, settings.game_layer_buttons.mute_sfx);
        controller_settings.insert_layer_button_binding(
            GameInput::MuteAmbience,
            settings.game_layer_buttons.mute_ambience,
        );
        controller_settings.insert_layer_button_binding(
            GameInput::ToggleWalk,
            settings.game_layer_buttons.toggle_walk,
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
