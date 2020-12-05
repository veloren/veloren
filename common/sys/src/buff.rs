use common::{
    comp::{
        BuffCategory, BuffChange, BuffEffect, BuffId, BuffSource, Buffs, Energy, Health,
        HealthChange, HealthSource, Inventory, ModifierKind,
    },
    event::{EventBus, ServerEvent},
    resources::DeltaTime,
    Damage, DamageSource,
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
        ReadStorage<'a, Inventory>,
        WriteStorage<'a, Health>,
        WriteStorage<'a, Energy>,
        WriteStorage<'a, Buffs>,
    );

    fn run(
        &mut self,
        (entities, dt, server_bus, inventories, mut healths, mut energies, mut buffs): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();
        // Set to false to avoid spamming server
        buffs.set_event_emission(false);
        healths.set_event_emission(false);
        energies.set_event_emission(false);
        for (entity, mut buff_comp, mut health, mut energy) in
            (&entities, &mut buffs, &mut healths, &mut energies).join()
        {
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

            if let Some(inventory) = inventories.get(entity) {
                let damage_reduction = Damage::compute_damage_reduction(inventory);
                if (damage_reduction - 1.0).abs() < f32::EPSILON {
                    for (id, buff) in buff_comp.buffs.iter() {
                        if !buff.kind.is_buff() {
                            expired_buffs.push(*id);
                        }
                    }
                }
            }

            // Call to reset health and energy to base values
            health.last_set();
            energy.last_set();
            health.reset_max();
            energy.reset_max();

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
                                            kind: DamageSource::Buff(buff.kind),
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
                                        change: (HealthChange { amount, cause }, 0),
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
                            BuffEffect::MaxEnergyModifier { value, kind } => match kind {
                                ModifierKind::Additive => {
                                    let new_max = (energy.maximum() as f32 + *value) as u32;
                                    energy.set_maximum(new_max);
                                },
                                ModifierKind::Fractional => {
                                    let new_max = (energy.maximum() as f32 + *value) as u32;
                                    energy.set_maximum(new_max);
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
        energies.set_event_emission(true);
    }
}
