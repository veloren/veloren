use crate::audio::AudioFrontend;
use client::Client;
use common::assets;
use rand::{seq::IteratorRandom, thread_rng};
use serde::Deserialize;
use std::time::Instant;

const DAY_START_SECONDS: u32 = 28800; // 8:00
const DAY_END_SECONDS: u32 = 70200; // 19:30

#[derive(Debug, Deserialize)]
struct SoundtrackCollection {
    tracks: Vec<SoundtrackItem>,
}

#[derive(Debug, Deserialize)]
pub struct SoundtrackItem {
    title: String,
    path: String,
    length: f64,
    timing: Option<DayPeriod>,
}

#[derive(Debug, Deserialize, PartialEq)]
enum DayPeriod {
    Day,   // 8:00 AM to 7:30 PM
    Night, // 7:31 PM to 6:59 AM
}

pub struct MusicMgr {
    soundtrack: SoundtrackCollection,
    began_playing: Instant,
    next_track_change: f64,
    last_track: String,
}

impl MusicMgr {
    pub fn new() -> Self {
        Self {
            soundtrack: Self::load_soundtrack_items(),
            began_playing: Instant::now(),
            next_track_change: 0.0,
            last_track: String::from("None"),
        }
    }

    pub fn maintain(&mut self, audio: &mut AudioFrontend, client: &Client) {
        if audio.music_enabled()
            && self.began_playing.elapsed().as_secs_f64() > self.next_track_change
        {
            self.play_random_track(audio, client);
        }
    }

    fn play_random_track(&mut self, audio: &mut AudioFrontend, client: &Client) {
        const SILENCE_BETWEEN_TRACKS_SECONDS: f64 = 45.0;

        let game_time = (client.state().get_time_of_day() as u64 % 86400) as u32;
        let current_period_of_day = Self::get_current_day_period(game_time);
        let mut rng = thread_rng();

        let track = self
            .soundtrack
            .tracks
            .iter()
            .filter(|track| {
                !track.title.eq(&self.last_track)
                    && match &track.timing {
                        Some(period_of_day) => period_of_day == &current_period_of_day,
                        None => true,
                    }
            })
            .choose(&mut rng)
            .expect("Failed to select a random track");

        self.last_track = String::from(&track.title);
        self.began_playing = Instant::now();
        self.next_track_change = track.length + SILENCE_BETWEEN_TRACKS_SECONDS;

        audio.play_exploration_music(&track.path);
    }

    fn get_current_day_period(game_time: u32) -> DayPeriod {
        if game_time > DAY_START_SECONDS && game_time < DAY_END_SECONDS {
            DayPeriod::Day
        } else {
            DayPeriod::Night
        }
    }

    fn load_soundtrack_items() -> SoundtrackCollection {
        let file = assets::load_file("voxygen.audio.soundtrack", &["ron"])
            .expect("Failed to load the soundtrack config file");

        ron::de::from_reader(file).expect("Error parsing soundtrack manifest")
    }
}
