use crate::{
    comp::{
        CharacterState, Ori, StateUpdate,
        character_state::OutputEvents,
        skills::{ClimbSkill::*, SKILL_MODIFIERS, Skill},
    },
    consts::GRAVITY,
    states::{
        behavior::{CharacterBehavior, JoinData},
        idle,
        utils::*,
    },
    util::Dir,
};
use serde::{Deserialize, Serialize};
use vek::*;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    pub energy_cost: f32,
    pub movement_speed: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
}

impl Data {
    pub fn create_adjusted_by_skills(join_data: &JoinData) -> Self {
        let modifiers = SKILL_MODIFIERS.general_tree.climb;
        let mut data = Data::default();
        if let Ok(level) = join_data.skill_set.skill_level(Skill::Climb(Cost)) {
            data.static_data.energy_cost *= modifiers.energy_cost.powi(level.into());
        }
        if let Ok(level) = join_data.skill_set.skill_level(Skill::Climb(Speed)) {
            data.static_data.movement_speed *= modifiers.speed.powi(level.into());
        }
        data
    }
}

impl Default for Data {
    fn default() -> Self {
        Data {
            static_data: StaticData {
                energy_cost: 15.0,
                movement_speed: 5.0,
            },
        }
    }
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        let Some(wall_dir) = data.physics.on_wall else {
            update.character = CharacterState::Idle(idle::Data::default());
            return update;
        };

        // Exit climb if ground below
        if data.physics.on_ground.is_some() {
            update.character = CharacterState::Idle(idle::Data::default());
            return update;
        }

        // Positive if walking into wall, negative if away
        let wall_relative_movement =
            data.inputs.move_dir * data.inputs.look_dir.map(|e| e.signum()) * wall_dir;

        // If we move relative to the wall use up energy else default of 1.0
        // (maybe something lower like 0.5)
        let energy_use = if wall_relative_movement.reduce_partial_max() > 0.5 {
            self.static_data.energy_cost
        } else {
            1.0
        };
        handle_walljump(data, output_events, &mut update);

        // Update energy and exit climbing state if not enough
        if update
            .energy
            .try_change_by(-energy_use * data.dt.0)
            .is_err()
        {
            update.character = CharacterState::Idle(idle::Data::default());
        }

        // Set orientation based on wall direction
        if let Some(ori_dir) = Dir::from_unnormalized(wall_dir.with_z(0.0)) {
            // Smooth orientation
            update.ori = update.ori.slerped_towards(
                Ori::from(ori_dir),
                if data.physics.on_ground.is_some() {
                    9.0
                } else {
                    2.0
                } * data.dt.0,
            );
        };

        // By default idle on wall
        update.vel.0.z += data.dt.0 * GRAVITY;

        // Map movement direction onto wall
        let upwards_vel = data.inputs.move_dir.dot(wall_dir.xy());
        let crossed = wall_dir.cross(Vec3::unit_z());
        let lateral_vel = crossed * data.inputs.move_dir.dot(crossed.xy());

        update.vel.0 += data.dt.0
            * (lateral_vel.with_z(upwards_vel) + wall_dir)
            * self.static_data.movement_speed.powi(2)
            * data.scale.map_or(1.0, |s| s.0);

        update
    }
}
