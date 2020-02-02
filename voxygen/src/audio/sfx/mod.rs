/// The Sfx Manager manages individual sfx event system, listens for
/// SFX events and plays the sound at the requested position, or the current
/// player position
mod event_mapper;

use crate::audio::AudioFrontend;
use client::Client;
use common::{
    assets,
    comp::{Ori, Pos},
    event::{EventBus, SfxEvent, SfxEventItem},
};
use event_mapper::SfxEventMapper;
use hashbrown::HashMap;
use serde::Deserialize;
use specs::WorldExt;
use vek::*;

#[derive(Deserialize)]
pub struct SfxTriggerItem {
    pub files: Vec<String>,
    pub threshold: f64,
}

#[derive(Deserialize)]
pub struct SfxTriggers(HashMap<SfxEvent, SfxTriggerItem>);

impl Default for SfxTriggers {
    fn default() -> Self { Self(HashMap::new()) }
}

impl SfxTriggers {
    pub fn get_trigger(&self, trigger: &SfxEvent) -> Option<&SfxTriggerItem> { self.0.get(trigger) }

    pub fn get_key_value(&self, trigger: &SfxEvent) -> Option<(&SfxEvent, &SfxTriggerItem)> {
        self.0.get_key_value(trigger)
    }
}

pub struct SfxMgr {
    triggers: SfxTriggers,
    event_mapper: SfxEventMapper,
}

impl SfxMgr {
    pub fn new() -> Self {
        Self {
            triggers: Self::load_sfx_items(),
            event_mapper: SfxEventMapper::new(),
        }
    }

    pub fn maintain(&mut self, audio: &mut AudioFrontend, client: &Client) {
        self.event_mapper.maintain(client, &self.triggers);

        let ecs = client.state().ecs();

        let player_position = ecs
            .read_storage::<Pos>()
            .get(client.entity())
            .map_or(Vec3::zero(), |pos| pos.0);

        let player_ori = ecs
            .read_storage::<Ori>()
            .get(client.entity())
            .map_or(Vec3::zero(), |pos| pos.0);

        audio.set_listener_pos(&player_position, &player_ori);

        let events = ecs.read_resource::<EventBus<SfxEventItem>>().recv_all();

        for event in events {
            let position = match event.pos {
                Some(pos) => pos,
                _ => player_position,
            };

            if let Some(item) = self.triggers.get_trigger(&event.sfx) {
                let sfx_file = match item.files.len() {
                    1 => item
                        .files
                        .last()
                        .expect("Failed to determine sound file for this trigger item."),
                    _ => {
                        let rand_step = rand::random::<usize>() % item.files.len();
                        &item.files[rand_step]
                    },
                };

                audio.play_sound(sfx_file, position);
            }
        }
    }

    fn load_sfx_items() -> SfxTriggers {
        let file = assets::load_file("voxygen.audio.sfx", &["ron"])
            .expect("Failed to load the sfx config file");

        match ron::de::from_reader(file) {
            Ok(config) => config,
            Err(e) => {
                log::warn!(
                    "Error parsing sfx config file, sfx will not be available: {}",
                    format!("{:#?}", e)
                );

                SfxTriggers::default()
            },
        }
    }
}
