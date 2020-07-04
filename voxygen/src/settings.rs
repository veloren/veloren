use crate::{
    hud::{BarNumbers, CrosshairType, Intro, PressBehavior, ShortcutNumbers, XpBar},
    i18n,
    render::RenderMode,
    ui::ScaleMode,
    window::{GameInput, KeyMouse},
};
use directories::{ProjectDirs, UserDirs};
use glutin::{MouseButton, VirtualKeyCode};
use hashbrown::{HashMap, HashSet};
use serde_derive::{Deserialize, Serialize};
use std::{fs, io::prelude::*, path::PathBuf};
use tracing::warn;

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

    pub fn default_binding(game_input: GameInput) -> KeyMouse {
        // If a new GameInput is added, be sure to update ControlSettings::default()
        // too!
        match game_input {
            GameInput::Primary => KeyMouse::Mouse(MouseButton::Left),
            GameInput::Secondary => KeyMouse::Mouse(MouseButton::Right),
            GameInput::ToggleCursor => KeyMouse::Key(VirtualKeyCode::Tab),
            GameInput::Escape => KeyMouse::Key(VirtualKeyCode::Escape),
            GameInput::Enter => KeyMouse::Key(VirtualKeyCode::Return),
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
            //GameInput::WallLeap => MIDDLE_CLICK_KEY,
            GameInput::ToggleLantern => KeyMouse::Key(VirtualKeyCode::G),
            GameInput::Mount => KeyMouse::Key(VirtualKeyCode::F),
            GameInput::Map => KeyMouse::Key(VirtualKeyCode::M),
            GameInput::Bag => KeyMouse::Key(VirtualKeyCode::B),
            GameInput::Social => KeyMouse::Key(VirtualKeyCode::O),
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
            //GameInput::Charge => KeyMouse::Key(VirtualKeyCode::Key1),
            GameInput::FreeLook => KeyMouse::Key(VirtualKeyCode::L),
            GameInput::AutoWalk => KeyMouse::Key(VirtualKeyCode::Period),
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
            GameInput::SwapLoadout => KeyMouse::Key(VirtualKeyCode::LAlt),
        }
    }
}
impl Default for ControlSettings {
    fn default() -> Self {
        let mut new_settings = Self {
            keybindings: HashMap::new(),
            inverse_keybindings: HashMap::new(),
        };
        // Sets the initial keybindings for those GameInputs. If a new one is created in
        // future, you'll have to update default_binding, and you should update this vec
        // too.
        let game_inputs = vec![
            GameInput::Primary,
            GameInput::Secondary,
            GameInput::ToggleCursor,
            GameInput::MoveForward,
            GameInput::MoveBack,
            GameInput::MoveLeft,
            GameInput::MoveRight,
            GameInput::Jump,
            GameInput::Sit,
            GameInput::Dance,
            GameInput::Glide,
            GameInput::Climb,
            GameInput::ClimbDown,
            //GameInput::WallLeap,
            GameInput::ToggleLantern,
            GameInput::Mount,
            GameInput::Enter,
            GameInput::Command,
            GameInput::Escape,
            GameInput::Map,
            GameInput::Bag,
            GameInput::Social,
            GameInput::Spellbook,
            GameInput::Settings,
            GameInput::ToggleInterface,
            GameInput::Help,
            GameInput::ToggleDebug,
            GameInput::Fullscreen,
            GameInput::Screenshot,
            GameInput::ToggleIngameUi,
            GameInput::Roll,
            GameInput::Respawn,
            GameInput::Interact,
            GameInput::ToggleWield,
            //GameInput::Charge,
            GameInput::FreeLook,
            GameInput::AutoWalk,
            GameInput::Slot1,
            GameInput::Slot2,
            GameInput::Slot3,
            GameInput::Slot4,
            GameInput::Slot5,
            GameInput::Slot6,
            GameInput::Slot7,
            GameInput::Slot8,
            GameInput::Slot9,
            GameInput::Slot10,
            GameInput::SwapLoadout,
        ];
        for game_input in game_inputs {
            new_settings.insert_binding(game_input, ControlSettings::default_binding(game_input));
        }
        new_settings
    }
}

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
    use serde_derive::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(default)]
    pub struct GameButtons {
        pub primary: Button,
        pub secondary: Button,
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
        //pub wall_leap: Button,
        pub toggle_lantern: Button,
        pub mount: Button,
        pub map: Button,
        pub bag: Button,
        pub quest_log: Button,
        pub character_window: Button,
        pub social: Button,
        pub spellbook: Button,
        pub settings: Button,
        pub help: Button,
        pub toggle_interface: Button,
        pub toggle_debug: Button,
        pub fullscreen: Button,
        pub screenshot: Button,
        pub toggle_ingame_ui: Button,
        pub roll: Button,
        pub respawn: Button,
        pub interact: Button,
        pub toggle_wield: Button,
        pub swap_loadout: Button,
        //pub charge: Button,
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

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(default)]
    pub struct GameAnalogButton {}

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(default)]
    pub struct MenuAnalogButton {}

    impl Default for GameButtons {
        fn default() -> Self {
            // binding to unknown = getting skipped from processing
            Self {
                primary: Button::Simple(GilButton::RightTrigger2),
                secondary: Button::Simple(GilButton::LeftTrigger2),
                toggle_cursor: Button::Simple(GilButton::Select),
                escape: Button::Simple(GilButton::Mode),
                enter: Button::Simple(GilButton::Unknown),
                command: Button::Simple(GilButton::Unknown),
                move_forward: Button::Simple(GilButton::Unknown),
                move_left: Button::Simple(GilButton::Unknown),
                move_back: Button::Simple(GilButton::Unknown),
                move_right: Button::Simple(GilButton::Unknown),
                jump: Button::Simple(GilButton::South),
                sit: Button::Simple(GilButton::West),
                dance: Button::Simple(GilButton::Unknown),
                glide: Button::Simple(GilButton::LeftTrigger),
                climb: Button::Simple(GilButton::South),
                climb_down: Button::Simple(GilButton::Unknown),
                //wall_leap: Button::Simple(GilButton::Unknown),
                toggle_lantern: Button::Simple(GilButton::East),
                mount: Button::Simple(GilButton::North),
                map: Button::Simple(GilButton::DPadRight),
                bag: Button::Simple(GilButton::DPadDown),
                quest_log: Button::Simple(GilButton::Unknown),
                character_window: Button::Simple(GilButton::Unknown),
                social: Button::Simple(GilButton::Unknown),
                spellbook: Button::Simple(GilButton::Unknown),
                settings: Button::Simple(GilButton::Unknown),
                help: Button::Simple(GilButton::Unknown),
                toggle_interface: Button::Simple(GilButton::Unknown),
                toggle_debug: Button::Simple(GilButton::Unknown),
                fullscreen: Button::Simple(GilButton::Unknown),
                screenshot: Button::Simple(GilButton::DPadUp),
                toggle_ingame_ui: Button::Simple(GilButton::Unknown),
                roll: Button::Simple(GilButton::RightTrigger),
                respawn: Button::Simple(GilButton::RightTrigger2),
                interact: Button::Simple(GilButton::LeftTrigger2),
                toggle_wield: Button::Simple(GilButton::DPadLeft),
                swap_loadout: Button::Simple(GilButton::Unknown),
                //charge: Button::Simple(GilButton::Unknown),
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

    impl Default for GameAnalogButton {
        fn default() -> Self { Self {} }
    }

    impl Default for MenuAnalogButton {
        fn default() -> Self { Self {} }
    }
}

/// `GameplaySettings` contains sensitivity and gameplay options.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GameplaySettings {
    pub pan_sensitivity: u32,
    pub zoom_sensitivity: u32,
    pub zoom_inversion: bool,
    pub toggle_debug: bool,
    pub sct: bool,
    pub sct_player_batch: bool,
    pub sct_damage_batch: bool,
    pub speech_bubble_dark_mode: bool,
    pub speech_bubble_icon: bool,
    pub mouse_y_inversion: bool,
    pub smooth_pan_enable: bool,
    pub crosshair_transp: f32,
    pub chat_transp: f32,
    pub chat_character_name: bool,
    pub crosshair_type: CrosshairType,
    pub intro_show: Intro,
    pub xp_bar: XpBar,
    pub shortcut_numbers: ShortcutNumbers,
    pub bar_numbers: BarNumbers,
    pub ui_scale: ScaleMode,
    pub free_look_behavior: PressBehavior,
    pub auto_walk_behavior: PressBehavior,
    pub stop_auto_walk_on_input: bool,
    pub map_zoom: f64,
}

impl Default for GameplaySettings {
    fn default() -> Self {
        Self {
            pan_sensitivity: 100,
            zoom_sensitivity: 100,
            zoom_inversion: false,
            mouse_y_inversion: false,
            smooth_pan_enable: true,
            toggle_debug: false,
            sct: true,
            sct_player_batch: true,
            sct_damage_batch: false,
            speech_bubble_dark_mode: false,
            speech_bubble_icon: true,
            crosshair_transp: 0.6,
            chat_transp: 0.4,
            chat_character_name: true,
            crosshair_type: CrosshairType::Round,
            intro_show: Intro::Show,
            xp_bar: XpBar::Always,
            shortcut_numbers: ShortcutNumbers::On,
            bar_numbers: BarNumbers::Off,
            ui_scale: ScaleMode::RelativeToWindow([1920.0, 1080.0].into()),
            free_look_behavior: PressBehavior::Toggle,
            auto_walk_behavior: PressBehavior::Toggle,
            stop_auto_walk_on_input: true,
            map_zoom: 4.0,
        }
    }
}

/// `NetworkingSettings` stores server and networking settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkingSettings {
    pub username: String,
    pub servers: Vec<String>,
    pub default_server: usize,
    pub trusted_auth_servers: HashSet<String>,
}

