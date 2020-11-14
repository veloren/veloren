//! Handles ambient non-positional sounds
use crate::{
    audio::{channel::AmbientChannelTag, AudioFrontend},
    scene::Camera,
};
use client::Client;
use common::{assets, state::State, terrain::BlockKind, vol::ReadVol};
use serde::Deserialize;
use std::time::Instant;
use tracing::warn;

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
    tag: AmbientChannelTag,
}

pub struct AmbientMgr {
    soundtrack: AmbientCollection,
    began_playing: Instant,
    next_track_change: f32,
    volume: f32,
    tree_multiplier: f32,
}

impl Default for AmbientMgr {
    fn default() -> Self {
        Self {
            soundtrack: Self::load_soundtrack_items(),
            began_playing: Instant::now(),
            next_track_change: 0.0,
            volume: 0.0,
            tree_multiplier: 0.0,
        }
    }
}

impl AmbientMgr {
    /// Checks whether the previous track has completed. If so, sends a
    /// request to play the next (random) track
    pub fn maintain(
        &mut self,
        audio: &mut AudioFrontend,
        state: &State,
        client: &Client,
        camera: &Camera,
    ) {
        if audio.sfx_enabled() && !self.soundtrack.tracks.is_empty() {
            let focus_off = camera.get_focus_pos().map(f32::trunc);
            let cam_pos = camera.dependents().cam_pos + focus_off;

            let cam_alt = cam_pos.z;
            let terrain_alt = Self::get_current_terrain_alt(client);

            // The following code is specifically for wind, as it is the only
            // non-positional ambient sound in the game. Others can be added
            // as seen fit.

            let alt_multiplier = (cam_alt / 1200.0).abs();

            // Tree density factors into ambient volume. The more trees,
            // the less ambient
            let mut tree_multiplier = self.tree_multiplier;
            let new_tree_multiplier = if (cam_alt - terrain_alt) < 150.0 {
                1.0 - Self::get_current_tree_density(client)
            } else {
                1.0
            };

            // Smooths tree_multiplier transitions between chunks
            if tree_multiplier < new_tree_multiplier {
                tree_multiplier += 0.001;
            } else if tree_multiplier > new_tree_multiplier {
                tree_multiplier -= 0.001;
            }
            self.tree_multiplier = tree_multiplier;

            let mut volume_multiplier = alt_multiplier * self.tree_multiplier;

            // Checks if the camera is underwater to stop ambient sounds
            if state
                .terrain()
                .get((cam_pos).map(|e| e.floor() as i32))
                .map(|b| b.kind())
                .unwrap_or(BlockKind::Air)
                == BlockKind::Water
            {
                volume_multiplier *= 0.1;
            }
            if cam_pos.z < Self::get_current_terrain_alt(client) - 10.0 {
                volume_multiplier = 0.0;
            }

            let target_volume = volume_multiplier.max(0.0).min(1.0);

            // Transitions the ambient sounds (more) smoothly
            self.volume = audio.get_ambient_volume();
            if self.volume < target_volume {
                audio.set_ambient_volume(self.volume + 0.001);
            } else if self.volume > target_volume {
                audio.set_ambient_volume(self.volume - 0.001);
            }

            if self.began_playing.elapsed().as_secs_f32() > self.next_track_change {
                //let game_time = (state.get_time_of_day() as u64 % 86400) as u32;
                //let current_period_of_day = Self::get_current_day_period(game_time);

                let track = &self
                    .soundtrack
                    .tracks
                    .iter()
                    .filter(|track| track.tag == AmbientChannelTag::Wind)
                    .next();

                if let Some(track) = track {
                    self.began_playing = Instant::now();
                    self.next_track_change = track.length;

                    audio.play_ambient(AmbientChannelTag::Wind, &track.path, volume_multiplier);
                }
            }
        }
    }

    fn get_current_terrain_alt(client: &Client) -> f32 {
        if let Some(chunk) = client.current_chunk() {
            chunk.meta().alt()
        } else {
            0.0
        }
    }

    fn get_current_tree_density(client: &Client) -> f32 {
        match client.current_chunk() {
            Some(current_chunk) => current_chunk.meta().tree_density(),
            None => 0.0,
        }
    }

    fn load_soundtrack_items() -> AmbientCollection {
        match assets::load_file("voxygen.audio.ambient", &["ron"]) {
            Ok(file) => match ron::de::from_reader(file) {
                Ok(config) => config,
                Err(error) => {
                    warn!(
                        "Error parsing music config file, music will not be available: {}",
                        format!("{:#?}", error)
                    );

                    AmbientCollection::default()
                },
            },
            Err(error) => {
                warn!(
                    "Error reading music config file, music will not be available: {}",
                    format!("{:#?}", error)
                );

                AmbientCollection::default()
            },
        }
    }
}
