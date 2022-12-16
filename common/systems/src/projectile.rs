use common::{
    combat::{self, AttackOptions, AttackSource, AttackerInfo, TargetInfo},
    comp::{
        agent::{Sound, SoundKind},
        projectile, Alignment, Body, Buffs, CharacterState, Combo, Energy, Group, Health,
        Inventory, Ori, PhysicsState, Player, Pos, Projectile, Stats, Vel,
    },
    event::{Emitter, EventBus, ServerEvent},
    outcome::Outcome,
    resources::{DeltaTime, Time},
    uid::{Uid, UidAllocator},
    util::Dir,
    GroupTarget,
};

use common::vol::ReadVol;
use common_ecs::{Job, Origin, Phase, System};
use rand::{thread_rng, Rng};
use specs::{
    saveload::MarkerAllocator, shred::ResourceId, Entities, Entity as EcsEntity, Join, Read,
    ReadExpect, ReadStorage, SystemData, World, WriteStorage,
};
use std::time::Duration;
use vek::*;

use common::terrain::TerrainGrid;

#[derive(SystemData)]
pub struct ReadData<'a> {
    time: Read<'a, Time>,
    entities: Entities<'a>,
    players: ReadStorage<'a, Player>,
    dt: Read<'a, DeltaTime>,
    uid_allocator: Read<'a, UidAllocator>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    uids: ReadStorage<'a, Uid>,
    positions: ReadStorage<'a, Pos>,
    alignments: ReadStorage<'a, Alignment>,
    physics_states: ReadStorage<'a, PhysicsState>,
    velocities: ReadStorage<'a, Vel>,
    inventories: ReadStorage<'a, Inventory>,
    groups: ReadStorage<'a, Group>,
    energies: ReadStorage<'a, Energy>,
    stats: ReadStorage<'a, Stats>,
    combos: ReadStorage<'a, Combo>,
    healths: ReadStorage<'a, Health>,
    bodies: ReadStorage<'a, Body>,
    character_states: ReadStorage<'a, CharacterState>,
    terrain: ReadExpect<'a, TerrainGrid>,
    buffs: ReadStorage<'a, Buffs>,
}

