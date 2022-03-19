//! Handles ambient non-positional sounds
use crate::{
    audio::{
        channel::{AmbientChannel, AmbientChannelTag},
        AudioFrontend,
    },
    scene::Camera,
};
use client::Client;
use common::{
    assets::{self, AssetExt, AssetHandle},
    vol::ReadVol,
};
use common_state::State;
use serde::Deserialize;
use strum::IntoEnumIterator;
use std::time::Instant;
use tracing::warn;
use vek::*;

#[derive(Debug, Default, Deserialize)]
pub struct AmbientCollection {
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

pub struct AmbientMgr {
    pub ambience: AssetHandle<AmbientCollection>,
}

impl AmbientMgr {
    pub fn maintain(
        &mut self,
        audio: &mut AudioFrontend,
        state: &State,
        client: &Client,
        camera: &Camera,
    ) {
        let sfx_volume = audio.get_sfx_volume();
        // iterate through each tag
        for tag in AmbientChannelTag::iter() {
            // iterate through the supposed number of channels - one for each tag
            for index in 0..AmbientChannelTag::iter().len() {
                // if index would exceed current number of channels, create a new one with
                // current tag
                if index >= audio.ambient_channels.len() {
                    audio.new_ambient_channel(tag);
                }
                // update with sfx volume
                audio.ambient_channels[index].set_volume(sfx_volume);
                // if current channel's tag is not the current tag, move on to next channel
                if audio.ambient_channels[index].get_tag() == tag {
                    // maintain: get the correct multiplier of whatever the tag of the current
                    // channel is
                    let target_volume =
                        audio.ambient_channels[index].maintain(state, client, camera);
                    // get multiplier of the current channel
                    let initial_volume = audio.ambient_channels[index].get_multiplier();

                    // lerp multiplier of current channel
                    audio.ambient_channels[index].set_multiplier(Lerp::lerp(
                        initial_volume,
                        target_volume,
                        0.01,
                    ));

                    // set the duration of the loop to whatever the current value is (0.0 by
                    // default)
                    let next_track_change = audio.ambient_channels[index].get_next_track_change();

                    // if the sound should loop at this point:
                    if audio.ambient_channels[index]
                        .get_began_playing()
                        .elapsed()
                        .as_secs_f32()
                        > next_track_change
                    {
                        let ambience = self.ambience.read();
                        let track = ambience.tracks.iter().find(|track| track.tag == tag);
                        // set the track's start point at this instant
                        audio.ambient_channels[index].set_began_playing(Instant::now());
                        if let Some(track) = track {
                            // set loop duration to the one specified in the ron
                            audio.ambient_channels[index].set_next_track_change(track.length);
                            // play the file of the current tag at the current multiplier
                            let current_multiplier = audio.ambient_channels[index].get_multiplier();
                            audio.play_ambient(tag, &track.path, current_multiplier);
                        }
                    };

                    // remove channel if not playing
                    if audio.ambient_channels[index].get_multiplier() == 0.0 {
                        audio.ambient_channels[index].stop();
                        audio.ambient_channels.remove(index);
                    };
                    // move on to next tag
                    break;
                } else {
                    // channel tag and current tag don't match, move on to next channel
                    continue;
                }
            }
        }
    }
}

impl AmbientChannel {
    pub fn maintain(&mut self, state: &State, client: &Client, camera: &Camera) -> f32 {
        let tag = self.get_tag();

        let focus_off = camera.get_focus_pos().map(f32::trunc);
        let cam_pos = camera.dependents().cam_pos + focus_off;

        let mut target_volume: f32 = match tag {
            // Get target volume of wind
            AmbientChannelTag::Wind => self.get_wind_volume(client, camera),
            // get target volume of rain
            AmbientChannelTag::Rain => self.get_rain_volume(client),
        };

        // TODO: make rain diminish with distance above terrain
        target_volume = self.check_camera(state, client, cam_pos, target_volume);

        return target_volume;
    }

    fn check_camera(
        &mut self,
        state: &State,
        client: &Client,
        cam_pos: Vec3<f32>,
        initial_volume: f32,
    ) -> f32 {
        let mut volume_multiplier = initial_volume;
        let terrain_alt = if let Some(chunk) = client.current_chunk() {
            chunk.meta().alt()
        } else {
            0.0
        };
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
    }

    fn get_wind_volume(&mut self, client: &Client, camera: &Camera) -> f32 {
        let focus_off = camera.get_focus_pos().map(f32::trunc);
        let cam_pos = camera.dependents().cam_pos + focus_off;

        let (terrain_alt, tree_density) = if let Some(chunk) = client.current_chunk() {
            (chunk.meta().alt(), chunk.meta().tree_density())
        } else {
            (0.0, 0.0)
        };

        // Wind volume increases with altitude
        let alt_multiplier = (cam_pos.z / 1200.0).abs();

        // Tree density factors into wind volume. The more trees,
        // the lower wind volume. The trees make more of an impact
        // the closer the camera is to the ground.
        let tree_multiplier =
            ((1.0 - tree_density) + ((cam_pos.z - terrain_alt).abs() / 150.0).powi(2)).min(1.0);

        return alt_multiplier * tree_multiplier;
    }

    fn get_rain_volume(&mut self, client: &Client) -> f32 {
        // multipler at end will have to change depending on how intense rain normally
        // is
        let rain_intensity = client.current_weather().rain * 5.0;

        return rain_intensity;
    }
}

pub fn load_ambience_items() -> AssetHandle<AmbientCollection> {
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
