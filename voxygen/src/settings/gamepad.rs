//! Module containing game layer specific actions for controller, all contents
//! are saved to settings.ron

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GamepadSettings {
    pub game_layer_buttons: con_settings::GameLayerEntries,
}

impl Default for GamepadSettings {
    fn default() -> Self {
        Self {
            game_layer_buttons: con_settings::GameLayerEntries::default(),
        }
    }
}

pub mod con_settings {
    use crate::settings::controller::*;
    use gilrs::Button as GilButton;
    use i18n::Localization;
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
        pub wall_jump: LayerEntry,
        pub sit: LayerEntry,
        pub crawl: LayerEntry,
        pub dance: LayerEntry,
        pub greet: LayerEntry,
        pub glide: LayerEntry,
        pub swimup: LayerEntry,
        pub swimdown: LayerEntry,
        pub fly: LayerEntry,
        pub sneak: LayerEntry,
        pub cancel_climb: LayerEntry,
        pub toggle_lantern: LayerEntry,
        pub mount: LayerEntry,
        pub stayfollow: LayerEntry,
        pub chat: LayerEntry,
        pub map: LayerEntry,
        pub inventory: LayerEntry,
        pub quest_log: LayerEntry,
        pub character_window: LayerEntry,
        pub trade: LayerEntry,
        pub social: LayerEntry,
        pub crafting: LayerEntry,
        pub diary: LayerEntry,
        pub settings: LayerEntry,
        pub controls: LayerEntry,
        pub toggle_interface: LayerEntry,
        pub toggle_debug: LayerEntry,
        #[cfg(feature = "egui-ui")]
        pub toggle_egui_debug: LayerEntry,
        pub toggle_chat: LayerEntry,
        pub fullscreen: LayerEntry,
        pub screenshot: LayerEntry,
        pub toggle_ingame_ui: LayerEntry,
        pub roll: LayerEntry,
        pub give_up: LayerEntry,
        pub respawn: LayerEntry,
        pub interact: LayerEntry,
        pub toggle_wield: LayerEntry,
        pub swap_loadout: LayerEntry,
        pub free_look: LayerEntry,
        pub auto_walk: LayerEntry,
        pub zoom_in: LayerEntry,
        pub zoom_out: LayerEntry,
        pub zoom_lock: LayerEntry,
        pub camera_clamp: LayerEntry,
        pub cycle_camera: LayerEntry,
        pub select: LayerEntry,
        pub accept_group_invite: LayerEntry,
        pub decline_group_invite: LayerEntry,
        pub map_zoom_in: LayerEntry,
        pub map_zoom_out: LayerEntry,
        pub map_set_marker: LayerEntry,
        pub spectate_speed_boost: LayerEntry,
        pub spectate_viewpoint: LayerEntry,
        pub mute_master: LayerEntry,
        pub mute_inactive_master: LayerEntry,
        pub mute_music: LayerEntry,
        pub mute_sfx: LayerEntry,
        pub mute_ambience: LayerEntry,
        pub toggle_walk: LayerEntry,
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
                wall_jump: LayerEntry::default(),
                sit: LayerEntry {
                    button: Button::Simple(GilButton::Select),
                    mod1: Button::Simple(GilButton::RightTrigger),
                    mod2: Button::Simple(GilButton::LeftTrigger),
                },
                crawl: LayerEntry::default(),
                dance: LayerEntry {
                    button: Button::Simple(GilButton::Select),
                    mod1: Button::Simple(GilButton::LeftTrigger),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                greet: LayerEntry::default(),
                glide: LayerEntry {
                    button: Button::Simple(GilButton::DPadUp),
                    mod1: Button::Simple(GilButton::Unknown),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                swimup: LayerEntry::default(),
                swimdown: LayerEntry::default(),
                fly: LayerEntry::default(),
                sneak: LayerEntry::default(),
                cancel_climb: LayerEntry::default(),
                toggle_lantern: LayerEntry {
                    button: Button::Simple(GilButton::DPadLeft),
                    mod1: Button::Simple(GilButton::RightTrigger),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                mount: LayerEntry::default(),
                stayfollow: LayerEntry::default(),
                chat: LayerEntry::default(),
                map: LayerEntry {
                    button: Button::Simple(GilButton::Unknown),
                    mod1: Button::Simple(GilButton::Unknown),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                inventory: LayerEntry::default(),
                quest_log: LayerEntry {
                    button: Button::Simple(GilButton::DPadRight),
                    mod1: Button::Simple(GilButton::Unknown),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                character_window: LayerEntry::default(),
                trade: LayerEntry::default(),
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
                diary: LayerEntry {
                    button: Button::Simple(GilButton::Select),
                    mod1: Button::Simple(GilButton::Unknown),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                settings: LayerEntry {
                    button: Button::Simple(GilButton::Start),
                    mod1: Button::Simple(GilButton::RightTrigger),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                controls: LayerEntry {
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
                give_up: LayerEntry::default(),
                respawn: LayerEntry::default(),
                interact: LayerEntry::default(),
                toggle_wield: LayerEntry::default(),
                swap_loadout: LayerEntry {
                    button: Button::Simple(GilButton::DPadDown),
                    mod1: Button::Simple(GilButton::Unknown),
                    mod2: Button::Simple(GilButton::Unknown),
                },
                free_look: LayerEntry::default(),
                auto_walk: LayerEntry::default(),
                zoom_in: LayerEntry::default(),
                zoom_out: LayerEntry::default(),
                zoom_lock: LayerEntry::default(),
                camera_clamp: LayerEntry::default(),
                cycle_camera: LayerEntry::default(),
                select: LayerEntry::default(),
                accept_group_invite: LayerEntry::default(),
                decline_group_invite: LayerEntry::default(),
                map_zoom_in: LayerEntry::default(),
                map_zoom_out: LayerEntry::default(),
                map_set_marker: LayerEntry::default(),
                spectate_speed_boost: LayerEntry::default(),
                spectate_viewpoint: LayerEntry::default(),
                mute_ambience: LayerEntry::default(),
                mute_inactive_master: LayerEntry::default(),
                mute_master: LayerEntry::default(),
                mute_music: LayerEntry::default(),
                mute_sfx: LayerEntry::default(),
                toggle_walk: LayerEntry::default(),
            }
        }
    }
}