impl Default for NetworkingSettings {
    fn default() -> Self {
        Self {
            username: "Username".to_string(),
            servers: vec!["server.veloren.net".to_string()],
            default_server: 0,
            trusted_auth_servers: ["https://auth.veloren.net"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

/// `Log` stores whether we should create a log file
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Log {
    // Whether to create a log file or not.
    // Default is to create one.
    pub log_to_file: bool,
    // The path on which the logs will be stored
    pub logs_path: PathBuf,
}

impl Default for Log {
    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    fn default() -> Self {
        let proj_dirs = ProjectDirs::from("net", "veloren", "voxygen")
            .expect("System's $HOME directory path not found!");

        // Chooses a path to store the logs by the following order:
        //  - The VOXYGEN_LOGS environment variable
        //  - The ProjectsDirs data local directory
        // This only selects if there isn't already an entry in the settings file
        let logs_path = std::env::var_os("VOXYGEN_LOGS")
            .map(PathBuf::from)
            .unwrap_or(proj_dirs.data_local_dir().join("logs"));

        Self {
            log_to_file: true,
            logs_path,
        }
    }
}

/// `GraphicsSettings` contains settings related to framerate and in-game
/// visuals.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicsSettings {
    pub view_distance: u32,
    pub sprite_render_distance: u32,
    pub figure_lod_render_distance: u32,
    pub max_fps: u32,
    pub fov: u16,
    pub gamma: f32,
    pub render_mode: RenderMode,
    pub window_size: [u16; 2],
    pub fullscreen: bool,
    pub lod_detail: u32,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            view_distance: 10,
            sprite_render_distance: 150,
            figure_lod_render_distance: 250,
            max_fps: 60,
            fov: 50,
            gamma: 1.0,
            render_mode: RenderMode::default(),
            window_size: [1920, 1080],
            fullscreen: false,
            lod_detail: 300,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AudioOutput {
    /// Veloren's audio system wont work on some systems,
    /// so you can use this to disable it, and allow the
    /// game to function
    // If this option is disabled, functions in the rodio
    // library MUST NOT be called.
    Off,
    Automatic,
    Device(String),
}

impl AudioOutput {
    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Off => false,
            _ => true,
        }
    }
}
/// `AudioSettings` controls the volume of different audio subsystems and which
/// device is used.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub max_sfx_channels: usize,

    /// Audio Device that Voxygen will use to play audio.
    pub output: AudioOutput,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            music_volume: 0.4,
            sfx_volume: 0.6,
            max_sfx_channels: 10,
            output: AudioOutput::Automatic,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LanguageSettings {
    pub selected_language: String,
}

impl Default for LanguageSettings {
    fn default() -> Self {
        Self {
            selected_language: i18n::REFERENCE_LANG.to_string(),
        }
    }
}

/// `Settings` contains everything that can be configured in the settings.ron
/// file.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub controls: ControlSettings,
    pub gameplay: GameplaySettings,
    pub networking: NetworkingSettings,
    pub log: Log,
    pub graphics: GraphicsSettings,
    pub audio: AudioSettings,
    pub show_disclaimer: bool,
    pub send_logon_commands: bool,
    // TODO: Remove at a later date, for dev testing
    pub logon_commands: Vec<String>,
    pub language: LanguageSettings,
    pub screenshots_path: PathBuf,
    pub controller: GamepadSettings,
}

impl Default for Settings {
    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587

