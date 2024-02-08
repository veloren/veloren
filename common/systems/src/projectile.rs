use common::{
    combat::{self, AttackOptions, AttackSource, AttackerInfo, TargetInfo},
    comp::{
        agent::{Sound, SoundKind},
        projectile, Alignment, Body, Buffs, CharacterState, Combo, Energy, Group, Health,
        Inventory, Ori, PhysicsState, Player, Pos, Projectile, Stats, Vel,
    },
    event::{
        BonkEvent, BuffEvent, ComboChangeEvent, DeleteEvent, EmitExt, Emitter, EnergyChangeEvent,
        EntityAttackedHookEvent, EventBus, ExplosionEvent, HealthChangeEvent, KnockbackEvent,
        ParryHookEvent, PoiseChangeEvent, PossessEvent, SoundEvent,
    },
    event_emitters,
    outcome::Outcome,
    resources::{DeltaTime, Time},
    uid::{IdMaps, Uid},
    util::Dir,
    GroupTarget,
};

use common::vol::ReadVol;
use common_ecs::{Job, Origin, Phase, System};
use rand::Rng;
use specs::{
    shred, Entities, Entity as EcsEntity, Join, Read, ReadExpect, ReadStorage, SystemData,
    WriteStorage,
};
use std::time::Duration;
use vek::*;

use common::terrain::TerrainGrid;

event_emitters! {
    struct Events[Emitters] {
        sound: SoundEvent,
        delete: DeleteEvent,
        explosion: ExplosionEvent,
        health_change: HealthChangeEvent,
        energy_change: EnergyChangeEvent,
        poise_change: PoiseChangeEvent,
        parry_hook: ParryHookEvent,
        kockback: KnockbackEvent,
        entity_attack_hoow: EntityAttackedHookEvent,
        combo_change: ComboChangeEvent,
        buff: BuffEvent,
        bonk: BonkEvent,
        possess: PossessEvent,
    }
}

