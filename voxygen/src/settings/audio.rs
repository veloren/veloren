use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AudioOutput {
    /// Veloren's audio system wont work on some systems,
    /// so you can use this to disable it, and allow the
    /// game to function
    // If this option is disabled, functions in the rodio
    // library MUST NOT be called.
    Off,
    #[serde(other)]
    Automatic,
}

impl AudioOutput {
    pub fn is_enabled(&self) -> bool { !matches!(self, Self::Off) }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct AudioVolume {
    pub volume: f32,
    pub muted: bool,
}

impl AudioVolume {
    pub fn new(volume: f32, muted: bool) -> Self { Self { volume, muted } }

    pub fn get_checked(&self) -> f32 {
        match self.muted {
            true => 0.0,
            false => self.volume,
        }
    }
}

/// `AudioSettings` controls the volume of different audio subsystems and which
/// device is used.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    pub master_volume: AudioVolume,
    #[serde(rename = "inactive_master_volume")]
    pub inactive_master_volume_perc: AudioVolume,
    pub music_volume: AudioVolume,
    pub sfx_volume: AudioVolume,
    pub ambience_volume: AudioVolume,
    pub num_sfx_channels: usize,
    pub num_ui_channels: usize,
    pub music_spacing: f32,

    /// Audio Device that Voxygen will use to play audio.
    pub output: AudioOutput,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: AudioVolume::new(1.0, false),
            inactive_master_volume_perc: AudioVolume::new(0.5, false),
            music_volume: AudioVolume::new(0.4, false),
            sfx_volume: AudioVolume::new(0.6, false),
            ambience_volume: AudioVolume::new(0.6, false),
            num_sfx_channels: 60,
            num_ui_channels: 10,
            music_spacing: 1.0,
            output: AudioOutput::Automatic,
        }
    }
}