/// This system is responsible for handling projectile effect triggers
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Projectile>,
        Read<'a, EventBus<Outcome>>,
    );

    const NAME: &'static str = "projectile";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (read_data, mut orientations, mut projectiles, outcomes): Self::SystemData,
    ) {
        let mut server_emitter = read_data.server_bus.emitter();
        let mut outcomes_emitter = outcomes.emitter();

        // Attacks
        'projectile_loop: for (entity, pos, physics, vel, mut projectile) in (
            &read_data.entities,
            &read_data.positions,
            &read_data.physics_states,
            &read_data.velocities,
            &mut projectiles,
        )
            .join()
        {
            let projectile_owner = projectile
                .owner
                .and_then(|uid| read_data.uid_allocator.retrieve_entity_internal(uid.into()));

            let mut rng = thread_rng();
            if physics.on_surface().is_none() && rng.gen_bool(0.05) {
                server_emitter.emit(ServerEvent::Sound {
                    sound: Sound::new(SoundKind::Projectile, pos.0, 4.0, read_data.time.0),
                });
            }

            let mut projectile_vanished: bool = false;

            // Hit entity
            for (&other, &pos_hit_other) in physics.touch_entities.iter() {
                let same_group = projectile_owner
                    // Note: somewhat inefficient since we do the lookup for every touching
                    // entity, but if we pull this out of the loop we would want to do it only
                    // if there is at least one touching entity
                    .and_then(|e| read_data.groups.get(e))
                    .map_or(false, |owner_group|
                        Some(owner_group) == read_data.uid_allocator
                        .retrieve_entity_internal(other.into())
                        .and_then(|e| read_data.groups.get(e))
                    );

                // Skip if in the same group
                let target_group = if same_group {
                    GroupTarget::InGroup
                } else {
                    GroupTarget::OutOfGroup
                };

                if projectile.ignore_group && same_group {
                    continue;
                }

                if projectile.owner == Some(other) {
                    continue;
                }

                let projectile = &mut *projectile;

                let entity_of =
                    |uid: Uid| read_data.uid_allocator.retrieve_entity_internal(uid.into());

                // Don't hit if there is terrain between the projectile and where the entity was
                // supposed to be hit by it.

                if physics.on_surface().is_some() {
                    let projectile_direction = orientations
                        .get(entity)
                        .map_or_else(Vec3::zero, |ori| ori.look_vec());
                    let pos_wall = pos.0 - 0.2 * projectile_direction;
                    if !matches!(
                        read_data
                            .terrain
                            .ray(pos_wall, pos_hit_other)
                            .until(|b| b.is_filled())
                            .cast()
                            .1,
                        Ok(None)
                    ) {
                        continue;
                    }
                }

                for effect in projectile.hit_entity.drain(..) {
                    let owner = projectile.owner.and_then(entity_of);
                    let projectile_info = ProjectileInfo {
                        entity,
                        effect,
                        owner_uid: projectile.owner,
                        owner,
                        ori: orientations.get(entity),
                        pos,
                    };

                    let target = entity_of(other);
                    let projectile_target_info = ProjectileTargetInfo {
                        uid: other,
                        entity: target,
                        target_group,
                        ori: target.and_then(|target| orientations.get(target)),
                    };

                    dispatch_hit(
                        projectile_info,
                        projectile_target_info,
                        &read_data,
                        &mut projectile_vanished,
                        &mut outcomes_emitter,
                        &mut server_emitter,
                    );
                }

                if projectile_vanished {
                    continue 'projectile_loop;
                }
            }

            if physics.on_surface().is_some() {
                let projectile = &mut *projectile;
                for effect in projectile.hit_solid.drain(..) {
                    match effect {
                        projectile::Effect::Explode(e) => {
                            // We offset position a little back on the way,
                            // so if we hit non-exploadable block
                            // we still can affect blocks around it.
                            //
                            // TODO: orientation of fallen projectile is
                            // fragile heuristic for direction, find more
                            // robust method.
                            let projectile_direction = orientations
                                .get(entity)
                                .map_or_else(Vec3::zero, |ori| ori.look_vec());
                            let offset = -0.2 * projectile_direction;
                            server_emitter.emit(ServerEvent::Explosion {
                                pos: pos.0 + offset,
                                explosion: e,
                                owner: projectile.owner,
                            });
                        },
                        projectile::Effect::Vanish => {
                            server_emitter.emit(ServerEvent::Delete(entity));
                            projectile_vanished = true;
                        },
                        projectile::Effect::Bonk => {
                            server_emitter.emit(ServerEvent::Bonk {
                                pos: pos.0,
                                owner: projectile.owner,
                                target: None,
                            });
                        },
                        _ => {},
                    }
                }

                if projectile_vanished {
                    continue 'projectile_loop;
                }
            } else if let Some(ori) = orientations.get_mut(entity) {
                if let Some(dir) = Dir::from_unnormalized(vel.0) {
                    *ori = dir.into();
                }
            }

            if projectile.time_left == Duration::default() {
                server_emitter.emit(ServerEvent::Delete(entity));
            }
            projectile.time_left = projectile
                .time_left
                .checked_sub(Duration::from_secs_f32(read_data.dt.0))
                .unwrap_or_default();
        }
    }
}

struct ProjectileInfo<'a> {
    entity: EcsEntity,
    effect: projectile::Effect,
    owner_uid: Option<Uid>,
    owner: Option<EcsEntity>,
    ori: Option<&'a Ori>,
    pos: &'a Pos,
}

struct ProjectileTargetInfo<'a> {
    uid: Uid,
    entity: Option<EcsEntity>,
    target_group: GroupTarget,
    ori: Option<&'a Ori>,
}

