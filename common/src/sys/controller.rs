use crate::{
    comp::{
        phys::{ForceUpdate, Ori, Pos, Vel},
        Animation, AnimationInfo, Attacking, Controller, Gliding, HealthSource, Jumping, Stats,
    },
    state::{DeltaTime, Uid},
    terrain::TerrainMap,
    vol::{ReadVol, Vox},
};
use log::warn;
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use vek::*;

const HUMANOID_ACCEL: f32 = 100.0;
const HUMANOID_SPEED: f32 = 500.0;
const HUMANOID_AIR_ACCEL: f32 = 10.0;
const HUMANOID_AIR_SPEED: f32 = 100.0;
const HUMANOID_JUMP_ACCEL: f32 = 16.0;
const GLIDE_ACCEL: f32 = 15.0;
const GLIDE_SPEED: f32 = 45.0;
// Gravity is 9.81 * 4, so this makes gravity equal to .15
const GLIDE_ANTIGRAV: f32 = 9.81 * 3.95;

pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Controller>,
        ReadStorage<'a, Stats>,
        ReadExpect<'a, TerrainMap>,
        ReadStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Jumping>,
        WriteStorage<'a, Attacking>,
        WriteStorage<'a, Gliding>,
        WriteStorage<'a, ForceUpdate>,
    );

    fn run(
        &mut self,
        (
            entities,
            dt,
            controllers,
            stats,
            terrain,
            positions,
            mut velocities,
            mut orientations,
            mut jumps,
            mut attacks,
            mut glides,
            mut force_updates,
        ): Self::SystemData,
    ) {
        for (entity, controller, stats, pos, mut vel, mut ori) in (
            &entities,
            &controllers,
            &stats,
            &positions,
            &mut velocities,
            &mut orientations,
        )
            .join()
        {
            if stats.is_dead {
                continue;
            }

            let on_ground = terrain
                .get((pos.0 - Vec3::unit_z() * 0.1).map(|e| e.floor() as i32))
                .map(|vox| !vox.is_empty())
                .unwrap_or(false)
                && vel.0.z <= 0.0;

            let gliding = controller.glide && vel.0.z < 0.0;
            let move_dir = if controller.move_dir.magnitude() > 1.0 {
                controller.move_dir.normalized()
            } else {
                controller.move_dir
            };

            if on_ground {
                // Move player according to move_dir
                if vel.0.magnitude() < HUMANOID_SPEED {
                    vel.0 += Vec2::broadcast(dt.0) * move_dir * HUMANOID_ACCEL;
                }

                // Jump
                if controller.jump && vel.0.z <= 0.0 {
                    vel.0.z = HUMANOID_JUMP_ACCEL;
                }
            } else if gliding && vel.0.magnitude() < GLIDE_SPEED {
                let anti_grav = GLIDE_ANTIGRAV + vel.0.z.powf(2.0) * 0.2;
                vel.0.z += dt.0 * anti_grav * Vec2::<f32>::from(vel.0 * 0.15).magnitude().min(1.0);
                vel.0 += Vec2::broadcast(dt.0) * move_dir * GLIDE_ACCEL;
            } else if vel.0.magnitude() < HUMANOID_AIR_SPEED {
                vel.0 += Vec2::broadcast(dt.0) * move_dir * HUMANOID_AIR_ACCEL;
            }

            // Set direction based on velocity
            if vel.0.magnitude_squared() != 0.0 {
                ori.0 = vel.0.normalized() * Vec3::new(1.0, 1.0, 0.0);
            }

            // Attack
            if controller.attack && attacks.get(entity).is_none() {
                attacks.insert(entity, Attacking::start());
            }
        }
    }
}
