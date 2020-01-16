use crate::{
    comp::{
        ActionState::*, Body, CharacterState, Controller, HealthChange, HealthSource, Item,
        ItemKind, Ori, Pos, Scale, Stats,
    },
    event::{EventBus, LocalEvent, ServerEvent},
    state::DeltaTime,
    sync::Uid,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use std::time::Duration;
use vek::*;

const BLOCK_EFFICIENCY: f32 = 0.9;

const ATTACK_RANGE: f32 = 3.5;
const ATTACK_ANGLE: f32 = 45.0;
const BLOCK_ANGLE: f32 = 180.0;

/// This system is responsible for handling accepted inputs like moving or attacking
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
        WriteStorage<'a, CharacterState>,
    );

    fn run(
        &mut self,
        (
            entities,
            server_bus,
            local_bus,
            dt,
            uids,
            positions,
            orientations,
            scales,
            controllers,
            bodies,
            stats,
            mut character_states,
        ): Self::SystemData,
    ) {
        // let mut server_emitter = server_bus.emitter();
        // let mut _local_emitter = local_bus.emitter();

        // Attacks
        for (entity, uid, pos, ori, scale_maybe, _, attacker_stats) in (
            &entities,
            &uids,
            &positions,
            &orientations,
            scales.maybe(),
            &controllers,
            &stats,
        )
            .join()
        {
            let recover_duration = if let Some(Item {
                kind: ItemKind::Tool { kind, .. },
                ..
            }) = attacker_stats.equipment.main
            {
                kind.attack_recover_duration()
            } else {
                Duration::from_secs(1)
            };

        //     let (deal_damage, should_end) = if let Some(Attack { time_left, applied }) =
        //         &mut character_states.get_mut(entity).map(|c| &mut c.action)
        //     {
        //         *time_left = time_left
        //             .checked_sub(Duration::from_secs_f32(dt.0))
        //             .unwrap_or_default();
        //         if !*applied && recover_duration > *time_left {
        //             *applied = true;
        //             (true, false)
        //         } else if *time_left == Duration::default() {
        //             (false, true)
        //         } else {
        //             (false, false)
        //         }
        //     } else {
        //         (false, false)
        //     };

        // if deal_damage {
        //     if let Some(Attack { .. }) = &character_states.get(entity).map(|c| c.action) {
        //         // Go through all other entities
        //         for (b, uid_b, pos_b, ori_b, scale_b_maybe, character_b, stats_b, body_b) in (
        //             &entities,
        //             &uids,
        //             &positions,
        //             &orientations,
        //             scales.maybe(),
        //             &character_states,
        //             &stats,
        //             &bodies,
        //         )
        //             .join()
        //         {
        //             // 2D versions
        //             let pos2 = Vec2::from(pos.0);
        //             let pos_b2: Vec2<f32> = Vec2::from(pos_b.0);
        //             let ori2 = Vec2::from(ori.0);

        //             // Scales
        //             let scale = scale_maybe.map_or(1.0, |s| s.0);
        //             let scale_b = scale_b_maybe.map_or(1.0, |s| s.0);
        //             let rad_b = body_b.radius() * scale_b;

        //             // Check if it is a hit
        //             if entity != b
        //                 && !stats_b.is_dead
        //                 // Spherical wedge shaped attack field
        //                 && pos.0.distance_squared(pos_b.0) < (rad_b + scale * ATTACK_RANGE).powi(2)
        //                 && ori2.angle_between(pos_b2 - pos2) < ATTACK_ANGLE.to_radians() / 2.0 + (rad_b / pos2.distance(pos_b2)).atan()
        //             {
        //                 // Weapon gives base damage
        //                 let mut dmg = if let Some(ItemKind::Tool { power, .. }) =
        //                     attacker_stats.equipment.main.as_ref().map(|i| &i.kind)
        //                 {
        //                     *power as i32
        //                 } else {
        //                     1
        //                 };

        //                 // Block
        //                 if character_b.action.is_block()
        //                     && ori_b.0.angle_between(pos.0 - pos_b.0)
        //                         < BLOCK_ANGLE.to_radians() / 2.0
        //                 {
        //                     dmg = (dmg as f32 * (1.0 - BLOCK_EFFICIENCY)) as i32
        //                 }

        //                     server_emitter.emit(ServerEvent::Damage {
        //                         uid: *uid_b,
        //                         change: HealthChange {
        //                             amount: -dmg,
        //                             cause: HealthSource::Attack { by: *uid },
        //                         },
        //                     });
        //                 }
        //             }
        //         }
        //     }

        //     if should_end {
        //         if let Some(character) = &mut character_states.get_mut(entity) {
        //             character.action = Wield {
        //                 time_left: Duration::default(),
        //             };
        //         }
        //     }

        //     if let Some(Wield { time_left }) =
        //         &mut character_states.get_mut(entity).map(|c| &mut c.action)
        //     {
        //         if *time_left != Duration::default() {
        //             *time_left = time_left
        //                 .checked_sub(Duration::from_secs_f32(dt.0))
        //                 .unwrap_or_default();
        //         }
        //     }
        // }
    }
}
