// Library
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::{
    comp::{
        phys::{Dir, Pos, Vel},
        Animation, AnimationHistory, Control,
    },
    state::DeltaTime,
    terrain::TerrainMap,
    vol::{ReadVol, Vox},
};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadExpect<'a, TerrainMap>,
        Read<'a, DeltaTime>,
        Entities<'a>,
        ReadStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Dir>,
        WriteStorage<'a, AnimationHistory>,
        ReadStorage<'a, Control>,
    );

    fn run(
        &mut self,
        (terrain, dt, entities, pos, mut vels, mut dirs, mut anims, controls): Self::SystemData,
    ) {
        for (entity, pos, mut vel, mut dir, control) in
            (&entities, &pos, &mut vels, &mut dirs, &controls).join()
        {
            let on_ground = terrain
                .get((pos.0 - Vec3::unit_z() * 0.1).map(|e| e.floor() as i32))
                .map(|vox| !vox.is_empty())
                .unwrap_or(false)
                && vel.0.z <= 0.0;

            let friction = if on_ground {
                // TODO: Don't hard-code this
                // Apply physics to the player: acceleration and non-linear decceleration
                vel.0 += control.move_dir * 4.0;

                if control.jumping {
                    vel.0.z += 16.0;
                }

                0.15
            } else {
                // TODO: Don't hard-code this
                // Apply physics to the player: acceleration and non-linear decceleration
                vel.0 += control.move_dir * 0.2 - vel.0.map(|e| e * e.abs() + e) * 0.002;

                if control.gliding && vel.0.z < 0.0 {
                    // TODO: Don't hard-code this
                    let anti_grav = 9.81 * 3.95;
                    vel.0.z += anti_grav * dt.0 * Vec2::<f32>::from(vel.0 * 0.15).magnitude().min(1.0);
                }

                0.006
            };

            // Friction
            vel.0 -= vel.0.map(|e| (e.abs() * friction * (vel.0.magnitude() * 0.1 + 0.5)).min(e.abs()).copysign(e)) * Vec3::new(1.0, 1.0, 0.0);

            if vel.0.magnitude_squared() != 0.0 {
                dir.0 = vel.0.normalized() * Vec3::new(1.0, 1.0, 0.0);
            }

            let animation = if on_ground {
                if control.move_dir.magnitude() > 0.01 {
                    Animation::Run
                } else {
                    Animation::Idle
                }
            } else {
                Animation::Jump
            };

            let last_history = anims.get_mut(entity).cloned();

            let time = if let Some((true, time)) =
                last_history.map(|last| (last.current == animation, last.time))
            {
                time
            } else {
                0.0
            };

            anims.insert(
                entity,
                AnimationHistory {
                    last: last_history.map(|last| last.current),
                    current: animation,
                    time,
                },
            );
        }
    }
}
