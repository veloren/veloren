// Library
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::{
    comp::{
        phys::{ForceUpdate, Ori, Pos, Vel},
        Animation, AnimationInfo, Attacking, Control, Gliding, HealthSource, Jumping, Stats,
    },
    state::{DeltaTime, Uid},
    terrain::TerrainMap,
    vol::{ReadVol, Vox},
};

// Basic ECS AI agent system
pub struct Sys;

const HUMANOID_ACCEL: f32 = 100.0;
const HUMANOID_SPEED: f32 = 500.0;
const HUMANOID_AIR_ACCEL: f32 = 10.0;
const HUMANOID_AIR_SPEED: f32 = 100.0;
const HUMANOID_JUMP_ACCEL: f32 = 16.0;
const GLIDE_ACCEL: f32 = 15.0;
const GLIDE_SPEED: f32 = 45.0;
// Gravity is 9.81 * 4, so this makes gravity equal to .15
const GLIDE_ANTIGRAV: f32 = 9.81 * 3.95;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Uid>,
        Read<'a, DeltaTime>,
        ReadExpect<'a, TerrainMap>,
        ReadStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, AnimationInfo>,
        WriteStorage<'a, Stats>,
        ReadStorage<'a, Control>,
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
            mut velocities,
            mut orientations,
            mut animation_infos,
            mut stats,
            controls,
            jumps,
            glides,
            mut attacks,
            mut force_updates,
        ): Self::SystemData,
    ) {
        for (entity, pos, control, stats, mut ori, mut vel) in (
            &entities,
            &positions,
            &controls,
            &stats,
            &mut orientations,
            &mut velocities,
        )
            .join()
        {
            // Disable while dead TODO: Replace with client states
            if stats.is_dead {
                continue;
            }

            let on_ground = terrain
                .get((pos.0 - Vec3::unit_z() * 0.1).map(|e| e.floor() as i32))
                .map(|vox| !vox.is_empty())
                .unwrap_or(false)
                && vel.0.z <= 0.0;

            let gliding = glides.get(entity).is_some() && vel.0.z < 0.0;
            let move_dir = if control.move_dir.magnitude() > 1.0 {
                control.move_dir.normalized()
            } else {
                control.move_dir
            };

            if on_ground {
                // Move player according to move_dir
                if vel.0.magnitude() < HUMANOID_SPEED {
                    vel.0 += Vec2::broadcast(dt.0) * move_dir * HUMANOID_ACCEL;
                }

                // Jump
                if jumps.get(entity).is_some() && vel.0.z <= 0.0 {
                    vel.0.z = HUMANOID_JUMP_ACCEL;
                    jumps.remove(entity);
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

            let animation = if on_ground {
                if control.move_dir.magnitude() > 0.01 {
                    Animation::Run
                } else if attacks.get(entity).is_some() {
                    Animation::Attack
                } else {
                    Animation::Idle
                }
            } else if glides.get(entity).is_some() {
                Animation::Gliding
            } else {
                Animation::Jump
            };

            let last = animation_infos
                .get_mut(entity)
                .cloned()
                .unwrap_or(AnimationInfo::default());
            let changed = last.animation != animation;

            animation_infos
                .insert(
                    entity,
                    AnimationInfo {
                        animation,
                        time: if changed { 0.0 } else { last.time },
                        changed,
                    },
                )
                .expect("Inserting animation_info for an entity failed!");
        }

        for (entity, &uid, pos, ori, attacking) in
            (&entities, &uids, &positions, &orientations, &mut attacks).join()
        {
            if !attacking.applied {
                for (b, pos_b, mut stat_b, mut vel_b) in
                    (&entities, &positions, &mut stats, &mut velocities).join()
                {
                    // Check if it is a hit
                    if entity != b
                        && !stat_b.is_dead
                        && pos.0.distance_squared(pos_b.0) < 50.0
                        && ori.0.angle_between(pos_b.0 - pos.0).to_degrees() < 70.0
                    {
                        // Deal damage
                        stat_b.hp.change_by(-10, HealthSource::Attack { by: uid }); // TODO: variable damage and weapon
                        vel_b.0 += (pos_b.0 - pos.0).normalized() * 10.0;
                        vel_b.0.z = 15.0;
                        force_updates
                            .insert(b, ForceUpdate)
                            .expect("Inserting a forced update for an entity failed!");
                    }
                }
                attacking.applied = true;
            }
        }
    }
}
