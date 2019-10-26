use super::phys::GRAVITY;
use crate::{
    comp::{
        ActionState::*, CharacterState, Controller, Mounting, MovementState::*, Ori, PhysicsState,
        Pos, Stats, Vel,
    },
    state::DeltaTime,
    terrain::TerrainGrid,
};
use specs::prelude::*;
use std::time::Duration;
use vek::*;

pub const ROLL_DURATION: Duration = Duration::from_millis(600);

const HUMANOID_ACCEL: f32 = 50.0;
const HUMANOID_SPEED: f32 = 120.0;
const HUMANOID_AIR_ACCEL: f32 = 10.0;
const HUMANOID_AIR_SPEED: f32 = 100.0;
const HUMANOID_WATER_ACCEL: f32 = 70.0;
const HUMANOID_WATER_SPEED: f32 = 120.0;
const HUMANOID_CLIMB_ACCEL: f32 = 5.0;
const ROLL_SPEED: f32 = 13.0;
const GLIDE_ACCEL: f32 = 15.0;
const GLIDE_SPEED: f32 = 45.0;
const BLOCK_ACCEL: f32 = 30.0;
const BLOCK_SPEED: f32 = 75.0;
// Gravity is 9.81 * 4, so this makes gravity equal to .15
const GLIDE_ANTIGRAV: f32 = GRAVITY * 0.96;
const CLIMB_SPEED: f32 = 5.0;

pub const MOVEMENT_THRESHOLD_VEL: f32 = 3.0;

