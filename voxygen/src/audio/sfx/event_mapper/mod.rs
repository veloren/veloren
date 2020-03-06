mod movement;
mod progression;

use common::state::State;

use movement::MovementEventMapper;
use progression::ProgressionEventMapper;

use super::SfxTriggers;

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

    pub fn maintain(
        &mut self,
        state: &State,
        player_entity: specs::Entity,
        triggers: &SfxTriggers,
    ) {
        self.progression_event_mapper
            .maintain(state, player_entity, triggers);
        self.movement_event_mapper
            .maintain(state, player_entity, triggers);
    }
}
