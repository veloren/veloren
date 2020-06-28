/// EventMapper::Progress watches the player entity's stats
/// and triggers sfx for gaining experience and levelling up
use super::EventMapper;

use crate::audio::sfx::{SfxEvent, SfxEventItem, SfxTriggers};

use common::{comp::Stats, event::EventBus, state::State};
use specs::WorldExt;

#[derive(Clone, PartialEq)]
struct ProgressionState {
    level: u32,
    exp: u32,
}

impl Default for ProgressionState {
    fn default() -> Self { Self { level: 1, exp: 0 } }
}

pub struct ProgressionEventMapper {
    state: ProgressionState,
}

impl EventMapper for ProgressionEventMapper {
    #[allow(clippy::op_ref)] // TODO: Pending review in #587
    fn maintain(&mut self, state: &State, player_entity: specs::Entity, triggers: &SfxTriggers) {
        let ecs = state.ecs();

        let next_state = ecs.read_storage::<Stats>().get(player_entity).map_or(
            ProgressionState::default(),
            |stats| ProgressionState {
                level: stats.level.level(),
                exp: stats.exp.current(),
            },
        );

        if &self.state != &next_state {
            if let Some(mapped_event) = self.map_event(&next_state) {
                let sfx_trigger_item = triggers.get_trigger(&mapped_event);

                if sfx_trigger_item.is_some() {
                    ecs.read_resource::<EventBus<SfxEventItem>>()
                        .emit_now(SfxEventItem::at_player_position(mapped_event));
                }
            }

            self.state = next_state;
        }
    }
}

impl ProgressionEventMapper {
    pub fn new() -> Self {
        Self {
            state: ProgressionState::default(),
        }
    }

    #[allow(clippy::let_and_return)] // TODO: Pending review in #587
    fn map_event(&mut self, next_state: &ProgressionState) -> Option<SfxEvent> {
        let sfx_event = if next_state.level > self.state.level {
            Some(SfxEvent::LevelUp)
        } else if next_state.exp > self.state.exp {
            Some(SfxEvent::ExperienceGained)
        } else {
            None
        };

        sfx_event
    }
}

#[cfg(test)] mod tests;
