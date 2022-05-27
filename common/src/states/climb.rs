use crate::{
    comp::{
        character_state::OutputEvents,
        skills::{ClimbSkill::*, Skill, SKILL_MODIFIERS},
        CharacterState, Climb, InputKind, Ori, StateUpdate,
    },
    consts::GRAVITY,
    event::LocalEvent,
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

        // If no wall is in front of character or we stopped climbing;
        let (wall_dir, climb) = if let (Some(wall_dir), Some(climb), None) = (
            data.physics.on_wall,
            data.inputs.climb,
            data.physics.on_ground,
        ) {
            (wall_dir, climb)
        } else {
            if let Some(impulse) = input_is_pressed(data, InputKind::Jump)
                .then(|| data.body.jump_impulse())
                .flatten()
            {
                // How strong the climb boost is relative to a normal jump
                const CLIMB_BOOST_JUMP_FACTOR: f32 = 0.5;
                // They've climbed atop something, give them a boost
                output_events.emit_local(LocalEvent::Jump(
                    data.entity,
                    CLIMB_BOOST_JUMP_FACTOR * impulse / data.mass.0,
                ));
            };
            update.character = CharacterState::Idle(idle::Data::default());
            return update;
        };
        // Move player
        update.vel.0 += Vec2::broadcast(data.dt.0)
            * data.inputs.move_dir
            * if update.vel.0.magnitude_squared() < self.static_data.movement_speed.powi(2) {
                self.static_data.movement_speed.powi(2)
            } else {
                0.0
            };

        // Expend energy if climbing
        let energy_use = match climb {
            Climb::Up => self.static_data.energy_cost,
            Climb::Down => 1.0,
            Climb::Hold => 1.0,
        };

        if update
            .energy
            .try_change_by(-energy_use * data.dt.0)
            .is_err()
        {
            update.character = CharacterState::Idle(idle::Data::default());
        }

        // Set orientation direction based on wall direction
        if let Some(ori_dir) = Dir::from_unnormalized(Vec2::from(wall_dir).into()) {
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

        // Apply Vertical Climbing Movement
        match climb {
            Climb::Down => {
                update.vel.0.z += data.dt.0 * (GRAVITY - self.static_data.movement_speed.powi(2))
            },
            Climb::Up => {
                update.vel.0.z += data.dt.0 * (GRAVITY + self.static_data.movement_speed.powi(2))
            },
            Climb::Hold => update.vel.0.z += data.dt.0 * GRAVITY,
        }

        update
    }
}
