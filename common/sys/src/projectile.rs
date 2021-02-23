use common::{
    combat::AttackerInfo,
    comp::{
        projectile, Energy, Group, HealthSource, Inventory, Ori, PhysicsState, Pos, Projectile, Vel,
    },
    event::{EventBus, ServerEvent},
    metrics::SysMetrics,
    resources::DeltaTime,
    span,
    uid::UidAllocator,
    util::Dir,
    GroupTarget,
};
use specs::{
    saveload::MarkerAllocator, shred::ResourceId, Entities, Join, Read, ReadExpect, ReadStorage,
    System, SystemData, World, WriteStorage,
};
use std::time::Duration;

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    dt: Read<'a, DeltaTime>,
    uid_allocator: Read<'a, UidAllocator>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    metrics: ReadExpect<'a, SysMetrics>,
    positions: ReadStorage<'a, Pos>,
    physics_states: ReadStorage<'a, PhysicsState>,
    velocities: ReadStorage<'a, Vel>,
    inventories: ReadStorage<'a, Inventory>,
    groups: ReadStorage<'a, Group>,
    energies: ReadStorage<'a, Energy>,
}

/// This system is responsible for handling projectile effect triggers
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Projectile>,
    );

    fn run(&mut self, (read_data, mut orientations, mut projectiles): Self::SystemData) {
        let start_time = std::time::Instant::now();
        span!(_guard, "run", "projectile::Sys::run");
        let mut server_emitter = read_data.server_bus.emitter();

        // Attacks
        'projectile_loop: for (entity, pos, physics, ori, mut projectile) in (
            &read_data.entities,
            &read_data.positions,
            &read_data.physics_states,
            &mut orientations,
            &mut projectiles,
        )
            .join()
        {
            let mut projectile_vanished: bool = false;
            // Hit entity
            for other in physics.touch_entities.iter().copied() {
                let same_group = projectile
                    .owner
                    // Note: somewhat inefficient since we do the lookup for every touching
                    // entity, but if we pull this out of the loop we would want to do it only
                    // if there is at least one touching entity
                    .and_then(|uid| read_data.uid_allocator.retrieve_entity_internal(uid.into()))
                    .and_then(|e| read_data.groups.get(e))
                    .map_or(false, |owner_group|
                        Some(owner_group) == read_data.uid_allocator
                        .retrieve_entity_internal(other.into())
                        .and_then(|e| read_data.groups.get(e))
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
                        projectile::Effect::Attack(attack) => {
                            if let Some(target_entity) = read_data
                                .uid_allocator
                                .retrieve_entity_internal(other.into())
                            {
                                let owner_entity = projectile.owner.and_then(|u| {
                                    read_data.uid_allocator.retrieve_entity_internal(u.into())
                                });

                                let attacker_info =
                                    owner_entity.zip(projectile.owner).map(|(entity, uid)| {
                                        AttackerInfo {
                                            entity,
                                            uid,
                                            energy: read_data.energies.get(entity),
                                        }
                                    });

                                attack.apply_attack(
                                    target_group,
                                    attacker_info,
                                    target_entity,
                                    read_data.inventories.get(target_entity),
                                    ori.look_dir(),
                                    false,
                                    1.0,
                                    |e| server_emitter.emit(e),
                                );
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
                        projectile::Effect::Vanish => {
                            server_emitter.emit(ServerEvent::Destroy {
                                entity,
                                cause: HealthSource::World,
                            });
                            projectile_vanished = true;
                        },
                        projectile::Effect::Possess => {
                            if other != projectile.owner.unwrap() {
                                if let Some(owner) = projectile.owner {
                                    server_emitter.emit(ServerEvent::Possess(owner, other));
                                }
                            }
                        },
                        _ => {},
                    }
                }

                if projectile_vanished {
                    continue 'projectile_loop;
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
                        projectile::Effect::Vanish => {
                            server_emitter.emit(ServerEvent::Destroy {
                                entity,
                                cause: HealthSource::World,
                            });
                            projectile_vanished = true;
                        },
                        _ => {},
                    }
                }

                if projectile_vanished {
                    continue 'projectile_loop;
                }
            } else if let Some(dir) = read_data
                .velocities
                .get(entity)
                .and_then(|vel| Dir::from_unnormalized(vel.0))
            {
                *ori = dir.into();
            }

            if projectile.time_left == Duration::default() {
                server_emitter.emit(ServerEvent::Destroy {
                    entity,
                    cause: HealthSource::World,
                });
            }
            projectile.time_left = projectile
                .time_left
                .checked_sub(Duration::from_secs_f32(read_data.dt.0))
                .unwrap_or_default();
        }
        read_data.metrics.projectile_ns.store(
            start_time.elapsed().as_nanos() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}
