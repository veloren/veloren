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
/// `AudioSettings` controls the volume of different audio subsystems and which
/// device is used.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    pub master_volume: f32,
    #[serde(rename = "inactive_master_volume")]
    pub inactive_master_volume_perc: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub ambience_volume: f32,
    pub num_sfx_channels: usize,
    pub num_ui_channels: usize,

    /// Audio Device that Voxygen will use to play audio.
    pub output: AudioOutput,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            inactive_master_volume_perc: 0.5,
            music_volume: 0.3,
            sfx_volume: 0.6,
            ambience_volume: 0.6,
            num_sfx_channels: 60,
            num_ui_channels: 10,
            output: AudioOutput::Automatic,
        }
    }
}
