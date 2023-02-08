use serde::{Serialize, Deserialize};
use vek::Vec3;

use crate::{comp::{character_state::OutputEvents, StateUpdate, CharacterState}, util::Dir};

use super::{behavior::{CharacterBehavior, JoinData}, utils::{handle_orientation, end_ability, handle_wield}, idle};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    pub mount_pos: Vec3<f32>,
    pub mount_dir: Vec3<f32>,
    /// Position sprite is located at
    pub sprite_pos: Vec3<i32>,
}


#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, _output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.pos.0 = self.static_data.mount_pos;

        handle_orientation(data, &mut update, 1.0, Some(Dir::new(self.static_data.mount_dir)));

        handle_wield(data, &mut update);

        // Try to Fall/Stand up/Move
        if data.physics.on_ground.is_none() || data.inputs.move_dir.magnitude_squared() > 0.0 {
            update.character = CharacterState::Idle(idle::Data::default());
        }

        update
    }

    fn stand(&self, data: &JoinData, _output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        end_ability(data, &mut update);

        update
    }
}