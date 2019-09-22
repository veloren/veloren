use crate::{
    comp::{
        ActionState::*, CharacterState, Controller, MovementState::*, Ori, PhysicsState, Pos,
        Stats, Vel,
    },
    state::DeltaTime,
    terrain::TerrainMap,
};
use specs::{Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use std::time::Duration;
use vek::*;

pub const ROLL_DURATION: Duration = Duration::from_millis(600);

const HUMANOID_ACCEL: f32 = 70.0;
const HUMANOID_SPEED: f32 = 120.0;
const HUMANOID_AIR_ACCEL: f32 = 10.0;
const HUMANOID_AIR_SPEED: f32 = 100.0;
const ROLL_SPEED: f32 = 13.0;
const BLOCK_ACCEL: f32 = 30.0;
const BLOCK_SPEED: f32 = 75.0;
// Glider constants
const MASS: f32 = 10.0;
const LIFT: f32 = 4.0; // This must be less than 3DRAG[2]^(1/3)(DRAG[0]/2)^(2/3) to conserve energy
const DRAG: [f32; 3] = [1.0, 1.5, 10.0]; // Drag coefficients
const ANG_INP: [f32; 2] = [2.0, 3.0]; // Angle changes from user input in a unit time step (pitch and roll)

pub const MOVEMENT_THRESHOLD_VEL: f32 = 3.0;

/// This system applies forces and calculates new positions and velocities.
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
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
            _terrain,
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
        for (stats, controller, physics, mut character, mut _pos, mut vel, mut ori) in (
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
                        (false, Jump)
                            if vel.0.magnitude_squared() < HUMANOID_AIR_SPEED.powf(2.0) =>
                        {
                            HUMANOID_AIR_ACCEL
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
            if let Glide { oriq: q } = &mut character.movement {
                character.action = Idle;
                // --- Calculate forces on the glider and apply the velocity change in this time step
                let rot = q.val(); // Rotation quaternion to change reference frames
                let rot_inv = rot.conjugate(); // The inverse rotation
                let vf = rot_inv * vel.0; // The character's velocity in the stationary reference frame that has the front of the glider aligned with +y
                let lift = Vec3::new(0.0, 0.0, LIFT * vf.y * vf.y.abs()); // Calculate lift force from the forwards-velocity
                let drag = Vec3::from(DRAG) * vf.map(|v| -v * v.abs()); // Quadratic drag along each axis
                let acc = rot * (lift + drag) / MASS; // Acceleration rotated back into the space frame
                vel.0 += dt.0 * acc;
                // --- Handle rotation changes from user input
                let (mx, my) = controller.control_dir.into_tuple();
                let deltatheta = my * ANG_INP[0] * dt.0; // Pitch change in this time step, forward = pitch down
                let deltachi = mx * ANG_INP[1] * dt.0; // Roll change in this time step
                *q *= Quaternion::rotation_3d(deltachi, q.ori()); // Apply roll change
                if deltatheta != 0.0 {
                    let v2 = q.left(); // Axis of rotation for pitch changes
                    *q *= Quaternion::rotation_3d(deltatheta, v2); // Apply pitch change
                }
                ori.0 = q.val() * ori.0; // Update the orientation vector so we are facing the right way when we land
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

            if physics.on_ground && (character.movement == Jump || character.movement.is_glide()) {
                character.movement = Stand;
            }

            if !physics.on_ground
                && (character.movement == Stand
                    || character.movement.is_roll()
                    || character.movement == Run)
            {
                character.movement = Jump;
            }
        }
    }
}
