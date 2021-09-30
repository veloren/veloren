use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GamepadSettings {
    pub game_buttons: con_settings::GameButtons,
    pub menu_buttons: con_settings::MenuButtons,
    pub game_axis: con_settings::GameAxis,
    pub menu_axis: con_settings::MenuAxis,
    pub game_analog_buttons: con_settings::GameAnalogButton,
    pub menu_analog_buttons: con_settings::MenuAnalogButton,
    pub pan_sensitivity: u32,
    pub pan_invert_y: bool,
    pub axis_deadzones: HashMap<crate::controller::Axis, f32>,
    pub button_deadzones: HashMap<crate::controller::AnalogButton, f32>,
    pub mouse_emulation_sensitivity: u32,
    pub inverted_axes: Vec<crate::controller::Axis>,
}

impl Default for GamepadSettings {
    fn default() -> Self {
        Self {
            game_buttons: con_settings::GameButtons::default(),
            menu_buttons: con_settings::MenuButtons::default(),
            game_axis: con_settings::GameAxis::default(),
            menu_axis: con_settings::MenuAxis::default(),
            game_analog_buttons: con_settings::GameAnalogButton::default(),
            menu_analog_buttons: con_settings::MenuAnalogButton::default(),
            pan_sensitivity: 10,
            pan_invert_y: false,
            axis_deadzones: HashMap::new(),
            button_deadzones: HashMap::new(),
            mouse_emulation_sensitivity: 12,
            inverted_axes: Vec::new(),
        }
    }
}

