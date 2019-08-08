use crate::{
    hud::{BarNumbers, CrosshairType, ShortcutNumbers, XpBar},
    render::AaMode,
    ui::ScaleMode,
    window::DigitalInput,
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
    pub primary: DigitalInput,
    pub secondary: DigitalInput,
    pub toggle_cursor: DigitalInput,
    pub escape: DigitalInput,
    pub enter: DigitalInput,
    pub command: DigitalInput,
    pub move_forward: DigitalInput,
    pub move_left: DigitalInput,
    pub move_back: DigitalInput,
    pub move_right: DigitalInput,
    pub jump: DigitalInput,
    pub sit: DigitalInput,
    pub glide: DigitalInput,
    pub climb: DigitalInput,
    pub climb_down: DigitalInput,
    pub wall_leap: DigitalInput,
    pub mount: DigitalInput,
    pub map: DigitalInput,
    pub bag: DigitalInput,
    pub quest_log: DigitalInput,
    pub character_window: DigitalInput,
    pub social: DigitalInput,
    pub spellbook: DigitalInput,
    pub settings: DigitalInput,
    pub help: DigitalInput,
    pub toggle_interface: DigitalInput,
    pub toggle_debug: DigitalInput,
    pub fullscreen: DigitalInput,
    pub screenshot: DigitalInput,
    pub toggle_ingame_ui: DigitalInput,
    pub roll: DigitalInput,
    pub respawn: DigitalInput,
    pub interact: DigitalInput,
}

impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            primary: DigitalInput::Mouse(MouseButton::Left),
            secondary: DigitalInput::Mouse(MouseButton::Right),
            toggle_cursor: DigitalInput::Key(VirtualKeyCode::Tab),
            escape: DigitalInput::Key(VirtualKeyCode::Escape),
            enter: DigitalInput::Key(VirtualKeyCode::Return),
            command: DigitalInput::Key(VirtualKeyCode::Slash),
            move_forward: DigitalInput::Key(VirtualKeyCode::W),
            move_left: DigitalInput::Key(VirtualKeyCode::A),
            move_back: DigitalInput::Key(VirtualKeyCode::S),
            move_right: DigitalInput::Key(VirtualKeyCode::D),
            jump: DigitalInput::Key(VirtualKeyCode::Space),
            sit: DigitalInput::Key(VirtualKeyCode::K),
            glide: DigitalInput::Key(VirtualKeyCode::LShift),
            climb: DigitalInput::Key(VirtualKeyCode::LShift),
            climb_down: DigitalInput::Key(VirtualKeyCode::LControl),
            wall_leap: DigitalInput::Mouse(MouseButton::Middle),
            mount: DigitalInput::Key(VirtualKeyCode::F),
            map: DigitalInput::Key(VirtualKeyCode::M),
            bag: DigitalInput::Key(VirtualKeyCode::B),
            quest_log: DigitalInput::Key(VirtualKeyCode::L),
            character_window: DigitalInput::Key(VirtualKeyCode::C),
            social: DigitalInput::Key(VirtualKeyCode::O),
            spellbook: DigitalInput::Key(VirtualKeyCode::P),
            settings: DigitalInput::Key(VirtualKeyCode::N),
            help: DigitalInput::Key(VirtualKeyCode::F1),
            toggle_interface: DigitalInput::Key(VirtualKeyCode::F2),
            toggle_debug: DigitalInput::Key(VirtualKeyCode::F3),
            fullscreen: DigitalInput::Key(VirtualKeyCode::F11),
            screenshot: DigitalInput::Key(VirtualKeyCode::F4),
            toggle_ingame_ui: DigitalInput::Key(VirtualKeyCode::F6),
            roll: DigitalInput::Mouse(MouseButton::Middle),
            respawn: DigitalInput::Mouse(MouseButton::Left),
            interact: DigitalInput::Key(VirtualKeyCode::E),
        }
    }
}

/// `GameplaySettings` contains sensitivity and gameplay options.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GameplaySettings {
    pub pan_sensitivity: u32,
    pub zoom_sensitivity: u32,
    pub crosshair_transp: f32,
    pub crosshair_type: CrosshairType,
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
            crosshair_transp: 0.6,
            crosshair_type: CrosshairType::Round,
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

/// `Log` stores the name to the log file.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Log {
    pub file: PathBuf,
}

impl Default for Log {
    fn default() -> Self {
        Self {
            file: "voxygen.log".into(),
        }
    }
}

/// `GraphicsSettings` contains settings related to framerate and in-game visuals.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicsSettings {
    pub view_distance: u32,
    pub max_fps: u32,
    pub fov: u16,
    pub aa_mode: AaMode,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            view_distance: 5,
            max_fps: 60,
            fov: 75,
            aa_mode: AaMode::Fxaa,
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
            audio_device: None,
            audio_on: true,
        }
    }
}

/// `Settings` contains everything that can be configured in the settings.ron file.
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
                }
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

    fn get_settings_path() -> PathBuf {
        let proj_dirs = ProjectDirs::from("net", "veloren", "voxygen")
            .expect("System's $HOME directory path not found!");
        proj_dirs
            .config_dir()
            .join("settings")
            .with_extension("ron")
    }
}
