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
    pub game_layer_buttons: con_settings::GameLayerEntries,
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
            game_layer_buttons: con_settings::GameLayerEntries::default(),
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

    // struct to associate each available GameInput with a LayerEntry
    // similar in function to the GameButtons struct
    // nothing prevents mapping a GameInput in both GameLayerEntries and GameButtons
    // it's likely not desirable to double map a GameInput
    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(default)]
    pub struct GameLayerEntries {
        pub primary: LayerEntry,
        pub secondary: LayerEntry,
        pub block: LayerEntry,
        pub slot1: LayerEntry,
        pub slot2: LayerEntry,
        pub slot3: LayerEntry,
        pub slot4: LayerEntry,
        pub slot5: LayerEntry,
        pub slot6: LayerEntry,
        pub slot7: LayerEntry,
        pub slot8: LayerEntry,
        pub slot9: LayerEntry,
        pub slot10: LayerEntry,
        pub toggle_cursor: LayerEntry,
        pub escape: LayerEntry,
        pub enter: LayerEntry,
        pub command: LayerEntry,
        pub move_forward: LayerEntry,
        pub move_left: LayerEntry,
        pub move_back: LayerEntry,
        pub move_right: LayerEntry,
        pub jump: LayerEntry,
        pub sit: LayerEntry,
        pub dance: LayerEntry,
        pub glide: LayerEntry,
        pub climb: LayerEntry,
        pub climb_down: LayerEntry,
        pub swimup: LayerEntry,
        pub swimdown: LayerEntry,
        pub sneak: LayerEntry,
        pub toggle_lantern: LayerEntry,
        pub mount: LayerEntry,
        pub map: LayerEntry,
        pub bag: LayerEntry,
        pub quest_log: LayerEntry,
        pub character_window: LayerEntry,
        pub social: LayerEntry,
        pub crafting: LayerEntry,
        pub spellbook: LayerEntry,
        pub settings: LayerEntry,
        pub help: LayerEntry,
        pub toggle_interface: LayerEntry,
        pub toggle_debug: LayerEntry,
        #[cfg(feature = "egui-ui")]
        pub toggle_egui_debug: LayerEntry,
        pub toggle_chat: LayerEntry,
        pub fullscreen: LayerEntry,
        pub screenshot: LayerEntry,
        pub toggle_ingame_ui: LayerEntry,
        pub roll: LayerEntry,
        pub respawn: LayerEntry,
        pub interact: LayerEntry,
        pub toggle_wield: LayerEntry,
        pub swap_loadout: LayerEntry,
    }

    impl Default for GameLayerEntries {
        fn default() -> Self {
            Self {
                primary: LayerEntry::default(),
                secondary: LayerEntry::default(),
                block: LayerEntry::default(),
                slot1: LayerEntry {
                    button: Button::Simple(GilButton::DPadRight),
                    mod1: Button::Simple(GilButton::RightTrigger),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                slot2: LayerEntry {
                    button: Button::Simple(GilButton::DPadDown),
                    mod1: Button::Simple(GilButton::RightTrigger),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                slot3: LayerEntry {
                    button: Button::Simple(GilButton::DPadUp),
                    mod1: Button::Simple(GilButton::LeftTrigger),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                slot4: LayerEntry {
                    button: Button::Simple(GilButton::DPadLeft),
                    mod1: Button::Simple(GilButton::LeftTrigger),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                slot5: LayerEntry {
                    button: Button::Simple(GilButton::DPadRight),
                    mod1: Button::Simple(GilButton::LeftTrigger),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                slot6: LayerEntry {
                    button: Button::Simple(GilButton::DPadDown),
                    mod1: Button::Simple(GilButton::LeftTrigger),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                slot7: LayerEntry {
                    button: Button::Simple(GilButton::DPadUp),
                    mod1: Button::Simple(GilButton::RightTrigger),
                    mod2: Button::Simple(GilButton::LeftTrigger),
                },
                slot8: LayerEntry {
                    button: Button::Simple(GilButton::DPadLeft),
                    mod1: Button::Simple(GilButton::RightTrigger),
                    mod2: Button::Simple(GilButton::LeftTrigger),
                },
                slot9: LayerEntry {
                    button: Button::Simple(GilButton::DPadRight),
                    mod1: Button::Simple(GilButton::RightTrigger),
                    mod2: Button::Simple(GilButton::LeftTrigger),
                },
                slot10: LayerEntry {
                    button: Button::Simple(GilButton::DPadDown),
                    mod1: Button::Simple(GilButton::RightTrigger),
                    mod2: Button::Simple(GilButton::LeftTrigger),
                },
                toggle_cursor: LayerEntry {
                    button: Button::Simple(GilButton::Start),
                    mod1: Button::Simple(GilButton::RightTrigger),
                    mod2: Button::Simple(GilButton::LeftTrigger),
                },
                escape: LayerEntry {
                    button: Button::Simple(GilButton::Start),
                    mod1: Button::Simple(GilButton::Unknown),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                enter: LayerEntry::default(),
                command: LayerEntry::default(),
                move_forward: LayerEntry::default(),
                move_left: LayerEntry::default(),
                move_back: LayerEntry::default(),
                move_right: LayerEntry::default(),
                jump: LayerEntry::default(),
                sit: LayerEntry {
                    button: Button::Simple(GilButton::Select),
                    mod1: Button::Simple(GilButton::RightTrigger),
                    mod2: Button::Simple(GilButton::LeftTrigger),
                },
                dance: LayerEntry {
                    button: Button::Simple(GilButton::Select),
                    mod1: Button::Simple(GilButton::LeftTrigger),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                glide: LayerEntry {
                    button: Button::Simple(GilButton::DPadUp),
                    mod1: Button::Simple(GilButton::Unknown),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                climb: LayerEntry::default(),
                climb_down: LayerEntry::default(),
                swimup: LayerEntry::default(),
                swimdown: LayerEntry::default(),
                sneak: LayerEntry::default(),
                toggle_lantern: LayerEntry {
                    button: Button::Simple(GilButton::DPadLeft),
                    mod1: Button::Simple(GilButton::RightTrigger),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                mount: LayerEntry::default(),
                map: LayerEntry {
                    button: Button::Simple(GilButton::DPadLeft),
                    mod1: Button::Simple(GilButton::Unknown),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                bag: LayerEntry::default(),
                quest_log: LayerEntry {
                    button: Button::Simple(GilButton::DPadRight),
                    mod1: Button::Simple(GilButton::Unknown),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                character_window: LayerEntry::default(),
                social: LayerEntry {
                    button: Button::Simple(GilButton::DPadUp),
                    mod1: Button::Simple(GilButton::RightTrigger),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                crafting: LayerEntry {
                    button: Button::Simple(GilButton::Select),
                    mod1: Button::Simple(GilButton::RightTrigger),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                spellbook: LayerEntry {
                    button: Button::Simple(GilButton::Select),
                    mod1: Button::Simple(GilButton::Unknown),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                settings: LayerEntry {
                    button: Button::Simple(GilButton::Start),
                    mod1: Button::Simple(GilButton::RightTrigger),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                help: LayerEntry {
                    button: Button::Simple(GilButton::Start),
                    mod1: Button::Simple(GilButton::LeftTrigger),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                toggle_interface: LayerEntry::default(),
                toggle_debug: LayerEntry::default(),
                #[cfg(feature = "egui-ui")]
                toggle_egui_debug: LayerEntry::default(),
                toggle_chat: LayerEntry::default(),
                fullscreen: LayerEntry::default(),
                screenshot: LayerEntry::default(),
                toggle_ingame_ui: LayerEntry::default(),
                roll: LayerEntry::default(),
                respawn: LayerEntry::default(),
                interact: LayerEntry::default(),
                toggle_wield: LayerEntry::default(),
                swap_loadout: LayerEntry {
                    button: Button::Simple(GilButton::DPadDown),
                    mod1: Button::Simple(GilButton::Unknown),
                    mod2: Button::Simple(GilButton::Unknown),
                },
            }
        }
    }

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
        pub stayfollow: Button,
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
                block: Button::Simple(GilButton::North),
                slot1: Button::Simple(GilButton::Unknown),
                slot2: Button::Simple(GilButton::Unknown),
                slot3: Button::Simple(GilButton::Unknown),
                slot4: Button::Simple(GilButton::Unknown),
                slot5: Button::Simple(GilButton::Unknown),
                slot6: Button::Simple(GilButton::Unknown),
                slot7: Button::Simple(GilButton::Unknown),
                slot8: Button::Simple(GilButton::Unknown),
                slot9: Button::Simple(GilButton::Unknown),
                slot10: Button::Simple(GilButton::Unknown),
                toggle_cursor: Button::Simple(GilButton::Unknown),
                escape: Button::Simple(GilButton::Unknown),
                enter: Button::Simple(GilButton::Unknown),
                command: Button::Simple(GilButton::Unknown),
                move_forward: Button::Simple(GilButton::Unknown),
                move_left: Button::Simple(GilButton::Unknown),
                move_back: Button::Simple(GilButton::Unknown),
                move_right: Button::Simple(GilButton::Unknown),
                jump: Button::Simple(GilButton::South),
                sit: Button::Simple(GilButton::Unknown),
                dance: Button::Simple(GilButton::Unknown),
                glide: Button::Simple(GilButton::Unknown),
                climb: Button::Simple(GilButton::South),
                climb_down: Button::Simple(GilButton::West),
                swimup: Button::Simple(GilButton::South),
                swimdown: Button::Simple(GilButton::West),
                sneak: Button::Simple(GilButton::LeftThumb),
                toggle_lantern: Button::Simple(GilButton::Unknown),
                mount: Button::Simple(GilButton::South),
                stayfollow: Button::Simple(GilButton::Unknown),
                map: Button::Simple(GilButton::Unknown),
                bag: Button::Simple(GilButton::East),
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
                roll: Button::Simple(GilButton::RightThumb),
                respawn: Button::Simple(GilButton::South),
                interact: Button::Simple(GilButton::West),
                toggle_wield: Button::Simple(GilButton::Unknown),
                swap_loadout: Button::Simple(GilButton::Unknown),
            }
        }
    }

    impl Default for MenuButtons {
        fn default() -> Self {
            Self {
                up: Button::Simple(GilButton::DPadUp),
                down: Button::Simple(GilButton::DPadDown),
                left: Button::Simple(GilButton::DPadLeft),
                right: Button::Simple(GilButton::DPadRight),
                scroll_up: Button::Simple(GilButton::Unknown),
                scroll_down: Button::Simple(GilButton::Unknown),
                scroll_left: Button::Simple(GilButton::Unknown),
                scroll_right: Button::Simple(GilButton::Unknown),
                home: Button::Simple(GilButton::Unknown),
                end: Button::Simple(GilButton::Unknown),
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
