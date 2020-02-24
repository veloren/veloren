use super::utils::*;
use crate::{
    comp::{EcsStateData, ItemKind::Tool, StateUpdate, ToolData},
    states::StateHandler,
};
use std::{collections::VecDeque, time::Duration};

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct State {
    /// How long before a new action can be performed
    /// after equipping
    pub equip_delay: Duration,
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
            equip_delay: tool_data.equip_time(),
        }
    }

    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
            local_events: VecDeque::new(),
            server_events: VecDeque::new(),
        };

        handle_move_dir(&ecs_data, &mut update);
        handle_jump(&ecs_data, &mut update);
        handle_sit(&ecs_data, &mut update);
        handle_climb(&ecs_data, &mut update);
        handle_glide(&ecs_data, &mut update);
        handle_unwield(&ecs_data, &mut update);
        handle_primary(&ecs_data, &mut update);
        handle_secondary(&ecs_data, &mut update);
        handle_dodge(&ecs_data, &mut update);

        update
    }
}
