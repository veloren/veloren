use crate::audio::{fader::Fader, AudioFrontend};
use client::Client;
use rand::seq::SliceRandom;
use rand::thread_rng;
use ron::de::from_str;
use serde::Deserialize;
use std::time::Instant;
use vek::*;

#[derive(Debug, Deserialize)]
struct SoundtrackCollection {
    tracks: Vec<SoundtrackItem>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SoundtrackItem {
    title: String,
    path: String,
    length: f64,
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

    pub fn maintain(&mut self, audio: &mut AudioFrontend, _client: &Client) {
        const TRACK_CROSSOVER_TIME_SECONDS: f64 = 10.0;

        audio.set_listener_pos(&Vec3::zero(), &Vec3::zero());

        // Kick off soundtrack if not yet playing
        if !self.playing {
            self.current_channel = self.play_random_track(audio);
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

            self.current_channel = self.play_random_track(audio);
        }
    }

    fn play_random_track(&mut self, audio: &mut AudioFrontend) -> usize {
        let mut rng = thread_rng();
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
          length: 154.0
        ),
        (
          title: \"Sacred Temple\",
          path: \"voxygen.audio.soundtrack.sacred_temple\",
          length: 75.0
        ),
        (
          title: \"Ruination\",
          path: \"voxygen.audio.soundtrack.Ruination\",
          length: 134.0
        ),
        (
          title: \"Snowtop Volume\",
          path: \"voxygen.audio.soundtrack.Snowtop_volume\",
          length: 30.0
        ),
        (
          title: \"Ethereal Bonds\",
          path: \"voxygen.audio.soundtrack.Ethereal_Bonds\",
          length: 59.0
        ),
        (
          title: \"Mineral Deposits\",
          path: \"voxygen.audio.soundtrack.Mineral_Deposits\",
          length: 148.0
        ),
        (
          title: \"Library Theme\",
          path: \"voxygen.audio.soundtrack.library_theme_with_harpsichord\",
          length: 64.0
        ),
        (
          title: \"Fiesta del Pueblo\",
          path: \"voxygen.audio.soundtrack.fiesta_del_pueblo\",
          length: 182.0
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
