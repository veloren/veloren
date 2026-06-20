//! Handles ambient non-positional sounds
use crate::{
    audio::{AudioFrontend, channel::AmbienceChannelTag},
    scene::{Camera, Terrain},
    settings::AudioSettings,
};
use client::Client;
use common::{
    assets::{AssetExt, AssetHandle, Ron},
    terrain::{CoordinateConversions, TerrainChunk, site::SiteKindMeta},
    vol::{ReadVol, RectRasterableVol},
};
use common_state::State;
use serde::Deserialize;
use std::time::{Duration, Instant};
use strum::IntoEnumIterator;
use tracing::warn;
use vek::*;

const RIVER_BLOCK_UPDATE_FREQ: f32 = 1.0 / 3.0;

#[derive(Debug, Default, Deserialize)]
pub struct AmbienceCollection {
    tracks: Vec<AmbienceItem>,
}

#[derive(Debug, Deserialize)]
pub struct AmbienceItem {
    path: String,
    /// Specifies which ambient channel to play on
    tag: AmbienceChannelTag,
    start: usize,
    end: usize,
}

pub struct AmbienceMgr {
    pub ambience: AssetHandle<Ron<AmbienceCollection>>,
    // Some tracked structures to avoid unnecessary repeated calculations and allocations
    cam_pos: Vec3<f32>,
    terrain_alt: f32,
    river_blocks: Vec<Vec3<i32>>,
    river_blocks_last_update: Instant,
    distance_to_closest_river_block: Option<f32>,
    river_strength: f32,
}

impl AmbienceMgr {
    pub fn new(assets: AssetHandle<Ron<AmbienceCollection>>) -> Self {
        Self {
            ambience: assets,
            cam_pos: Vec3::zero(),
            terrain_alt: 0.0,
            river_blocks: Vec::new(),
            river_blocks_last_update: Instant::now(),
            distance_to_closest_river_block: None,
            river_strength: 0.0,
        }
    }

    pub fn maintain(
        &mut self,
        audio: &mut AudioFrontend,
        audio_settings: &AudioSettings,
        state: &State,
        client: &Client,
        camera: &Camera,
        terrain: &Terrain,
    ) {
        if !audio.ambience_enabled() {
            return;
        }

        let ambience_sounds = self.ambience.read();

        self.cam_pos = camera.get_pos_with_focus();

        self.terrain_alt = if let Some(chunk) = client.current_chunk() {
            chunk.meta().alt()
        } else {
            0.0
        };

        // Lowpass if underwater
        if state
            .terrain()
            .get(self.cam_pos.map(|e| e.floor() as i32))
            .map(|b| b.is_liquid())
            .unwrap_or(false)
        {
            audio.set_ambience_master_filter(888);
        } else {
            audio.set_ambience_master_filter(20000);
        }

        // Periodically check nearby chunks for river blocks
        if self.river_blocks_last_update.elapsed()
            > Duration::from_secs_f32(RIVER_BLOCK_UPDATE_FREQ)
        {
            self.get_river_blocks(client, terrain);
            self.river_blocks_last_update = Instant::now();
        }

        let closest_water_block = self.river_blocks.iter().min_by(|a, b| {
            let distance_a = self
                .cam_pos
                .distance(Vec3::new(a.x as f32, a.y as f32, a.z as f32));
            let distance_b = self
                .cam_pos
                .distance(Vec3::new(b.x as f32, b.y as f32, b.z as f32));
            distance_a.total_cmp(&distance_b)
        });
        self.distance_to_closest_river_block = if let Some(block) = closest_water_block {
            Some(Vec3::new(block.x as f32, block.y as f32, block.z as f32).distance(self.cam_pos))
        } else {
            None
        };

        // TODO: The init could be done when the audio context is first created?
        // Iterate through each tag
        for tag in AmbienceChannelTag::iter() {
            // Init: Spawn a channel for each tag
            // TODO: Find a good way to cull unneeded channels
            if let Some(inner) = audio.inner.as_mut()
                && inner.channels.get_ambience_channel(tag).is_none()
            {
                inner.new_ambience_channel(tag);
                let track = ambience_sounds
                    .0
                    .tracks
                    .iter()
                    .find(|track| track.tag == tag);
                if let Some(track) = track {
                    audio.play_ambience_looping(tag, &track.path, track.start, track.end);
                }
            }
            if let Some(inner) = audio.inner.as_mut()
                && let Some(channel) = inner.channels.get_ambience_channel(tag)
            {
                // Maintain: get the correct volume of whatever the tag of the current
                // channel is
                let target_volume = if !audio_settings.rain_ambience_enabled
                    && tag == AmbienceChannelTag::Rain
                {
                    0.0
                } else {
                    let volume = self.get_tag_volume(tag, client);

                    // Is the camera underneath the terrain? Fade out the lower it goes beneath.
                    // Unless, of course, the player is in a cave.
                    if tag != AmbienceChannelTag::Cave {
                        (volume
                            * ((self.cam_pos.z - self.terrain_alt) / 50.0 + 1.0).clamped(0.0, 1.0))
                        .min(tag.get_max_volume())
                    } else {
                        volume.min(tag.get_max_volume())
                    }
                };

                // Fade to the target volume over a short period of time
                channel.fade_to(target_volume, 1.0);
            }
        }
    }