/// This system applies forces and calculates new positions and velocities.
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, TerrainGrid>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Controller>,
        ReadStorage<'a, PhysicsState>,
        WriteStorage<'a, CharacterState>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        ReadStorage<'a, Mounting>,
    );

    fn run(
        &mut self,
        (
            entities,
            _terrain,
            dt,
            stats,
            controllers,
            physics_states,
            mut character_states,
            mut positions,
            mut velocities,
            mut orientations,
            mountings,
        ): Self::SystemData,
    ) {
        // Apply movement inputs
        for (
            _entity,
            stats,
            controller,
            physics,
            mut character,
            mut _pos,
            mut vel,
            mut ori,
            mounting,
        ) in (
            &entities,
            &stats,
            &controllers,
            &physics_states,
            &mut character_states,
            &mut positions,
            &mut velocities,
            &mut orientations,
            mountings.maybe(),
        )
            .join()
        {
            if stats.is_dead {
                continue;
            }

            if mounting.is_some() {
                character.movement = Sit;
                continue;
            }

            let inputs = &controller.inputs;

            if character.movement.is_roll() {
                vel.0 = Vec3::new(0.0, 0.0, vel.0.z)
                    + (vel.0 * Vec3::new(1.0, 1.0, 0.0)
                        + 1.5 * inputs.move_dir.try_normalized().unwrap_or_default())
                    .try_normalized()
                    .unwrap_or_default()
                        * ROLL_SPEED;
            }
            if character.action.is_block() || character.action.is_attack() {
                vel.0 += Vec2::broadcast(dt.0)
                    * inputs.move_dir
                    * match physics.on_ground {
                        true if vel.0.magnitude_squared() < BLOCK_SPEED.powf(2.0) => BLOCK_ACCEL,
                        _ => 0.0,
                    }
            } else {
                // Move player according to move_dir
                vel.0 += Vec2::broadcast(dt.0)
                    * inputs.move_dir
                    * match (physics.on_ground, &character.movement) {
                        (true, Run) if vel.0.magnitude_squared() < HUMANOID_SPEED.powf(2.0) => {
                            HUMANOID_ACCEL
                        }
                        (false, Climb) if vel.0.magnitude_squared() < HUMANOID_SPEED.powf(2.0) => {
                            HUMANOID_CLIMB_ACCEL
                        }
                        (false, Glide) if vel.0.magnitude_squared() < GLIDE_SPEED.powf(2.0) => {
                            GLIDE_ACCEL
                        }
                        (false, Jump)
                            if vel.0.magnitude_squared() < HUMANOID_AIR_SPEED.powf(2.0) =>
                        {
                            HUMANOID_AIR_ACCEL
                        }
                        (false, Swim)
                            if vel.0.magnitude_squared() < HUMANOID_WATER_SPEED.powf(2.0) =>
                        {
                            HUMANOID_WATER_ACCEL
                        }
                        _ => 0.0,
                    };
            }

            // Set direction based on move direction when on the ground
            let ori_dir = if character.action.is_wield()
                || character.action.is_attack()
                || character.action.is_block()
            {
                Vec2::from(inputs.look_dir).normalized()
            } else if let (Climb, Some(wall_dir)) = (character.movement, physics.on_wall) {
                if Vec2::<f32>::from(wall_dir).magnitude_squared() > 0.001 {
                    Vec2::from(wall_dir).normalized()
                } else {
                    Vec2::from(vel.0)
                }
            } else {
                Vec2::from(vel.0)
            };

            if ori_dir.magnitude_squared() > 0.0001
                && (ori.0.normalized() - Vec3::from(ori_dir).normalized()).magnitude_squared()
                    > 0.001
            {
                ori.0 = vek::ops::Slerp::slerp(
                    ori.0,
                    ori_dir.into(),
                    if physics.on_ground { 9.0 } else { 2.0 } * dt.0,
                );
            }

            // Glide
            if character.movement == Glide
                && Vec2::<f32>::from(vel.0).magnitude_squared() < GLIDE_SPEED.powf(2.0)
                && vel.0.z < 0.0
            {
                character.action = Idle;
                let lift = GLIDE_ANTIGRAV + vel.0.z.abs().powf(2.0) * 0.15;
                vel.0.z += dt.0
                    * lift
                    * (Vec2::<f32>::from(vel.0).magnitude() * 0.075)
                        .min(1.0)
                        .max(0.2);
            }

            // Roll
            if let Roll { time_left } = &mut character.movement {
                character.action = Idle;
                if *time_left == Duration::default() || vel.0.magnitude_squared() < 10.0 {
                    character.movement = Run;
                } else {
                    *time_left = time_left
                        .checked_sub(Duration::from_secs_f32(dt.0))
                        .unwrap_or_default();
                }
            }

            // Climb
            if let (true, Some(_wall_dir)) = (
                (inputs.climb | inputs.climb_down) && vel.0.z <= CLIMB_SPEED,
                physics.on_wall,
            ) {
                if inputs.climb_down && !inputs.climb {
                    vel.0 -= dt.0 * vel.0.map(|e| e.abs().powf(1.5) * e.signum() * 6.0);
                } else if inputs.climb && !inputs.climb_down {
                    vel.0.z = (vel.0.z + dt.0 * GRAVITY * 1.25).min(CLIMB_SPEED);
                } else {
                    vel.0.z = vel.0.z + dt.0 * GRAVITY * 1.5;
                    vel.0 = Lerp::lerp(
                        vel.0,
                        Vec3::zero(),
                        30.0 * dt.0 / (1.0 - vel.0.z.min(0.0) * 5.0),
                    );
                }

                character.movement = Climb;
                character.action = Idle;
            } else if let Climb = character.movement {
                character.movement = Jump;
            }

            if physics.on_ground
                && (character.movement == Jump
                    || character.movement == Climb
                    || character.movement == Glide
                    || character.movement == Swim)
            {
                character.movement = Stand;
            }

            if !physics.on_ground
                && (character.movement == Stand
                    || character.movement.is_roll()
                    || character.movement == Run)
            {
                character.movement = Jump;
            }

            if !physics.on_ground && physics.in_fluid {
                character.movement = Swim;
            } else if let Swim = character.movement {
                character.movement = Stand;
            }
        }
    }
}
