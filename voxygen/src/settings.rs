use crate::{
    hud::{BarNumbers, CrosshairType, Intro, ShortcutNumbers, XpBar},
    i18n,
    render::{AaMode, CloudMode, FluidMode},
    ui::ScaleMode,
    window::KeyMouse,
};
use directories::{ProjectDirs, UserDirs};
use glutin::{MouseButton, VirtualKeyCode};
use hashbrown::{HashMap, HashSet};
use log::warn;
use serde_derive::{Deserialize, Serialize};
use std::{fs, io::prelude::*, path::PathBuf};

/// `ControlSettings` contains keybindings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlSettings {
    pub primary: KeyMouse,
    pub secondary: KeyMouse,
    pub toggle_cursor: KeyMouse,
    pub escape: KeyMouse,
    pub enter: KeyMouse,
    pub command: KeyMouse,
    pub move_forward: KeyMouse,
    pub move_left: KeyMouse,
    pub move_back: KeyMouse,
    pub move_right: KeyMouse,
    pub jump: KeyMouse,
    pub sit: KeyMouse,
    pub glide: KeyMouse,
    pub climb: KeyMouse,
    pub climb_down: KeyMouse,
    pub wall_leap: KeyMouse,
    pub mount: KeyMouse,
    pub map: KeyMouse,
    pub bag: KeyMouse,
    pub quest_log: KeyMouse,
    pub character_window: KeyMouse,
    pub social: KeyMouse,
    pub spellbook: KeyMouse,
    pub settings: KeyMouse,
    pub help: KeyMouse,
    pub toggle_interface: KeyMouse,
    pub toggle_debug: KeyMouse,
    pub fullscreen: KeyMouse,
    pub screenshot: KeyMouse,
    pub toggle_ingame_ui: KeyMouse,
    pub roll: KeyMouse,
    pub respawn: KeyMouse,
    pub interact: KeyMouse,
    pub toggle_wield: KeyMouse,
    pub charge: KeyMouse,
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

impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            primary: KeyMouse::Mouse(MouseButton::Left),
            secondary: KeyMouse::Mouse(MouseButton::Right),
            toggle_cursor: KeyMouse::Key(VirtualKeyCode::Tab),
            escape: KeyMouse::Key(VirtualKeyCode::Escape),
            enter: KeyMouse::Key(VirtualKeyCode::Return),
            command: KeyMouse::Key(VirtualKeyCode::Slash),
            move_forward: KeyMouse::Key(VirtualKeyCode::W),
            move_left: KeyMouse::Key(VirtualKeyCode::A),
            move_back: KeyMouse::Key(VirtualKeyCode::S),
            move_right: KeyMouse::Key(VirtualKeyCode::D),
            jump: KeyMouse::Key(VirtualKeyCode::Space),
            sit: KeyMouse::Key(VirtualKeyCode::K),
            glide: KeyMouse::Key(VirtualKeyCode::LShift),
            climb: KeyMouse::Key(VirtualKeyCode::Space),
            climb_down: KeyMouse::Key(VirtualKeyCode::LControl),
            wall_leap: MIDDLE_CLICK_KEY,
            mount: KeyMouse::Key(VirtualKeyCode::F),
            map: KeyMouse::Key(VirtualKeyCode::M),
            bag: KeyMouse::Key(VirtualKeyCode::B),
            quest_log: KeyMouse::Key(VirtualKeyCode::L),
            character_window: KeyMouse::Key(VirtualKeyCode::C),
            social: KeyMouse::Key(VirtualKeyCode::O),
            spellbook: KeyMouse::Key(VirtualKeyCode::P),
            settings: KeyMouse::Key(VirtualKeyCode::N),
            help: KeyMouse::Key(VirtualKeyCode::F1),
            toggle_interface: KeyMouse::Key(VirtualKeyCode::F2),
            toggle_debug: KeyMouse::Key(VirtualKeyCode::F3),
            fullscreen: KeyMouse::Key(VirtualKeyCode::F11),
            screenshot: KeyMouse::Key(VirtualKeyCode::F4),
            toggle_ingame_ui: KeyMouse::Key(VirtualKeyCode::F6),
            roll: MIDDLE_CLICK_KEY,
            respawn: KeyMouse::Key(VirtualKeyCode::Space),
            interact: KeyMouse::Mouse(MouseButton::Right),
            toggle_wield: KeyMouse::Key(VirtualKeyCode::T),
            charge: KeyMouse::Key(VirtualKeyCode::Key1),
        }
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
        pub glide: Button,
        pub climb: Button,
        pub climb_down: Button,
        pub wall_leap: Button,
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
        pub charge: Button,
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
                glide: Button::Simple(GilButton::LeftTrigger),
                climb: Button::Simple(GilButton::South),
                climb_down: Button::Simple(GilButton::Unknown),
                wall_leap: Button::Simple(GilButton::Unknown),
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
                charge: Button::Simple(GilButton::Unknown),
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
    pub mouse_y_inversion: bool,
    pub crosshair_transp: f32,
    pub chat_transp: f32,
    pub crosshair_type: CrosshairType,
    pub intro_show: Intro,
    pub xp_bar: XpBar,
    pub shortcut_numbers: ShortcutNumbers,
    pub bar_numbers: BarNumbers,
    pub ui_scale: ScaleMode,
}

