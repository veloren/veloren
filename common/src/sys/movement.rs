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

const HUMANOID_ACCEL: f32 = 70.0;
const HUMANOID_SPEED: f32 = 120.0;
const HUMANOID_AIR_ACCEL: f32 = 10.0;
const HUMANOID_AIR_SPEED: f32 = 100.0;
const HUMANOID_WATER_ACCEL: f32 = 70.0;
const HUMANOID_WATER_SPEED: f32 = 120.0;
const HUMANOID_CLIMB_ACCEL: f32 = 5.0;
const ROLL_SPEED: f32 = 13.0;
const BLOCK_ACCEL: f32 = 30.0;
const BLOCK_SPEED: f32 = 75.0;
// Glider constants
const MASS: f32 = 10.0;
const LIFT: f32 = 4.0; // This must be less than 3DRAG[2]^(1/3)(DRAG[0]/2)^(2/3) to conserve energy
const DRAG: [f32; 3] = [1.0, 3.5, 10.0]; // Drag coefficients (forwards/back, left/right, up/down)
const ANG_INP: [f32; 2] = [0.75, 1.0]; // Angle changes from user input in a unit time step (pitch and roll)
const ANG_DRAG: f32 = 10.0; // The interpolation factor for angular drag in a time step (will be multiplied by dt)
const ANG_SPRING_K: f32 = 1.1; // "" for the glider tending to return to facing forwards
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

            if character.movement.is_roll() {
                vel.0 = Vec3::new(0.0, 0.0, vel.0.z)
                    + controller
                        .move_dir
                        .try_normalized()
                        .unwrap_or(Vec2::from(vel.0).try_normalized().unwrap_or_default())
                        * ROLL_SPEED
            }
            if character.action.is_block() || character.action.is_attack() {
                vel.0 += Vec2::broadcast(dt.0)
                    * controller.move_dir
                    * match physics.on_ground {
                        true if vel.0.magnitude_squared() < BLOCK_SPEED.powf(2.0) => BLOCK_ACCEL,
                        _ => 0.0,
                    }
            } else {
                // Move player according to move_dir
                vel.0 += Vec2::broadcast(dt.0)
                    * controller.move_dir
                    * match (physics.on_ground, &character.movement) {
                        (true, Run) if vel.0.magnitude_squared() < HUMANOID_SPEED.powf(2.0) => {
                            HUMANOID_ACCEL
                        }
                        (false, Climb) if vel.0.magnitude_squared() < HUMANOID_SPEED.powf(2.0) => {
                            HUMANOID_CLIMB_ACCEL
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
                Vec2::from(controller.look_dir).normalized()
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
                    if physics.on_ground { 12.0 } else if character.movement.is_glide() { 0.0 } else { 2.0 } * dt.0,
                );
            }

            // Glide
            if let Glide { oriq: q, rotq: dq } = &mut character.movement {
                character.action = Idle;
                // --- Calculate forces on the glider and apply the velocity change in this time step
                let mut frame = *q; // Rotation quaternion to change from the body frame to the space frame
                let mut rot = *dq; // Rotation in this time step from angular velocity
                let frame_inv = frame.conjugate();
                let frame_v = Quaternion::rotation_from_to_3d(frame * Vec3::unit_y(), vel.0); // Rotation to the direction of the velocity
                let vf = frame_inv * vel.0; // The character's velocity in the stationary reference frame that has the front of the glider aligned with +y
                let lift = Vec3::new(0.0, 0.0, LIFT * vf.y * vf.y.abs()); // Calculate lift force from the forwards-velocity
                let drag = Vec3::from(DRAG) * vf.map(|v| -v * v.abs()); // Quadratic drag along each axis
                let acc = frame * (lift + drag) / MASS; // Acceleration rotated back into the space frame
                vel.0 += dt.0 * acc;
                // --- Handle rotation changes from user input
                let (mx, my) = controller.control_dir.into_tuple();
                let deltatheta = my * ANG_INP[0] * dt.0; // Pitch change in this time step, forward = positive = pitch down
                let deltachi = mx * ANG_INP[1] * dt.0; // Roll change in this time step, positive = roll right
                rot = Slerp::slerp(rot, Quaternion::identity(), ANG_DRAG * dt.0); // Angular velocity decay
                rot = Quaternion::rotation_3d(deltachi, frame * Vec3::unit_y()) * rot; // Apply roll change
                if deltatheta != 0.0 {
                    rot = Quaternion::rotation_3d(deltatheta, frame * -Vec3::unit_x()) * rot; // Apply pitch change
                }
                frame = Slerp::slerp(frame, frame_v, ANG_SPRING_K * dt.0); // Spring back towards facing forwards
                *dq = rot.normalized();
                frame = rot * frame;
                *q = frame.normalized();
                ori.0 = frame * ori.0; // Update the orientation vector so we are facing the right way when we land
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
                (controller.climb | controller.climb_down) && vel.0.z <= CLIMB_SPEED,
                physics.on_wall,
            ) {
                if controller.climb_down && !controller.climb {
                    vel.0 -= dt.0 * vel.0.map(|e| e.abs().powf(1.5) * e.signum() * 6.0);
                } else if controller.climb && !controller.climb_down {
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
                    || character.movement.is_glide()
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
