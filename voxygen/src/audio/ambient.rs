//! Handles ambient non-positional sounds
use crate::{
    audio::{channel::AmbientChannelTag, AudioFrontend},
    scene::Camera,
};
use client::Client;
use common::{
    assets::{self, AssetExt, AssetHandle},
    terrain::site::SiteKindMeta,
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
        // Checks if the ambience volume is set to zero or audio is disabled
        // This prevents us from running all the following code unnecessarily
        if !audio.ambience_enabled() {
            return;
        }
        let ambience_volume = audio.get_ambience_volume();
        let ambience = self.ambience.read();
        // Iterate through each tag
        for tag in AmbientChannelTag::iter() {
            // If the conditions warrant creating a channel of that tag
            if AmbientChannelTag::get_tag_volume(tag, client, camera)
                > match tag {
                    AmbientChannelTag::Wind => 0.1,
                    AmbientChannelTag::Rain => 0.1,
                    AmbientChannelTag::Thunder => 0.1,
                    AmbientChannelTag::Leaves => 0.05,
                    AmbientChannelTag::Cave => 0.1,
                }
                && audio.get_ambient_channel(tag).is_none()
            {
                audio.new_ambient_channel(tag);
            }
            // If a channel exists run volume code
            if let Some(channel_index) = audio.get_ambient_channel_index(tag) {
                let channel = &mut audio.ambient_channels[channel_index];

                // Maintain: get the correct multiplier of whatever the tag of the current
                // channel is
                let target_volume = get_target_volume(tag, state, client, camera);
                // Get multiplier of the current channel
                let initial_volume = channel.multiplier;

                // Lerp multiplier of current channel
                // TODO: Make this not framerate dependent
                channel.multiplier = Lerp::lerp(initial_volume, target_volume, 0.02);

                // Update with sfx volume
                channel.set_volume(ambience_volume);

                // If the sound should loop at this point:
                if channel.began_playing.elapsed().as_secs_f32() > channel.next_track_change {
                    let track = ambience.tracks.iter().find(|track| track.tag == tag);
                    // Set the channel's start point to this instant
                    channel.began_playing = Instant::now();
                    if let Some(track) = track {
                        // Set loop duration to the one specified in the ron
                        channel.next_track_change = track.length;
                        // Play the file of the current tag at the current multiplier;
                        let current_multiplier = channel.multiplier;
                        audio.play_ambient(
                            tag,
                            &track.path,
                            Some(current_multiplier * ambience_volume),
                        );
                    }
                };

                // Remove channel if not playing
                if audio.ambient_channels[channel_index].multiplier <= 0.001 {
                    audio.ambient_channels.remove(channel_index);
                };
            }
        }
    }
}

impl AmbientChannelTag {
    pub fn tag_max_volume(tag: AmbientChannelTag) -> f32 {
        match tag {
            AmbientChannelTag::Wind => 1.0,
            AmbientChannelTag::Rain => 0.95,
            AmbientChannelTag::Thunder => 1.33,
            AmbientChannelTag::Leaves => 1.33,
            AmbientChannelTag::Cave => 1.0,
        }
    }

    // Gets appropriate volume for each tag
    pub fn get_tag_volume(tag: AmbientChannelTag, client: &Client, camera: &Camera) -> f32 {
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
                let tree_multiplier = ((1.0 - (tree_density * 0.5))
                    + ((cam_pos.z - terrain_alt).abs() / 150.0).powi(2))
                .min(1.0);

                // Lastly, we of course have to take into account actual wind speed from
                // weathersim
                // Client wind speed is a float approx. -30.0 to 30.0 (polarity depending on
                // direction)
                let wind_speed_multiplier = (client.weather_at_player().wind.magnitude_squared()
                    / 15.0_f32.powi(2))
                .min(1.33);

                (alt_multiplier
                    * tree_multiplier
                    * (wind_speed_multiplier + ((cam_pos.z - terrain_alt).abs() / 150.0).powi(2)))
                    + (alt_multiplier * 0.15) * tree_multiplier
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

                (client.weather_at_player().rain * 3.0) * camera_multiplier
            },
            AmbientChannelTag::Thunder => {
                let rain_intensity = client.weather_at_player().rain * 3.0;

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
                        + ((cam_pos.z - terrain_alt - 20.0).abs() / 150.0).powi(2))
                    .min(1.1));

                // Take into account wind speed too, which amplifies tree noise
                let wind_speed_multiplier = (client.weather_at_player().wind.magnitude_squared()
                    / 20.0_f32.powi(2))
                .min(1.0);

                if tree_multiplier > 0.1 {
                    tree_multiplier * (1.0 + wind_speed_multiplier)
                } else {
                    0.0
                }
            },
            AmbientChannelTag::Cave => {
                let focus_off = camera.get_focus_pos().map(f32::trunc);
                let cam_pos = camera.dependents().cam_pos + focus_off;

                let terrain_alt = if let Some(chunk) = client.current_chunk() {
                    chunk.meta().alt()
                } else {
                    0.0
                };

                // When the camera is roughly above ground, don't play cave sounds
                let camera_multiplier = (-(cam_pos.z - terrain_alt) / 100.0).max(0.0);

                if client.current_site() == SiteKindMeta::Cave {
                    camera_multiplier
                } else {
                    0.0
                }
            },
        }
    }
}

/// Checks various factors to determine the target volume to lerp to
fn get_target_volume(
    tag: AmbientChannelTag,
    state: &State,
    client: &Client,
    camera: &Camera,
) -> f32 {
    let focus_off = camera.get_focus_pos().map(f32::trunc);
    let cam_pos = camera.dependents().cam_pos + focus_off;

    let mut volume_multiplier: f32 = AmbientChannelTag::get_tag_volume(tag, client, camera);

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

    // Is the camera underneath the terrain? Fade out the lower it goes beneath.
    // Unless, of course, the player is in a cave.
    if tag != AmbientChannelTag::Cave {
        (volume_multiplier * ((cam_pos.z - terrain_alt) / 50.0 + 1.0).clamped(0.0, 1.0))
            .min(AmbientChannelTag::tag_max_volume(tag))
    } else {
        volume_multiplier.min(AmbientChannelTag::tag_max_volume(tag))
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
