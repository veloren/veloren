use crate::window::{GameInput, KeyMouse};
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use winit::event::{MouseButton, VirtualKeyCode};

// ControlSetting-like struct used by Serde, to handle not serializing/building
// post-deserializing the inverse_keybindings hashmap
#[derive(Serialize, Deserialize)]
struct ControlSettingsSerde {
    keybindings: HashMap<GameInput, KeyMouse>,
}

impl From<ControlSettings> for ControlSettingsSerde {
    fn from(control_settings: ControlSettings) -> Self {
        let mut user_bindings: HashMap<GameInput, KeyMouse> = HashMap::new();
        // Do a delta between default() ControlSettings and the argument, and let
        // keybindings be only the custom keybindings chosen by the user.
        for (k, v) in control_settings.keybindings {
            if ControlSettings::default_binding(k) != v {
                // Keybinding chosen by the user
                user_bindings.insert(k, v);
            }
        }
        ControlSettingsSerde {
            keybindings: user_bindings,
        }
    }
}

/// `ControlSettings` contains keybindings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(from = "ControlSettingsSerde", into = "ControlSettingsSerde")]
pub struct ControlSettings {
    pub keybindings: HashMap<GameInput, KeyMouse>,
    pub inverse_keybindings: HashMap<KeyMouse, HashSet<GameInput>>, // used in event loop
}

impl From<ControlSettingsSerde> for ControlSettings {
    fn from(control_serde: ControlSettingsSerde) -> Self {
        let user_keybindings = control_serde.keybindings;
        let mut control_settings = ControlSettings::default();
        for (k, v) in user_keybindings {
            control_settings.modify_binding(k, v);
        }
        control_settings
    }
}

/// Since Macbook trackpads lack middle click, on OS X we default to LShift
/// instead It is an imperfect heuristic, but hopefully it will be a slightly
/// better default, and the two places we default to middle click currently
/// (roll and wall jump) are both situations where you cannot glide (the other
/// default mapping for LShift).
#[cfg(target_os = "macos")]
const MIDDLE_CLICK_KEY: KeyMouse = KeyMouse::Key(VirtualKeyCode::LShift);
#[cfg(not(target_os = "macos"))]
const MIDDLE_CLICK_KEY: KeyMouse = KeyMouse::Mouse(MouseButton::Middle);

impl ControlSettings {
    pub fn get_binding(&self, game_input: GameInput) -> Option<KeyMouse> {
        self.keybindings.get(&game_input).copied()
    }

    pub fn get_associated_game_inputs(&self, key_mouse: &KeyMouse) -> Option<&HashSet<GameInput>> {
        self.inverse_keybindings.get(key_mouse)
    }

    pub fn insert_binding(&mut self, game_input: GameInput, key_mouse: KeyMouse) {
        self.keybindings.insert(game_input, key_mouse);
        self.inverse_keybindings
            .entry(key_mouse)
            .or_default()
            .insert(game_input);
    }

    pub fn modify_binding(&mut self, game_input: GameInput, key_mouse: KeyMouse) {
        // For the KeyMouse->GameInput hashmap, we first need to remove the GameInput
        // from the old binding
        if let Some(old_binding) = self.get_binding(game_input) {
            self.inverse_keybindings
                .entry(old_binding)
                .or_default()
                .remove(&game_input);
        }
        // then we add the GameInput to the proper key
        self.inverse_keybindings
            .entry(key_mouse)
            .or_default()
            .insert(game_input);
        // For the GameInput->KeyMouse hashmap, just overwrite the value
        self.keybindings.insert(game_input, key_mouse);
    }

