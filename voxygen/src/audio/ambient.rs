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
use std::time::Instant;
use strum::IntoEnumIterator;
use tracing::warn;
use vek::*;

#[derive(Debug, Default, Deserialize)]
pub struct AmbientCollection {
    tracks: Vec<AmbientItem>,
}

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
        let ambience_volume = audio.get_ambience_volume();
        // Iterate through each tag
        for tag in AmbientChannelTag::iter() {
            // If the conditions warrant creating a channel of that tag
            if self.check_ambience_necessity(tag, client, camera)
                && audio.get_ambient_channel(tag).is_none()
            {
                // Iterate through the supposed number of channels - one for each tag
                for index in 0..AmbientChannelTag::iter().len() {
                    // If index would exceed current number of channels, create a new one with
                    // current tag
                    if index >= audio.ambient_channels.len() {
                        audio.new_ambient_channel(tag);
                        break;
                    }
                }
                // If the conditions don't warrant the creation of a
                // channel with that tag, but a channel with
                // that tag remains nonetheless, run the volume code
            } else if audio.get_ambient_channel(tag).is_some() {
                for index in 0..AmbientChannelTag::iter().len() {
                    // Update with sfx volume
                    audio.ambient_channels[index].set_volume(ambience_volume);
                    // If current channel's tag is not the current tag, move on to next channel
                    if audio.ambient_channels[index].get_tag() == tag {
                        // Maintain: get the correct multiplier of whatever the tag of the current
                        // channel is
                        let target_volume =
                            audio.ambient_channels[index].maintain(state, client, camera);
                        // Get multiplier of the current channel
                        let initial_volume = audio.ambient_channels[index].get_multiplier();

                        // Lerp multiplier of current channel
                        audio.ambient_channels[index].set_multiplier(Lerp::lerp(
                            initial_volume,
                            target_volume,
                            0.03,
                        ));

                        // Set the duration of the loop to whatever the current value is (0.0 by
                        // default)
                        let next_track_change =
                            audio.ambient_channels[index].get_next_track_change();

                        // If the sound should loop at this point:
                        if audio.ambient_channels[index]
                            .get_began_playing()
                            .elapsed()
                            .as_secs_f32()
                            > next_track_change
                        {
                            let ambience = self.ambience.read();
                            let track = ambience.tracks.iter().find(|track| track.tag == tag);
                            // Set the channel's start point at this instant
                            audio.ambient_channels[index].set_began_playing(Instant::now());
                            if let Some(track) = track {
                                // Set loop duration to the one specified in the ron
                                audio.ambient_channels[index].set_next_track_change(track.length);
                                // Play the file of the current tag at the current multiplier
                                let current_multiplier =
                                    audio.ambient_channels[index].get_multiplier();
                                audio.play_ambient(tag, &track.path, current_multiplier);
                            }
                        };

                        // Remove channel if not playing
                        if audio.ambient_channels[index].get_multiplier() == 0.0 {
                            audio.ambient_channels[index].stop();
                            audio.ambient_channels.remove(index);
                        };
                        // Move on to next tag
                        break;
                    } else {
                        // Channel tag and current tag don't match, move on to next channel
                        continue;
                    }
                }
            } else {
                // No need to run code at all, move on to the next tag
                continue;
            }
        }
    }

    fn check_ambience_necessity(
        &mut self,
        tag: AmbientChannelTag,
        client: &Client,
        camera: &Camera,
    ) -> bool {
        match tag {
            AmbientChannelTag::Wind => {
                let focus_off = camera.get_focus_pos().map(f32::trunc);
                let cam_pos = camera.dependents().cam_pos + focus_off;

                let (terrain_alt, tree_density) = if let Some(chunk) = client.current_chunk() {
                    (chunk.meta().alt(), chunk.meta().tree_density())
                } else {
                    (0.0, 0.0)
                };

                let alt_multiplier = (cam_pos.z / 1200.0).abs();

                let tree_multiplier = ((1.0 - tree_density)
                    + ((cam_pos.z - terrain_alt).abs() / 150.0).powi(2))
                .min(1.0);

                return alt_multiplier * tree_multiplier > 0.0;
            },
            AmbientChannelTag::Rain => {
                let focus_off = camera.get_focus_pos().map(f32::trunc);
                let cam_pos = camera.dependents().cam_pos + focus_off;

                let terrain_alt = if let Some(chunk) = client.current_chunk() {
                    chunk.meta().alt()
                } else {
                    0.0
                };
                let camera_multiplier =
                    1.0 - ((cam_pos.z - terrain_alt).abs() / 75.0).powi(2).min(1.0);

                return client.weather_at_player().rain > 0.001 || camera_multiplier > 0.0;
            },
            AmbientChannelTag::Thunder => return client.weather_at_player().rain * 500.0 > 0.7,
            AmbientChannelTag::Leaves => {
                let focus_off = camera.get_focus_pos().map(f32::trunc);
                let cam_pos = camera.dependents().cam_pos + focus_off;

                let (terrain_alt, tree_density) = if let Some(chunk) = client.current_chunk() {
                    (chunk.meta().alt(), chunk.meta().tree_density())
                } else {
                    (0.0, 0.0)
                };
                let tree_multiplier = 1.0
                    - (((1.0 - tree_density)
                        + ((cam_pos.z - terrain_alt + 20.0).abs() / 150.0).powi(2))
                    .min(1.0));

                return tree_multiplier > 0.1;
            },
        }
    }
}

