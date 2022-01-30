use common::{
    combat::DamageContributor,
    comp::{
        body::{object, Body},
        buff::{
            Buff, BuffCategory, BuffChange, BuffData, BuffEffect, BuffId, BuffKind, BuffSource,
            Buffs,
        },
        fluid_dynamics::{Fluid, LiquidKind},
        item::MaterialStatManifest,
        Energy, Group, Health, HealthChange, Inventory, LightEmitter, ModifierKind, PhysicsState,
        Stats,
    },
    event::{EventBus, ServerEvent},
    resources::{DeltaTime, Time},
    terrain::SpriteKind,
    uid::UidAllocator,
    Damage, DamageSource,
};
use common_base::prof_span;
use common_ecs::{Job, Origin, ParMode, Phase, System};
use hashbrown::HashMap;
use rayon::iter::ParallelIterator;
use specs::{
    saveload::MarkerAllocator, shred::ResourceId, Entities, Join, ParJoin, Read, ReadExpect,
    ReadStorage, SystemData, World, WriteStorage,
};
use std::time::Duration;

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    dt: Read<'a, DeltaTime>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    inventories: ReadStorage<'a, Inventory>,
    healths: ReadStorage<'a, Health>,
    energies: ReadStorage<'a, Energy>,
    physics_states: ReadStorage<'a, PhysicsState>,
    groups: ReadStorage<'a, Group>,
    uid_allocator: Read<'a, UidAllocator>,
    time: Read<'a, Time>,
    msm: ReadExpect<'a, MaterialStatManifest>,
}

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, Buffs>,
        WriteStorage<'a, Stats>,
        WriteStorage<'a, Body>,
        WriteStorage<'a, LightEmitter>,
    );

    const NAME: &'static str = "buff";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(
        job: &mut Job<Self>,
        (read_data, mut buffs, mut stats, mut bodies, mut light_emitters): Self::SystemData,
    ) {
        let mut server_emitter = read_data.server_bus.emitter();
        let dt = read_data.dt.0;
        // Set to false to avoid spamming server
        buffs.set_event_emission(false);
        stats.set_event_emission(false);
        // Put out underwater campfires. Logically belongs here since this system also
        // removes burning, but campfires don't have healths/stats/energies/buffs, so
        // this needs a separate loop.
        job.cpu_stats.measure(ParMode::Rayon);
        let to_put_out_campfires = (&read_data.entities, &bodies, &read_data.physics_states)
            .par_join()
            .map_init(
                || {
                    prof_span!(guard, "buff campfire deactivate");
                    guard
                },
                |_guard, (entity, body, physics_state)| {
                    if matches!(*body, Body::Object(object::Body::CampfireLit))
                        && matches!(
                            physics_state.in_fluid,
                            Some(Fluid::Liquid {
                                kind: LiquidKind::Water,
                                ..
                            })
                        )
                    {
                        Some(entity)
                    } else {
                        None
                    }
                },
            )
            .fold(Vec::new, |mut to_put_out_campfires, put_out_campfire| {
                put_out_campfire.map(|put| to_put_out_campfires.push(put));
                to_put_out_campfires
            })
            .reduce(
                Vec::new,
                |mut to_put_out_campfires_a, mut to_put_out_campfires_b| {
                    to_put_out_campfires_a.append(&mut to_put_out_campfires_b);
                    to_put_out_campfires_a
                },
            );
        job.cpu_stats.measure(ParMode::Single);
        {
            prof_span!(_guard, "write deferred campfire deletion");
            // Assume that to_put_out_campfires is near to zero always, so this access isn't
            // slower than parallel checking above
            for e in to_put_out_campfires {
                {
                    bodies
                        .get_mut(e)
                        .map(|mut body| *body = Body::Object(object::Body::Campfire));
                    light_emitters.remove(e);
                }
            }
        }

        for (entity, mut buff_comp, mut stat, health, energy, physics_state) in (
            &read_data.entities,
            &mut buffs,
            &mut stats,
            &read_data.healths,
            &read_data.energies,
            read_data.physics_states.maybe(),
        )
            .join()
        {
            // Apply buffs to entity based off of their current physics_state
            if let Some(physics_state) = physics_state {
                if matches!(
                    physics_state.on_ground.and_then(|b| b.get_sprite()),
                    Some(SpriteKind::EnsnaringVines) | Some(SpriteKind::EnsnaringWeb)
                ) {
                    // If on ensnaring vines, apply ensnared debuff
                    server_emitter.emit(ServerEvent::Buff {
                        entity,
                        buff_change: BuffChange::Add(Buff::new(
                            BuffKind::Ensnared,
                            BuffData::new(1.0, Some(Duration::from_secs_f32(1.0))),
                            Vec::new(),
                            BuffSource::World,
                        )),
                    });
                }
                if matches!(
                    physics_state.in_fluid,
                    Some(Fluid::Liquid {
                        kind: LiquidKind::Lava,
                        ..
                    })
                ) {
                    // If in lava fluid, apply burning debuff
                    server_emitter.emit(ServerEvent::Buff {
                        entity,
                        buff_change: BuffChange::Add(Buff::new(
                            BuffKind::Burning,
                            BuffData::new(20.0, None),
                            vec![BuffCategory::Natural],
                            BuffSource::World,
                        )),
                    });
                } else if matches!(
                    physics_state.in_fluid,
                    Some(Fluid::Liquid {
                        kind: LiquidKind::Water,
                        ..
                    })
                ) && buff_comp.kinds.contains_key(&BuffKind::Burning)
                {
                    // If in water fluid and currently burning, remove burning debuffs
                    server_emitter.emit(ServerEvent::Buff {
                        entity,
                        buff_change: BuffChange::RemoveByKind(BuffKind::Burning),
                    });
                }
            }

            let (buff_comp_kinds, buff_comp_buffs): (
                &HashMap<BuffKind, Vec<BuffId>>,
                &mut HashMap<BuffId, Buff>,
            ) = buff_comp.parts();
            let mut expired_buffs = Vec::<BuffId>::new();

            // For each buff kind present on entity, if the buff kind queues, only ticks
            // duration of strongest buff of that kind, else it ticks durations of all buffs
            // of that kind. Any buffs whose durations expire are marked expired.
            for (kind, ids) in buff_comp_kinds.iter() {
                if kind.queues() {
                    if let Some((Some(buff), id)) =
                        ids.get(0).map(|id| (buff_comp_buffs.get_mut(id), id))
                    {
                        tick_buff(*id, buff, dt, |id| expired_buffs.push(id));
                    }
                } else {
                    for (id, buff) in buff_comp_buffs
                        .iter_mut()
                        .filter(|(i, _)| ids.iter().any(|id| id == *i))
                    {
                        tick_buff(*id, buff, dt, |id| expired_buffs.push(id));
                    }
                }
            }

            let damage_reduction = Damage::compute_damage_reduction(
                None,
                read_data.inventories.get(entity),
                Some(&stat),
                &read_data.msm,
            );
            if (damage_reduction - 1.0).abs() < f32::EPSILON {
                for (id, buff) in buff_comp.buffs.iter() {
                    if !buff.kind.is_buff() {
                        expired_buffs.push(*id);
                    }
                }
            }

            // Call to reset stats to base values
            stat.reset_temp_modifiers();

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
                                *accumulated += *rate * dt;
                                // Apply health change only once per second, per health, or
                                // when a buff is removed
                                if accumulated.abs() > rate.abs().min(1.0)
                                    || buff.time.map_or(false, |dur| dur == Duration::default())
                                {
                                    let (cause, by) = if *accumulated < 0.0 {
                                        (Some(DamageSource::Buff(buff.kind)), buff_owner)
                                    } else {
                                        (None, None)
                                    };
                                    let amount = match *kind {
                                        ModifierKind::Additive => *accumulated,
                                        ModifierKind::Fractional => health.maximum() * *accumulated,
                                    };
                                    let damage_contributor = by.and_then(|uid| {
                                        read_data.uid_allocator.retrieve_entity_internal(uid.0).map(
                                            |entity| {
                                                DamageContributor::new(
                                                    uid,
                                                    read_data.groups.get(entity).cloned(),
                                                )
                                            },
                                        )
                                    });
                                    server_emitter.emit(ServerEvent::HealthChange {
                                        entity,
                                        change: HealthChange {
                                            amount,
                                            by: damage_contributor,
                                            cause,
                                            time: *read_data.time,
                                            crit_mult: None,
                                            instance: rand::random(),
                                        },
                                    });
                                    *accumulated = 0.0;
                                };
                            },
                            BuffEffect::EnergyChangeOverTime {
                                rate,
                                accumulated,
                                kind,
                            } => {
                                *accumulated += *rate * dt;
                                // Apply energy change only once per second, per energy, or
                                // when a buff is removed
                                if accumulated.abs() > rate.abs().min(10.0)
                                    || buff.time.map_or(false, |dur| dur == Duration::default())
                                {
                                    let amount = match *kind {
                                        ModifierKind::Additive => *accumulated,
                                        ModifierKind::Fractional => {
                                            energy.maximum() as f32 * *accumulated
                                        },
                                    };
                                    server_emitter.emit(ServerEvent::EnergyChange {
                                        entity,
                                        change: amount,
                                    });
                                    *accumulated = 0.0;
                                };
                            },
                            BuffEffect::MaxHealthModifier { value, kind } => match kind {
                                ModifierKind::Additive => {
                                    stat.max_health_modifiers.add_mod += *value;
                                },
                                ModifierKind::Fractional => {
                                    stat.max_health_modifiers.mult_mod *= *value;
                                },
                            },
                            BuffEffect::MaxEnergyModifier { value, kind } => match kind {
                                ModifierKind::Additive => {
                                    stat.max_energy_modifiers.add_mod += *value;
                                },
                                ModifierKind::Fractional => {
                                    stat.max_energy_modifiers.mult_mod *= *value;
                                },
                            },
                            BuffEffect::DamageReduction(dr) => {
                                stat.damage_reduction = stat.damage_reduction.max(*dr).min(1.0);
                            },
                            BuffEffect::MaxHealthChangeOverTime {
                                rate,
                                kind,
                                target_fraction,
                                achieved_fraction,
                            } => {
                                // Current fraction uses information from last tick, which is
                                // necessary as buffs from this tick are not guaranteed to have
                                // finished applying
                                let current_fraction = health.maximum() / health.base_max();

                                // If achieved_fraction not initialized, initialize it to health
                                // fraction
                                if achieved_fraction.is_none() {
                                    *achieved_fraction = Some(current_fraction)
                                }

                                if let Some(achieved_fraction) = achieved_fraction {
                                    // Percentage change that should be applied to max_health
                                    let health_tick = match kind {
                                        ModifierKind::Additive => {
                                            // `rate * dt` is amount of health, dividing by base max
                                            // creates fraction
                                            *rate * dt / health.base_max() as f32
                                        },
                                        ModifierKind::Fractional => {
                                            // `rate * dt` is the fraction
                                            *rate * dt
                                        },
                                    };

                                    let potential_fraction = *achieved_fraction + health_tick;

                                    // Potential progress towards target fraction, if
                                    // target_fraction ~ 1.0 then set progress to 1.0 to avoid
                                    // divide by zero
                                    let progress = if (1.0 - *target_fraction).abs() > f32::EPSILON
                                    {
                                        (1.0 - potential_fraction) / (1.0 - *target_fraction)
                                    } else {
                                        1.0
                                    };

                                    // Change achieved_fraction depending on what other buffs have
                                    // occurred
                                    if progress > 1.0 {
                                        // If potential fraction already beyond target fraction,
                                        // simply multiply max_health_modifier by the target
                                        // fraction, and set achieved fraction to target_fraction
                                        *achieved_fraction = *target_fraction;
                                    } else {
                                        // Else have not achieved target yet, update
                                        // achieved_fraction
                                        *achieved_fraction = potential_fraction;
                                    }

                                    // Apply achieved_fraction to max_health_modifier
                                    stat.max_health_modifiers.mult_mod *= *achieved_fraction;
                                }
                            },
                            BuffEffect::MovementSpeed(speed) => {
                                stat.move_speed_modifier *= *speed;
                            },
                            BuffEffect::AttackSpeed(speed) => {
                                stat.attack_speed_modifier *= *speed;
                            },
                            BuffEffect::GroundFriction(gf) => {
                                stat.friction_modifier *= *gf;
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
        stats.set_event_emission(true);
    }
}

fn tick_buff(id: u64, buff: &mut Buff, dt: f32, mut expire_buff: impl FnMut(u64)) {
    // If a buff is recently applied from an aura, do not tick duration
    if buff
        .cat_ids
        .iter()
        .any(|cat_id| matches!(cat_id, BuffCategory::FromAura(true)))
    {
        return;
    }
    if let Some(remaining_time) = &mut buff.time {
        if let Some(new_duration) = remaining_time.checked_sub(Duration::from_secs_f32(dt)) {
            // The buff still continues.
            *remaining_time = new_duration;
        } else {
            // checked_sub returns None when remaining time
            // went below 0, so set to 0
            *remaining_time = Duration::default();
            // The buff has expired.
            // Remove it.
            expire_buff(id);
        }
    }
}
