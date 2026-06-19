use std::{fmt::Debug, ops::Deref};
use tracing::warn;

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

/// A versioned setting field. If the current version does not match the version
/// read, it resets the value to its default automatically and writes it to the
/// file as well. Takes the form `Versioned<[type], [current version]>`.
#[derive(Copy, Clone, Debug, Default)]
pub struct Versioned<T, const CURRENT: usize>(pub T);

impl<T: Serialize, const CURRENT: usize> Serialize for Versioned<T, CURRENT> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (CURRENT, &self.0).serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de> + Default + Debug, const CURRENT: usize> Deserialize<'de>
    for Versioned<T, CURRENT>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let deserialized = <(usize, T)>::deserialize(deserializer)?;
        let old_ver = deserialized.0;
        if old_ver != CURRENT {
            let new = T::default();
            warn!("New default setting detected (ver. {old_ver} -> {CURRENT}), setting to {new:?}",);
            Ok(Versioned(new))
        } else {
            Ok(Versioned(deserialized.1))
        }
    }
}

impl<T, const CURRENT: usize> Deref for Versioned<T, CURRENT> {
    type Target = T;

    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct BufferSize {
    pub samples: usize,
}

impl Default for BufferSize {
    fn default() -> Self { Self { samples: 2048 } }
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
    pub rain_ambience_enabled: bool,
    pub num_sfx_channels: usize,
    pub num_ui_channels: usize,
    pub music_spacing: f32,
    pub subtitles: bool,
    pub combat_music_enabled: bool,
    /// The size of the sample buffer Kira uses. Increasing this may improve
    /// audio performance at the cost of audio latency.
    /// This is a versioned setting, so change the default value above and then
    /// increment the second element to force reset it.
    #[serde(default)]
    pub buffer_size: Versioned<BufferSize, 1>,
    /// Set to None to use the default samplerate determined by the game;
    /// otherwise, use Some(samplerate); the game will attempt to force
    /// samplerate to this.
    pub sample_rate: Option<u32>,

    /// Audio Device that Voxygen will use to play audio.
    pub output: AudioOutput,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: AudioVolume::new(0.8, false),
            inactive_master_volume_perc: AudioVolume::new(0.5, false),
            music_volume: AudioVolume::new(0.5, false),
            sfx_volume: AudioVolume::new(0.8, false),
            ambience_volume: AudioVolume::new(0.8, false),
            num_sfx_channels: 32,
            rain_ambience_enabled: true,
            num_ui_channels: 16,
            music_spacing: 1.0,
            subtitles: false,
            output: AudioOutput::Automatic,
            combat_music_enabled: false,
            buffer_size: Versioned(BufferSize::default()),
            sample_rate: None,
        }
    }
}
