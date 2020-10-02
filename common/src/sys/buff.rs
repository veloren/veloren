use crate::{
    comp::{BuffChange, BuffEffect, Buffs, HealthChange, HealthSource, Stats},
    event::{EventBus, ServerEvent},
    state::DeltaTime,
    sync::Uid,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use std::time::Duration;

/// This system modifies entity stats, changing them using buffs
/// Currently, the system is VERY, VERY CRUDE and SYNC UN-FRIENDLY.
/// It does not use events and uses `Vec`s stored in component.
///
/// TODO: Make this production-quality system/design
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        Read<'a, EventBus<ServerEvent>>,
        ReadStorage<'a, Uid>,
        WriteStorage<'a, Stats>,
        WriteStorage<'a, Buffs>,
    );

    fn run(&mut self, (entities, dt, server_bus, uids, mut stats, mut buffs): Self::SystemData) {
        let mut server_emitter = server_bus.emitter();
        for (entity, uid, mut buffs) in (&entities, &uids, &mut buffs.restrict_mut()).join() {
            let buff_comp = buffs.get_mut_unchecked();
            let (mut active_buff_indices_for_removal, mut inactive_buff_indices_for_removal) =
                (Vec::<usize>::new(), Vec::<usize>::new());
            // Tick all de/buffs on a Buffs component.
            for i in 0..buff_comp.active_buffs.len() {
                // First, tick the buff and subtract delta from it
                // and return how much "real" time the buff took (for tick independence).
                // TODO: handle delta for "indefinite" buffs, i.e. time since they got removed.
                let buff_delta = if let Some(remaining_time) = &mut buff_comp.active_buffs[i].time {
                    let pre_tick = remaining_time.as_secs_f32();
                    let new_duration = remaining_time.checked_sub(Duration::from_secs_f32(dt.0));
                    let post_tick = if let Some(dur) = new_duration {
                        // The buff still continues.
                        *remaining_time -= Duration::from_secs_f32(dt.0);
                        dur.as_secs_f32()
                    } else {
                        // The buff has expired.
                        // Remove it.
                        active_buff_indices_for_removal.push(i);
                        0.0
                    };
                    pre_tick - post_tick
                } else {
                    // The buff is indefinite, and it takes full tick (delta).
                    // TODO: Delta for indefinite buffs might be shorter since they can get removed
                    // *during a tick* and this treats it as it always happens on a *tick end*.
                    dt.0
                };

                // Now, execute the buff, based on it's delta
                for effect in &mut buff_comp.active_buffs[i].effects {
                    #[allow(clippy::single_match)]
                    // Remove clippy when more effects are added here
                    match effect {
                        // Only add an effect here if it is continuous or it is not immediate
                        BuffEffect::HealthChangeOverTime { rate, accumulated } => {
                            *accumulated += *rate * buff_delta;
                            // Apply only 0.5 or higher damage
                            if accumulated.abs() > 5.0 {
                                if let Some(stats) = stats.get_mut(entity) {
                                    let change = HealthChange {
                                        amount: *accumulated as i32,
                                        cause: HealthSource::Unknown,
                                    };
                                    stats.health.change_by(change);
                                }
                                *accumulated = 0.0;
                            };
                        },
                        _ => {},
                    };
                }
            }
            for i in 0..buff_comp.inactive_buffs.len() {
                // First, tick the buff and subtract delta from it
                // and return how much "real" time the buff took (for tick independence).
                // TODO: handle delta for "indefinite" buffs, i.e. time since they got removed.
                if let Some(remaining_time) = &mut buff_comp.inactive_buffs[i].time {
                    let new_duration = remaining_time.checked_sub(Duration::from_secs_f32(dt.0));
                    if new_duration.is_some() {
                        // The buff still continues.
                        *remaining_time -= Duration::from_secs_f32(dt.0);
                    } else {
                        // The buff has expired.
                        // Remove it.
                        inactive_buff_indices_for_removal.push(i);
                    };
                }
            }
            server_emitter.emit(ServerEvent::Buff {
                uid: *uid,
                buff_change: BuffChange::RemoveByIndex(
                    active_buff_indices_for_removal,
                    inactive_buff_indices_for_removal,
                ),
            });
        }
    }
}
