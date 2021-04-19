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
    pub inactive_master_volume: f32,
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
            inactive_master_volume: 0.5,
            music_volume: 0.4,
            sfx_volume: 0.6,
            max_sfx_channels: 30,
            output: AudioOutput::Automatic,
        }
    }
}
