use crate::{
    comp::{
        ActionState::*, CharacterState, Controller, MovementState::*, Ori, PhysicsState, Pos,
        Stats, Vel,
    },
    state::DeltaTime,
    terrain::TerrainMap,
    vol::{ReadVol, Vox},
};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use std::time::Duration;
use vek::*;

const HUMANOID_ACCEL: f32 = 70.0;
const HUMANOID_SPEED: f32 = 120.0;
const WIELD_ACCEL: f32 = 70.0;
const WIELD_SPEED: f32 = 120.0;
const HUMANOID_AIR_ACCEL: f32 = 10.0;
const HUMANOID_AIR_SPEED: f32 = 100.0;
const ROLL_ACCEL: f32 = 160.0;
const ROLL_SPEED: f32 = 550.0;
const GLIDE_ACCEL: f32 = 15.0;
const GLIDE_SPEED: f32 = 45.0;
// Gravity is 9.81 * 4, so this makes gravity equal to .15
const GLIDE_ANTIGRAV: f32 = 9.81 * 3.95;

pub const MOVEMENT_THRESHOLD_VEL: f32 = 3.0;

/// This system applies forces and calculates new positions and velocities.
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, TerrainMap>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Controller>,
        ReadStorage<'a, PhysicsState>,
        WriteStorage<'a, CharacterState>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
    );

    fn run(
        &mut self,
        (
            entities,
            terrain,
            dt,
            stats,
            controllers,
            physics_states,
            mut character_states,
            mut positions,
            mut velocities,
            mut orientations,
        ): Self::SystemData,
    ) {
        // Apply movement inputs
        for (entity, stats, controller, physics, mut character, mut pos, mut vel, mut ori) in (
            &entities,
            &stats,
            &controllers,
            &physics_states,
            &mut character_states,
            &mut positions,
            &mut velocities,
            &mut orientations,
        )
            .join()
        {
            if stats.is_dead {
                continue;
            }

            // Move player according to move_dir
            vel.0 += Vec2::broadcast(dt.0)
                * controller.move_dir
                * match (physics.on_ground, &character.movement) {
                    (true, Run) if vel.0.magnitude_squared() < HUMANOID_SPEED.powf(2.0) => {
                        HUMANOID_ACCEL
                    }
                    (false, Glide) if vel.0.magnitude_squared() < GLIDE_SPEED.powf(2.0) => {
                        GLIDE_ACCEL
                    }
                    (false, Jump) if vel.0.magnitude_squared() < HUMANOID_AIR_SPEED.powf(2.0) => {
                        HUMANOID_AIR_ACCEL
                    }
                    (true, Roll { .. }) if vel.0.magnitude_squared() < ROLL_SPEED.powf(2.0) => {
                        ROLL_ACCEL
                    }
                    _ => 0.0,
                };

            // Set direction based on move direction when on the ground
            let ori_dir = if character.action.is_wield()
                || character.action.is_attack()
                || character.action.is_block()
            {
                Vec2::from(controller.look_dir).normalized()
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
                    if physics.on_ground { 12.0 } else { 2.0 } * dt.0,
                );
            }

            // Glide
            if character.movement == Glide
                && vel.0.magnitude_squared() < GLIDE_SPEED.powf(2.0)
                && vel.0.z < 0.0
            {
                character.action = Idle;
                let lift = GLIDE_ANTIGRAV + vel.0.z.powf(2.0) * 0.2;
                vel.0.z += dt.0 * lift * Vec2::<f32>::from(vel.0 * 0.15).magnitude().min(1.0);
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

            if physics.on_ground && (character.movement == Jump || character.movement == Glide) {
                character.movement = Stand;
            }

            if !physics.on_ground && (character.movement == Stand || character.movement == Run) {
                character.movement = Jump;
            }
        }
    }
}
