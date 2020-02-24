use crate::{
    comp::{CharacterState, EcsStateData, ItemKind::Tool, StateUpdate, ToolData},
    states::StateHandler,
};
use std::{collections::VecDeque, time::Duration};
use vek::Vec3;

const ROLL_SPEED: f32 = 17.0;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct State {
    /// How long the state has until exitting
    remaining_duration: Duration,
}

impl StateHandler for State {
    fn new(ecs_data: &EcsStateData) -> Self {
        let tool_data =
            if let Some(Tool(data)) = ecs_data.stats.equipment.main.as_ref().map(|i| i.kind) {
                data
            } else {
                ToolData::default()
            };
        Self {
            remaining_duration: tool_data.attack_duration(),
        }
    }

    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
            energy: *ecs_data.energy,
            local_events: VecDeque::new(),
            server_events: VecDeque::new(),
        };

        // Update velocity
        update.vel.0 = Vec3::new(0.0, 0.0, update.vel.0.z)
            + (update.vel.0 * Vec3::new(1.0, 1.0, 0.0)
                + 1.5
                    * ecs_data
                        .inputs
                        .move_dir
                        .try_normalized()
                        .unwrap_or_default())
            .try_normalized()
            .unwrap_or_default()
                * ROLL_SPEED;

        if self.remaining_duration == Duration::default() {
            // Roll duration has expired
            update.character = CharacterState::Idle(None);
        } else {
            // Otherwise, tick down remaining_duration
            update.character = CharacterState::Roll(Some(State {
                remaining_duration: self
                    .remaining_duration
                    .checked_sub(Duration::from_secs_f32(ecs_data.dt.0))
                    .unwrap_or_default(),
            }));
        }

        update
    }
}
