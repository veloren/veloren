pub mod movement;
pub mod progression;

use movement::MovementEventMapper;
use progression::ProgressionEventMapper;

use super::SfxTriggers;
use client::Client;

pub struct SfxEventMapper {
    progression_event_mapper: ProgressionEventMapper,
    movement_event_mapper: MovementEventMapper,
}

impl SfxEventMapper {
    pub fn new() -> Self {
        Self {
            progression_event_mapper: ProgressionEventMapper::new(),
            movement_event_mapper: MovementEventMapper::new(),
        }
    }

    pub fn maintain(&mut self, client: &Client, triggers: &SfxTriggers) {
        self.progression_event_mapper.maintain(client, triggers);
        self.movement_event_mapper.maintain(client, triggers);
    }
}