impl AmbientChannel {
    pub fn maintain(&mut self, state: &State, client: &Client, camera: &Camera) -> f32 {
        let tag = self.get_tag();

        let focus_off = camera.get_focus_pos().map(f32::trunc);
        let cam_pos = camera.dependents().cam_pos + focus_off;

        let mut target_volume: f32 = self.get_ambience_volume(tag, client, camera);

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
        // Checks if the camera is underwater to diminish ambient sounds
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

    // Gets appropriate volume for each tag
    fn get_ambience_volume(
        &mut self,
        tag: AmbientChannelTag,
        client: &Client,
        camera: &Camera,
    ) -> f32 {
        match tag {
            AmbientChannelTag::Wind => {
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
                let tree_multiplier = ((1.0 - tree_density)
                    + ((cam_pos.z - terrain_alt).abs() / 150.0).powi(2))
                .min(1.0);

                // Lastly, we of course have to take into account actual wind speed from
                // weathersim
                // Client wind speed is a float approx. -30.0 to 30.0 (polarity depending on
                // direction)
                let wind_speed_multiplier = (client.weather_at_player().wind.magnitude_squared()
                    / 30.0_f32.powi(2))
                .min(1.0);

                return alt_multiplier
                    * tree_multiplier
                    * (wind_speed_multiplier + ((cam_pos.z - terrain_alt).abs() / 150.0).powi(2))
                        .min(1.0);
            },
            AmbientChannelTag::Rain => {
                let focus_off = camera.get_focus_pos().map(f32::trunc);
                let cam_pos = camera.dependents().cam_pos + focus_off;

                let terrain_alt = if let Some(chunk) = client.current_chunk() {
                    chunk.meta().alt()
                } else {
                    0.0
                };
                // Make rain diminish with camera distance above terrain
                let camera_multiplier =
                    1.0 - ((cam_pos.z - terrain_alt).abs() / 75.0).powi(2).min(1.0);

                let rain_intensity = (client.weather_at_player().rain * 500.0) * camera_multiplier;

                return rain_intensity.min(0.9);
            },
            AmbientChannelTag::Thunder => {
                let rain_intensity = client.weather_at_player().rain * 500.0;

                if rain_intensity < 0.7 {
                    0.0
                } else {
                    rain_intensity
                }
            },
            AmbientChannelTag::Leaves => {
                let focus_off = camera.get_focus_pos().map(f32::trunc);
                let cam_pos = camera.dependents().cam_pos + focus_off;

                let (terrain_alt, tree_density) = if let Some(chunk) = client.current_chunk() {
                    (chunk.meta().alt(), chunk.meta().tree_density())
                } else {
                    (0.0, 0.0)
                };

                // Tree density factors into leaves volume. The more trees,
                // the higher volume. The trees make more of an impact
                // the closer the camera is to the ground
                let tree_multiplier = 1.0
                    - (((1.0 - tree_density)
                        + ((cam_pos.z - terrain_alt + 20.0).abs() / 150.0).powi(2))
                    .min(1.0));

                if tree_multiplier > 0.1 {
                    tree_multiplier
                } else {
                    0.0
                }
            },
        }
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