    /// Gets appropriate volume for each tag
    pub fn get_tag_volume(&self, tag: AmbienceChannelTag, client: &Client) -> f32 {
        match tag {
            AmbienceChannelTag::Wind => {
                let tree_density = if let Some(chunk) = client.current_chunk() {
                    chunk.meta().tree_density()
                } else {
                    0.0
                };

                // Wind volume increases with altitude
                let alt_factor = (self.cam_pos.z / 1200.0).abs();

                // Tree density factors into wind volume. The more trees,
                // the lower wind volume. The trees make more of an impact
                // the closer the camera is to the ground.
                let tree_factor = ((1.0 - (tree_density * 0.5))
                    + ((self.cam_pos.z - self.terrain_alt).abs() / 150.0).powi(2))
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
                    * (wind_speed_factor
                        + ((self.cam_pos.z - self.terrain_alt).abs() / 150.0).powi(2)))
                    + (alt_factor * 0.15) * tree_factor
            },
            AmbienceChannelTag::Rain => {
                // Make rain diminish with camera distance above terrain
                let camera_factor = 1.0
                    - ((self.cam_pos.z - self.terrain_alt).abs() / 75.0)
                        .powi(2)
                        .min(1.0);

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
                let tree_density = if let Some(chunk) = client.current_chunk() {
                    chunk.meta().tree_density()
                } else {
                    0.0
                };

                // Tree density factors into leaves volume. The more trees,
                // the higher volume. The trees make more of an impact
                // the closer the camera is to the ground
                let tree_factor = 1.0
                    - (((1.0 - tree_density)
                        + ((self.cam_pos.z - self.terrain_alt - 20.0).abs() / 150.0).powi(2))
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
                // When the camera is roughly above ground, don't play cave sounds
                let camera_factor = (-(self.cam_pos.z - self.terrain_alt) / 100.0).max(0.0);

                if client.current_site() == SiteKindMeta::Cave {
                    camera_factor
                } else {
                    0.0
                }
            },
            AmbienceChannelTag::RiverLoud => {
                if let Some(distance) = self.distance_to_closest_river_block {
                    let hearing_distance = (TerrainChunk::RECT_SIZE.x * 2) as f32;
                    let listener_factor = (hearing_distance - distance) / hearing_distance;
                    // A linear upward slope starting at 0.1 and ending with a volume of 1.2.
                    listener_factor.max(0.0) * (self.river_strength - 0.1) * 1.1
                } else {
                    0.0
                }
            },
            AmbienceChannelTag::RiverQuiet => {
                if let Some(distance) = self.distance_to_closest_river_block {
                    let hearing_distance = (TerrainChunk::RECT_SIZE.x * 2) as f32;
                    let listener_factor = (hearing_distance - distance) / hearing_distance;
                    if self.river_strength < 0.3 {
                        // A parabolic swing starting fast at approx. 0 and easing to 1 at 0.3.
                        listener_factor.max(0.0)
                            * (-11.0 * (self.river_strength - 0.3).powi(2) + 1.0)
                    } else {
                        // A linear slide starting with 1 at 0.3 and hitting 0 at 0.9.
                        listener_factor.max(0.0) * ((-self.river_strength + 0.9) / 0.6)
                    }
                } else {
                    0.0
                }
            },
            _ => 1.0,
        }
    }

    fn get_river_blocks(&mut self, client: &Client, terrain: &Terrain) {
        self.river_blocks.clear();
        let mut river_velocities = Vec::new();
        // Sample a chunk spiral of radius 2 around the player character
        if let Some(chonks) = client.chunks_around(2) {
            // Skip if no river nearby
            if chonks
                .iter()
                .all(|(chonk, _)| !chonk.meta().contains_river())
            {
                self.river_strength = 0.0;
                return;
            }

            for (chonk, chunk_pos) in &chonks {
                if let Some(block_data) = terrain.get(*chunk_pos) {
                    if block_data.blocks_of_interest.water.is_empty() {
                        continue;
                    }
                    let wpos = Vec3::<i32>::from(chunk_pos.cpos_to_wpos());
                    self.river_blocks.extend(
                        block_data
                            .blocks_of_interest
                            .water
                            .iter()
                            .map(|b| wpos + *b),
                    );
                    river_velocities.push(chonk.meta().river_velocity());
                }
            }
        }
        // Calculate the average velocity, counting chunks not containing the river (but
        // still containing water) as zero velocity. This makes the system more
        // resilient to sudden chunk-to-chunk changes in river velocity.
        let avg_river_velocity = if river_velocities.is_empty() {
            Vec3::<f32>::zero()
        } else {
            let total_velocity: Vec3<f32> = river_velocities.iter().copied().sum();
            total_velocity / river_velocities.len() as f32
        };
        self.river_strength = avg_river_velocity.magnitude_squared() * 2.0;
    }
}

impl AmbienceChannelTag {
    pub fn get_max_volume(&self) -> f32 {
        match *self {
            AmbienceChannelTag::Wind => 1.0,
            AmbienceChannelTag::Rain => 0.95,
            AmbienceChannelTag::ThunderRumbling => 1.33,
            AmbienceChannelTag::Leaves => 1.33,
            AmbienceChannelTag::Cave => 1.0,
            AmbienceChannelTag::Thunder => 1.0,
            AmbienceChannelTag::RiverLoud => 1.2,
            AmbienceChannelTag::RiverQuiet => 1.0,
        }
    }
}

pub fn load_ambience_items() -> AssetHandle<Ron<AmbienceCollection>> {
    Ron::load_or_insert_with("voxygen.audio.ambience", |error| {
        warn!(
            "Error reading ambience config file, ambience will not be available: {:#?}",
            error
        );
        Ron(AmbienceCollection::default())
    })
}
