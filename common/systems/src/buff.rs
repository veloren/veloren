use common::{
    comp::{
        fluid_dynamics::{Fluid, LiquidKind},
        Buff, BuffCategory, BuffChange, BuffData, BuffEffect, BuffId, BuffKind, BuffSource, Buffs,
        Energy, Health, HealthChange, HealthSource, Inventory, ModifierKind, PhysicsState, Stats,
    },
    event::{EventBus, ServerEvent},
    resources::DeltaTime,
    Damage, DamageSource,
};
use common_ecs::{Job, Origin, Phase, System};
use hashbrown::HashMap;
use specs::{
    shred::ResourceId, Entities, Join, Read, ReadStorage, SystemData, World, WriteStorage,
};
use std::time::Duration;

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    dt: Read<'a, DeltaTime>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    inventories: ReadStorage<'a, Inventory>,
    healths: ReadStorage<'a, Health>,
    physics_states: ReadStorage<'a, PhysicsState>,
    energies: ReadStorage<'a, Energy>,
}

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, Buffs>,
        WriteStorage<'a, Stats>,
    );

    const NAME: &'static str = "buff";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, (read_data, mut buffs, mut stats): Self::SystemData) {
        let mut server_emitter = read_data.server_bus.emitter();
        let dt = read_data.dt.0;
        // Set to false to avoid spamming server
        buffs.set_event_emission(false);
        stats.set_event_emission(false);
        for (entity, mut buff_comp, energy, mut stat, health, physics_state) in (
            &read_data.entities,
            &mut buffs,
            &read_data.energies,
            &mut stats,
            &read_data.healths,
            read_data.physics_states.maybe(),
        )
            .join()
        {
            let in_fluid = physics_state.and_then(|p| p.in_fluid);

            if matches!(
                in_fluid,
                Some(Fluid::Liquid {
                    kind: LiquidKind::Lava,
                    ..
                })
            ) && !buff_comp.contains(BuffKind::Burning)
            {
                server_emitter.emit(ServerEvent::Buff {
                    entity,
                    buff_change: BuffChange::Add(Buff::new(
                        BuffKind::Burning,
                        BuffData::new(200.0, None),
                        vec![BuffCategory::Natural],
                        BuffSource::World,
                    )),
                });
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
                        tick_buff(*id, buff, dt, in_fluid, |id| expired_buffs.push(id));
                    }
                } else {
                    for (id, buff) in buff_comp_buffs
                        .iter_mut()
                        .filter(|(i, _)| ids.iter().any(|id| id == *i))
                    {
                        tick_buff(*id, buff, dt, in_fluid, |id| expired_buffs.push(id));
                    }
                }
            }

            let damage_reduction = Damage::compute_damage_reduction(
                read_data.inventories.get(entity),
                Some(&stat),
                None,
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
                                if accumulated.abs() > rate.abs().min(10.0)
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
                                        change: HealthChange { amount, cause },
                                    });
                                    *accumulated = 0.0;
                                };
                            },
                            BuffEffect::MaxHealthModifier { value, kind } => match kind {
                                ModifierKind::Additive => {
                                    stat.max_health_modifier += *value / (health.base_max() as f32);
                                },
                                ModifierKind::Fractional => {
                                    stat.max_health_modifier *= *value;
                                },
                            },
                            BuffEffect::MaxEnergyModifier { value, kind } => match kind {
                                ModifierKind::Additive => {
                                    stat.max_energy_modifier += *value / (energy.base_max() as f32);
                                },
                                ModifierKind::Fractional => {
                                    stat.max_energy_modifier *= *value;
                                },
                            },
                            BuffEffect::DamageReduction(dr) => {
                                stat.damage_reduction = stat.damage_reduction.max(*dr).min(1.0);
                            },
                            BuffEffect::MaxHealthChangeOverTime {
                                rate,
                                accumulated,
                                kind,
                                target_fraction,
                            } => {
                                *accumulated += *rate * dt;
                                let current_fraction = health.maximum() as f32
                                    / (health.base_max() as f32 * stat.max_health_modifier);
                                let progress = (1.0 - current_fraction) / (1.0 - *target_fraction);
                                if progress > 1.0 {
                                    stat.max_health_modifier *= *target_fraction;
                                } else if accumulated.abs() > rate.abs() {
                                    match kind {
                                        ModifierKind::Additive => {
                                            stat.max_health_modifier = stat.max_health_modifier
                                                * current_fraction
                                                + *accumulated / health.maximum() as f32;
                                        },
                                        ModifierKind::Fractional => {
                                            stat.max_health_modifier *=
                                                current_fraction * (1.0 - *accumulated);
                                        },
                                    }
                                    *accumulated = 0.0;
                                } else {
                                    stat.max_health_modifier *= current_fraction;
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

fn tick_buff(
    id: u64,
    buff: &mut Buff,
    dt: f32,
    in_fluid: Option<Fluid>,
    mut expire_buff: impl FnMut(u64),
) {
    // If a buff is recently applied from an aura, do not tick duration
    if buff
        .cat_ids
        .iter()
        .any(|cat_id| matches!(cat_id, BuffCategory::FromAura(true)))
    {
        return;
    }
    if let Some(remaining_time) = &mut buff.time {
        // Extinguish Burning buff when in water
        if matches!(buff.kind, BuffKind::Burning)
            && matches!(
                in_fluid,
                Some(Fluid::Liquid {
                    kind: LiquidKind::Water,
                    ..
                })
            )
        {
            *remaining_time = Duration::default();
        }

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
