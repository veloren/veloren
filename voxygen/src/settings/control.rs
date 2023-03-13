use crate::{game_input::GameInput, window::KeyMouse};
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use winit::event::{MouseButton, VirtualKeyCode};

// ControlSetting-like struct used by Serde, to handle not serializing/building
// post-deserializing the inverse_keybindings hashmap
#[derive(Serialize, Deserialize)]
struct ControlSettingsSerde {
    keybindings: HashMap<GameInput, Option<KeyMouse>>,
}

impl From<ControlSettings> for ControlSettingsSerde {
    fn from(control_settings: ControlSettings) -> Self {
        let mut user_bindings: HashMap<GameInput, Option<KeyMouse>> = HashMap::new();
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
    pub keybindings: HashMap<GameInput, Option<KeyMouse>>,
    pub inverse_keybindings: HashMap<KeyMouse, HashSet<GameInput>>, // used in event loop
}

impl From<ControlSettingsSerde> for ControlSettings {
    fn from(control_serde: ControlSettingsSerde) -> Self {
        let user_keybindings = control_serde.keybindings;
        let mut control_settings = ControlSettings::default();
        for (k, maybe_v) in user_keybindings {
            match maybe_v {
                Some(v) => control_settings.modify_binding(k, v),
                None => control_settings.remove_binding(k),
            }
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
const MIDDLE_CLICK_KEY: KeyMouse = KeyMouse::Key(VirtualKeyCode::Grave);
#[cfg(not(target_os = "macos"))]
const MIDDLE_CLICK_KEY: KeyMouse = KeyMouse::Mouse(MouseButton::Middle);

impl ControlSettings {
    pub fn remove_binding(&mut self, game_input: GameInput) {
        if let Some(inverse) = self
            .keybindings
            .insert(game_input, None)
            .flatten()
            .and_then(|key_mouse| self.inverse_keybindings.get_mut(&key_mouse))
        {
            inverse.remove(&game_input);
        }
    }

    pub fn get_binding(&self, game_input: GameInput) -> Option<KeyMouse> {
        self.keybindings.get(&game_input).copied().flatten()
    }

    pub fn get_associated_game_inputs(&self, key_mouse: &KeyMouse) -> Option<&HashSet<GameInput>> {
        self.inverse_keybindings.get(key_mouse)
    }

    pub fn insert_binding(&mut self, game_input: GameInput, key_mouse: KeyMouse) {
        self.keybindings.insert(game_input, Some(key_mouse));
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
        self.keybindings.insert(game_input, Some(key_mouse));
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

    pub fn default_binding(game_input: GameInput) -> Option<KeyMouse> {
        // If a new GameInput is added, be sure to update GameInput::iterator() too!
        match game_input {
            GameInput::Primary => Some(KeyMouse::Mouse(MouseButton::Left)),
            GameInput::Secondary => Some(KeyMouse::Mouse(MouseButton::Right)),
            GameInput::Block => Some(KeyMouse::Key(VirtualKeyCode::LAlt)),
            GameInput::ToggleCursor => Some(KeyMouse::Key(VirtualKeyCode::Comma)),
            GameInput::Escape => Some(KeyMouse::Key(VirtualKeyCode::Escape)),
            GameInput::Chat => Some(KeyMouse::Key(VirtualKeyCode::Return)),
            GameInput::Command => Some(KeyMouse::Key(VirtualKeyCode::Slash)),
            GameInput::MoveForward => Some(KeyMouse::Key(VirtualKeyCode::W)),
            GameInput::MoveLeft => Some(KeyMouse::Key(VirtualKeyCode::A)),
            GameInput::MoveBack => Some(KeyMouse::Key(VirtualKeyCode::S)),
            GameInput::MoveRight => Some(KeyMouse::Key(VirtualKeyCode::D)),
            GameInput::Jump => Some(KeyMouse::Key(VirtualKeyCode::Space)),
            GameInput::Sit => Some(KeyMouse::Key(VirtualKeyCode::K)),
            GameInput::Dance => Some(KeyMouse::Key(VirtualKeyCode::J)),
            GameInput::Greet => Some(KeyMouse::Key(VirtualKeyCode::H)),
            GameInput::Glide => Some(KeyMouse::Key(VirtualKeyCode::LControl)),
            GameInput::Climb => Some(KeyMouse::Key(VirtualKeyCode::Space)),
            GameInput::ClimbDown => Some(KeyMouse::Key(VirtualKeyCode::LShift)),
            GameInput::SwimUp => Some(KeyMouse::Key(VirtualKeyCode::Space)),
            GameInput::SwimDown => Some(KeyMouse::Key(VirtualKeyCode::LShift)),
            GameInput::Fly => Some(KeyMouse::Key(VirtualKeyCode::H)),
            GameInput::Sneak => Some(KeyMouse::Key(VirtualKeyCode::LShift)),
            GameInput::ToggleLantern => Some(KeyMouse::Key(VirtualKeyCode::G)),
            GameInput::Mount => Some(KeyMouse::Key(VirtualKeyCode::F)),
            GameInput::Map => Some(KeyMouse::Key(VirtualKeyCode::M)),
            GameInput::Bag => Some(KeyMouse::Key(VirtualKeyCode::B)),
            GameInput::Trade => Some(KeyMouse::Key(VirtualKeyCode::T)),
            GameInput::Social => Some(KeyMouse::Key(VirtualKeyCode::O)),
            GameInput::Crafting => Some(KeyMouse::Key(VirtualKeyCode::C)),
            GameInput::Spellbook => Some(KeyMouse::Key(VirtualKeyCode::P)),
            GameInput::Settings => Some(KeyMouse::Key(VirtualKeyCode::F10)),
            GameInput::Help => Some(KeyMouse::Key(VirtualKeyCode::F1)),
            GameInput::ToggleInterface => Some(KeyMouse::Key(VirtualKeyCode::F2)),
            GameInput::ToggleDebug => Some(KeyMouse::Key(VirtualKeyCode::F3)),
            #[cfg(feature = "egui-ui")]
            GameInput::ToggleEguiDebug => Some(KeyMouse::Key(VirtualKeyCode::F7)),
            GameInput::ToggleChat => Some(KeyMouse::Key(VirtualKeyCode::F5)),
            GameInput::Fullscreen => Some(KeyMouse::Key(VirtualKeyCode::F11)),
            GameInput::Screenshot => Some(KeyMouse::Key(VirtualKeyCode::F4)),
            GameInput::ToggleIngameUi => Some(KeyMouse::Key(VirtualKeyCode::F6)),
            GameInput::Roll => Some(MIDDLE_CLICK_KEY),
            GameInput::Respawn => Some(KeyMouse::Key(VirtualKeyCode::Space)),
            GameInput::Interact => Some(KeyMouse::Key(VirtualKeyCode::E)),
            GameInput::ToggleWield => Some(KeyMouse::Key(VirtualKeyCode::R)),
            GameInput::FreeLook => Some(KeyMouse::Key(VirtualKeyCode::L)),
            GameInput::AutoWalk => Some(KeyMouse::Key(VirtualKeyCode::Period)),
            GameInput::ZoomLock => None,
            GameInput::CameraClamp => Some(KeyMouse::Key(VirtualKeyCode::Apostrophe)),
            GameInput::CycleCamera => Some(KeyMouse::Key(VirtualKeyCode::Key0)),
            GameInput::Slot1 => Some(KeyMouse::Key(VirtualKeyCode::Key1)),
            GameInput::Slot2 => Some(KeyMouse::Key(VirtualKeyCode::Key2)),
            GameInput::Slot3 => Some(KeyMouse::Key(VirtualKeyCode::Key3)),
            GameInput::Slot4 => Some(KeyMouse::Key(VirtualKeyCode::Key4)),
            GameInput::Slot5 => Some(KeyMouse::Key(VirtualKeyCode::Key5)),
            GameInput::Slot6 => Some(KeyMouse::Key(VirtualKeyCode::Key6)),
            GameInput::Slot7 => Some(KeyMouse::Key(VirtualKeyCode::Key7)),
            GameInput::Slot8 => Some(KeyMouse::Key(VirtualKeyCode::Key8)),
            GameInput::Slot9 => Some(KeyMouse::Key(VirtualKeyCode::Key9)),
            GameInput::Slot10 => Some(KeyMouse::Key(VirtualKeyCode::Q)),
            GameInput::SwapLoadout => Some(KeyMouse::Key(VirtualKeyCode::Tab)),
            GameInput::Select => Some(KeyMouse::Key(VirtualKeyCode::X)),
            GameInput::AcceptGroupInvite => Some(KeyMouse::Key(VirtualKeyCode::Y)),
            GameInput::DeclineGroupInvite => Some(KeyMouse::Key(VirtualKeyCode::N)),
            GameInput::MapZoomIn => Some(KeyMouse::Key(VirtualKeyCode::Plus)),
            GameInput::MapZoomOut => Some(KeyMouse::Key(VirtualKeyCode::Minus)),
            GameInput::MapSetMarker => Some(KeyMouse::Mouse(MouseButton::Middle)),
            GameInput::SpectateSpeedBoost => Some(KeyMouse::Key(VirtualKeyCode::LControl)),
            GameInput::SpectateViewpoint => Some(KeyMouse::Mouse(MouseButton::Middle)),
            GameInput::MuteMaster => Some(KeyMouse::Key(VirtualKeyCode::Mute)),
            GameInput::MuteInactiveMaster => None,
            GameInput::MuteMusic => Some(KeyMouse::Key(VirtualKeyCode::F8)),
            GameInput::MuteSfx => None,
            GameInput::MuteAmbience => None,
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
        for game_input in GameInput::iter() {
            match ControlSettings::default_binding(game_input) {
                None => {},
                Some(default) => new_settings.insert_binding(game_input, default),
            };
        }
        new_settings
    }
}
