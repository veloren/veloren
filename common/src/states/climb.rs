use crate::{
    comp::{CharacterState, Climb, EnergySource, InputKind, Ori, StateUpdate},
    consts::GRAVITY,
    event::LocalEvent,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
    util::Dir,
};
use serde::{Deserialize, Serialize};
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub energy_cost: f32,
    pub movement_speed: f32,
}

impl Default for Data {
    fn default() -> Self {
        Data {
            energy_cost: 5.0,
            movement_speed: 5.0,
        }
    }
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        // If no wall is in front of character or we stopped climbing;
        let (wall_dir, climb) = if let (Some(wall_dir), Some(climb), false) = (
            data.physics.on_wall,
            data.inputs.climb,
            data.physics.on_ground,
        ) {
            (wall_dir, climb)
        } else {
            if input_is_pressed(data, InputKind::Jump) {
                // They've climbed atop something, give them a boost
                update
                    .local_events
                    .push_front(LocalEvent::Jump(data.entity, BASE_JUMP_IMPULSE * 0.5));
            }
            update.character = CharacterState::Idle {};
            return update;
        };
        // Move player
        update.vel.0 += Vec2::broadcast(data.dt.0)
            * data.inputs.move_dir
            * if update.vel.0.magnitude_squared() < self.movement_speed.powi(2) {
                self.movement_speed.powi(2)
            } else {
                0.0
            };

        // Expend energy if climbing
        let energy_use = match climb {
            Climb::Up => self.energy_cost as i32,
            Climb::Down => 1,
            Climb::Hold => 1,
        };

        if update
            .energy
            .try_change_by(-energy_use, EnergySource::Climb)
            .is_err()
        {
            update.character = CharacterState::Idle {};
        }

        // Set orientation direction based on wall direction
        if let Some(ori_dir) = Dir::from_unnormalized(Vec2::from(wall_dir).into()) {
            // Smooth orientation
            update.ori = update.ori.slerped_towards(
                Ori::from(ori_dir),
                if data.physics.on_ground { 9.0 } else { 2.0 } * data.dt.0,
            );
        };

        // Apply Vertical Climbing Movement
        match climb {
            Climb::Down => update.vel.0.z += data.dt.0 * (GRAVITY - self.movement_speed.powi(2)),
            Climb::Up => update.vel.0.z += data.dt.0 * (GRAVITY + self.movement_speed.powi(2)),
            Climb::Hold => update.vel.0.z += data.dt.0 * GRAVITY,
        }

        update
    }
}
