mod combat;
mod movement;
mod progression;

use common::state::State;

use combat::CombatEventMapper;
use movement::MovementEventMapper;
use progression::ProgressionEventMapper;

use super::SfxTriggers;

pub struct SfxEventMapper {
    progression: ProgressionEventMapper,
    movement: MovementEventMapper,
    combat: CombatEventMapper,
}

impl SfxEventMapper {
    pub fn new() -> Self {
        Self {
            progression: ProgressionEventMapper::new(),
            combat: CombatEventMapper::new(),
            movement: MovementEventMapper::new(),
        }
    }

    pub fn maintain(
        &mut self,
        state: &State,
        player_entity: specs::Entity,
        triggers: &SfxTriggers,
    ) {
        self.progression.maintain(state, player_entity, triggers);
        self.movement.maintain(state, player_entity, triggers);
        self.combat.maintain(state, player_entity, triggers);
    }
}
