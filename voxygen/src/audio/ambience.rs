//! Handles ambient non-positional sounds
use crate::{
    audio::{channel::AmbienceChannelTag, AudioFrontend},
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
use strum::IntoEnumIterator;
use tracing::warn;
use vek::*;

#[derive(Debug, Default, Deserialize)]
pub struct AmbienceCollection {
    tracks: Vec<AmbienceItem>,
}

#[derive(Debug, Deserialize)]
pub struct AmbienceItem {
    path: String,
    /// Specifies which ambient channel to play on
    tag: AmbienceChannelTag,
}

pub struct AmbienceMgr {
    pub ambience: AssetHandle<AmbienceCollection>,
}

impl AmbienceMgr {
    pub fn maintain(
        &mut self,
        audio: &mut AudioFrontend,
        state: &State,
        client: &Client,
        camera: &Camera,
    ) {
        if !audio.ambience_enabled() {
            return;
        }

        let ambience_sounds = self.ambience.read();

        let cam_pos = camera.get_pos_with_focus();

        // Lowpass if underwater
        if state
            .terrain()
            .get(cam_pos.map(|e| e.floor() as i32))
            .map(|b| b.is_liquid())
            .unwrap_or(false)
        {
            audio.set_ambience_master_filter(888);
        } else {
            audio.set_ambience_master_filter(20000);
        }

        // TODO: The init could be done when the audio context is first created?
        // Iterate through each tag
        for tag in AmbienceChannelTag::iter() {
            // Init: Spawn a channel for each tag
            // TODO: Find a good way to cull unneeded channels
            if let Some(inner) = audio.inner.as_mut()
                && inner.channels.get_ambience_channel(tag).is_none()
            {
                inner.new_ambience_channel(tag);
                let track = ambience_sounds.tracks.iter().find(|track| track.tag == tag);
                if let Some(track) = track {
                    audio.play_ambience_looping(tag, &track.path);
                }
            }
            if let Some(inner) = audio.inner.as_mut()
                && let Some(channel) = inner.channels.get_ambience_channel(tag)
            {
                // Maintain: get the correct volume of whatever the tag of the current
                // channel is
                let target_volume = get_target_volume(tag, client, camera);

                // Fade to the target volume over a short period of time
                channel.fade_to(target_volume, 1.0);
            }
        }
    }
}

impl AmbienceChannelTag {
    pub fn tag_max_volume(tag: AmbienceChannelTag) -> f32 {
        match tag {
            AmbienceChannelTag::Wind => 1.0,
            AmbienceChannelTag::Rain => 0.95,
            AmbienceChannelTag::ThunderRumbling => 1.33,
            AmbienceChannelTag::Leaves => 1.33,
            AmbienceChannelTag::Cave => 1.0,
            _ => 1.0,
        }
    }

    // Gets appropriate volume for each tag
    pub fn get_tag_volume(tag: AmbienceChannelTag, client: &Client, camera: &Camera) -> f32 {
        match tag {
            AmbienceChannelTag::Wind => {
                let focus_off = camera.get_focus_pos().map(f32::trunc);
                let cam_pos = camera.dependents().cam_pos + focus_off;

                let (terrain_alt, tree_density) = if let Some(chunk) = client.current_chunk() {
                    (chunk.meta().alt(), chunk.meta().tree_density())
                } else {
                    (0.0, 0.0)
                };

                // Wind volume increases with altitude
                let alt_factor = (cam_pos.z / 1200.0).abs();

                // Tree density factors into wind volume. The more trees,
                // the lower wind volume. The trees make more of an impact
                // the closer the camera is to the ground.
                let tree_factor = ((1.0 - (tree_density * 0.5))
                    + ((cam_pos.z - terrain_alt).abs() / 150.0).powi(2))
                .min(1.0);

                // Lastly, we of course have to take into account actual wind speed from
                // weathersim
                // Client wind speed is a float approx. -30.0 to 30.0 (polarity depending on
                // direction)
                let wind_speed_factor = (client.weather_at_player().wind.magnitude_squared()
                    / 15.0_f32.powi(2))
                .min(1.33);

                (alt_factor
                    * tree_factor
                    * (wind_speed_factor + ((cam_pos.z - terrain_alt).abs() / 150.0).powi(2)))
                    + (alt_factor * 0.15) * tree_factor
            },
            AmbienceChannelTag::Rain => {
                let focus_off = camera.get_focus_pos().map(f32::trunc);
                let cam_pos = camera.dependents().cam_pos + focus_off;

                let terrain_alt = if let Some(chunk) = client.current_chunk() {
                    chunk.meta().alt()
                } else {
                    0.0
                };
                // Make rain diminish with camera distance above terrain
                let camera_factor = 1.0 - ((cam_pos.z - terrain_alt).abs() / 75.0).powi(2).min(1.0);

                (client.weather_at_player().rain * 3.0) * camera_factor
            },
            AmbienceChannelTag::ThunderRumbling => {
                let rain_intensity = client.weather_at_player().rain * 3.0;

                if rain_intensity < 0.7 {
                    0.0
                } else {
                    rain_intensity
                }
            },
            AmbienceChannelTag::Leaves => {
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
                let tree_factor = 1.0
                    - (((1.0 - tree_density)
                        + ((cam_pos.z - terrain_alt - 20.0).abs() / 150.0).powi(2))
                    .min(1.1));

                // Take into account wind speed too, which amplifies tree noise
                let wind_speed_factor = (client.weather_at_player().wind.magnitude_squared()
                    / 20.0_f32.powi(2))
                .min(1.0);

                if tree_factor > 0.1 {
                    tree_factor * (1.0 + wind_speed_factor)
                } else {
                    0.0
                }
            },
            AmbienceChannelTag::Cave => {
                let focus_off = camera.get_focus_pos().map(f32::trunc);
                let cam_pos = camera.dependents().cam_pos + focus_off;

                let terrain_alt = if let Some(chunk) = client.current_chunk() {
                    chunk.meta().alt()
                } else {
                    0.0
                };

                // When the camera is roughly above ground, don't play cave sounds
                let camera_factor = (-(cam_pos.z - terrain_alt) / 100.0).max(0.0);

                if client.current_site() == SiteKindMeta::Cave {
                    camera_factor
                } else {
                    0.0
                }
            },
            _ => 1.0,
        }
    }
}

/// Checks various factors to determine the target volume to lerp to
fn get_target_volume(tag: AmbienceChannelTag, client: &Client, camera: &Camera) -> f32 {
    let focus_off = camera.get_focus_pos().map(f32::trunc);
    let cam_pos = camera.dependents().cam_pos + focus_off;

    let volume: f32 = AmbienceChannelTag::get_tag_volume(tag, client, camera);

    let terrain_alt = if let Some(chunk) = client.current_chunk() {
        chunk.meta().alt()
    } else {
        0.0
    };

    // Is the camera underneath the terrain? Fade out the lower it goes beneath.
    // Unless, of course, the player is in a cave.
    if tag != AmbienceChannelTag::Cave {
        (volume * ((cam_pos.z - terrain_alt) / 50.0 + 1.0).clamped(0.0, 1.0))
            .min(AmbienceChannelTag::tag_max_volume(tag))
    } else {
        volume.min(AmbienceChannelTag::tag_max_volume(tag))
    }
}

pub fn load_ambience_items() -> AssetHandle<AmbienceCollection> {
    AmbienceCollection::load_or_insert_with("voxygen.audio.ambience", |error| {
        warn!(
            "Error reading ambience config file, ambience will not be available: {:#?}",
            error
        );
        AmbienceCollection::default()
    })
}

impl assets::Asset for AmbienceCollection {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}
