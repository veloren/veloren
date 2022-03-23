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
            // check if current conditions necessitate the current tag at all
            let should_create: bool = match tag {
                AmbientChannelTag::Wind => self.check_wind_necessity(client, camera),
                AmbientChannelTag::Rain => self.check_rain_necessity(client),
                AmbientChannelTag::Thunder => self.check_thunder_necessity(client),
                AmbientChannelTag::Leaves => self.check_leaves_necessity(client, camera),
            };
            // if the conditions warrant creating a channel of that tag
            if should_create && audio.get_ambient_channel(tag).is_none() {
                // iterate through the supposed number of channels - one for each tag
                for index in 0..AmbientChannelTag::iter().len() {
                    // if index would exceed current number of channels, create a new one with
                    // current tag
                    if index >= audio.ambient_channels.len() {
                        audio.new_ambient_channel(tag);
                        break;
                    }
                }
                // even if the conditions don't warrant the creation of a
                // channel with that tag, but a channel with
                // that tag remains nonetheless, run the code
            } else if audio.get_ambient_channel(tag).is_some() {
                for index in 0..AmbientChannelTag::iter().len() {
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
                        let next_track_change =
                            audio.ambient_channels[index].get_next_track_change();

                        // if the sound should loop at this point:
                        if audio.ambient_channels[index]
                            .get_began_playing()
                            .elapsed()
                            .as_secs_f32()
                            > next_track_change
                        {
                            let ambience = self.ambience.read();
                            let track = ambience.tracks.iter().find(|track| track.tag == tag);
                            // set the channel's start point at this instant
                            audio.ambient_channels[index].set_began_playing(Instant::now());
                            if let Some(track) = track {
                                // set loop duration to the one specified in the ron
                                audio.ambient_channels[index].set_next_track_change(track.length);
                                // play the file of the current tag at the current multiplier
                                let current_multiplier =
                                    audio.ambient_channels[index].get_multiplier();
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
            } else {
                // no need to run code at all, move on to the next tag
                continue;
            }
        }
    }

    fn check_wind_necessity(&mut self, client: &Client, camera: &Camera) -> bool {
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

        return alt_multiplier * tree_multiplier > 0.0;
    }

    fn check_rain_necessity(&mut self, client: &Client) -> bool {
        client.current_weather().rain * 500.0 > 0.0
    }

    fn check_thunder_necessity(&mut self, client: &Client) -> bool {
        client.current_weather().rain * 500.0 > 0.7
    }

    fn check_leaves_necessity(&mut self, client: &Client, camera: &Camera) -> bool {
        let focus_off = camera.get_focus_pos().map(f32::trunc);
        let cam_pos = camera.dependents().cam_pos + focus_off;

        let (terrain_alt, tree_density) = if let Some(chunk) = client.current_chunk() {
            (chunk.meta().alt(), chunk.meta().tree_density())
        } else {
            (0.0, 0.0)
        };

        // Tree density factors into wind volume. The more trees,
        // the lower wind volume. The trees make more of an impact
        // the closer the camera is to the ground.
        let tree_multiplier =
            1.0 - (((1.0 - tree_density) + ((cam_pos.z - terrain_alt + 20.0).abs() / 150.0).powi(2)).min(1.0));

        return tree_multiplier > 0.1;
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
            // Get target volume of rain
            AmbientChannelTag::Rain => self.get_rain_volume(client),
            // Get target volume of thunder
            AmbientChannelTag::Thunder => self.get_thunder_volume(client),
            // Get target volume of leaves
            AmbientChannelTag::Leaves => self.get_leaves_volume(client, camera),
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
        // Float from around -30.0 to 30.0
        let client_wind_speed_sq = client.current_weather().wind.magnitude_squared();

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

        // Lastly, we of course have to take into account actual wind speed from
        // weathersim
        let wind_speed_multiplier = (client_wind_speed_sq / 30.0_f32.powi(2)).min(1.0);

        return alt_multiplier
            * tree_multiplier
            * (wind_speed_multiplier + ((cam_pos.z - terrain_alt).abs() / 150.0).powi(2)).min(1.0);
    }

    fn get_rain_volume(&mut self, client: &Client) -> f32 {
        // multipler at end will have to change depending on how intense rain normally
        // is
        let rain_intensity = client.current_weather().rain * 500.0;

        return rain_intensity;
    }

    fn get_thunder_volume(&mut self, client: &Client) -> f32 {
        let thunder_intensity = client.current_weather().rain * 500.0;

        if thunder_intensity < 0.7 {
            0.0
        } else {
            thunder_intensity
        }
    }

    fn get_leaves_volume(&mut self, client: &Client, camera: &Camera) -> f32 {
        let focus_off = camera.get_focus_pos().map(f32::trunc);
        let cam_pos = camera.dependents().cam_pos + focus_off;

        let (terrain_alt, tree_density) = if let Some(chunk) = client.current_chunk() {
            (chunk.meta().alt(), chunk.meta().tree_density())
        } else {
            (0.0, 0.0)
        };

        // Tree density factors into wind volume. The more trees,
        // the lower wind volume. The trees make more of an impact
        // the closer the camera is to the ground.
        let tree_multiplier =
            1.0 - (((1.0 - tree_density) + ((cam_pos.z - terrain_alt + 20.0).abs() / 150.0).powi(2)).min(1.0));

        if tree_multiplier > 0.1 {
            tree_multiplier
        } else {
            0.0
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
