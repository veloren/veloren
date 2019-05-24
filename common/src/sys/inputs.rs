// Library
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::{
    comp::{
        phys::{Dir, ForceUpdate, Pos, Vel},
        Actions, Animation, AnimationInfo, InputEvent, Inputs, Respawn, Stats,
    },
    state::{DeltaTime, Time},
    terrain::TerrainMap,
    vol::{ReadVol, Vox},
};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, Time>,
        Read<'a, DeltaTime>,
        ReadExpect<'a, TerrainMap>,
        WriteStorage<'a, Inputs>,
        WriteStorage<'a, Actions>,
        ReadStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Dir>,
        WriteStorage<'a, AnimationInfo>,
        WriteStorage<'a, Stats>,
        WriteStorage<'a, Respawn>,
        WriteStorage<'a, ForceUpdate>,
    );

    fn run(
        &mut self,
        (
            entities,
            time,
            dt,
            terrain,
            mut inputs,
            mut actions,
            positions,
            mut velocities,
            mut directions,
            mut animation_infos,
            mut stats,
            mut respawns,
            mut force_updates,
        ): Self::SystemData,
    ) {
        for (entity, inputs, pos, mut dir, mut vel) in (
            &entities,
            &mut inputs,
            &positions,
            &mut directions,
            &mut velocities,
        )
            .join()
        {
            // Handle held-down inputs
            let on_ground = terrain
                .get((pos.0 - Vec3::unit_z() * 0.1).map(|e| e.floor() as i32))
                .map(|vox| !vox.is_empty())
                .unwrap_or(false)
                && vel.0.z <= 0.0;

            let (gliding, friction) = if on_ground {
                // TODO: Don't hard-code this.
                // Apply physics to the player: acceleration and non-linear deceleration.
                vel.0 += Vec2::broadcast(dt.0) * inputs.move_dir * 200.0;

                if inputs.jumping {
                    vel.0.z += 16.0;
                }

                (false, 0.15)
            } else {
                // TODO: Don't hard-code this.
                // Apply physics to the player: acceleration and non-linear deceleration.
                vel.0 += Vec2::broadcast(dt.0) * inputs.move_dir * 10.0;

                if inputs.gliding && vel.0.z < 0.0 {
                    // TODO: Don't hard-code this.
                    let anti_grav = 9.81 * 3.95 + vel.0.z.powf(2.0) * 0.2;
                    vel.0.z +=
                        dt.0 * anti_grav * Vec2::<f32>::from(vel.0 * 0.15).magnitude().min(1.0);

                    (true, 0.008)
                } else {
                    (false, 0.015)
                }
            };

            // Friction
            vel.0 -= Vec2::broadcast(dt.0)
                * 50.0
                * vel.0.map(|e| {
                    (e.abs() * friction * (vel.0.magnitude() * 0.1 + 0.5))
                        .min(e.abs() * dt.0 * 50.0)
                        .copysign(e)
                })
                * Vec3::new(1.0, 1.0, 0.0);

            if vel.0.magnitude_squared() != 0.0 {
                dir.0 = vel.0.normalized() * Vec3::new(1.0, 1.0, 0.0);
            }

            let animation = if on_ground {
                if inputs.move_dir.magnitude() > 0.01 {
                    Animation::Run
                } else {
                    Animation::Idle
                }
            } else if gliding {
                Animation::Gliding
            } else {
                Animation::Jump
            };

            let last = animation_infos
                .get_mut(entity)
                .cloned()
                .unwrap_or(AnimationInfo::new());
            let changed = last.animation != animation;

            animation_infos.insert(
                entity,
                AnimationInfo {
                    animation,
                    time: if changed { 0.0 } else { last.time },
                    changed,
                },
            );
        }
        for (entity, inputs) in (&entities, &mut inputs).join() {
            // Handle event-based inputs
            for event in inputs.events.drain(..) {
                match event {
                    InputEvent::Attack => {
                        // Attack delay
                        if let (Some(pos), Some(dir), Some(action)) = (
                            positions.get(entity),
                            directions.get(entity),
                            actions.get_mut(entity),
                        ) {
                            for (b, pos_b, mut stat_b, mut vel_b) in
                                (&entities, &positions, &mut stats, &mut velocities).join()
                            {
                                // Check if it is a hit
                                if entity != b
                                    && pos.0.distance_squared(pos_b.0) < 50.0
                                    && dir.0.angle_between(pos_b.0 - pos.0).to_degrees() < 70.0
                                {
                                    // Set action
                                    action.attack_time = Some(0.0);

                                    // Deal damage
                                    stat_b.hp.change_by(-10); // TODO: variable damage
                                    vel_b.0 += (pos_b.0 - pos.0).normalized() * 20.0;
                                    vel_b.0.z = 20.0;
                                    force_updates.insert(b, ForceUpdate);
                                }
                            }
                        }
                    }
                    InputEvent::RequestRespawn => {
                        respawns.insert(entity, Respawn);
                    }
                    InputEvent::Jump => {}
                }
            }
        }
    }
}
