use crate::{
    comp::{
        Agent, Attacking, Body, CharacterState, HealthChange, HealthSource, Ori, Pos, Scale, Stats,
    },
    event::{EventBus, LocalEvent, ServerEvent},
    sync::Uid,
    util::Dir,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

pub const BLOCK_EFFICIENCY: f32 = 0.9;
pub const BLOCK_ANGLE: f32 = 180.0;

/// This system is responsible for handling accepted inputs like moving or
/// attacking
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, EventBus<LocalEvent>>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Ori>,
        ReadStorage<'a, Scale>,
        ReadStorage<'a, Agent>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, Stats>,
        WriteStorage<'a, Attacking>,
        WriteStorage<'a, CharacterState>,
    );

    fn run(
        &mut self,
        (
            entities,
            server_bus,
            local_bus,
            uids,
            positions,
            orientations,
            scales,
            agents,
            bodies,
            stats,
            mut attacking_storage,
            character_states,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();
        let mut local_emitter = local_bus.emitter();
        // Attacks
        for (entity, uid, pos, ori, scale_maybe, attack) in (
            &entities,
            &uids,
            &positions,
            &orientations,
            scales.maybe(),
            &mut attacking_storage,
        )
            .join()
        {
            if attack.applied {
                continue;
            }
            attack.applied = true;

            // Go through all other entities
            for (
                b,
                uid_b,
                pos_b,
                ori_b,
                scale_b_maybe,
                agent_b_maybe,
                character_b,
                stats_b,
                body_b,
            ) in (
                &entities,
                &uids,
                &positions,
                &orientations,
                scales.maybe(),
                agents.maybe(),
                &character_states,
                &stats,
                &bodies,
            )
                .join()
            {
                // 2D versions
                let pos2 = Vec2::from(pos.0);
                let pos_b2 = Vec2::<f32>::from(pos_b.0);
                let ori2 = Vec2::from(*ori.0);

                // Scales
                let scale = scale_maybe.map_or(1.0, |s| s.0);
                let scale_b = scale_b_maybe.map_or(1.0, |s| s.0);
                let rad_b = body_b.radius() * scale_b;

                // Check if it is a hit
                if entity != b
                    && !stats_b.is_dead
                    // Spherical wedge shaped attack field
                    && pos.0.distance_squared(pos_b.0) < (rad_b + scale * attack.range).powi(2)
                    && ori2.angle_between(pos_b2 - pos2) < attack.max_angle + (rad_b / pos2.distance(pos_b2)).atan()
                {
                    // Weapon gives base damage
                    let mut healthchange = attack.base_healthchange as f32;

                    //// NPCs do less damage:
                    //if agent_maybe.is_some() {
                    //    healthchange = (healthchange / 1.5).min(-1.0);
                    //}

                    // Don't heal npc's hp
                    if agent_b_maybe.is_some() && healthchange > 0.0 {
                        healthchange = 0.0;
                    }

                    if rand::random() {
                        healthchange = healthchange * 1.2;
                    }

                    // Block
                    if character_b.is_block()
                        && ori_b.0.angle_between(pos.0 - pos_b.0) < BLOCK_ANGLE.to_radians() / 2.0
                    {
                        healthchange = healthchange * (1.0 - BLOCK_EFFICIENCY)
                    }

                    server_emitter.emit(ServerEvent::Damage {
                        uid: *uid_b,
                        change: HealthChange {
                            amount: healthchange as i32,
                            cause: HealthSource::Attack { by: *uid },
                        },
                    });
                    if attack.knockback != 0.0 {
                        local_emitter.emit(LocalEvent::ApplyForce {
                            entity: b,
                            force: attack.knockback
                                * *Dir::slerp(ori.0, Dir::new(Vec3::new(0.0, 0.0, 1.0)), 0.5),
                        });
                    }
                    attack.hit_count += 1;
                }
            }
        }
    }
}