    /// Return true if this key is used for multiple GameInputs that aren't
    /// expected to be safe to have bound to the same key at the same time
    pub fn has_conflicting_bindings(&self, key_mouse: KeyMouse) -> bool {
        if let Some(game_inputs) = self.inverse_keybindings.get(&key_mouse) {
            for a in game_inputs.iter() {
                for b in game_inputs.iter() {
                    if !GameInput::can_share_bindings(*a, *b) {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn default_binding(game_input: GameInput) -> KeyMouse {
        // If a new GameInput is added, be sure to update GameInput::iterator() too!
        match game_input {
            GameInput::Primary => KeyMouse::Mouse(MouseButton::Left),
            GameInput::Secondary => KeyMouse::Mouse(MouseButton::Right),
            GameInput::ToggleCursor => KeyMouse::Key(VirtualKeyCode::Comma),
            GameInput::Escape => KeyMouse::Key(VirtualKeyCode::Escape),
            GameInput::Chat => KeyMouse::Key(VirtualKeyCode::Return),
            GameInput::Command => KeyMouse::Key(VirtualKeyCode::Slash),
            GameInput::MoveForward => KeyMouse::Key(VirtualKeyCode::W),
            GameInput::MoveLeft => KeyMouse::Key(VirtualKeyCode::A),
            GameInput::MoveBack => KeyMouse::Key(VirtualKeyCode::S),
            GameInput::MoveRight => KeyMouse::Key(VirtualKeyCode::D),
            GameInput::Jump => KeyMouse::Key(VirtualKeyCode::Space),
            GameInput::Sit => KeyMouse::Key(VirtualKeyCode::K),
            GameInput::Dance => KeyMouse::Key(VirtualKeyCode::J),
            GameInput::Glide => KeyMouse::Key(VirtualKeyCode::LShift),
            GameInput::Climb => KeyMouse::Key(VirtualKeyCode::Space),
            GameInput::ClimbDown => KeyMouse::Key(VirtualKeyCode::LControl),
            GameInput::SwimUp => KeyMouse::Key(VirtualKeyCode::Space),
            GameInput::SwimDown => KeyMouse::Key(VirtualKeyCode::LShift),
            GameInput::Fly => KeyMouse::Key(VirtualKeyCode::H),
            GameInput::Sneak => KeyMouse::Key(VirtualKeyCode::LControl),
            GameInput::ToggleLantern => KeyMouse::Key(VirtualKeyCode::G),
            GameInput::Mount => KeyMouse::Key(VirtualKeyCode::F),
            GameInput::Map => KeyMouse::Key(VirtualKeyCode::M),
            GameInput::Bag => KeyMouse::Key(VirtualKeyCode::B),
            GameInput::Trade => KeyMouse::Key(VirtualKeyCode::R),
            GameInput::Social => KeyMouse::Key(VirtualKeyCode::O),
            GameInput::Crafting => KeyMouse::Key(VirtualKeyCode::C),
            GameInput::Spellbook => KeyMouse::Key(VirtualKeyCode::P),
            GameInput::Settings => KeyMouse::Key(VirtualKeyCode::N),
            GameInput::Help => KeyMouse::Key(VirtualKeyCode::F1),
            GameInput::ToggleInterface => KeyMouse::Key(VirtualKeyCode::F2),
            GameInput::ToggleDebug => KeyMouse::Key(VirtualKeyCode::F3),
            GameInput::Fullscreen => KeyMouse::Key(VirtualKeyCode::F11),
            GameInput::Screenshot => KeyMouse::Key(VirtualKeyCode::F4),
            GameInput::ToggleIngameUi => KeyMouse::Key(VirtualKeyCode::F6),
            GameInput::Roll => MIDDLE_CLICK_KEY,
            GameInput::Respawn => KeyMouse::Key(VirtualKeyCode::Space),
            GameInput::Interact => KeyMouse::Key(VirtualKeyCode::E),
            GameInput::ToggleWield => KeyMouse::Key(VirtualKeyCode::T),
            GameInput::FreeLook => KeyMouse::Key(VirtualKeyCode::L),
            GameInput::AutoWalk => KeyMouse::Key(VirtualKeyCode::Period),
            GameInput::CameraClamp => KeyMouse::Key(VirtualKeyCode::Apostrophe),
            GameInput::CycleCamera => KeyMouse::Key(VirtualKeyCode::Key0),
            GameInput::Slot1 => KeyMouse::Key(VirtualKeyCode::Key1),
            GameInput::Slot2 => KeyMouse::Key(VirtualKeyCode::Key2),
            GameInput::Slot3 => KeyMouse::Key(VirtualKeyCode::Key3),
            GameInput::Slot4 => KeyMouse::Key(VirtualKeyCode::Key4),
            GameInput::Slot5 => KeyMouse::Key(VirtualKeyCode::Key5),
            GameInput::Slot6 => KeyMouse::Key(VirtualKeyCode::Key6),
            GameInput::Slot7 => KeyMouse::Key(VirtualKeyCode::Key7),
            GameInput::Slot8 => KeyMouse::Key(VirtualKeyCode::Key8),
            GameInput::Slot9 => KeyMouse::Key(VirtualKeyCode::Key9),
            GameInput::Slot10 => KeyMouse::Key(VirtualKeyCode::Q),
            GameInput::SwapLoadout => KeyMouse::Key(VirtualKeyCode::Tab),
            GameInput::Select => KeyMouse::Key(VirtualKeyCode::Y),
            GameInput::AcceptGroupInvite => KeyMouse::Key(VirtualKeyCode::U),
            GameInput::DeclineGroupInvite => KeyMouse::Key(VirtualKeyCode::I),
        }
    }
}

impl Default for ControlSettings {
    fn default() -> Self {
        let mut new_settings = Self {
            keybindings: HashMap::new(),
            inverse_keybindings: HashMap::new(),
        };
        // Sets the initial keybindings for those GameInputs.
        for game_input in GameInput::iterator() {
            new_settings.insert_binding(game_input, ControlSettings::default_binding(game_input));
        }
        new_settings
    }
}