    fn default() -> Self {
        let user_dirs = UserDirs::new().expect("System's $HOME directory path not found!");

        // Chooses a path to store the screenshots by the following order:
        //  - The VOXYGEN_SCREENSHOT environment variable
        //  - The user's picture directory
        //  - The executable's directory
        // This only selects if there isn't already an entry in the settings file
        let screenshots_path = std::env::var_os("VOXYGEN_SCREENSHOT")
            .map(PathBuf::from)
            .or(user_dirs.picture_dir().map(|dir| dir.join("veloren")))
            .or(std::env::current_exe()
                .ok()
                .and_then(|dir| dir.parent().map(PathBuf::from)))
            .expect("Couldn't choose a place to store the screenshots");

        Settings {
            controls: ControlSettings::default(),
            gameplay: GameplaySettings::default(),
            networking: NetworkingSettings::default(),
            log: Log::default(),
            graphics: GraphicsSettings::default(),
            audio: AudioSettings::default(),
            show_disclaimer: true,
            send_logon_commands: false,
            logon_commands: Vec::new(),
            language: LanguageSettings::default(),
            screenshots_path,
            controller: GamepadSettings::default(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = Settings::get_settings_path();

        if let Ok(file) = fs::File::open(&path) {
            match ron::de::from_reader(file) {
                Ok(s) => return s,
                Err(e) => {
                    warn!(?e, "Failed to parse setting file! Fallback to default.");
                    // Rename the corrupted settings file
                    let mut new_path = path.to_owned();
                    new_path.pop();
                    new_path.push("settings.invalid.ron");
                    if let Err(e) = std::fs::rename(path.clone(), new_path.clone()) {
                        warn!(?e, ?path, ?new_path, "Failed to rename settings file.");
                    }
                },
            }
        }
        // This is reached if either:
        // - The file can't be opened (presumably it doesn't exist)
        // - Or there was an error parsing the file
        let default_settings = Self::default();
        default_settings.save_to_file_warn();
        default_settings
    }

    pub fn save_to_file_warn(&self) {
        if let Err(e) = self.save_to_file() {
            warn!(?e, "Failed to save settings");
        }
    }

    pub fn save_to_file(&self) -> std::io::Result<()> {
        let path = Settings::get_settings_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut config_file = fs::File::create(path)?;

        let s: &str = &ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap();
        config_file.write_all(s.as_bytes()).unwrap();
        Ok(())
    }

    pub fn get_settings_path() -> PathBuf {
        if let Some(path) = std::env::var_os("VOXYGEN_CONFIG") {
            let settings = PathBuf::from(path.clone()).join("settings.ron");
            if settings.exists() || settings.parent().map(|x| x.exists()).unwrap_or(false) {
                return settings;
            }
            warn!(?path, "VOXYGEN_CONFIG points to invalid path.");
        }

        let proj_dirs = ProjectDirs::from("net", "veloren", "voxygen")
            .expect("System's $HOME directory path not found!");
        proj_dirs
            .config_dir()
            .join("settings")
            .with_extension("ron")
    }
}