impl Default for GameplaySettings {
    fn default() -> Self {
        Self {
            pan_sensitivity: 100,
            zoom_sensitivity: 100,
            zoom_inversion: false,
            mouse_y_inversion: false,
            toggle_debug: false,
            sct: true,
            sct_player_batch: true,
            sct_damage_batch: false,
            crosshair_transp: 0.6,
            chat_transp: 0.4,
            crosshair_type: CrosshairType::Round,
            intro_show: Intro::Show,
            xp_bar: XpBar::OnGain,
            shortcut_numbers: ShortcutNumbers::On,
            bar_numbers: BarNumbers::Off,
            ui_scale: ScaleMode::RelativeToWindow([1920.0, 1080.0].into()),
        }
    }
}

/// `NetworkingSettings` stores server and networking settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkingSettings {
    pub username: String,
    pub password: String,
    pub servers: Vec<String>,
    pub default_server: usize,
    pub trusted_auth_servers: HashSet<String>,
}

impl Default for NetworkingSettings {
    fn default() -> Self {
        Self {
            username: "Username".to_string(),
            password: String::default(),
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
    pub max_fps: u32,
    pub fov: u16,
    pub gamma: f32,
    pub aa_mode: AaMode,
    pub cloud_mode: CloudMode,
    pub fluid_mode: FluidMode,
    pub window_size: [u16; 2],
    pub fullscreen: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            view_distance: 10,
            max_fps: 60,
            fov: 50,
            gamma: 1.0,
            aa_mode: AaMode::Fxaa,
            cloud_mode: CloudMode::Regular,
            fluid_mode: FluidMode::Shiny,
            window_size: [1920, 1080],
            fullscreen: false,
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
    pub audio_device: Option<String>,
    pub audio_on: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            music_volume: 0.4,
            sfx_volume: 0.6,
            max_sfx_channels: 10,
            audio_device: None,
            audio_on: true,
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
                .and_then(|dir| dir.parent().map(|val| PathBuf::from(val))))
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
                    log::warn!("Failed to parse setting file! Fallback to default. {}", e);
                    // Rename the corrupted settings file
                    let mut new_path = path.to_owned();
                    new_path.pop();
                    new_path.push("settings.invalid.ron");
                    if let Err(err) = std::fs::rename(path, new_path) {
                        log::warn!("Failed to rename settings file. {}", err);
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
        if let Err(err) = self.save_to_file() {
            warn!("Failed to save settings: {:?}", err);
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
        if let Some(val) = std::env::var_os("VOXYGEN_CONFIG") {
            let settings = PathBuf::from(val).join("settings.ron");
            if settings.exists() || settings.parent().map(|x| x.exists()).unwrap_or(false) {
                return settings;
            }
            log::warn!("VOXYGEN_CONFIG points to invalid path.");
        }

        let proj_dirs = ProjectDirs::from("net", "veloren", "voxygen")
            .expect("System's $HOME directory path not found!");
        proj_dirs
            .config_dir()
            .join("settings")
            .with_extension("ron")
    }
}
