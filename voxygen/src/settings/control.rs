use crate::{game_input::GameInput, window::KeyMouse};
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use winit::{
    event::MouseButton,
    keyboard::{Key, NamedKey},
};

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

/// Since Macbook trackpads lack middle click, on OS X we default to Shift
/// instead. It is an imperfect heuristic, but hopefully it will be a slightly
/// better default, and the two places we default to middle click currently
/// (roll and wall jump) are both situations where you cannot glide (the other
/// default mapping for Shift).
#[cfg(target_os = "macos")]
const MIDDLE_CLICK_KEY: KeyMouse = KeyMouse::Key(Key(NamedKey::Shift));
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
        self.keybindings.get(&game_input).cloned().flatten()
    }

    pub fn get_associated_game_inputs(&self, key_mouse: &KeyMouse) -> Option<&HashSet<GameInput>> {
        self.inverse_keybindings.get(key_mouse)
    }

    pub fn insert_binding(&mut self, game_input: GameInput, key_mouse: KeyMouse) {
        self.keybindings.insert(game_input, Some(key_mouse.clone()));
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
            .entry(key_mouse.clone())
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
        let char = |s| Key::Character(winit::keyboard::SmolStr::new(s));

        // If a new GameInput is added, be sure to update GameInput::iterator() too!
        Some(KeyMouse::Key(match game_input {
            GameInput::Primary => return Some(KeyMouse::Mouse(MouseButton::Left)),
            GameInput::Secondary => return Some(KeyMouse::Mouse(MouseButton::Right)),
            GameInput::Block => Key::Named(NamedKey::Alt),
            GameInput::ToggleCursor => char(","),
            GameInput::Escape => Key::Named(NamedKey::Escape),
            GameInput::Chat => Key::Named(NamedKey::Enter),
            GameInput::Command => char("/"),
            GameInput::MoveForward => char("W"),
            GameInput::MoveLeft => char("A"),
            GameInput::MoveBack => char("S"),
            GameInput::MoveRight => char("D"),
            GameInput::Jump => Key::Named(NamedKey::Space),
            GameInput::Sit => char("K"),
            GameInput::Crawl => Key::Named(NamedKey::ArrowDown),
            GameInput::Dance => char("J"),
            GameInput::Greet => char("H"),
            GameInput::Glide => Key::Named(NamedKey::Control),
            GameInput::SwimUp => Key::Named(NamedKey::Space),
            GameInput::SwimDown => Key::Named(NamedKey::Shift),
            GameInput::Fly => char("H"),
            GameInput::Sneak => Key::Named(NamedKey::Shift),
            GameInput::CancelClimb => Key::Named(NamedKey::Shift),
            GameInput::ToggleLantern => char("G"),
            GameInput::Mount => char("F"),
            GameInput::StayFollow => char("V"),
            GameInput::Map => char("M"),
            GameInput::Inventory => char("I"),
            GameInput::Trade => char("T"),
            GameInput::Social => char("O"),
            GameInput::Crafting => char("C"),
            GameInput::Diary => char("P"),
            GameInput::Settings => Key::Named(NamedKey::F10),
            GameInput::Controls => Key::Named(NamedKey::F1),
            GameInput::ToggleInterface => Key::Named(NamedKey::F2),
            GameInput::ToggleDebug => Key::Named(NamedKey::F3),
            #[cfg(feature = "egui-ui")]
            GameInput::ToggleEguiDebug => Key::Named(NamedKey::F7),
            GameInput::ToggleChat => Key::Named(NamedKey::F5),
            GameInput::Fullscreen => Key::Named(NamedKey::F11),
            GameInput::Screenshot => Key::Named(NamedKey::F4),
            GameInput::ToggleIngameUi => Key::Named(NamedKey::F6),
            GameInput::Roll => return Some(MIDDLE_CLICK_KEY),
            GameInput::GiveUp => Key::Named(NamedKey::Space),
            GameInput::Respawn => Key::Named(NamedKey::Space),
            GameInput::Interact => char("E"),
            GameInput::ToggleWield => char("R"),
            GameInput::FreeLook => char("L"),
            GameInput::AutoWalk => char("."),
            GameInput::ZoomIn => char(")"),
            GameInput::ZoomOut => char("("),
            GameInput::ZoomLock => return None,
            GameInput::CameraClamp => char("'"),
            GameInput::CycleCamera => char("0"),
            GameInput::Slot1 => char("1"),
            GameInput::Slot2 => char("2"),
            GameInput::Slot3 => char("3"),
            GameInput::Slot4 => char("4"),
            GameInput::Slot5 => char("5"),
            GameInput::Slot6 => char("6"),
            GameInput::Slot7 => char("7"),
            GameInput::Slot8 => char("8"),
            GameInput::Slot9 => char("9"),
            GameInput::Slot10 => char("Q"),
            GameInput::SwapLoadout => Key::Named(NamedKey::Tab),
            GameInput::Select => char("X"),
            GameInput::AcceptGroupInvite => char("Y"),
            GameInput::DeclineGroupInvite => char("N"),
            GameInput::MapZoomIn => char("+"),
            GameInput::MapZoomOut => char("-"),
            GameInput::MapSetMarker => return Some(KeyMouse::Mouse(MouseButton::Middle)),
            GameInput::SpectateSpeedBoost => Key::Named(NamedKey::Control),
            GameInput::SpectateViewpoint => return Some(KeyMouse::Mouse(MouseButton::Middle)),
            GameInput::MuteMaster => Key::Named(NamedKey::AudioVolumeMute),
            GameInput::MuteInactiveMaster => return None,
            GameInput::MuteMusic => Key::Named(NamedKey::F8),
            GameInput::MuteSfx => return None,
            GameInput::MuteAmbience => return None,
            GameInput::ToggleWalk => char("B"),
        }))
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
