use super::phys::GRAVITY;
use crate::{
    comp::{
        ActionState, CharacterState, Controller, Energy, EnergySource, Mounting, MovementState::*,
        Ori, PhysicsState, Pos, Stats, Vel,
    },
    event::{EventBus, ServerEvent},
    state::DeltaTime,
    sync::Uid,
    terrain::TerrainGrid,
};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use std::time::Duration;
use vek::*;

pub const ROLL_DURATION: Duration = Duration::from_millis(600);

const BASE_HUMANOID_ACCEL: f32 = 100.0;
const BASE_HUMANOID_SPEED: f32 = 120.0;
const BASE_HUMANOID_AIR_ACCEL: f32 = 15.0;
const BASE_HUMANOID_AIR_SPEED: f32 = 100.0;
const BASE_HUMANOID_WATER_ACCEL: f32 = 70.0;
const BASE_HUMANOID_WATER_SPEED: f32 = 120.0;
const BASE_HUMANOID_CLIMB_ACCEL: f32 = 10.0;
const ROLL_SPEED: f32 = 17.0;
const CHARGE_SPEED: f32 = 20.0;
const GLIDE_ACCEL: f32 = 15.0;
const GLIDE_SPEED: f32 = 45.0;
const BLOCK_ACCEL: f32 = 30.0;
const BLOCK_SPEED: f32 = 75.0;
// Gravity is 9.81 * 4, so this makes gravity equal to .15
const GLIDE_ANTIGRAV: f32 = GRAVITY * 0.96;
const CLIMB_SPEED: f32 = 5.0;
const CLIMB_COST: i32 = 5;

pub const MOVEMENT_THRESHOLD_VEL: f32 = 3.0;

