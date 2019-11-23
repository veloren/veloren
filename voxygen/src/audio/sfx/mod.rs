/// The SfxManager listens for SFX events and plays the sound at the provided position
mod event_mapper;

use crate::audio::AudioFrontend;
use client::Client;
use common::{
    assets,
    comp::{Ori, Pos},
    event::{EventBus, SfxEvent, SfxEventItem},
};
use serde::Deserialize;
use vek::*;

#[derive(Deserialize)]
pub struct SfxTriggerItem {
    pub trigger: SfxEvent,
    pub files: Vec<String>,
    pub threshold: f64,
}

#[derive(Deserialize)]
pub struct SfxTriggers {
    pub items: Vec<SfxTriggerItem>,
}

pub struct SfxMgr {
    triggers: SfxTriggers,
    event_mapper: event_mapper::SfxEventMapper,
}

impl SfxMgr {
    pub fn new() -> Self {
        Self {
            triggers: Self::load_sfx_items(),
            event_mapper: event_mapper::SfxEventMapper::new(),
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

            // Get the SFX config entry for this movement
            let sfx_trigger_item: Option<&SfxTriggerItem> = self
                .triggers
                .items
                .iter()
                .find(|item| item.trigger == event.sfx);

            if sfx_trigger_item.is_some() {
                let item = sfx_trigger_item.expect("Invalid sfx item");

                let sfx_file = match item.files.len() {
                    1 => item
                        .files
                        .last()
                        .expect("Failed to determine sound file for this trigger item."),
                    _ => {
                        let rand_step = rand::random::<usize>() % item.files.len();
                        &item.files[rand_step]
                    }
                };

                audio.play_sound(sfx_file, position);
            }
        }
    }

    fn load_sfx_items() -> SfxTriggers {
        let file = assets::load_file("voxygen.audio.sfx", &["ron"])
            .expect("Failed to load the sfx config file");

        ron::de::from_reader(file).expect("Error parsing sfx manifest")
    }
}
