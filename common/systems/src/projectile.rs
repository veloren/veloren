use common::{
    Damage, DamageKind, Explosion, GroupTarget, RadiusEffect,
    combat::{self, AttackOptions, AttackSource, AttackerInfo, TargetInfo},
    comp::{
        Alignment, Body, Buffs, CharacterState, Combo, Content, Energy, Group, Health, Inventory,
        Mass, Ori, PhysicsState, Player, Poise, Pos, Projectile, Stats, Vel,
        agent::{Sound, SoundKind},
        aura::EnteredAuras,
        object, projectile,
    },
    effect,
    event::{
        ArcEvent, BonkEvent, BuffEvent, ComboChangeEvent, CreateNpcEvent, DeleteEvent, EmitExt,
        Emitter, EnergyChangeEvent, EntityAttackedHookEvent, EventBus, ExplosionEvent,
        HealthChangeEvent, KnockbackEvent, NpcBuilder, ParryHookEvent, PoiseChangeEvent,
        PossessEvent, ShootEvent, SoundEvent, TransformEvent,
    },
    event_emitters,
    outcome::Outcome,
    resources::{DeltaTime, Secs, Time},
    uid::{IdMaps, Uid},
    util::Dir,
};

use common::vol::ReadVol;
use common_ecs::{Job, Origin, Phase, System};
use rand::Rng;
use specs::{
    Entities, Entity as EcsEntity, Join, Read, ReadExpect, ReadStorage, SystemData, WriteStorage,
    shred,
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
        knockback: KnockbackEvent,
        entity_attack_hook: EntityAttackedHookEvent,
        shoot: ShootEvent,
        create_npc: CreateNpcEvent,
        combo_change: ComboChangeEvent,
        buff: BuffEvent,
        bonk: BonkEvent,
        possess: PossessEvent,
        arc: ArcEvent,
        transform: TransformEvent,
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
    entered_auras: ReadStorage<'a, EnteredAuras>,
    masses: ReadStorage<'a, Mass>,
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
        WriteStorage<'a, Vel>,
    );

    const NAME: &'static str = "projectile";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (read_data, mut orientations, mut projectiles, outcomes, mut velocities): Self::SystemData,
    ) {
        let mut emitters = read_data.events.get_emitters();
        let mut outcomes_emitter = outcomes.emitter();
        let mut rng = rand::rng();

        // Attacks
        'projectile_loop: for (entity, pos, physics, body, projectile) in (
            &read_data.entities,
            &read_data.positions,
            &read_data.physics_states,
            &read_data.bodies,
            &mut projectiles,
        )
            .join()
        {
            let projectile_owner = projectile
                .owner
                .and_then(|uid| read_data.id_maps.uid_entity(uid));

            if physics.on_surface().is_none() && rng.random_bool(0.05) {
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
                    .and_then(|e| read_data.groups.get(e)).is_some_and(|owner_group|
                        Some(owner_group) == read_data.id_maps
                        .uid_entity(other)
                        .and_then(|e| read_data.groups.get(e)));

                // Skip if in the same group
                let target_group = if same_group {
                    GroupTarget::InGroup
                } else {
                    GroupTarget::OutOfGroup
                };

                if projectile.ignore_group
                    && same_group
                    && projectile
                        .owner
                        .and_then(|owner| {
                            read_data
                                .id_maps
                                .uid_entity(owner)
                                .zip(read_data.id_maps.uid_entity(other))
                        })
                        .is_none_or(|(owner, other)| {
                            !combat::allow_friendly_fire(&read_data.entered_auras, owner, other)
                        })
                {
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
                        vel: velocities.get(entity).map_or(Vec3::zero(), |v| v.0),
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
                        projectile::Effect::SurpriseEgg => {
                            outcomes_emitter.emit(Outcome::SurpriseEgg { pos: pos.0 });
                        },
                        projectile::Effect::TrainingDummy => {
                            let body = Body::Object(object::Body::TrainingDummy);
                            emitters.emit(CreateNpcEvent {
                                pos: *pos,
                                ori: Ori::default(),
                                npc: NpcBuilder::new(
                                    Stats::new(
                                        Content::with_attr("name-custom-village-dummy", "neut"),
                                        body,
                                    ),
                                    body,
                                    Alignment::Npc,
                                )
                                .with_health(Health::new(body))
                                .with_poise(Poise::new(body)),
                            });
                        },
                        _ => {},
                    }
                }

                if projectile_vanished {
                    continue 'projectile_loop;
                }
            } else {
                if let Some(ori) = orientations.get_mut(entity)
                    && let Some(dir) = velocities
                        .get(entity)
                        .and_then(|v| Dir::from_unnormalized(v.0))
                {
                    *ori = dir.into();
                }

                if let Some(vel) = velocities.get_mut(entity)
                    && let Some((tgt_uid, rate)) = projectile.homing
                    && let Some(tgt_pos) = read_data
                        .id_maps
                        .uid_entity(tgt_uid)
                        .and_then(|e| read_data.positions.get(e))
                    && let Some((init_dir, tgt_dir)) = Dir::from_unnormalized(vel.0).zip(
                        Dir::from_unnormalized(tgt_pos.0.with_z(tgt_pos.0.z + 1.0) - pos.0),
                    )
                {
                    // We want the homing to be weaker when projectile first fired
                    let time_factor = (projectile.init_time.0 as f32
                        - projectile.time_left.as_secs_f32())
                    .min(1.0);
                    let factor = (rate * read_data.dt.0 / init_dir.angle_between(*tgt_dir)
                        * time_factor)
                        .min(1.0);
                    let new_dir = init_dir.slerped_to(tgt_dir, factor);
                    *vel = Vel(*new_dir * vel.0.magnitude());
                }
            }

            if projectile.time_left == Duration::ZERO {
                emitters.emit(DeleteEvent(entity));

                for effect in projectile.timeout.drain(..) {
                    if let projectile::Effect::Firework(reagent) = effect {
                        const ENABLE_RECURSIVE_FIREWORKS: bool = true;
                        if ENABLE_RECURSIVE_FIREWORKS {
                            use common::{comp::LightEmitter, event::ShootEvent};
                            use std::f32::consts::PI;
                            // Note that if the expected fireworks per firework is > 1, this
                            // will eventually cause
                            // enough server lag that more players can't log in.
                            let thresholds: &[(f32, usize)] = &[(0.25, 2), (0.7, 1)];
                            let expected = {
                                let mut total = 0.0;
                                let mut cumulative_probability = 0.0;
                                for (p, n) in thresholds {
                                    total += (p - cumulative_probability) * *n as f32;
                                    cumulative_probability += p;
                                }
                                total
                            };
                            assert!(expected < 1.0);
                            let num_fireworks = (|| {
                                let x = rng.random_range(0.0..1.0);
                                for (p, n) in thresholds {
                                    if x < *p {
                                        return *n;
                                    }
                                }
                                0
                            })();
                            for _ in 0..num_fireworks {
                                let speed: f32 = rng.random_range(40.0..80.0);
                                let theta: f32 = rng.random_range(0.0..2.0 * PI);
                                let phi: f32 = rng.random_range(0.25 * PI..0.5 * PI);
                                let dir = Dir::from_unnormalized(Vec3::new(
                                    theta.cos(),
                                    theta.sin(),
                                    phi.sin(),
                                ))
                                .expect("nonzero vector should normalize");
                                emitters.emit(ShootEvent {
                                    entity: Some(entity),
                                    pos: *pos,
                                    dir,
                                    body: *body,
                                    light: Some(LightEmitter {
                                        animated: true,
                                        flicker: 2.0,
                                        strength: 2.0,
                                        col: Rgb::new(1.0, 1.0, 0.0),
                                        dir: None,
                                    }),
                                    projectile: Projectile {
                                        hit_solid: Vec::new(),
                                        hit_entity: Vec::new(),
                                        timeout: vec![projectile::Effect::Firework(reagent)],
                                        time_left: Duration::from_secs(1),
                                        init_time: Secs(1.0),
                                        ignore_group: true,
                                        is_sticky: true,
                                        is_point: true,
                                        owner: projectile.owner,
                                        homing: None,
                                    },
                                    speed,
                                    object: None,
                                });
                            }
                        }
                        emitters.emit(DeleteEvent(entity));
                        emitters.emit(ExplosionEvent {
                            pos: pos.0,
                            explosion: Explosion {
                                effects: vec![
                                    RadiusEffect::Entity(effect::Effect::Damage(Damage {
                                        kind: DamageKind::Energy,
                                        value: 5.0,
                                    })),
                                    RadiusEffect::Entity(effect::Effect::Poise(-40.0)),
                                    RadiusEffect::TerrainDestruction(4.0, Rgb::black()),
                                ],
                                radius: 12.0,
                                reagent: Some(reagent),
                                min_falloff: 0.0,
                            },
                            owner: projectile.owner,
                        });
                    }
                }
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
    vel: Vec3<f32>,
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
                        mass: read_data.masses.get(entity),
                        pos: read_data.positions.get(entity).map(|p| p.0),
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
                mass: read_data.masses.get(target),
                player: read_data.players.get(target),
            };

            // TODO: Is it possible to have projectile without body??
            if let Some(&body) = read_data.bodies.get(projectile_entity) {
                outcomes_emitter.emit(Outcome::ProjectileHit {
                    pos: target_pos,
                    body,
                    vel: projectile_info.vel,
                    source: projectile_info.owner_uid,
                    target: read_data.uids.get(target).copied(),
                });
            }

            let allow_friendly_fire = owner.is_some_and(|owner| {
                combat::allow_friendly_fire(&read_data.entered_auras, owner, target)
            });

            // PvP check
            let permit_pvp = combat::permit_pvp(
                &read_data.alignments,
                &read_data.players,
                &read_data.entered_auras,
                &read_data.id_maps,
                owner,
                target,
            );

            let target_dodging = read_data
                .character_states
                .get(target)
                .and_then(|cs| cs.roll_attack_immunities())
                .is_some_and(|i| i.projectiles);

            let precision_from_flank = combat::precision_mult_from_flank(
                *projectile_dir,
                target_info.ori,
                Default::default(),
                false,
            );

            let precision_from_head = {
                // This performs a cylinder and line segment intersection check. The cylinder is
                // the upper 10% of an entity's dimensions. The line segment is from the
                // projectile's positions on the current and previous tick.
                let curr_pos = projectile_info.pos.0;
                let last_pos = projectile_info.pos.0 - projectile_info.vel * read_data.dt.0;
                let vel = projectile_info.vel;
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
                permit_pvp,
                allow_friendly_fire,
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
        projectile::Effect::Arc(a) => {
            emitters.emit(ArcEvent {
                arc: a,
                owner: projectile_info.owner_uid,
                target: projectile_target_info.uid,
                pos: *projectile_info.pos,
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
            if let Some(owner_uid) = owner_uid
                && target_uid != owner_uid
            {
                emitters.emit(PossessEvent(owner_uid, target_uid));
            }
        },
        projectile::Effect::Stick => {},
        projectile::Effect::Firework(_) => {},
        projectile::Effect::SurpriseEgg => {
            let Pos(pos) = *projectile_info.pos;
            outcomes_emitter.emit(Outcome::SurpriseEgg { pos });
        },
        projectile::Effect::TrainingDummy => emitters.emit(CreateNpcEvent {
            pos: *projectile_info.pos,
            ori: Ori::default(),
            npc: NpcBuilder::new(
                Stats::new(
                    Content::with_attr("name-custom-village-dummy", "neut"),
                    Body::Object(object::Body::TrainingDummy),
                ),
                Body::Object(object::Body::TrainingDummy),
                Alignment::Npc,
            ),
        }),
    }
}