/// # Movement System
/// #### Applies forces, calculates new positions and velocities,7
/// #### based on Controller(Inputs) and CharacterState.
/// ----
///
/// **Writes:**
/// Pos, Vel, Ori
///
/// **Reads:**
/// Uid, Stats, Controller, PhysicsState, CharacterState, Mounting
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, TerrainGrid>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, DeltaTime>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Energy>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Controller>,
        ReadStorage<'a, PhysicsState>,
        ReadStorage<'a, CharacterState>,
        ReadStorage<'a, Mounting>,
    );

    fn run(
        &mut self,
        (
            entities,
            _terrain,
            _server_bus,
            dt,
            mut positions,
            mut velocities,
            mut orientations,
            mut energies,
            uids,
            stats,
            controllers,
            physics_states,
            character_states,
            mountings,
        ): Self::SystemData,
    ) {
        // Apply movement inputs
        for (
            _entity,
            mut _pos,
            mut vel,
            mut ori,
            mut energy,
            _uid,
            stats,
            controller,
            physics,
            character,
            mount,
        ) in (
            &entities,
            &mut positions,
            &mut velocities,
            &mut orientations,
            &mut energies.restrict_mut(),
            &uids,
            &stats,
            &controllers,
            &physics_states,
            &character_states,
            mountings.maybe(),
        )
            .join()
        {
            if stats.is_dead {
                continue;
            }

            if mount.is_some() {
                continue;
            }

            let inputs = &controller.inputs;

            if character.action.is_roll() {
                vel.0 = Vec3::new(0.0, 0.0, vel.0.z)
                    + (vel.0 * Vec3::new(1.0, 1.0, 0.0)
                        + 1.5 * inputs.move_dir.try_normalized().unwrap_or_default())
                    .try_normalized()
                    .unwrap_or_default()
                        * ROLL_SPEED;
            } else if character.action.is_charge() {
                vel.0 = Vec3::new(0.0, 0.0, vel.0.z)
                    + (vel.0 * Vec3::new(1.0, 1.0, 0.0)
                        + 1.5 * inputs.move_dir.try_normalized().unwrap_or_default())
                    .try_normalized()
                    .unwrap_or_default()
                        * CHARGE_SPEED;
            } else if character.action.is_block() {
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
                        (true, Run)
                            if vel.0.magnitude_squared()
                                < (BASE_HUMANOID_SPEED + stats.fitness as f32 * 50.0).powf(2.0) =>
                        {
                            BASE_HUMANOID_ACCEL
                        },
                        (false, Climb)
                            if vel.0.magnitude_squared() < BASE_HUMANOID_SPEED.powf(2.0) =>
                        {
                            BASE_HUMANOID_CLIMB_ACCEL
                        },
                        (false, Glide) if vel.0.magnitude_squared() < GLIDE_SPEED.powf(2.0) => {
                            GLIDE_ACCEL
                        },
                        (false, Fall) | (false, Jump)
                            if vel.0.magnitude_squared()
                                < (BASE_HUMANOID_AIR_SPEED + stats.fitness as f32 * 10.0)
                                    .powf(2.0) =>
                        {
                            BASE_HUMANOID_AIR_ACCEL
                        },
                        (false, Swim)
                            if vel.0.magnitude_squared()
                                < (BASE_HUMANOID_WATER_SPEED + stats.fitness as f32 * 30.0)
                                    .powf(2.0) =>
                        {
                            BASE_HUMANOID_WATER_ACCEL + stats.fitness as f32 * 10.0
                        },
                        _ => 0.0,
                    };
            }

            // Set direction based on move direction when on the ground
            let ori_dir = if
            //character.action.is_wield() ||
            character.action.is_attack() || character.action.is_block() {
                Vec2::from(inputs.look_dir).normalized()
            } else if let (Climb, Some(wall_dir)) = (character.movement, physics.on_wall) {
                if Vec2::<f32>::from(wall_dir).magnitude_squared() > 0.001 {
                    Vec2::from(wall_dir).normalized()
                } else {
                    Vec2::from(inputs.move_dir)
                }
            } else if let Glide = character.movement {
                // Note: non-gliding forces will also affect velocity and thus orientation
                // producing potentially unexpected changes in direction
                Vec2::from(vel.0)
            } else {
                if let ActionState::Roll { .. } = character.action {
                    // So can can't spin/slip around while rolling
                    Vec2::from(vel.0)
                } else {
                    Vec2::from(inputs.move_dir)
                }
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
                let lift = GLIDE_ANTIGRAV + vel.0.z.abs().powf(2.0) * 0.15;
                vel.0.z += dt.0
                    * lift
                    * (Vec2::<f32>::from(vel.0).magnitude() * 0.075)
                        .min(1.0)
                        .max(0.2);
            }

            // Climb
            if let (true, Some(_wall_dir)) = (
                character.movement == Climb && vel.0.z <= CLIMB_SPEED,
                physics.on_wall,
            ) {
                if inputs.climb_down.is_pressed() && !inputs.climb.is_pressed() {
                    if energy
                        .get_mut_unchecked()
                        .try_change_by(-CLIMB_COST, EnergySource::Climb)
                        .is_ok()
                    {
                        vel.0 -= dt.0 * vel.0.map(|e| e.abs().powf(1.5) * e.signum() * 6.0);
                    }
                } else if inputs.climb.is_pressed() && !inputs.climb_down.is_pressed() {
                    if energy
                        .get_mut_unchecked()
                        .try_change_by(-CLIMB_COST, EnergySource::Climb)
                        .is_ok()
                    {
                        vel.0.z = (vel.0.z + dt.0 * GRAVITY * 1.25).min(CLIMB_SPEED).max(0.0);
                    }
                } else {
                    vel.0.z = (vel.0.z - dt.0 * GRAVITY * 0.01).min(CLIMB_SPEED);
                }
            }

            if character.movement == Swim && inputs.jump.is_pressed() {
                vel.0.z = (vel.0.z + dt.0 * GRAVITY * 1.25).min(BASE_HUMANOID_WATER_SPEED);
            }
        }
    }
}
