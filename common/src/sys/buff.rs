use crate::{
    comp::{
        BuffCategoryId, BuffChange, BuffEffect, BuffSource, Buffs, HealthChange, HealthSource,
        Stats,
    },
    event::{EventBus, ServerEvent},
    state::DeltaTime,
    sync::Uid,
};
use specs::{Join, Read, ReadStorage, System, WriteStorage};
use std::time::Duration;

pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Read<'a, DeltaTime>,
        Read<'a, EventBus<ServerEvent>>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Stats>,
        WriteStorage<'a, Buffs>,
    );

    fn run(&mut self, (dt, server_bus, uids, stats, mut buffs): Self::SystemData) {
        let mut server_emitter = server_bus.emitter();
        // Set to false to avoid spamming server
        buffs.set_event_emission(false);
        for (buff_comp, uid) in (&mut buffs, &uids).join() {
            let (mut active_buff_indices_for_removal, mut inactive_buff_indices_for_removal) =
                (Vec::<usize>::new(), Vec::<usize>::new());
            for (i, active_buff) in buff_comp.active_buffs.iter_mut().enumerate() {
                // Tick the buff and subtract delta from it
                if let Some(remaining_time) = &mut active_buff.time {
                    let new_duration = remaining_time.checked_sub(Duration::from_secs_f32(dt.0));
                    if new_duration.is_some() {
                        // The buff still continues.
                        *remaining_time -= Duration::from_secs_f32(dt.0);
                    } else {
                        // The buff has expired.
                        // Remove it.
                        active_buff_indices_for_removal.push(i);
                        active_buff.time = Some(Duration::default());
                    };
                }
            }

            for (i, inactive_buff) in buff_comp.inactive_buffs.iter_mut().enumerate() {
                // Tick the buff and subtract delta from it
                if let Some(remaining_time) = &mut inactive_buff.time {
                    let new_duration = remaining_time.checked_sub(Duration::from_secs_f32(dt.0));
                    if new_duration.is_some() {
                        // The buff still continues.
                        *remaining_time -= Duration::from_secs_f32(dt.0);
                    } else {
                        // The buff has expired.
                        // Remove it.
                        inactive_buff_indices_for_removal.push(i);
                        inactive_buff.time = Some(Duration::default());
                    };
                }
            }

            if !active_buff_indices_for_removal.is_empty()
                || !inactive_buff_indices_for_removal.is_empty()
            {
                server_emitter.emit(ServerEvent::Buff {
                    uid: *uid,
                    buff_change: BuffChange::RemoveExpiredByIndex(
                        active_buff_indices_for_removal,
                        inactive_buff_indices_for_removal,
                    ),
                });
            }
        }
        // Set back to true after timer decrement
        buffs.set_event_emission(true);
        for (uid, stat, mut buffs) in (&uids, &stats, &mut buffs.restrict_mut()).join() {
            let buff_comp = buffs.get_mut_unchecked();
            // Tick all de/buffs on a Buffs component.
            for active_buff in buff_comp.active_buffs.iter_mut() {
                // Get buff owner
                let buff_owner = if let BuffSource::Character { by: owner } = active_buff.source {
                    Some(owner)
                } else {
                    None
                };
                // Now, execute the buff, based on it's delta
                for effect in &mut active_buff.effects {
                    match effect {
                        // Only add an effect here if it is continuous or it is not immediate
                        BuffEffect::HealthChangeOverTime { rate, accumulated } => {
                            *accumulated += *rate * dt.0;
                            // Apply damage only once a second (with a minimum of 1 damage), or when
                            // a buff is removed
                            if accumulated.abs() > rate.abs().max(10.0)
                                || active_buff
                                    .time
                                    .map_or(false, |dur| dur == Duration::default())
                            {
                                let cause = if *accumulated > 0.0 {
                                    HealthSource::Healing { by: buff_owner }
                                } else {
                                    HealthSource::Buff { owner: buff_owner }
                                };
                                server_emitter.emit(ServerEvent::Damage {
                                    uid: *uid,
                                    change: HealthChange {
                                        amount: *accumulated as i32,
                                        cause,
                                    },
                                });
                                *accumulated = 0.0;
                            };
                        },
                        BuffEffect::NameChange { .. } => {},
                    };
                }
            }

            if stat.is_dead {
                server_emitter.emit(ServerEvent::Buff {
                    uid: *uid,
                    buff_change: BuffChange::RemoveByCategory {
                        required: vec![],
                        optional: vec![],
                        blacklisted: vec![BuffCategoryId::PersistOnDeath],
                    },
                });
            }
        }
    }
}
