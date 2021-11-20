//! Handles ambient non-positional sounds
use crate::{
    audio::{channel::AmbientChannelTag, AudioFrontend},
    scene::Camera,
};
use client::Client;
use common::{
    assets::{self, AssetExt, AssetHandle},
    vol::ReadVol,
};
use common_state::State;
use serde::Deserialize;
use std::time::Instant;
use tracing::warn;
use vek::*;

#[derive(Debug, Default, Deserialize)]
struct AmbientCollection {
    tracks: Vec<AmbientItem>,
}

/// Configuration for a single music track in the soundtrack
#[derive(Debug, Deserialize)]
pub struct AmbientItem {
    path: String,
    /// Length of the track in seconds
    length: f32,
    /// Specifies which ambient channel to play on
    tag: AmbientChannelTag,
}

pub struct AmbientWindMgr {
    ambience: AssetHandle<AmbientCollection>,
    began_playing: Instant,
    next_track_change: f32,
    volume: f32,
    tree_multiplier: f32,
}

pub struct AmbientRainMgr {
    ambience: AssetHandle<AmbientCollection>,
    began_playing: Instant,
    next_track_change: f32,
    volume: f32,
    rain_intensity: f32,
}

impl Default for AmbientWindMgr {
    fn default() -> Self {
        Self {
            ambience: load_ambience_items(),
            began_playing: Instant::now(),
            next_track_change: 0.0,
            volume: 0.0,
            tree_multiplier: 0.0,
        }
    }
}

impl Default for AmbientRainMgr {
    fn default() -> Self {
        Self {
            ambience: load_ambience_items(),
            began_playing: Instant::now(),
            next_track_change: 0.0,
            volume: 0.0,
            rain_intensity: 0.0,
        }
    }
}

impl AmbientWindMgr {
    /// Checks whether the previous track has completed. If so, sends a
    /// request to play the next (random) track
    pub fn maintain(
        &mut self,
        audio: &mut AudioFrontend,
        state: &State,
        client: &Client,
        camera: &Camera,
    ) {
        if audio.sfx_enabled() && !self.ambience.read().tracks.is_empty() {
            let focus_off = camera.get_focus_pos().map(f32::trunc);
            let cam_pos = camera.dependents().cam_pos + focus_off;

            let (terrain_alt, tree_density) = if let Some(chunk) = client.current_chunk() {
                (chunk.meta().alt(), chunk.meta().tree_density())
            } else {
                (0.0, 0.0)
            };

            // The following code is specifically for wind, as it is the only
            // non-positional ambient sound in the game. Others can be added
            // as seen fit.

            let target_volume = {
                // Wind volume increases with altitude
                let alt_multiplier = (cam_pos.z / 1200.0).abs();

                // Tree density factors into wind volume. The more trees,
                // the lower wind volume. The trees make more of an impact
                // the closer the camera is to the ground.
                self.tree_multiplier = ((1.0 - tree_density)
                    + ((cam_pos.z - terrain_alt).abs() / 150.0).powi(2))
                .min(1.0);

                let mut volume_multiplier = alt_multiplier * self.tree_multiplier;

                // Checks if the camera is underwater to stop ambient sounds
                if state
                    .terrain()
                    .get((cam_pos).map(|e| e.floor() as i32))
                    .map(|b| b.is_liquid())
                    .unwrap_or(false)
                {
                    volume_multiplier *= 0.1;
                }
                // Is the camera roughly under the terrain?
                if cam_pos.z < terrain_alt - 10.0 {
                    volume_multiplier = 0.0;
                }

                volume_multiplier.clamped(0.0, 1.0)
            };

            // Transitions the ambient sounds (more) smoothly
            if audio.get_ambient_channel(AmbientChannelTag::Wind).is_none() {
                audio.new_ambient_channel(AmbientChannelTag::Wind);
            } else {
                self.volume = audio.get_ambient_volume(AmbientChannelTag::Wind);
                audio.set_ambient_volume(
                    AmbientChannelTag::Wind,
                    Lerp::lerp(self.volume, target_volume, 0.01),
                );
            }

            if self.began_playing.elapsed().as_secs_f32() > self.next_track_change {
                let ambience = self.ambience.read();
                let wind_track = &ambience
                    .tracks
                    .iter()
                    .find(|track| track.tag == AmbientChannelTag::Wind);
                self.began_playing = Instant::now();
                if let Some(wind_track) = wind_track {
                    self.next_track_change = wind_track.length;
                    audio.play_ambient(AmbientChannelTag::Wind, &wind_track.path, target_volume);
                }
            }
        }
    }
}

impl AmbientRainMgr {
    /// Checks whether the previous track has completed. If so, sends a
    /// request to play the next (random) track
    pub fn maintain(
        &mut self,
        audio: &mut AudioFrontend,
        state: &State,
        client: &Client,
        camera: &Camera,
    ) {
        if audio.sfx_enabled() && !self.ambience.read().tracks.is_empty() {
            let focus_off = camera.get_focus_pos().map(f32::trunc);
            let cam_pos = camera.dependents().cam_pos + focus_off;

            let terrain_alt = if let Some(chunk) = client.current_chunk() {
                chunk.meta().alt()
            } else {
                0.0
            };

            // multipler at end will have to change depending on how intense rain normally
            // is
            self.rain_intensity = client.current_weather().rain * 5.0;

            let mut volume_multiplier = self.rain_intensity;

            // TODO: make rain diminish with distance above terrain
            let target_volume = {
                // Checks if the camera is underwater to stop ambient sounds
                if state
                    .terrain()
                    .get((cam_pos).map(|e| e.floor() as i32))
                    .map(|b| b.is_liquid())
                    .unwrap_or(false)
                {
                    volume_multiplier *= 0.1;
                }
                // Is the camera roughly under the terrain?
                if cam_pos.z < terrain_alt - 10.0 {
                    volume_multiplier = 0.0;
                }

                volume_multiplier = volume_multiplier.clamped(0.0, 1.0);

                // possibly remove noise
                if volume_multiplier < 0.05 {
                    0.0
                } else {
                    volume_multiplier
                }
            };

            // Transitions the ambient sounds (more) smoothly
            if audio.get_ambient_channel(AmbientChannelTag::Rain).is_none() {
                audio.new_ambient_channel(AmbientChannelTag::Rain);
            } else {
                self.volume = audio.get_ambient_volume(AmbientChannelTag::Rain);
                audio.set_ambient_volume(
                    AmbientChannelTag::Rain,
                    Lerp::lerp(self.volume, target_volume, 0.01),
                );
            }

            if self.began_playing.elapsed().as_secs_f32() > self.next_track_change {
                let ambience = self.ambience.read();
                let rain_track = &ambience
                    .tracks
                    .iter()
                    .find(|track| track.tag == AmbientChannelTag::Rain);
                self.began_playing = Instant::now();
                if let Some(rain_track) = rain_track {
                    self.next_track_change = rain_track.length;
                    audio.play_ambient(AmbientChannelTag::Rain, &rain_track.path, target_volume);
                }
            }
        }
    }
}

fn load_ambience_items() -> AssetHandle<AmbientCollection> {
    AmbientCollection::load_or_insert_with("voxygen.audio.ambient", |error| {
        warn!(
            "Error reading ambience config file, ambience will not be available: {:#?}",
            error
        );
        AmbientCollection::default()
    })
}

impl assets::Asset for AmbientCollection {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}
