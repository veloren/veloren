use super::utils::*;
use crate::{
    comp::{EcsStateData, StateUpdate},
    states::StateHandler,
};
use std::{collections::VecDeque, time::Duration};
use vek::Vec2;

const BLOCK_ACCEL: f32 = 30.0;
const BLOCK_SPEED: f32 = 75.0;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct State {}

impl StateHandler for State {
    fn new(_ecs_data: &EcsStateData) -> Self { Self {} }

    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
            character: *ecs_data.character,
            local_events: VecDeque::new(),
            server_events: VecDeque::new(),
        };

        update
    }
}
