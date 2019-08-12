use crate::{
    comp::{Ability, Glide, Jump, MoveDir, Ori, PhysicsState, Pos, Roll, Stats, Vel, Wield},
    state::DeltaTime,
    terrain::TerrainMap,
    vol::{ReadVol, Vox},
};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use vek::*;

const HUMANOID_ACCEL: f32 = 70.0;
const HUMANOID_SPEED: f32 = 120.0;
const WIELD_ACCEL: f32 = 60.0;
const WIELD_SPEED: f32 = 100.0;
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

/// This system applies movement inputs
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, TerrainMap>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Ability<MoveDir>>,
        ReadStorage<'a, Stats>,
        WriteStorage<'a, Ability<Jump>>,
        WriteStorage<'a, Ability<Glide>>,
        WriteStorage<'a, Ability<Wield>>,
        WriteStorage<'a, Ability<Roll>>,
        WriteStorage<'a, PhysicsState>,
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
            mut jumps,
            mut glides,
            mut wields,
            mut rolls,
            mut physics_state,
            mut positions,
            mut velocities,
            mut orientations,
        ): Self::SystemData,
    ) {
        for (entity, stats, move_dir, mut physics_state, mut pos, mut vel, mut ori) in (
            &entities,
            &stats,
            move_dirs.maybe(),
            &mut physics_state,
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
            if let Some(move_dir) = move_dir.filter(|m| m.started()) {
                vel.0 += Vec2::broadcast(dt.0)
                    * move_dir.0
                    * match (
                        physics_state.on_ground,
                        glides.get(entity).filter(|g| g.started()).is_some(),
                        rolls.get(entity).filter(|r| r.started()).is_some(),
                        wields.get(entity).filter(|w| w.started()).is_some(),
                    ) {
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
                        (true, false, true, false)
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
                let ori_dir = if glides.get(entity).filter(|g| g.started()).is_some()
                    || rolls.get(entity).filter(|r| r.started()).is_some()
                {
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
                        if physics_state.on_ground { 12.0 } else { 2.0 } * dt.0,
                    );
                }
            }

            // Jump
            if jumps.get(entity).filter(|j| j.started()).is_some() {
                vel.0.z = HUMANOID_JUMP_ACCEL;
                jumps.get_mut(entity).map(|j| j.stop());
            }

            // Glide
            if glides.get(entity).filter(|g| g.started()).is_some()
                && vel.0.magnitude_squared() < GLIDE_SPEED.powf(2.0)
                && vel.0.z < 0.0
            {
                wields.get_mut(entity).map(|w| w.stop());
                let lift = GLIDE_ANTIGRAV + vel.0.z.powf(2.0) * 0.2;
                vel.0.z += dt.0 * lift * Vec2::<f32>::from(vel.0 * 0.15).magnitude().min(1.0);
            }

            // Roll
            if let Some(roll) = rolls.get_mut(entity).filter(|r| r.started()) {
                wields.get_mut(entity).map(|w| w.stop());
                if roll.time() > 0.6 || vel.0.magnitude_squared() < 0.4 {
                    rolls.get_mut(entity).map(|r| r.stop());
                }
            }
        }
    }
}
