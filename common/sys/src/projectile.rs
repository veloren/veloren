use common::{
    comp::{
        buff::{Buff, BuffChange, BuffSource},
        projectile, EnergyChange, EnergySource, Group, HealthSource, Inventory, Ori, PhysicsState,
        Pos, Projectile, Vel,
    },
    event::{EventBus, ServerEvent},
    metrics::SysMetrics,
    resources::DeltaTime,
    span,
    uid::UidAllocator,
    GroupTarget,
};
use rand::{thread_rng, Rng};
use specs::{
    saveload::MarkerAllocator, Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage,
};
use std::time::Duration;

/// This system is responsible for handling projectile effect triggers
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        Read<'a, UidAllocator>,
        Read<'a, EventBus<ServerEvent>>,
        ReadExpect<'a, SysMetrics>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, PhysicsState>,
        ReadStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Projectile>,
        ReadStorage<'a, Inventory>,
        ReadStorage<'a, Group>,
    );

    fn run(
        &mut self,
        (
            entities,
            dt,
            uid_allocator,
            server_bus,
            sys_metrics,
            positions,
            physics_states,
            velocities,
            mut orientations,
            mut projectiles,
            inventories,
            groups,
        ): Self::SystemData,
    ) {
        let start_time = std::time::Instant::now();
        span!(_guard, "run", "projectile::Sys::run");
        let mut server_emitter = server_bus.emitter();

        // Attacks
        for (entity, pos, physics, ori, mut projectile) in (
            &entities,
            &positions,
            &physics_states,
            &mut orientations,
            &mut projectiles,
        )
            .join()
        {
            // Hit entity
            for other in physics.touch_entities.iter().copied() {
                let same_group = projectile
                    .owner
                    // Note: somewhat inefficient since we do the lookup for every touching
                    // entity, but if we pull this out of the loop we would want to do it only
                    // if there is at least one touching entity
                    .and_then(|uid| uid_allocator.retrieve_entity_internal(uid.into()))
                    .and_then(|e| groups.get(e))
                    .map_or(false, |owner_group|
                        Some(owner_group) == uid_allocator
                        .retrieve_entity_internal(other.into())
                        .and_then(|e| groups.get(e))
                    );

                let target_group = if same_group {
                    GroupTarget::InGroup
                } else {
                    GroupTarget::OutOfGroup
                };

                if projectile.ignore_group
                    // Skip if in the same group
                    && same_group
                {
                    continue;
                }

                if projectile.owner == Some(other) {
                    continue;
                }

                let projectile = &mut *projectile;
                for effect in projectile.hit_entity.drain(..) {
                    match effect {
                        projectile::Effect::Damage(target, damage) => {
                            if Some(other) == projectile.owner {
                                continue;
                            }

                            if let Some(target) = target {
                                if target != target_group {
                                    continue;
                                }
                            }

                            if let Some(other_entity) =
                                uid_allocator.retrieve_entity_internal(other.into())
                            {
                                let other_entity_inventory = inventories.get(other_entity);
                                let change =
                                    damage.modify_damage(other_entity_inventory, projectile.owner);
                                server_emitter.emit(ServerEvent::Damage {
                                    entity: other_entity,
                                    change,
                                });
                            }
                        },
                        projectile::Effect::PoiseChange(target, poise_damage) => {
                            if Some(other) == projectile.owner {
                                continue;
                            }

                            if let Some(target) = target {
                                if target != target_group {
                                    continue;
                                }
                            }

                            if let Some(other_entity) =
                                uid_allocator.retrieve_entity_internal(other.into())
                            {
                                let other_entity_inventory = inventories.get(other_entity);
                                let poise_change =
                                    poise_damage.modify_poise_damage(other_entity_inventory);
                                server_emitter.emit(ServerEvent::PoiseChange {
                                    entity: other_entity,
                                    change: poise_change,
                                    kb_dir: *ori.0,
                                });
                            }
                        },
                        projectile::Effect::Knockback(knockback) => {
                            if let Some(other_entity) =
                                uid_allocator.retrieve_entity_internal(other.into())
                            {
                                let impulse = knockback.calculate_impulse(ori.0);
                                if !impulse.is_approx_zero() {
                                    server_emitter.emit(ServerEvent::Knockback {
                                        entity: other_entity,
                                        impulse,
                                    });
                                }
                            }
                        },
                        projectile::Effect::RewardEnergy(energy) => {
                            if let Some(entity_owner) = projectile
                                .owner
                                .and_then(|u| uid_allocator.retrieve_entity_internal(u.into()))
                            {
                                server_emitter.emit(ServerEvent::EnergyChange {
                                    entity: entity_owner,
                                    change: EnergyChange {
                                        amount: energy as i32,
                                        source: EnergySource::HitEnemy,
                                    },
                                });
                            }
                        },
                        projectile::Effect::Explode(e) => {
                            server_emitter.emit(ServerEvent::Explosion {
                                pos: pos.0,
                                explosion: e,
                                owner: projectile.owner,
                                reagent: None,
                            })
                        },
                        projectile::Effect::Vanish => server_emitter.emit(ServerEvent::Destroy {
                            entity,
                            cause: HealthSource::World,
                        }),
                        projectile::Effect::Possess => {
                            if other != projectile.owner.unwrap() {
                                if let Some(owner) = projectile.owner {
                                    server_emitter.emit(ServerEvent::Possess(owner, other));
                                }
                            }
                        },
                        // TODO: Change to effect after !1472 merges
                        projectile::Effect::Buff { buff, chance } => {
                            if let Some(entity) =
                                uid_allocator.retrieve_entity_internal(other.into())
                            {
                                if chance.map_or(true, |c| thread_rng().gen::<f32>() < c) {
                                    let source = if let Some(owner) = projectile.owner {
                                        BuffSource::Character { by: owner }
                                    } else {
                                        BuffSource::Unknown
                                    };
                                    let buff =
                                        Buff::new(buff.kind, buff.data, buff.cat_ids, source);
                                    server_emitter.emit(ServerEvent::Buff {
                                        entity,
                                        buff_change: BuffChange::Add(buff),
                                    });
                                }
                            }
                        },
                        _ => {},
                    }
                }
            }

            // Hit something solid
            if physics.on_wall.is_some() || physics.on_ground || physics.on_ceiling {
                let projectile = &mut *projectile;
                for effect in projectile.hit_solid.drain(..) {
                    match effect {
                        projectile::Effect::Explode(e) => {
                            server_emitter.emit(ServerEvent::Explosion {
                                pos: pos.0,
                                explosion: e,
                                owner: projectile.owner,
                                reagent: None,
                            })
                        },
                        projectile::Effect::Vanish => server_emitter.emit(ServerEvent::Destroy {
                            entity,
                            cause: HealthSource::World,
                        }),
                        _ => {},
                    }
                }
            } else if let Some(dir) = velocities
                .get(entity)
                .and_then(|vel| vel.0.try_normalized())
            {
                ori.0 = dir.into();
            }

            if projectile.time_left == Duration::default() {
                server_emitter.emit(ServerEvent::Destroy {
                    entity,
                    cause: HealthSource::World,
                });
            }
            projectile.time_left = projectile
                .time_left
                .checked_sub(Duration::from_secs_f32(dt.0))
                .unwrap_or_default();
        }
        sys_metrics.projectile_ns.store(
            start_time.elapsed().as_nanos() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}
