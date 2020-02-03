/// event_mapper::progression watches the the current player's level
/// and experience and emits associated SFX
use crate::audio::sfx::SfxTriggers;

use client::Client;
use common::{
    comp::Stats,
    event::{EventBus, SfxEvent, SfxEventItem},
};
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

impl ProgressionEventMapper {
    pub fn new() -> Self {
        Self {
            state: ProgressionState::default(),
        }
    }

    pub fn maintain(&mut self, client: &Client, triggers: &SfxTriggers) {
        let ecs = client.state().ecs();

        // level and exp changes
        let next_state =
            ecs.read_storage::<Stats>()
                .get(client.entity())
                .map_or(self.state.clone(), |stats| ProgressionState {
                    level: stats.level.level(),
                    exp: stats.exp.current(),
                });

        if &self.state != &next_state {
            if let Some(mapped_event) = self.map_event(&next_state) {
                let sfx_trigger_item = triggers.get_trigger(&mapped_event);

                if sfx_trigger_item.is_some() {
                    ecs.read_resource::<EventBus<SfxEventItem>>()
                        .emitter()
                        .emit(SfxEventItem::at_player_position(mapped_event));
                }
            }

            self.state = next_state;
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use common::event::SfxEvent;

    #[test]
    fn no_change_returns_none() {
        let mut mapper = ProgressionEventMapper::new();
        let next_client_state = ProgressionState::default();

        assert_eq!(mapper.map_event(&next_client_state), None);
    }

    #[test]
    fn change_level_returns_levelup() {
        let mut mapper = ProgressionEventMapper::new();
        let next_client_state = ProgressionState { level: 2, exp: 0 };

        assert_eq!(
            mapper.map_event(&next_client_state),
            Some(SfxEvent::LevelUp)
        );
    }

    #[test]
    fn change_exp_returns_expup() {
        let mut mapper = ProgressionEventMapper::new();
        let next_client_state = ProgressionState { level: 1, exp: 100 };

        assert_eq!(
            mapper.map_event(&next_client_state),
            Some(SfxEvent::ExperienceGained)
        );
    }

    #[test]
    fn level_up_and_gained_exp_prioritises_levelup() {
        let mut mapper = ProgressionEventMapper::new();
        let next_client_state = ProgressionState { level: 2, exp: 100 };

        assert_eq!(
            mapper.map_event(&next_client_state),
            Some(SfxEvent::LevelUp)
        );
    }
}
