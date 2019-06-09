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

// Basic ECS AI agent system
pub struct Sys;
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
        ReadStorage<'a, Controller>,
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
            controllers,
            mut jumps,
            glides,
            mut attacks,
            mut force_updates,
        ): Self::SystemData,
    ) {
        for (entity, pos, controller, stats, mut ori, mut vel) in (
            &entities,
            &positions,
            &controllers,
            &stats,
            &mut orientations,
            &mut velocities,
        )
            .join()
        {
            let on_ground = terrain
                .get((pos.0 - Vec3::unit_z() * 0.1).map(|e| e.floor() as i32))
                .map(|vox| !vox.is_empty())
                .unwrap_or(false)
                && vel.0.z <= 0.0;

            let animation = if on_ground {
                if controller.move_dir.magnitude() > 0.01 {
                    Animation::Run
                } else if attacks.get(entity).is_some() {
                    Animation::Attack
                } else {
                    Animation::Idle
                }
            } else if controller.glide {
                Animation::Gliding
            } else {
                Animation::Jump
            };

            let last = animation_infos
                .get_mut(entity)
                .cloned()
                .unwrap_or(AnimationInfo::default());
            let changed = last.animation != animation;

            if let Err(err) = animation_infos.insert(
                entity,
                AnimationInfo {
                    animation,
                    time: if changed { 0.0 } else { last.time },
                    changed,
                },
            ) {
                warn!("Inserting AnimationInfo for an entity failed: {:?}", err);
            }
        }

        for (entity, &uid, pos, ori, attacking) in
            (&entities, &uids, &positions, &orientations, &mut attacks).join()
        {
            if !attacking.applied {
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
                        stat_b.hp.change_by(-10, HealthSource::Attack { by: uid }); // TODO: variable damage and weapon
                        vel_b.0 += (pos_b.0 - pos.0).normalized() * 10.0;
                        vel_b.0.z = 15.0;
                        if let Err(err) = force_updates.insert(b, ForceUpdate) {
                            warn!("Inserting ForceUpdate for an entity failed: {:?}", err);
                        }
                    }
                }
                attacking.applied = true;
            }
        }
    }
}
