use crate::{
    comp::{
        ActionState::*, CharacterState, Controller, ForceUpdate, HealthSource, Ori, Pos, Stats, Vel,
    },
    state::{DeltaTime, Uid},
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use std::time::Duration;
use vek::*;

const BASE_DMG: i32 = 10;
const BLOCK_EFFICIENCY: f32 = 0.9;

const ATTACK_RANGE: f32 = 4.0;
const BLOCK_ANGLE: f32 = 180.0;

const KNOCKBACK_XY: f32 = 2.0;
const KNOCKBACK_Z: f32 = 2.0;

/// This system is responsible for handling accepted inputs like moving or attacking
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Uid>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Ori>,
        ReadStorage<'a, Controller>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, CharacterState>,
        WriteStorage<'a, Stats>,
        WriteStorage<'a, ForceUpdate>,
    );

    fn run(
        &mut self,
        (
            entities,
            uids,
            dt,
            positions,
            orientations,
            controllers,
            mut velocities,
            mut character_states,
            mut stats,
            mut force_updates,
        ): Self::SystemData,
    ) {
        // Attacks
        for (entity, uid, pos, ori, controller) in
            (&entities, &uids, &positions, &orientations, &controllers).join()
        {
            let mut todo_end = false;

            // Go through all other entities
            if let Some(Attack { time_left, applied }) =
                &mut character_states.get(entity).map(|c| c.action)
            {
                if !*applied {
                    for (b, pos_b, ori_b, character_b, mut vel_b, stat_b) in (
                        &entities,
                        &positions,
                        &orientations,
                        &character_states,
                        &mut velocities,
                        &mut stats,
                    )
                        .join()
                    {
                        // 2D versions
                        let pos2 = Vec2::from(pos.0);
                        let pos_b2: Vec2<f32> = Vec2::from(pos_b.0);
                        let ori2 = Vec2::from(ori.0);

                        // Check if it is a hit
                        if entity != b
                            && !stat_b.is_dead
                            && pos.0.distance_squared(pos_b.0) < ATTACK_RANGE.powi(2)
                            // TODO: Use size instead of 1.0
                            && ori2.angle_between(pos_b2 - pos2) < (1.0 / pos2.distance(pos_b2)).atan()
                        {
                            let dmg = if character_b.action.is_block()
                                && ori_b.0.angle_between(pos.0 - pos_b.0).to_degrees()
                                    < BLOCK_ANGLE / 2.0
                            {
                                (BASE_DMG as f32 * (1.0 - BLOCK_EFFICIENCY)) as i32
                            } else {
                                BASE_DMG
                            };

                            // Deal damage
                            stat_b
                                .health
                                .change_by(-dmg, HealthSource::Attack { by: *uid }); // TODO: variable damage and weapon
                            vel_b.0 += (pos_b.0 - pos.0).normalized() * KNOCKBACK_XY;
                            vel_b.0.z = KNOCKBACK_Z;
                            let _ = force_updates.insert(b, ForceUpdate);
                        }
                    }
                }

                if let Some(Attack { time_left, applied }) =
                    &mut character_states.get_mut(entity).map(|c| &mut c.action)
                {
                    // Only attack once
                    *applied = true;

                    if *time_left == Duration::default() {
                        todo_end = true;
                    } else {
                        *time_left = time_left
                            .checked_sub(Duration::from_secs_f32(dt.0))
                            .unwrap_or_default();
                    }
                }
            }
            if todo_end {
                if let Some(character) = &mut character_states.get_mut(entity) {
                    character.action = Wield {
                        time_left: Duration::default(),
                    };
                }
            }

            if let Some(Wield { time_left }) =
                &mut character_states.get_mut(entity).map(|c| &mut c.action)
            {
                if *time_left != Duration::default() {
                    *time_left = time_left
                        .checked_sub(Duration::from_secs_f32(dt.0))
                        .unwrap_or_default();
                }
            }
        }
    }
}