pub mod con_settings {
    use crate::controller::*;
    use gilrs::{Axis as GilAxis, Button as GilButton};
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(default)]
    pub struct GameButtons {
        pub primary: Button,
        pub secondary: Button,
        pub block: Button,
        pub slot1: Button,
        pub slot2: Button,
        pub slot3: Button,
        pub slot4: Button,
        pub slot5: Button,
        pub slot6: Button,
        pub slot7: Button,
        pub slot8: Button,
        pub slot9: Button,
        pub slot10: Button,
        pub toggle_cursor: Button,
        pub escape: Button,
        pub enter: Button,
        pub command: Button,
        pub move_forward: Button,
        pub move_left: Button,
        pub move_back: Button,
        pub move_right: Button,
        pub jump: Button,
        pub sit: Button,
        pub dance: Button,
        pub glide: Button,
        pub climb: Button,
        pub climb_down: Button,
        pub swimup: Button,
        pub swimdown: Button,
        pub sneak: Button,
        pub toggle_lantern: Button,
        pub mount: Button,
        pub map: Button,
        pub bag: Button,
        pub quest_log: Button,
        pub character_window: Button,
        pub social: Button,
        pub crafting: Button,
        pub spellbook: Button,
        pub settings: Button,
        pub help: Button,
        pub toggle_interface: Button,
        pub toggle_debug: Button,
        #[cfg(feature = "egui-ui")]
        pub toggle_egui_debug: Button,
        pub toggle_chat: Button,
        pub fullscreen: Button,
        pub screenshot: Button,
        pub toggle_ingame_ui: Button,
        pub roll: Button,
        pub respawn: Button,
        pub interact: Button,
        pub toggle_wield: Button,
        pub swap_loadout: Button,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(default)]
    pub struct MenuButtons {
        pub up: Button,
        pub down: Button,
        pub left: Button,
        pub right: Button,
        pub scroll_up: Button,
        pub scroll_down: Button,
        pub scroll_left: Button,
        pub scroll_right: Button,
        pub home: Button,
        pub end: Button,
        pub apply: Button,
        pub back: Button,
        pub exit: Button,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(default)]
    pub struct GameAxis {
        pub movement_x: Axis,
        pub movement_y: Axis,
        pub camera_x: Axis,
        pub camera_y: Axis,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(default)]
    pub struct MenuAxis {
        pub move_x: Axis,
        pub move_y: Axis,
        pub scroll_x: Axis,
        pub scroll_y: Axis,
    }

    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    #[serde(default)]
    pub struct GameAnalogButton {}

    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    #[serde(default)]
    pub struct MenuAnalogButton {}

    impl Default for GameButtons {
        fn default() -> Self {
            // binding to unknown = getting skipped from processing
            Self {
                primary: Button::Simple(GilButton::RightTrigger2),
                secondary: Button::Simple(GilButton::LeftTrigger2),
                block: Button::Simple(GilButton::LeftThumb),
                slot1: Button::Simple(GilButton::RightThumb),
                slot2: Button::Simple(GilButton::Unknown),
                slot3: Button::Simple(GilButton::Unknown),
                slot4: Button::Simple(GilButton::Unknown),
                slot5: Button::Simple(GilButton::Unknown),
                slot6: Button::Simple(GilButton::Unknown),
                slot7: Button::Simple(GilButton::Unknown),
                slot8: Button::Simple(GilButton::Unknown),
                slot9: Button::Simple(GilButton::Unknown),
                slot10: Button::Simple(GilButton::DPadDown),
                toggle_cursor: Button::Simple(GilButton::DPadRight),
                escape: Button::Simple(GilButton::Select),
                enter: Button::Simple(GilButton::Unknown),
                command: Button::Simple(GilButton::Unknown),
                move_forward: Button::Simple(GilButton::Unknown),
                move_left: Button::Simple(GilButton::Unknown),
                move_back: Button::Simple(GilButton::Unknown),
                move_right: Button::Simple(GilButton::Unknown),
                jump: Button::Simple(GilButton::South),
                sit: Button::Simple(GilButton::Unknown),
                dance: Button::Simple(GilButton::Unknown),
                glide: Button::Simple(GilButton::LeftTrigger),
                climb: Button::Simple(GilButton::South),
                climb_down: Button::Simple(GilButton::East),
                swimup: Button::Simple(GilButton::South),
                swimdown: Button::Simple(GilButton::East),
                sneak: Button::Simple(GilButton::East),
                toggle_lantern: Button::Simple(GilButton::DPadLeft),
                mount: Button::Simple(GilButton::North),
                map: Button::Simple(GilButton::Start),
                bag: Button::Simple(GilButton::Unknown),
                quest_log: Button::Simple(GilButton::Unknown),
                character_window: Button::Simple(GilButton::Unknown),
                social: Button::Simple(GilButton::Unknown),
                crafting: Button::Simple(GilButton::Unknown),
                spellbook: Button::Simple(GilButton::Unknown),
                settings: Button::Simple(GilButton::Unknown),
                help: Button::Simple(GilButton::Unknown),
                toggle_interface: Button::Simple(GilButton::Unknown),
                toggle_debug: Button::Simple(GilButton::Unknown),
                #[cfg(feature = "egui-ui")]
                toggle_egui_debug: Button::Simple(GilButton::Unknown),
                toggle_chat: Button::Simple(GilButton::Unknown),
                fullscreen: Button::Simple(GilButton::Unknown),
                screenshot: Button::Simple(GilButton::Unknown),
                toggle_ingame_ui: Button::Simple(GilButton::Unknown),
                roll: Button::Simple(GilButton::RightTrigger),
                respawn: Button::Simple(GilButton::South),
                interact: Button::Simple(GilButton::North),
                toggle_wield: Button::Simple(GilButton::West),
                swap_loadout: Button::Simple(GilButton::DPadUp),
            }
        }
    }

    impl Default for MenuButtons {
        fn default() -> Self {
            Self {
                up: Button::Simple(GilButton::Unknown),
                down: Button::Simple(GilButton::Unknown),
                left: Button::Simple(GilButton::Unknown),
                right: Button::Simple(GilButton::Unknown),
                scroll_up: Button::Simple(GilButton::Unknown),
                scroll_down: Button::Simple(GilButton::Unknown),
                scroll_left: Button::Simple(GilButton::Unknown),
                scroll_right: Button::Simple(GilButton::Unknown),
                home: Button::Simple(GilButton::DPadUp),
                end: Button::Simple(GilButton::DPadDown),
                apply: Button::Simple(GilButton::South),
                back: Button::Simple(GilButton::East),
                exit: Button::Simple(GilButton::Mode),
            }
        }
    }

    impl Default for GameAxis {
        fn default() -> Self {
            Self {
                movement_x: Axis::Simple(GilAxis::LeftStickX),
                movement_y: Axis::Simple(GilAxis::LeftStickY),
                camera_x: Axis::Simple(GilAxis::RightStickX),
                camera_y: Axis::Simple(GilAxis::RightStickY),
            }
        }
    }

    impl Default for MenuAxis {
        fn default() -> Self {
            Self {
                move_x: Axis::Simple(GilAxis::RightStickX),
                move_y: Axis::Simple(GilAxis::RightStickY),
                scroll_x: Axis::Simple(GilAxis::LeftStickX),
                scroll_y: Axis::Simple(GilAxis::LeftStickY),
            }
        }
    }
}
