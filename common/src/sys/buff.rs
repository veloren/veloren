use crate::{
    comp::{
        BuffCategory, BuffChange, BuffEffect, BuffId, BuffSource, Buffs, HealthChange,
        HealthSource, Stats,
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
        for (buff_comp, uid, stat) in (&mut buffs, &uids, &stats).join() {
            let mut expired_buffs = Vec::<BuffId>::new();
            for (id, buff) in buff_comp.buffs.iter_mut() {
                // Tick the buff and subtract delta from it
                if let Some(remaining_time) = &mut buff.time {
                    if let Some(new_duration) =
                        remaining_time.checked_sub(Duration::from_secs_f32(dt.0))
                    {
                        // The buff still continues.
                        *remaining_time = new_duration;
                    } else {
                        // The buff has expired.
                        // Remove it.
                        expired_buffs.push(*id);
                    }
                }
            }

            for buff_ids in buff_comp.kinds.values() {
                if let Some(buff) = buff_comp.buffs.get_mut(&buff_ids[0]) {
                    // Get buff owner
                    let buff_owner = if let BuffSource::Character { by: owner } = buff.source {
                        Some(owner)
                    } else {
                        None
                    };
                    // Now, execute the buff, based on it's delta
                    for effect in &mut buff.effects {
                        match effect {
                            // Only add an effect here if it is continuous or it is not immediate
                            BuffEffect::HealthChangeOverTime { rate, accumulated } => {
                                *accumulated += *rate * dt.0;
                                // Apply damage only once a second (with a minimum of 1 damage), or
                                // when a buff is removed
                                if accumulated.abs() > rate.abs().max(10.0)
                                    || buff.time.map_or(false, |dur| dur == Duration::default())
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
            }

            // Remove buffs that expire
            if !expired_buffs.is_empty() {
                server_emitter.emit(ServerEvent::Buff {
                    uid: *uid,
                    buff_change: BuffChange::RemoveById(expired_buffs),
                });
            }

            // Remove stats that don't persist on death
            if stat.is_dead {
                server_emitter.emit(ServerEvent::Buff {
                    uid: *uid,
                    buff_change: BuffChange::RemoveByCategory {
                        all_required: vec![],
                        any_required: vec![],
                        none_required: vec![BuffCategory::PersistOnDeath],
                    },
                });
            }
        }
        buffs.set_event_emission(true);
    }
}
