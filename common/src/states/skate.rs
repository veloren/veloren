use super::utils::*;
use crate::{
    comp::{
        character_state::OutputEvents, item::armor::Friction, CharacterState, InventoryAction,
        StateUpdate,
    },
    states::{
        behavior::{CharacterBehavior, JoinData},
        idle,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub(crate) footwear: Friction,
    // hints for animation
    pub turn: f32,       // negative to left, positive to right, 1.0=45°
    pub accelerate: f32, // negative to brake
    pub sidewalk: f32,   // negative to left
}

impl Data {
    pub fn new(_: &JoinData, footwear: Friction) -> Self {
        Self {
            footwear,
            turn: Default::default(),
            accelerate: Default::default(),
            sidewalk: Default::default(),
        }
    }
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_wield(data, &mut update);
        handle_jump(data, output_events, &mut update, 1.0);

        if !data.physics.skating_active {
            update.character = CharacterState::Idle(idle::Data {
                is_sneaking: false,
                time_entered: *data.time,
                footwear: Some(self.footwear),
            });
        } else {
            let plane_ori = data.inputs.look_dir.xy();
            let orthogonal = vek::Vec2::new(plane_ori.y, -plane_ori.x);
            update.ori = vek::Vec3::new(plane_ori.x, plane_ori.y, 0.0).into();
            let current_planar_velocity = data.vel.0.xy().magnitude();
            let long_input = data.inputs.move_dir.dot(plane_ori);
            let lat_input = data.inputs.move_dir.dot(orthogonal);
            let acceleration = if long_input.abs() > lat_input.abs() {
                if long_input > 0.0 {
                    if let CharacterState::Skate(data) = &mut update.character {
                        data.accelerate = 1.0;
                        data.sidewalk = 0.0;
                    }
                    // forward, max at 8u/s
                    (data.dt.0 * 3.0).clamp(0.0, 8.0 - current_planar_velocity)
                } else {
                    if let CharacterState::Skate(data) = &mut update.character {
                        data.accelerate = -1.0;
                        data.sidewalk = 0.0;
                    }
                    //brake up to 4u/s², but never backwards
                    (data.dt.0 * 4.0).min(current_planar_velocity)
                }
            } else {
                if let CharacterState::Skate(data) = &mut update.character {
                    data.accelerate = 0.0;
                    data.sidewalk = lat_input;
                }
                // sideways: constant speed
                (0.5 - current_planar_velocity).max(0.0)
            };
            if let CharacterState::Skate(skate_data) = &mut update.character {
                skate_data.turn = orthogonal.dot(data.vel.0.xy());
            }
            let delta_vel = acceleration * data.inputs.move_dir;
            update.vel.0 += vek::Vec3::new(delta_vel.x, delta_vel.y, 0.0);
        }

        update
    }

    fn manipulate_loadout(
        &self,
        data: &JoinData,
        output_events: &mut OutputEvents,
        inv_action: InventoryAction,
    ) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        handle_manipulate_loadout(data, output_events, &mut update, inv_action);
        update
    }

    fn wield(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_wield(data, &mut update);
        update
    }

    fn sit(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_sit(data, &mut update);
        update
    }

    fn stand(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        // Try to Fall/Stand up/Move
        update.character = CharacterState::Idle(idle::Data::default());
        update
    }
}
