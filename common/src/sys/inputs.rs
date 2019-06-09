use crate::{
    comp::{
        phys::{ForceUpdate, Ori, Pos, Vel},
        Animation, AnimationInfo, Attacking, Gliding, HealthSource, Jumping, MoveDir, OnGround,
        Respawning, Stats,
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

/// This system is responsible for handling accepted inputs like moving or attacking
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Uid>,
        Read<'a, DeltaTime>,
        ReadExpect<'a, TerrainMap>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, OnGround>,
        ReadStorage<'a, MoveDir>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, AnimationInfo>,
        WriteStorage<'a, Stats>,
        WriteStorage<'a, Respawning>,
        WriteStorage<'a, Jumping>,
        WriteStorage<'a, Gliding>,
        WriteStorage<'a, Attacking>,
        WriteStorage<'a, ForceUpdate>,
    );

    fn run(
        &mut self,
        (
            entities,
            uids,
            dt,
            terrain,
            positions,
            on_grounds,
            move_dirs,
            mut velocities,
            mut orientations,
            mut animation_infos,
            mut stats,
            mut respawnings,
            mut jumpings,
            glidings,
            mut attackings,
            mut force_updates,
        ): Self::SystemData,
    ) {
        let finished_attacks: Vec<_> =
            (&entities, &uids, &positions, &orientations, &mut attackings)
                .join()
                .filter_map(|(entity, uid, pos, ori, mut attacking)| {
                    if !attacking.applied {
                        // Go through all other entities
                        for (b, pos_b, stat_b, mut vel_b) in
                            (&entities, &positions, &mut stats, &mut velocities).join()
                        {
                            // Check if it is a hit
                            if entity != b
                                && !stat_b.is_dead
                                && pos.0.distance_squared(pos_b.0) < 50.0
                                && ori.0.angle_between(pos_b.0 - pos.0).to_degrees() < 70.0
                            {
                                // Deal damage
                                stat_b.hp.change_by(-10, HealthSource::Attack { by: *uid }); // TODO: variable damage and weapon
                                vel_b.0 += (pos_b.0 - pos.0).normalized() * 10.0;
                                vel_b.0.z = 15.0;
                                if let Err(err) = force_updates.insert(b, ForceUpdate) {
                                    warn!("Inserting ForceUpdate for an entity failed: {:?}", err);
                                }
                            }
                        }
                        attacking.applied = true;
                    }

                    attacking.time += dt.0;
                    if attacking.time > 0.5 {
                        Some(entity)
                    } else {
                        None
                    }
                })
                .collect();

        // Finish attack
        for entity in finished_attacks {
            attackings.remove(entity);
        }

        for (entity, mut vel, mut ori, move_dir, jumping, gliding) in (
            &entities,
            &mut velocities,
            &mut orientations,
            move_dirs.maybe(),
            jumpings.maybe(),
            glidings.maybe(),
        )
            .join()
        {
            // Move player according to move_dir
            if let Some(move_dir) = move_dir {
                if on_grounds.get(entity).is_some() && vel.0.magnitude() < HUMANOID_SPEED {
                    vel.0 += Vec2::broadcast(dt.0) * move_dir.0 * HUMANOID_ACCEL;
                } else if glidings.get(entity).is_some() && vel.0.magnitude() < GLIDE_SPEED {
                    vel.0 += Vec2::broadcast(dt.0) * move_dir.0 * GLIDE_ACCEL;
                } else if vel.0.magnitude() < HUMANOID_AIR_SPEED {
                    vel.0 += Vec2::broadcast(dt.0) * move_dir.0 * HUMANOID_AIR_ACCEL;
                }
            }

            // Jump
            if jumping.is_some() {
                vel.0.z = HUMANOID_JUMP_ACCEL;
            }

            // Glide
            if gliding.is_some() && vel.0.magnitude() < GLIDE_SPEED {
                let anti_grav = GLIDE_ANTIGRAV + vel.0.z.powf(2.0) * 0.2;
                vel.0.z += dt.0 * anti_grav * Vec2::<f32>::from(vel.0 * 0.15).magnitude().min(1.0);
            }

            // Set direction based on velocity
            if vel.0.magnitude_squared() != 0.0 {
                ori.0 = vel.0.normalized() * Vec3::new(1.0, 1.0, 0.0);
            }
        }
    }
}
