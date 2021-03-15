use super::utils::*;
use crate::{
    comp::StateUpdate,
    states::behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};

// const BLOCK_ACCEL: f32 = 30.0;
// const BLOCK_SPEED: f32 = 75.0;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(&data, &mut update, 0.4);

        update
    }
}
