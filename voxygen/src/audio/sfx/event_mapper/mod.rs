mod combat;
mod movement;
mod progression;

use common::state::State;

use combat::CombatEventMapper;
use movement::MovementEventMapper;
use progression::ProgressionEventMapper;

use super::SfxTriggers;

trait EventMapper {
    fn maintain(&mut self, state: &State, player_entity: specs::Entity, triggers: &SfxTriggers);
}

pub struct SfxEventMapper {
    mappers: Vec<Box<dyn EventMapper>>,
}

impl SfxEventMapper {
    pub fn new() -> Self {
        Self {
            mappers: vec![
                Box::new(CombatEventMapper::new()),
                Box::new(MovementEventMapper::new()),
                Box::new(ProgressionEventMapper::new()),
            ],
        }
    }

    pub fn maintain(
        &mut self,
        state: &State,
        player_entity: specs::Entity,
        triggers: &SfxTriggers,
    ) {
        for mapper in &mut self.mappers {
            mapper.maintain(state, player_entity, triggers);
        }
    }
}
