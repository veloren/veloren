use crate::audio::{fader::Fader, AudioFrontend};
use client::Client;
use common::terrain::BiomeKind;
use rand::seq::{SliceRandom};
use rand::thread_rng;
use ron::de::from_str;
use serde::Deserialize;
use std::time::Instant;
use vek::*;

#[derive(Debug, Deserialize)]
struct SoundtrackCollection {
    tracks: Vec<SoundtrackItem>,
}

#[derive(Debug, Deserialize)]
pub struct SoundtrackItem {
    path: String,
    length: f64,
    biome: Vec<BiomeKind>,
}

#[derive(Debug, Deserialize)]
enum DayCyclePeriod {
    Day,
    Night,
}

pub struct MusicMgr {
    playing: bool,
    soundtrack: SoundtrackCollection,
    began_playing: Instant,
    next_track_change: f64,
    current_channel: usize,
}

impl MusicMgr {
    pub fn new() -> Self {
        Self {
            soundtrack: Self::load_soundtrack_data(),
            playing: false,
            began_playing: Instant::now(),
            next_track_change: 0.0,
            current_channel: 0,
        }
    }

    pub fn maintain(&mut self, audio: &mut AudioFrontend, client: &Client) {
        const TRACK_CROSSOVER_TIME_SECONDS: f64 = 10.0;

        audio.set_listener_pos(&Vec3::zero(), &Vec3::zero());

        // Kick off soundtrack if not yet playing
        if !self.playing {
            self.current_channel = self.play_random_track(audio, client);
            self.playing = true;
        }

        // Check whether the current track will finish soon
        if self.began_playing.elapsed().as_secs_f64()
            > (self.next_track_change - TRACK_CROSSOVER_TIME_SECONDS)
        {
            audio.stop_channel(
                self.current_channel,
                Fader::fade_out(TRACK_CROSSOVER_TIME_SECONDS as f32),
            );

            self.current_channel = self.play_random_track(audio, client);
        }
    }

    fn play_random_track(&mut self, audio: &mut AudioFrontend, client: &Client) -> usize {
        let chunk = client.current_chunk();
        let biome = match chunk {
            Some(chunk) => chunk.meta().biome(),
            None => BiomeKind::Void,
        };

        let mut rng = thread_rng();

        let tracks = self
            .soundtrack
            .tracks
            .iter()
            .filter(|track| track.biome.is_empty() || track.biome.contains(&biome))
            .collect::<Vec<_>>();

        log::warn!("current biome is {}", format!("{:#?}", biome));
        log::warn!("Available Tracks {}", format!("{:#?}", tracks));

        let track = self.soundtrack.tracks.choose(&mut rng).unwrap();

        self.began_playing = Instant::now();
        self.next_track_change = track.length;

        audio.play_music(&track.path)
    }

    fn load_soundtrack_data() -> SoundtrackCollection {
        // slapping it here while the format is in flux
        const CONFIG: &str = "
    (
      tracks: [
        (
          title: \"Field Grazing\",
          path: \"voxygen.audio.soundtrack.field_grazing\",
          length: 154.0,
          biome: [
            Grassland,
            Forest
          ],
        ),
        (
          title: \"Sacred Temple\",
          path: \"voxygen.audio.soundtrack.sacred_temple\",
          length: 75.0,
          biome: [],
        ),
        (
          title: \"Ruination\",
          path: \"voxygen.audio.soundtrack.Ruination\",
          length: 134.0,
          biome: [],
        ),
        (
          title: \"Snowtop Volume\",
          path: \"voxygen.audio.soundtrack.Snowtop_volume\",
          length: 30.0,
          biome: [
            Mountain,
            Snowlands
          ],
        ),
        (
          title: \"Ethereal Bonds\",
          path: \"voxygen.audio.soundtrack.Ethereal_Bonds\",
          length: 59.0,
          biome: [],
        ),
        (
          title: \"Mineral Deposits\",
          path: \"voxygen.audio.soundtrack.Mineral_Deposits\",
          length: 148.0,
          biome: [
            Mountain
          ],
        ),
        (
          title: \"Library Theme\",
          path: \"voxygen.audio.soundtrack.library_theme_with_harpsichord\",
          length: 64.0,
          biome: [],
        ),
        (
          title: \"Fiesta del Pueblo\",
          path: \"voxygen.audio.soundtrack.fiesta_del_pueblo\",
          length: 182.0,
          biome: [],
        ),
      ],
    )";

        let collection: SoundtrackCollection = match from_str(CONFIG) {
            Ok(x) => x,
            Err(e) => {
                println!("Failed to load config: {}", e);

                std::process::exit(1);
            }
        };

        collection
    }
}
