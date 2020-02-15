use crate::{
    hud::{BarNumbers, CrosshairType, Intro, ShortcutNumbers, XpBar},
    i18n,
    render::{AaMode, CloudMode, FluidMode},
    ui::ScaleMode,
    window::KeyMouse,
};
use directories::ProjectDirs;
use glutin::{MouseButton, VirtualKeyCode};
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
}

impl Default for NetworkingSettings {
    fn default() -> Self {
        Self {
            username: "Username".to_string(),
            password: String::default(),
            servers: vec!["server.veloren.net".to_string()],
            default_server: 0,
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
}

impl Default for Log {
    fn default() -> Self { Self { log_to_file: true } }
}

/// `GraphicsSettings` contains settings related to framerate and in-game
/// visuals.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicsSettings {
    pub view_distance: u32,
    pub max_fps: u32,
    pub fov: u16,
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
}

impl Default for Settings {
    fn default() -> Self {
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
