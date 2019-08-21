use crate::{
    comp::{ActionState, Jumping, MoveDir, OnGround, Ori, Pos, Rolling, Stats, Vel, Wielding},
    state::DeltaTime,
    terrain::TerrainMap,
    vol::{ReadVol, Vox},
};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use vek::*;

const HUMANOID_ACCEL: f32 = 70.0;
const HUMANOID_SPEED: f32 = 120.0;
const WIELD_ACCEL: f32 = 70.0;
const WIELD_SPEED: f32 = 120.0;
const HUMANOID_AIR_ACCEL: f32 = 10.0;
const HUMANOID_AIR_SPEED: f32 = 100.0;
const HUMANOID_JUMP_ACCEL: f32 = 18.0;
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
        ReadStorage<'a, MoveDir>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, ActionState>,
        WriteStorage<'a, Jumping>,
        WriteStorage<'a, Wielding>,
        WriteStorage<'a, Rolling>,
        WriteStorage<'a, OnGround>,
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
            move_dirs,
            stats,
            action_states,
            mut jumpings,
            mut wieldings,
            mut rollings,
            mut on_grounds,
            mut positions,
            mut velocities,
            mut orientations,
        ): Self::SystemData,
    ) {
        // Apply movement inputs
        for (entity, stats, a, move_dir, mut pos, mut vel, mut ori) in (
            &entities,
            &stats,
            &action_states,
            move_dirs.maybe(),
            &mut positions,
            &mut velocities,
            &mut orientations,
        )
            .join()
        {
            // Disable while dead TODO: Replace with client states?
            if stats.is_dead {
                continue;
            }

            // Move player according to move_dir
            if let Some(move_dir) = move_dir {
                vel.0 += Vec2::broadcast(dt.0)
                    * move_dir.0
                    * match (a.on_ground, a.gliding, a.rolling, a.wielding) {
                        (true, false, false, false)
                            if vel.0.magnitude_squared() < HUMANOID_SPEED.powf(2.0) =>
                        {
                            HUMANOID_ACCEL
                        }
                        (false, true, false, false)
                            if vel.0.magnitude_squared() < GLIDE_SPEED.powf(2.0) =>
                        {
                            GLIDE_ACCEL
                        }
                        (false, false, false, false)
                            if vel.0.magnitude_squared() < HUMANOID_AIR_SPEED.powf(2.0) =>
                        {
                            HUMANOID_AIR_ACCEL
                        }
                        (true, false, true, _)
                            if vel.0.magnitude_squared() < ROLL_SPEED.powf(2.0) =>
                        {
                            ROLL_ACCEL
                        }
                        (true, false, false, true)
                            if vel.0.magnitude_squared() < WIELD_SPEED.powf(2.0) =>
                        {
                            WIELD_ACCEL
                        }
                        _ => 0.0,
                    };

                // Set direction based on move direction when on the ground
                let ori_dir = if a.gliding || a.rolling {
                    Vec2::from(vel.0)
                } else {
                    move_dir.0
                };
                if ori_dir.magnitude_squared() > 0.0001
                    && (ori.0.normalized() - Vec3::from(ori_dir).normalized()).magnitude_squared()
                        > 0.001
                {
                    ori.0 = vek::ops::Slerp::slerp(
                        ori.0,
                        ori_dir.into(),
                        if a.on_ground { 12.0 } else { 2.0 } * dt.0,
                    );
                }
            }

            // Jump
            if jumpings.get(entity).is_some() {
                vel.0.z = HUMANOID_JUMP_ACCEL;
                jumpings.remove(entity);
            }

            // Glide
            if a.gliding && vel.0.magnitude_squared() < GLIDE_SPEED.powf(2.0) && vel.0.z < 0.0 {
                let _ = wieldings.remove(entity);
                let lift = GLIDE_ANTIGRAV + vel.0.z.powf(2.0) * 0.2;
                vel.0.z += dt.0 * lift * Vec2::<f32>::from(vel.0 * 0.15).magnitude().min(1.0);
            }

            // Roll
            if let Some(time) = rollings.get_mut(entity).map(|r| &mut r.time) {
                let _ = wieldings.remove(entity);
                *time += dt.0;
                if *time > 0.6 || !a.moving {
                    rollings.remove(entity);
                }
            }
        }
    }
}
