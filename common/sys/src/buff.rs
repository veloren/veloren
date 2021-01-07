use common::{
    comp::{
        BuffCategory, BuffChange, BuffEffect, BuffId, BuffSource, Buffs, Health, HealthChange,
        HealthSource, Loadout, ModifierKind,
    },
    event::{EventBus, ServerEvent},
    resources::DeltaTime,
    DamageSource,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use std::time::Duration;

pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        Read<'a, EventBus<ServerEvent>>,
        ReadStorage<'a, Loadout>,
        WriteStorage<'a, Health>,
        WriteStorage<'a, Buffs>,
    );

    fn run(
        &mut self,
        (entities, dt, server_bus, loadouts, mut healths, mut buffs): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();
        // Set to false to avoid spamming server
        buffs.set_event_emission(false);
        healths.set_event_emission(false);
        for (entity, mut buff_comp, mut health) in (&entities, &mut buffs, &mut healths).join() {
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
                        // checked_sub returns None when remaining time
                        // went below 0, so set to 0
                        *remaining_time = Duration::default();
                        // The buff has expired.
                        // Remove it.
                        expired_buffs.push(*id);
                    }
                }
            }

            if let Some(loadout) = loadouts.get(entity) {
                let damage_reduction = loadout.get_damage_reduction();
                if (damage_reduction - 1.0).abs() < f32::EPSILON {
                    for (id, buff) in buff_comp.buffs.iter() {
                        if !buff.kind.is_buff() {
                            expired_buffs.push(*id);
                        }
                    }
                }
            }

            // Call to reset health to base values
            health.reset_max();

            // Iterator over the lists of buffs by kind
            let buff_comp = &mut *buff_comp;
            for buff_ids in buff_comp.kinds.values() {
                // Get the strongest of this buff kind
                if let Some(buff) = buff_comp.buffs.get_mut(&buff_ids[0]) {
                    // Get buff owner?
                    let buff_owner = if let BuffSource::Character { by: owner } = buff.source {
                        Some(owner)
                    } else {
                        None
                    };

                    // Now, execute the buff, based on it's delta
                    for effect in &mut buff.effects {
                        match effect {
                            BuffEffect::HealthChangeOverTime {
                                rate,
                                accumulated,
                                kind,
                            } => {
                                *accumulated += *rate * dt.0;
                                // Apply damage only once a second (with a minimum of 1 damage), or
                                // when a buff is removed
                                if accumulated.abs() > rate.abs().max(10.0)
                                    || buff.time.map_or(false, |dur| dur == Duration::default())
                                {
                                    let cause = if *accumulated > 0.0 {
                                        HealthSource::Heal { by: buff_owner }
                                    } else {
                                        HealthSource::Damage {
                                            kind: DamageSource::Other,
                                            by: buff_owner,
                                        }
                                    };
                                    let amount = match *kind {
                                        ModifierKind::Additive => *accumulated as i32,
                                        ModifierKind::Fractional => {
                                            (health.maximum() as f32 * *accumulated) as i32
                                        },
                                    };
                                    server_emitter.emit(ServerEvent::Damage {
                                        entity,
                                        change: HealthChange { amount, cause },
                                    });
                                    *accumulated = 0.0;
                                };
                            },
                            BuffEffect::MaxHealthModifier { value, kind } => match kind {
                                ModifierKind::Additive => {
                                    let health = &mut *health;
                                    let buffed_health_max =
                                        (health.maximum() as f32 + *value) as u32;
                                    health.set_maximum(buffed_health_max);
                                },
                                ModifierKind::Fractional => {
                                    let health = &mut *health;
                                    health.set_maximum((health.maximum() as f32 * *value) as u32);
                                },
                            },
                        };
                    }
                }
            }

            // Remove buffs that expire
            if !expired_buffs.is_empty() {
                server_emitter.emit(ServerEvent::Buff {
                    entity,
                    buff_change: BuffChange::RemoveById(expired_buffs),
                });
            }

            // Remove buffs that don't persist on death
            if health.is_dead {
                server_emitter.emit(ServerEvent::Buff {
                    entity,
                    buff_change: BuffChange::RemoveByCategory {
                        all_required: vec![],
                        any_required: vec![],
                        none_required: vec![BuffCategory::PersistOnDeath],
                    },
                });
            }
        }
        // Turned back to true
        buffs.set_event_emission(true);
        healths.set_event_emission(true);
    }
}
