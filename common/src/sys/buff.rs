use crate::{
    comp::{BuffChange, BuffData, BuffId, Buffs, HealthChange, HealthSource, Stats},
    state::DeltaTime,
};
use specs::{Entities, Join, Read, System, WriteStorage};
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
        WriteStorage<'a, Stats>,
        WriteStorage<'a, Buffs>,
    );

    fn run(&mut self, (entities, dt, mut stats, mut buffs): Self::SystemData) {
        // Increment last change timer
        buffs.set_event_emission(false);
        for buff in (&mut buffs).join() {
            buff.last_change += f64::from(dt.0);
        }
        buffs.set_event_emission(true);

        for (entity, mut buffs) in (&entities, &mut buffs.restrict_mut()).join() {
            let buff_comp = buffs.get_mut_unchecked();
            // Add/Remove de/buffs
            // While adding/removing buffs, it could call respective hooks
            // Currently, it is done based on enum variant
            let changes = buff_comp.changes.drain(0..buff_comp.changes.len());
            for change in changes {
                match change {
                    // Hooks for on_add could be here
                    BuffChange::Add(new_buff) => {
                        match &new_buff.data {
                            BuffData::NameChange { prefix } => {
                                if let Some(stats) = stats.get_mut(entity) {
                                    let mut pref = String::from(prefix);
                                    pref.push_str(&stats.name);
                                    stats.name = pref;
                                }
                            },
                            _ => {},
                        }
                        buff_comp.buffs.push(new_buff.clone());
                    },
                    // Hooks for on_remove could be here
                    BuffChange::Remove(id) => {
                        let some_predicate = |current_id: &BuffId| *current_id == id;
                        let mut i = 0;
                        while i != buff_comp.buffs.len() {
                            if some_predicate(&mut buff_comp.buffs[i].id) {
                                let buff = buff_comp.buffs.remove(i);
                                match &buff.data {
                                    BuffData::NameChange { prefix } => {
                                        if let Some(stats) = stats.get_mut(entity) {
                                            stats.name = stats.name.replace(prefix, "");
                                        }
                                    },
                                    _ => {},
                                }
                            } else {
                                i += 1;
                            }
                        }
                    },
                }
            }

            let mut buffs_for_removal = Vec::new();
            // Tick all de/buffs on a Buffs component.
            for active_buff in &mut buff_comp.buffs {
                // First, tick the buff and subtract delta from it
                // and return how much "real" time the buff took (for tick independence).
                // TODO: handle delta for "indefinite" buffs, i.e. time since they got removed.
                let buff_delta = if let Some(remaining_time) = &mut active_buff.time {
                    let pre_tick = remaining_time.as_secs_f32();
                    let new_duration = remaining_time.checked_sub(Duration::from_secs_f32(dt.0));
                    let post_tick = if let Some(dur) = new_duration {
                        // The buff still continues.
                        *remaining_time -= Duration::from_secs_f32(dt.0);
                        dur.as_secs_f32()
                    } else {
                        // The buff has expired.
                        // Mark it for removal.
                        // TODO: This removes by ID! better method required
                        buffs_for_removal.push(active_buff.id.clone());
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
                match &mut active_buff.data {
                    BuffData::RepeatedHealthChange { speed, accumulated } => {
                        *accumulated += *speed * buff_delta;
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
            // Truly mark expired buffs for removal.
            // TODO: Review this, as it is ugly.
            for to_remove in buffs_for_removal {
                buff_comp.remove_buff_by_id(to_remove);
            }
        }
    }
}
