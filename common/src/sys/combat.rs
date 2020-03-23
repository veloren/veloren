use crate::{
    comp::{
        Agent, Attacking, Body, CharacterState, Controller, HealthChange, HealthSource, Ori, Pos,
        Scale, Stats,
    },
    event::{EventBus, ServerEvent},
    sync::Uid,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

const BLOCK_EFFICIENCY: f32 = 0.9;
const BLOCK_ANGLE: f32 = 180.0;

/// This system is responsible for handling accepted inputs like moving or
/// attacking
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Ori>,
        ReadStorage<'a, Scale>,
        ReadStorage<'a, Agent>,
        ReadStorage<'a, Controller>,
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
            uids,
            positions,
            orientations,
            scales,
            agents,
            controllers,
            bodies,
            stats,
            mut attacking_storage,
            character_states,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();
        // Attacks
        for (entity, uid, pos, ori, scale_maybe, agent_maybe, _, _attacker_stats, attack) in (
            &entities,
            &uids,
            &positions,
            &orientations,
            scales.maybe(),
            agents.maybe(),
            &controllers,
            &stats,
            &mut attacking_storage,
        )
            .join()
        {
            if attack.applied {
                continue;
            }
            attack.applied = true;

            // Go through all other entities
            for (b, uid_b, pos_b, ori_b, scale_b_maybe, character_b, stats_b, body_b) in (
                &entities,
                &uids,
                &positions,
                &orientations,
                scales.maybe(),
                &character_states,
                &stats,
                &bodies,
            )
                .join()
            {
                // 2D versions
                let pos2 = Vec2::from(pos.0);
                let pos_b2 = Vec2::<f32>::from(pos_b.0);
                let ori2 = Vec2::from(ori.0);

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
                    let mut dmg = attack.base_damage;

                    // NPCs do less damage:
                    if agent_maybe.is_some() {
                        dmg = (dmg / 2).max(1);
                    }

                    if rand::random() {
                        dmg += 1;
                    }

                    // Block
                    if character_b.is_block()
                        && ori_b.0.angle_between(pos.0 - pos_b.0) < BLOCK_ANGLE.to_radians() / 2.0
                    {
                        dmg = (dmg as f32 * (1.0 - BLOCK_EFFICIENCY)) as u32
                    }

                    server_emitter.emit(ServerEvent::Damage {
                        uid: *uid_b,
                        change: HealthChange {
                            amount: -(dmg as i32),
                            cause: HealthSource::Attack { by: *uid },
                        },
                    });
                    attack.hit_count += 1;
                }
            }
        }
    }
}
