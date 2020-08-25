use crate::{
    comp::{
        projectile, Damage, DamageSource, Energy, EnergySource, HealthChange, HealthSource,
        Loadout, Ori, PhysicsState, Pos, Projectile, Vel,
    },
    event::{EventBus, LocalEvent, ServerEvent},
    state::DeltaTime,
    sync::UidAllocator,
    util::Dir,
};
use specs::{saveload::MarkerAllocator, Entities, Join, Read, ReadStorage, System, WriteStorage};
use std::time::Duration;
use vek::*;

/// This system is responsible for handling projectile effect triggers
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        Read<'a, UidAllocator>,
        Read<'a, EventBus<LocalEvent>>,
        Read<'a, EventBus<ServerEvent>>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, PhysicsState>,
        ReadStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Projectile>,
        WriteStorage<'a, Energy>,
        ReadStorage<'a, Loadout>,
    );

    fn run(
        &mut self,
        (
            entities,
            dt,
            uid_allocator,
            local_bus,
            server_bus,
            positions,
            physics_states,
            velocities,
            mut orientations,
            mut projectiles,
            mut energies,
            loadouts,
        ): Self::SystemData,
    ) {
        let mut local_emitter = local_bus.emitter();
        let mut server_emitter = server_bus.emitter();

        // Attacks
        for (entity, pos, physics, ori, projectile) in (
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
                if projectile.owner == Some(other) {
                    continue;
                }

                for effect in projectile.hit_entity.iter().cloned() {
                    match effect {
                        projectile::Effect::Damage(healthchange) => {
                            let owner_uid = projectile.owner.unwrap();
                            let mut damage = Damage {
                                healthchange: healthchange as f32,
                                source: DamageSource::Projectile,
                            };

                            let other_entity = uid_allocator.retrieve_entity_internal(other.into());
                            if let Some(loadout) = other_entity.and_then(|e| loadouts.get(e)) {
                                damage.modify_damage(false, loadout);
                            }

                            if other != owner_uid {
                                server_emitter.emit(ServerEvent::Damage {
                                    uid: other,
                                    change: HealthChange {
                                        amount: damage.healthchange as i32,
                                        cause: HealthSource::Attack { by: owner_uid },
                                    },
                                });
                            }
                        },
                        projectile::Effect::Knockback(knockback) => {
                            if let Some(entity) =
                                uid_allocator.retrieve_entity_internal(other.into())
                            {
                                local_emitter.emit(LocalEvent::ApplyForce {
                                    entity,
                                    force: knockback
                                        * *Dir::slerp(ori.0, Dir::new(Vec3::unit_z()), 0.5),
                                });
                            }
                        },
                        projectile::Effect::RewardEnergy(energy) => {
                            if let Some(energy_mut) = projectile
                                .owner
                                .and_then(|o| uid_allocator.retrieve_entity_internal(o.into()))
                                .and_then(|o| energies.get_mut(o))
                            {
                                energy_mut.change_by(energy as i32, EnergySource::HitEnemy);
                            }
                        },
                        projectile::Effect::Explode { power } => {
                            server_emitter.emit(ServerEvent::Explosion {
                                pos: pos.0,
                                power,
                                owner: projectile.owner,
                                friendly_damage: false,
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
                        _ => {},
                    }
                }
            }

            // Hit something solid
            if physics.on_wall.is_some() || physics.on_ground || physics.on_ceiling {
                for effect in projectile.hit_solid.drain(..) {
                    match effect {
                        projectile::Effect::Explode { power } => {
                            server_emitter.emit(ServerEvent::Explosion {
                                pos: pos.0,
                                power,
                                owner: projectile.owner,
                                friendly_damage: false,
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
            }

            if let Some(dir) = velocities
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
    }
}