#[derive(SystemData)]
pub struct ReadData<'a> {
    time: Read<'a, Time>,
    entities: Entities<'a>,
    players: ReadStorage<'a, Player>,
    dt: Read<'a, DeltaTime>,
    id_maps: Read<'a, IdMaps>,
    events: Events<'a>,
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
        let mut emitters = read_data.events.get_emitters();
        let mut outcomes_emitter = outcomes.emitter();
        let mut rng = rand::thread_rng();

        // Attacks
        'projectile_loop: for (entity, pos, physics, vel, projectile) in (
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
                .and_then(|uid| read_data.id_maps.uid_entity(uid));

            if physics.on_surface().is_none() && rng.gen_bool(0.05) {
                emitters.emit(SoundEvent {
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
                        Some(owner_group) == read_data.id_maps
                        .uid_entity(other)
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

                let entity_of = |uid: Uid| read_data.id_maps.uid_entity(uid);

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
                        vel,
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
                        &mut emitters,
                        &mut rng,
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
                            emitters.emit(ExplosionEvent {
                                pos: pos.0 + offset,
                                explosion: e,
                                owner: projectile.owner,
                            });
                        },
                        projectile::Effect::Vanish => {
                            emitters.emit(DeleteEvent(entity));
                            projectile_vanished = true;
                        },
                        projectile::Effect::Bonk => {
                            emitters.emit(BonkEvent {
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
                emitters.emit(DeleteEvent(entity));
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
    vel: &'a Vel,
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
    emitters: &mut Emitters,
    rng: &mut rand::rngs::ThreadRng,
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
                &read_data.id_maps,
                owner,
                target,
            );

            let target_dodging = read_data
                .character_states
                .get(target)
                .and_then(|cs| cs.attack_immunities())
                .map_or(false, |i| i.projectiles);

            let precision_from_flank =
                combat::precision_mult_from_flank(*projectile_dir, target_info.ori);

            let precision_from_head = {
                // This performs a cylinder and line segment intersection check. The cylinder is
                // the upper 10% of an entity's dimensions. The line segment is from the
                // projectile's positions on the current and previous tick.
                let curr_pos = projectile_info.pos.0;
                let last_pos = projectile_info.pos.0 - projectile_info.vel.0 * read_data.dt.0;
                let vel = projectile_info.vel.0;
                let (target_height, target_radius) = read_data
                    .bodies
                    .get(target)
                    .map_or((0.0, 0.0), |b| (b.height(), b.max_radius()));
                let head_top_pos = target_pos.with_z(target_pos.z + target_height);
                let head_bottom_pos = head_top_pos.with_z(
                    head_top_pos.z - target_height * combat::PROJECTILE_HEADSHOT_PROPORTION,
                );
                if (curr_pos.z < head_bottom_pos.z && last_pos.z < head_bottom_pos.z)
                    || (curr_pos.z > head_top_pos.z && last_pos.z > head_top_pos.z)
                {
                    None
                } else if curr_pos.z > head_top_pos.z
                    || curr_pos.z < head_bottom_pos.z
                    || last_pos.z > head_top_pos.z
                    || last_pos.z < head_bottom_pos.z
                {
                    let proj_top_intersection = {
                        let t = (head_top_pos.z - last_pos.z) / vel.z;
                        last_pos + vel * t
                    };
                    let proj_bottom_intersection = {
                        let t = (head_bottom_pos.z - last_pos.z) / vel.z;
                        last_pos + vel * t
                    };
                    let intersected_bottom = head_bottom_pos
                        .distance_squared(proj_bottom_intersection)
                        < target_radius.powi(2);
                    let intersected_top = head_top_pos.distance_squared(proj_top_intersection)
                        < target_radius.powi(2);
                    let hit_head = intersected_bottom || intersected_top;
                    let hit_from_bottom = last_pos.z < head_bottom_pos.z && intersected_bottom;
                    let hit_from_top = last_pos.z > head_top_pos.z && intersected_top;
                    // If projectile from bottom, do not award precision damage because it trivial
                    // to get from up close If projectile from top, reduce
                    // precision damage to mitigate cheesing benefits
                    if !hit_head || hit_from_bottom {
                        None
                    } else if hit_from_top {
                        Some(combat::MAX_TOP_HEADSHOT_PRECISION)
                    } else {
                        Some(combat::MAX_HEADSHOT_PRECISION)
                    }
                } else {
                    let trajectory = LineSegment3 {
                        start: last_pos,
                        end: curr_pos,
                    };
                    let head_middle_pos = head_bottom_pos.with_z(
                        head_bottom_pos.z
                            + target_height * combat::PROJECTILE_HEADSHOT_PROPORTION * 0.5,
                    );
                    if trajectory.distance_to_point(head_middle_pos) < target_radius {
                        Some(combat::MAX_HEADSHOT_PRECISION)
                    } else {
                        None
                    }
                }
            };

            let precision_mult = match (precision_from_flank, precision_from_head) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (Some(a), None) | (None, Some(a)) => Some(a),
                (None, None) => None,
            };

            let attack_options = AttackOptions {
                target_dodging,
                may_harm,
                target_group: projectile_target_info.target_group,
                precision_mult,
            };

            attack.apply_attack(
                attacker_info,
                &target_info,
                projectile_dir,
                attack_options,
                1.0,
                AttackSource::Projectile,
                *read_data.time,
                emitters,
                |o| outcomes_emitter.emit(o),
                rng,
                0,
            );
        },
        projectile::Effect::Explode(e) => {
            let Pos(pos) = *projectile_info.pos;
            let owner_uid = projectile_info.owner_uid;
            emitters.emit(ExplosionEvent {
                pos,
                explosion: e,
                owner: owner_uid,
            });
        },
        projectile::Effect::Bonk => {
            let Pos(pos) = *projectile_info.pos;
            let owner_uid = projectile_info.owner_uid;
            emitters.emit(BonkEvent {
                pos,
                owner: owner_uid,
                target: Some(projectile_target_info.uid),
            });
        },
        projectile::Effect::Vanish => {
            let entity = projectile_info.entity;
            emitters.emit(DeleteEvent(entity));
            *projectile_vanished = true;
        },
        projectile::Effect::Possess => {
            let target_uid = projectile_target_info.uid;
            let owner_uid = projectile_info.owner_uid;
            if let Some(owner_uid) = owner_uid {
                if target_uid != owner_uid {
                    emitters.emit(PossessEvent(owner_uid, target_uid));
                }
            }
        },
        projectile::Effect::Stick => {},
    }
}