fn dispatch_hit(
    projectile_info: ProjectileInfo,
    projectile_target_info: ProjectileTargetInfo,
    read_data: &ReadData,
    projectile_vanished: &mut bool,
    outcomes_emitter: &mut Emitter<Outcome>,
    server_emitter: &mut Emitter<ServerEvent>,
) {
    match projectile_info.effect {
        projectile::Effect::Attack(attack) => {
            let target_uid = projectile_target_info.uid;
            let target = if let Some(entity) = projectile_target_info.entity {
                entity
            } else {
                return;
            };

            let (target_pos, projectile_dir) = {
                let target_pos = read_data.positions.get(target);
                let projectile_ori = projectile_info.ori;
                match target_pos.zip(projectile_ori) {
                    Some((tgt_pos, proj_ori)) => {
                        let Pos(tgt_pos) = tgt_pos;
                        (*tgt_pos, proj_ori.look_dir())
                    },
                    None => return,
                }
            };

            let owner = projectile_info.owner;
            let projectile_entity = projectile_info.entity;

            let attacker_info =
                owner
                    .zip(projectile_info.owner_uid)
                    .map(|(entity, uid)| AttackerInfo {
                        entity,
                        uid,
                        group: read_data.groups.get(entity),
                        energy: read_data.energies.get(entity),
                        combo: read_data.combos.get(entity),
                        inventory: read_data.inventories.get(entity),
                        stats: read_data.stats.get(entity),
                    });

            let target_info = TargetInfo {
                entity: target,
                uid: target_uid,
                inventory: read_data.inventories.get(target),
                stats: read_data.stats.get(target),
                health: read_data.healths.get(target),
                pos: target_pos,
                ori: projectile_target_info.ori,
                char_state: read_data.character_states.get(target),
                energy: read_data.energies.get(target),
                buffs: read_data.buffs.get(target),
            };

            // TODO: Is it possible to have projectile without body??
            if let Some(&body) = read_data.bodies.get(projectile_entity) {
                outcomes_emitter.emit(Outcome::ProjectileHit {
                    pos: target_pos,
                    body,
                    vel: read_data
                        .velocities
                        .get(projectile_entity)
                        .map_or(Vec3::zero(), |v| v.0),
                    source: projectile_info.owner_uid,
                    target: read_data.uids.get(target).copied(),
                });
            }

            // PvP check
            let may_harm = combat::may_harm(
                &read_data.alignments,
                &read_data.players,
                &read_data.uid_allocator,
                owner,
                target,
            );

            let target_dodging = read_data
                .character_states
                .get(target)
                .and_then(|cs| cs.attack_immunities())
                .map_or(false, |i| i.projectiles);
            let attack_options = AttackOptions {
                target_dodging,
                may_harm,
                target_group: projectile_target_info.target_group,
            };

            attack.apply_attack(
                attacker_info,
                target_info,
                projectile_dir,
                attack_options,
                1.0,
                AttackSource::Projectile,
                *read_data.time,
                |e| server_emitter.emit(e),
                |o| outcomes_emitter.emit(o),
            );
        },
        projectile::Effect::Explode(e) => {
            let Pos(pos) = *projectile_info.pos;
            let owner_uid = projectile_info.owner_uid;
            server_emitter.emit(ServerEvent::Explosion {
                pos,
                explosion: e,
                owner: owner_uid,
            });
        },
        projectile::Effect::Bonk => {
            let Pos(pos) = *projectile_info.pos;
            let owner_uid = projectile_info.owner_uid;
            server_emitter.emit(ServerEvent::Bonk {
                pos,
                owner: owner_uid,
                target: Some(projectile_target_info.uid),
            });
        },
        projectile::Effect::Vanish => {
            let entity = projectile_info.entity;
            server_emitter.emit(ServerEvent::Delete(entity));
            *projectile_vanished = true;
        },
        projectile::Effect::Possess => {
            let target_uid = projectile_target_info.uid;
            let owner_uid = projectile_info.owner_uid;
            if let Some(owner_uid) = owner_uid {
                if target_uid != owner_uid {
                    server_emitter.emit(ServerEvent::Possess(owner_uid, target_uid));
                }
            }
        },
        projectile::Effect::Stick => {},
    }
}
