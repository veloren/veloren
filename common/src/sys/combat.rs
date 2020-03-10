use crate::{
    comp::{
        Attacking, Body, CharacterState, Controller, HealthChange, HealthSource, Ori, Pos, Scale,
        Stats,
    },
    event::{EventBus, LocalEvent, ServerEvent},
    state::DeltaTime,
    sync::Uid,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
// use std::time::Duration;
use vek::*;

const BLOCK_EFFICIENCY: f32 = 0.9;

const ATTACK_RANGE: f32 = 3.5;
const ATTACK_ANGLE: f32 = 45.0;
const BLOCK_ANGLE: f32 = 180.0;

/// This system is responsible for handling accepted inputs like moving or
/// attacking
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, EventBus<LocalEvent>>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Ori>,
        ReadStorage<'a, Scale>,
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
            _local_bus,
            _dt,
            uids,
            positions,
            orientations,
            scales,
            controllers,
            bodies,
            stats,
            mut attacking_storage,
            character_states,
        ): Self::SystemData,
    ) {
        // Attacks
        for (entity, uid, pos, ori, scale_maybe, _, _attacker_stats, attack) in (
            &entities,
            &uids,
            &positions,
            &orientations,
            scales.maybe(),
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
                let pos_b2: Vec2<f32> = Vec2::from(pos_b.0);
                let ori2 = Vec2::from(ori.0);

                // Scales
                let scale = scale_maybe.map_or(1.0, |s| s.0);
                let scale_b = scale_b_maybe.map_or(1.0, |s| s.0);
                let rad_b = body_b.radius() * scale_b;

                // Check if it is a hit
                if entity != b
                    && !stats_b.is_dead
                    // Spherical wedge shaped attack field
                    && pos.0.distance_squared(pos_b.0) < (rad_b + scale * ATTACK_RANGE).powi(2)
                    && ori2.angle_between(pos_b2 - pos2) < ATTACK_ANGLE.to_radians() / 2.0 + (rad_b / pos2.distance(pos_b2)).atan()
                {
                    // Weapon gives base damage
                    let mut dmg = attack.weapon.map(|w| w.base_damage as i32).unwrap_or(3);

                    // Block
                    if character_b.is_block()
                        && ori_b.0.angle_between(pos.0 - pos_b.0) < BLOCK_ANGLE.to_radians() / 2.0
                    {
                        dmg = (dmg as f32 * (1.0 - BLOCK_EFFICIENCY)) as i32
                    }

                    server_bus.emitter().emit(ServerEvent::Damage {
                        uid: *uid_b,
                        change: HealthChange {
                            amount: -dmg,
                            cause: HealthSource::Attack { by: *uid },
                        },
                    });
                    attack.hit_count += 1;
                }
            }
        }
    }
}
