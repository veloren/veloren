use crate::{
    client::Client,
    comp::{
        ability,
        agent::{Agent, AgentEvent, Sound, SoundKind},
        loot_owner::LootOwner,
        skillset::SkillGroupKind,
        BuffKind, BuffSource, PhysicsState,
    },
    rtsim,
    sys::terrain::SAFE_ZONE_RADIUS,
    Server, SpawnPoint, StateExt,
};
use authc::Uuid;
use common::{
    combat,
    combat::{AttackSource, DamageContributor},
    comp::{
        self, aura, buff,
        chat::{KillSource, KillType},
        inventory::item::{AbilityMap, MaterialStatManifest},
        item::flatten_counted_items,
        loot_owner::LootOwnerKind,
        Alignment, Auras, Body, CharacterState, Energy, Group, Health, HealthChange, Inventory,
        Object, Player, Poise, Pos, SkillSet, Stats,
    },
    consts::TELEPORTER_RADIUS,
    event::{EventBus, ServerEvent},
    lottery::distribute_many,
    outcome::{HealthChangeInfo, Outcome},
    resources::{Secs, Time},
    spiral::Spiral2d,
    states::utils::StageSection,
    terrain::{Block, BlockKind, TerrainGrid},
    trade::{TradeResult, Trades},
    uid::{IdMaps, Uid},
    util::Dir,
    vol::ReadVol,
    Damage, DamageKind, DamageSource, Explosion, GroupTarget, RadiusEffect,
};
use common_net::{msg::ServerGeneral, sync::WorldSyncExt};
use common_state::{AreasContainer, BlockChange, NoDurabilityArea};
use hashbrown::HashSet;
use rand::Rng;
use specs::{Builder, Entity as EcsEntity, Entity, Join, LendJoin, WorldExt};
use std::{collections::HashMap, iter, sync::Arc, time::Duration};
use tracing::{debug, error, warn};
use vek::{Vec2, Vec3};

#[derive(Hash, Eq, PartialEq)]
enum DamageContrib {
    Solo(EcsEntity),
    Group(Group),
    NotFound,
}

pub fn handle_poise(server: &Server, entity: EcsEntity, change: comp::PoiseChange) {
    let ecs = &server.state.ecs();
    if let Some(character_state) = ecs.read_storage::<CharacterState>().get(entity) {
        // Entity is invincible to poise change during stunned character state
        if !matches!(character_state, CharacterState::Stunned(_)) {
            if let Some(mut poise) = ecs.write_storage::<Poise>().get_mut(entity) {
                poise.change(change);
            }
        }
    }
}

pub fn handle_health_change(server: &Server, entity: EcsEntity, change: HealthChange) {
    let ecs = &server.state.ecs();
    if let Some(mut health) = ecs.write_storage::<Health>().get_mut(entity) {
        // If the change amount was not zero
        let changed = health.change_by(change);
        if let (Some(pos), Some(uid)) = (
            ecs.read_storage::<Pos>().get(entity),
            ecs.read_storage::<Uid>().get(entity),
        ) {
            if changed {
                let outcomes = ecs.write_resource::<EventBus<Outcome>>();
                outcomes.emit_now(Outcome::HealthChange {
                    pos: pos.0,
                    info: HealthChangeInfo {
                        amount: change.amount,
                        by: change.by,
                        target: *uid,
                        cause: change.cause,
                        precise: change.precise,
                        instance: change.instance,
                    },
                });
            }
        }
    }
    // This if statement filters out anything under 5 damage, for DOT ticks
    // TODO: Find a better way to separate direct damage from DOT here
    let damage = -change.amount;
    if damage > 5.0 {
        if let Some(agent) = ecs.write_storage::<Agent>().get_mut(entity) {
            agent.inbox.push_back(AgentEvent::Hurt);
        }
    }
}

pub fn handle_knockback(server: &Server, entity: EcsEntity, impulse: Vec3<f32>) {
    let ecs = &server.state.ecs();
    let clients = ecs.read_storage::<Client>();

    if let Some(physics) = ecs.read_storage::<PhysicsState>().get(entity) {
        //Check if the entity is on a surface. If it is not, reduce knockback.
        let mut impulse = impulse
            * if physics.on_surface().is_some() {
                1.0
            } else {
                0.4
            };
        if let Some(mass) = ecs.read_storage::<comp::Mass>().get(entity) {
            // we go easy on the little ones (because they fly so far)
            impulse /= mass.0.max(40.0);
        }
        let mut velocities = ecs.write_storage::<comp::Vel>();
        if let Some(vel) = velocities.get_mut(entity) {
            vel.0 += impulse;
        }
        if let Some(client) = clients.get(entity) {
            client.send_fallible(ServerGeneral::Knockback(impulse));
        }
    }
}

/// Handle an entity dying. If it is a player, it will send a message to all
/// other players. If the entity that killed it had stats, then give it exp for
/// the kill. Experience given is equal to the level of the entity that was
/// killed times 10.
pub fn handle_destroy(server: &mut Server, entity: EcsEntity, last_change: HealthChange) {
    let state = server.state_mut();

    // TODO: Investigate duplicate `Destroy` events (but don't remove this).
    // If the entity was already deleted, it can't be destroyed again.
    if !state.ecs().is_alive(entity) {
        return;
    }

    // Remove components that should not persist across death
    state.ecs().write_storage::<comp::Melee>().remove(entity);
    state.ecs().write_storage::<comp::Beam>().remove(entity);

    let get_attacker_name = |cause_of_death: KillType, by: Uid| -> KillSource {
        // Get attacker entity
        if let Some(char_entity) = state.ecs().entity_from_uid(by) {
            // Check if attacker is another player or entity with stats (npc)
            if state
                .ecs()
                .read_storage::<Player>()
                .get(char_entity)
                .is_some()
            {
                KillSource::Player(by, cause_of_death)
            } else if let Some(stats) = state.ecs().read_storage::<Stats>().get(char_entity) {
                KillSource::NonPlayer(stats.name.clone(), cause_of_death)
            } else {
                KillSource::NonExistent(cause_of_death)
            }
        } else {
            KillSource::NonExistent(cause_of_death)
        }
    };

    // Push an outcome if entity is has a character state (entities that don't have
    // one, we probably don't care about emitting death outcome)
    if state
        .ecs()
        .read_storage::<CharacterState>()
        .get(entity)
        .is_some()
    {
        if let Some(pos) = state.ecs().read_storage::<Pos>().get(entity) {
            state
                .ecs()
                .read_resource::<EventBus<Outcome>>()
                .emit_now(Outcome::Death { pos: pos.0 });
        }
    }

    // Chat message
    // If it was a player that died
    if let Some(_player) = state.ecs().read_storage::<Player>().get(entity) {
        if let Some(uid) = state.ecs().read_storage::<Uid>().get(entity) {
            let kill_source = match (last_change.cause, last_change.by.map(|x| x.uid())) {
                (Some(DamageSource::Melee), Some(by)) => get_attacker_name(KillType::Melee, by),
                (Some(DamageSource::Projectile), Some(by)) => {
                    get_attacker_name(KillType::Projectile, by)
                },
                (Some(DamageSource::Explosion), Some(by)) => {
                    get_attacker_name(KillType::Explosion, by)
                },
                (Some(DamageSource::Energy), Some(by)) => get_attacker_name(KillType::Energy, by),
                (Some(DamageSource::Buff(buff_kind)), by) => {
                    if let Some(by) = by {
                        get_attacker_name(KillType::Buff(buff_kind), by)
                    } else {
                        KillSource::NonExistent(KillType::Buff(buff_kind))
                    }
                },
                (Some(DamageSource::Other), Some(by)) => get_attacker_name(KillType::Other, by),
                (Some(DamageSource::Falling), _) => KillSource::FallDamage,
                // HealthSource::Suicide => KillSource::Suicide,
                _ => KillSource::Other,
            };

            state.send_chat(comp::ChatType::Kill(kill_source, *uid).into_plain_msg(""));
        }
    }

    let mut exp_awards = Vec::<(Entity, f32, Option<Group>)>::new();
    // Award EXP to damage contributors
    //
    // NOTE: Debug logging is disabled by default for this module - to enable it add
    // veloren_server::events::entity_manipulation=debug to RUST_LOG
    (|| {
        let mut skill_sets = state.ecs().write_storage::<SkillSet>();
        let healths = state.ecs().read_storage::<Health>();
        let energies = state.ecs().read_storage::<Energy>();
        let inventories = state.ecs().read_storage::<Inventory>();
        let players = state.ecs().read_storage::<Player>();
        let bodies = state.ecs().read_storage::<Body>();
        let poises = state.ecs().read_storage::<Poise>();
        let positions = state.ecs().read_storage::<Pos>();
        let groups = state.ecs().read_storage::<Group>();

        let (
            entity_skill_set,
            entity_health,
            entity_energy,
            entity_inventory,
            entity_body,
            entity_poise,
            entity_pos,
        ) = match (|| {
            Some((
                skill_sets.get(entity)?,
                healths.get(entity)?,
                energies.get(entity)?,
                inventories.get(entity)?,
                bodies.get(entity)?,
                poises.get(entity)?,
                positions.get(entity)?,
            ))
        })() {
            Some(comps) => comps,
            None => return,
        };

        // Calculate the total EXP award for the kill
        let msm = state.ecs().read_resource::<MaterialStatManifest>();
        let exp_reward = combat::combat_rating(
            entity_inventory,
            entity_health,
            entity_energy,
            entity_poise,
            entity_skill_set,
            *entity_body,
            &msm,
        ) * 20.0;

        let mut damage_contributors = HashMap::<DamageContrib, (u64, f32)>::new();
        for (damage_contributor, damage) in entity_health.damage_contributions() {
            match damage_contributor {
                DamageContributor::Solo(uid) => {
                    if let Some(attacker) = state.ecs().entity_from_uid(*uid) {
                        damage_contributors.insert(DamageContrib::Solo(attacker), (*damage, 0.0));
                    } else {
                        // An entity who was not in a group contributed damage but is now either
                        // dead or offline. Add a placeholder to ensure that the contributor's exp
                        // is discarded, not distributed between the other contributors
                        damage_contributors.insert(DamageContrib::NotFound, (*damage, 0.0));
                    }
                },
                DamageContributor::Group {
                    entity_uid: _,
                    group,
                } => {
                    // Damage made by entities who were in a group at the time of attack is
                    // attributed to their group rather than themselves. This allows for all
                    // members of a group to receive EXP, not just the damage dealers.
                    let entry = damage_contributors
                        .entry(DamageContrib::Group(*group))
                        .or_insert((0, 0.0));
                    entry.0 += damage;
                },
            }
        }

        // Calculate the percentage of total damage that each DamageContributor
        // contributed
        let total_damage: f64 = damage_contributors
            .values()
            .map(|(damage, _)| *damage as f64)
            .sum();
        damage_contributors
            .iter_mut()
            .for_each(|(_, (damage, percentage))| {
                *percentage = (*damage as f64 / total_damage) as f32
            });

        let alignments = state.ecs().read_storage::<Alignment>();
        let uids = state.ecs().read_storage::<Uid>();
        let mut outcomes = state.ecs().write_resource::<EventBus<Outcome>>();
        let inventories = state.ecs().read_storage::<Inventory>();

        let destroyed_group = groups.get(entity);

        let within_range = |attacker_pos: &Pos| {
            // Maximum distance that an attacker must be from an entity at the time of its
            // death to receive EXP for the kill
            const MAX_EXP_DIST: f32 = 150.0;
            entity_pos.0.distance_squared(attacker_pos.0) < MAX_EXP_DIST.powi(2)
        };

        let is_pvp_kill =
            |attacker: Entity| players.get(entity).is_some() && players.get(attacker).is_some();

        // Iterate through all contributors of damage for the killed entity, calculating
        // how much EXP each contributor should be awarded based on their
        // percentage of damage contribution
        exp_awards = damage_contributors.iter().filter_map(|(damage_contributor, (_, damage_percent))| {
            let contributor_exp = exp_reward * damage_percent;
            match damage_contributor {
                DamageContrib::Solo(attacker) => {
                    // No exp for self kills or PvP
                    if *attacker == entity || is_pvp_kill(*attacker) { return None; }

                    // Only give EXP to the attacker if they are within EXP range of the killed entity
                    positions.get(*attacker).and_then(|attacker_pos| {
                        if within_range(attacker_pos) {
                            debug!("Awarding {} exp to individual {:?} who contributed {}% damage to the kill of {:?}", contributor_exp, attacker, *damage_percent * 100.0, entity);
                            Some(iter::once((*attacker, contributor_exp, None)).collect())
                        } else {
                            None
                        }
                    })
                },
                DamageContrib::Group(group) => {
                    // Don't give EXP to members in the destroyed entity's group
                    if destroyed_group == Some(group) { return None; }

                    // Only give EXP to members of the group that are within EXP range of the killed entity and aren't a pet
                    let members_in_range = (
                        &state.ecs().entities(),
                        &groups,
                        &positions,
                        alignments.maybe(),
                        &uids,
                    )
                        .join()
                        .filter_map(|(member_entity, member_group, member_pos, alignment, uid)| {
                            if *member_group == *group && within_range(member_pos) && !is_pvp_kill(member_entity) && !matches!(alignment, Some(Alignment::Owned(owner)) if owner != uid) {
                                Some(member_entity)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();

                    if members_in_range.is_empty() { return None; }

                    // Divide EXP reward by square root of number of people in group for group EXP scaling
                    let exp_per_member = contributor_exp / (members_in_range.len() as f32).sqrt();

                    debug!("Awarding {} exp per member of group ID {:?} with {} members which contributed {}% damage to the kill of {:?}", exp_per_member, group, members_in_range.len(), *damage_percent * 100.0, entity);
                    Some(members_in_range.into_iter().map(|entity| (entity, exp_per_member, Some(*group))).collect::<Vec<(Entity, f32, Option<Group>)>>())
                },
                DamageContrib::NotFound => {
                    // Discard exp for dead/offline individual damage contributors
                    None
                }
            }
        }).flatten().collect::<Vec<(Entity, f32, Option<Group>)>>();

        exp_awards.iter().for_each(|(attacker, exp_reward, _)| {
            // Process the calculated EXP rewards
            if let (Some(mut attacker_skill_set), Some(attacker_uid), Some(attacker_inventory)) = (
                skill_sets.get_mut(*attacker),
                uids.get(*attacker),
                inventories.get(*attacker),
            ) {
                handle_exp_gain(
                    *exp_reward,
                    attacker_inventory,
                    &mut attacker_skill_set,
                    attacker_uid,
                    &mut outcomes,
                );
            }
        });
    })();

    let should_delete = if state
        .ecs()
        .write_storage::<Client>()
        .get_mut(entity)
        .is_some()
    {
        state
            .ecs()
            .write_storage()
            .insert(entity, comp::Vel(Vec3::zero()))
            .err()
            .map(|e| error!(?e, ?entity, "Failed to set zero vel on dead client"));
        state
            .ecs()
            .write_storage::<comp::ForceUpdate>()
            .get_mut(entity)
            .map(|force_update| force_update.update());
        state
            .ecs()
            .write_storage::<Energy>()
            .get_mut(entity)
            .map(|mut energy| {
                let energy = &mut *energy;
                energy.refresh()
            });
        let _ = state
            .ecs()
            .write_storage::<CharacterState>()
            .insert(entity, CharacterState::default());

        false
    } else if state.ecs().read_storage::<Agent>().contains(entity)
        && !matches!(
            state.ecs().read_storage::<comp::Alignment>().get(entity),
            Some(comp::Alignment::Owned(_))
        )
    {
        // Only drop loot if entity has agency (not a player),
        // and if it is not owned by another entity (not a pet)

        // Decide for a loot drop before turning into a lootbag

        let items = {
            let mut item_drops = state.ecs().write_storage::<comp::ItemDrops>();
            item_drops.remove(entity).map(|comp::ItemDrops(item)| item)
        };

        if let Some(items) = items {
            let pos = state.ecs().read_storage::<Pos>().get(entity).cloned();
            let vel = state.ecs().read_storage::<comp::Vel>().get(entity).cloned();
            if let Some(pos) = pos {
                // Remove entries where zero exp was awarded - this happens because some
                // entities like Object bodies don't give EXP.
                let mut item_receivers = HashMap::new();
                for (entity, exp, group) in exp_awards {
                    if exp >= f32::EPSILON {
                        let loot_owner = if let Some(group) = group {
                            Some(LootOwnerKind::Group(group))
                        } else {
                            let uid = state
                                .ecs()
                                .read_storage::<Body>()
                                .get(entity)
                                .and_then(|body| {
                                    // Only humanoids are awarded loot ownership - if the winner
                                    // was a non-humanoid NPC the loot will be free-for-all
                                    if matches!(body, Body::Humanoid(_)) {
                                        Some(state.ecs().read_storage::<Uid>().get(entity).cloned())
                                    } else {
                                        None
                                    }
                                })
                                .flatten();

                            uid.map(LootOwnerKind::Player)
                        };

                        *item_receivers.entry(loot_owner).or_insert(0.0) += exp;
                    }
                }

                let mut item_offset_spiral =
                    Spiral2d::new().map(|offset| offset.as_::<f32>() * 0.5);

                let mut spawn_item = |item, loot_owner| {
                    let offset = item_offset_spiral.next().unwrap_or_default();
                    state.create_item_drop(
                        Pos(pos.0 + Vec3::unit_z() * 0.25 + offset),
                        vel.unwrap_or(comp::Vel(Vec3::zero())),
                        item,
                        if let Some(loot_owner) = loot_owner {
                            debug!("Assigned UID {loot_owner:?} as the winner for the loot drop");
                            Some(LootOwner::new(loot_owner, false))
                        } else {
                            None
                        },
                    );
                };

                let msm = &MaterialStatManifest::load().read();
                let ability_map = &AbilityMap::load().read();
                if item_receivers.is_empty() {
                    for item in flatten_counted_items(&items, ability_map, msm) {
                        spawn_item(item, None)
                    }
                } else {
                    let mut rng = rand::thread_rng();
                    distribute_many(
                        item_receivers
                            .iter()
                            .map(|(loot_owner, weight)| (*weight, *loot_owner)),
                        &mut rng,
                        &items,
                        |(amount, _)| *amount,
                        |(_, item), loot_owner, count| {
                            for item in item.stacked_duplicates(ability_map, msm, count) {
                                spawn_item(item, loot_owner)
                            }
                        },
                    );
                }
            } else {
                error!(
                    ?entity,
                    "Entity doesn't have a position, no bag is being dropped"
                )
            }
        }

        true
    } else {
        true
    };

    let resists_durability = state
        .ecs()
        .read_storage::<Pos>()
        .get(entity)
        .cloned()
        .map_or(false, |our_pos| {
            let areas_container = state
                .ecs()
                .read_resource::<AreasContainer<NoDurabilityArea>>();
            let our_pos = our_pos.0.map(|i| i as i32);

            let is_in_area = areas_container
                .areas()
                .iter()
                .any(|(_, area)| area.contains_point(our_pos));

            is_in_area
        });

    // TODO: Do we need to do this if `should_delete` is true?
    // Modify durability on all equipped items
    if !resists_durability
        && let Some(mut inventory) = state.ecs().write_storage::<Inventory>().get_mut(entity)
    {
        let ecs = state.ecs();
        let ability_map = ecs.read_resource::<AbilityMap>();
        let msm = ecs.read_resource::<MaterialStatManifest>();
        let time = ecs.read_resource::<Time>();
        inventory.damage_items(&ability_map, &msm, *time);
    }

    if let Some(actor) = state.entity_as_actor(entity) {
        state
            .ecs()
            .write_resource::<rtsim::RtSim>()
            .hook_rtsim_actor_death(
                &state.ecs().read_resource::<Arc<world::World>>(),
                state
                    .ecs()
                    .read_resource::<world::IndexOwned>()
                    .as_index_ref(),
                actor,
                state.ecs().read_storage::<Pos>().get(entity).map(|p| p.0),
                last_change
                    .by
                    .as_ref()
                    .and_then(
                        |(DamageContributor::Solo(entity_uid)
                         | DamageContributor::Group { entity_uid, .. })| {
                            state
                                .ecs()
                                .read_resource::<IdMaps>()
                                .uid_entity(*entity_uid)
                        },
                    )
                    .and_then(|killer| state.entity_as_actor(killer)),
            );
    }

    if should_delete {
        if let Err(e) = state.delete_entity_recorded(entity) {
            error!(?e, ?entity, "Failed to delete destroyed entity");
        }
    }
}

/// Delete an entity without any special actions (this is generally used for
/// temporarily unloading an entity when it leaves the view distance). As much
/// as possible, this function should simply make an entity cease to exist.
pub fn handle_delete(server: &mut Server, entity: EcsEntity) {
    let _ = server
        .state_mut()
        .delete_entity_recorded(entity)
        .map_err(|e| error!(?e, ?entity, "Failed to delete destroyed entity"));
}

pub fn handle_land_on_ground(
    server: &Server,
    entity: EcsEntity,
    vel: Vec3<f32>,
    surface_normal: Vec3<f32>,
) {
    let ecs = server.state.ecs();

    // HACK: Certain ability movements currently take us above the fall damage
    // threshold in the horizontal axis. This factor dampens velocity in the
    // horizontal axis when applying fall damage.
    let horizontal_damp = 0.5
        + vel
            .try_normalized()
            .unwrap_or_default()
            .dot(Vec3::unit_z())
            .abs()
            * 0.5;

    let relative_vel = vel.dot(-surface_normal) * horizontal_damp;

    // The second part of this if statement disables all fall damage when in the
    // water. This was added as a *temporary* fix a bug that causes you to take
    // fall damage while swimming downwards. FIXME: Fix the actual bug and
    // remove the following relevant part of the if statement.
    if relative_vel >= 30.0
        && ecs
            .read_storage::<PhysicsState>()
            .get(entity)
            .map_or(true, |ps| ps.in_liquid().is_none())
    {
        let char_states = ecs.read_storage::<CharacterState>();

        let reduced_vel = if let Some(CharacterState::DiveMelee(c)) = char_states.get(entity) {
            (relative_vel + c.static_data.vertical_speed).min(0.0)
        } else {
            relative_vel
        };

        let mass = ecs
            .read_storage::<comp::Mass>()
            .get(entity)
            .copied()
            .unwrap_or_default();
        let impact_energy = mass.0 * reduced_vel.powi(2) / 2.0;
        let falldmg = impact_energy / 1000.0;

        let inventories = ecs.read_storage::<Inventory>();
        let stats = ecs.read_storage::<Stats>();
        let time = ecs.read_resource::<Time>();
        let msm = ecs.read_resource::<MaterialStatManifest>();
        let server_eventbus = ecs.read_resource::<EventBus<ServerEvent>>();

        // Emit health change
        let damage = Damage {
            source: DamageSource::Falling,
            kind: DamageKind::Crushing,
            value: falldmg,
        };
        let damage_reduction = Damage::compute_damage_reduction(
            Some(damage),
            inventories.get(entity),
            stats.get(entity),
            &msm,
        );
        let change = damage.calculate_health_change(
            damage_reduction,
            None,
            None,
            0.0,
            1.0,
            *time,
            rand::random(),
        );

        server_eventbus.emit_now(ServerEvent::HealthChange { entity, change });

        // Emit poise change
        let poise_damage = -(mass.0 * reduced_vel.powi(2) / 1500.0);
        let poise_change = Poise::apply_poise_reduction(
            poise_damage,
            inventories.get(entity),
            &msm,
            char_states.get(entity),
            stats.get(entity),
        );
        let poise_change = comp::PoiseChange {
            amount: poise_change,
            impulse: Vec3::unit_z(),
            by: None,
            cause: None,
            time: *time,
        };
        server_eventbus.emit_now(ServerEvent::PoiseChange {
            entity,
            change: poise_change,
        });
    }
}

pub fn handle_respawn(server: &Server, entity: EcsEntity) {
    let state = &server.state;

    // Only clients can respawn
    if state
        .ecs()
        .write_storage::<Client>()
        .get_mut(entity)
        .is_some()
    {
        let respawn_point = state
            .read_component_copied::<comp::Waypoint>(entity)
            .map(|wp| wp.get_pos())
            .unwrap_or(state.ecs().read_resource::<SpawnPoint>().0);

        state
            .ecs()
            .write_storage::<Health>()
            .get_mut(entity)
            .map(|mut health| health.revive());
        state
            .ecs()
            .write_storage::<comp::Combo>()
            .get_mut(entity)
            .map(|mut combo| combo.reset());
        state
            .ecs()
            .write_storage::<Pos>()
            .get_mut(entity)
            .map(|pos| pos.0 = respawn_point);
        state
            .ecs()
            .write_storage::<comp::PhysicsState>()
            .get_mut(entity)
            .map(|phys_state| phys_state.reset());
        state
            .ecs()
            .write_storage::<comp::ForceUpdate>()
            .get_mut(entity)
            .map(|force_update| force_update.update());
    }
}

pub fn handle_explosion(server: &Server, pos: Vec3<f32>, explosion: Explosion, owner: Option<Uid>) {
    // Go through all other entities
    let ecs = &server.state.ecs();
    let settings = server.settings();
    let server_eventbus = ecs.read_resource::<EventBus<ServerEvent>>();
    let time = ecs.read_resource::<Time>();
    let owner_entity = owner.and_then(|uid| ecs.read_resource::<IdMaps>().uid_entity(uid));

    let explosion_volume = 6.25 * explosion.radius;
    let mut emitter = server_eventbus.emitter();
    emitter.emit(ServerEvent::Sound {
        sound: Sound::new(SoundKind::Explosion, pos, explosion_volume, time.0),
    });

    // Add an outcome
    // Uses radius as outcome power for now
    let outcome_power = explosion.radius;
    let outcomes = ecs.read_resource::<EventBus<Outcome>>();
    let mut outcomes_emitter = outcomes.emitter();
    outcomes_emitter.emit(Outcome::Explosion {
        pos,
        power: outcome_power,
        radius: explosion.radius,
        is_attack: explosion
            .effects
            .iter()
            .any(|e| matches!(e, RadiusEffect::Attack(_))),
        reagent: explosion.reagent,
    });
    let groups = ecs.read_storage::<Group>();

    // Used to get strength of explosion effects as they falloff over distance
    fn cylinder_sphere_strength(
        sphere_pos: Vec3<f32>,
        radius: f32,
        min_falloff: f32,
        cyl_pos: Vec3<f32>,
        cyl_body: Body,
    ) -> f32 {
        // 2d check
        let horiz_dist = Vec2::<f32>::from(sphere_pos - cyl_pos).distance(Vec2::default())
            - cyl_body.max_radius();
        // z check
        let half_body_height = cyl_body.height() / 2.0;
        let vert_distance =
            (sphere_pos.z - (cyl_pos.z + half_body_height)).abs() - half_body_height;

        // Use whichever gives maximum distance as that closer to real value. Sets
        // minimum to 0 as negative values would indicate inside entity.
        let distance = horiz_dist.max(vert_distance).max(0.0);

        if distance > radius {
            // If further than exploion radius, no strength
            0.0
        } else {
            // Falloff inversely proportional to radius
            let fall_off = ((distance / radius).min(1.0) - 1.0).abs();
            let min_falloff = min_falloff.clamp(0.0, 1.0);
            min_falloff + fall_off * (1.0 - min_falloff)
        }
    }

    // TODO: Faster RNG?
    let mut rng = rand::thread_rng();
    // TODO: Process terrain destruction first so that entities don't get protected
    // by terrain that gets destroyed?
    'effects: for effect in explosion.effects {
        match effect {
            RadiusEffect::TerrainDestruction(power, new_color) => {
                const RAYS: usize = 500;

                let spatial_grid = ecs.read_resource::<common::CachedSpatialGrid>();
                let auras = ecs.read_storage::<Auras>();
                let positions = ecs.read_storage::<Pos>();

                // Prevent block colour changes within the radius of a safe zone aura
                if spatial_grid
                    .0
                    .in_circle_aabr(pos.xy(), SAFE_ZONE_RADIUS)
                    .filter_map(|entity| {
                        auras
                            .get(entity)
                            .and_then(|entity_auras| {
                                positions.get(entity).map(|pos| (entity_auras, pos))
                            })
                            .and_then(|(entity_auras, pos)| {
                                entity_auras
                                    .auras
                                    .iter()
                                    .find(|(_, aura)| {
                                        matches!(aura.aura_kind, aura::AuraKind::Buff {
                                            kind: BuffKind::Invulnerability,
                                            source: BuffSource::World,
                                            ..
                                        })
                                    })
                                    .map(|(_, aura)| (*pos, aura.radius))
                            })
                    })
                    .any(|(aura_pos, aura_radius)| {
                        pos.distance_squared(aura_pos.0) < aura_radius.powi(2)
                    })
                {
                    continue 'effects;
                }

                // Color terrain
                let mut touched_blocks = Vec::new();
                let color_range = power * 2.7;
                for _ in 0..RAYS {
                    let dir = Vec3::new(
                        rng.gen::<f32>() - 0.5,
                        rng.gen::<f32>() - 0.5,
                        rng.gen::<f32>() - 0.5,
                    )
                    .normalized();

                    let _ = ecs
                        .read_resource::<TerrainGrid>()
                        .ray(pos, pos + dir * color_range)
                        .until(|_| rng.gen::<f32>() < 0.05)
                        .for_each(|_: &Block, pos| touched_blocks.push(pos))
                        .cast();
                }

                let terrain = ecs.read_resource::<TerrainGrid>();
                let mut block_change = ecs.write_resource::<BlockChange>();
                for block_pos in touched_blocks {
                    if let Ok(block) = terrain.get(block_pos) {
                        if !matches!(block.kind(), BlockKind::Lava | BlockKind::GlowingRock)
                            && (
                                // Check that owner is not player or explosion_burn_marks by players
                                // is enabled
                                owner_entity
                                    .map_or(true, |e| ecs.read_storage::<Player>().get(e).is_none())
                                    || settings.gameplay.explosion_burn_marks
                            )
                        {
                            let diff2 = block_pos.map(|b| b as f32).distance_squared(pos);
                            let fade = (1.0 - diff2 / color_range.powi(2)).max(0.0);
                            if let Some(mut color) = block.get_color() {
                                let r = color[0] as f32
                                    + (fade
                                        * (color[0] as f32 * 0.5 - color[0] as f32 + new_color[0]));
                                let g = color[1] as f32
                                    + (fade
                                        * (color[1] as f32 * 0.3 - color[1] as f32 + new_color[1]));
                                let b = color[2] as f32
                                    + (fade
                                        * (color[2] as f32 * 0.3 - color[2] as f32 + new_color[2]));
                                // Darken blocks, but not too much
                                color[0] = (r as u8).max(30);
                                color[1] = (g as u8).max(30);
                                color[2] = (b as u8).max(30);
                                block_change.set(block_pos, Block::new(block.kind(), color));
                            }
                        }

                        if block.is_bonkable() {
                            emitter.emit(ServerEvent::Bonk {
                                pos: block_pos.map(|e| e as f32 + 0.5),
                                owner,
                                target: None,
                            });
                        }
                    }
                }

                // Destroy terrain
                for _ in 0..RAYS {
                    let dir = Vec3::new(
                        rng.gen::<f32>() - 0.5,
                        rng.gen::<f32>() - 0.5,
                        rng.gen::<f32>() - 0.15,
                    )
                    .normalized();

                    let mut ray_energy = power;

                    let terrain = ecs.read_resource::<TerrainGrid>();
                    let from = pos;
                    let to = pos + dir * power;
                    let _ = terrain
                        .ray(from, to)
                        .while_(|block: &Block| {
                            ray_energy -=
                                block.explode_power().unwrap_or(0.0) + rng.gen::<f32>() * 0.1;

                            // Stop if:
                            // 1) Block is liquid
                            // 2) Consumed all energy
                            // 3) Can't explode block (for example we hit stone wall)
                            block.is_liquid()
                                || block.explode_power().is_none()
                                || ray_energy <= 0.0
                        })
                        .for_each(|block: &Block, pos| {
                            if block.explode_power().is_some() {
                                block_change.set(pos, block.into_vacant());
                            }
                        })
                        .cast();
                }
            },
            RadiusEffect::Attack(attack) => {
                let energies = &ecs.read_storage::<Energy>();
                let combos = &ecs.read_storage::<comp::Combo>();
                let inventories = &ecs.read_storage::<Inventory>();
                let alignments = &ecs.read_storage::<Alignment>();
                let id_maps = &ecs.read_resource::<IdMaps>();
                let players = &ecs.read_storage::<Player>();
                let buffs = &ecs.read_storage::<comp::Buffs>();
                let stats = &ecs.read_storage::<comp::Stats>();
                let terrain = ecs.read_resource::<TerrainGrid>();
                for (
                    entity_b,
                    pos_b,
                    health_b,
                    (body_b_maybe, ori_b_maybe, char_state_b_maybe, uid_b),
                ) in (
                    &ecs.entities(),
                    &ecs.read_storage::<Pos>(),
                    &ecs.read_storage::<Health>(),
                    (
                        ecs.read_storage::<Body>().maybe(),
                        ecs.read_storage::<comp::Ori>().maybe(),
                        ecs.read_storage::<CharacterState>().maybe(),
                        &ecs.read_storage::<Uid>(),
                    ),
                )
                    .join()
                    .filter(|(_, _, h, _)| !h.is_dead)
                {
                    let dist_sqrd = pos.distance_squared(pos_b.0);

                    // Check if it is a hit
                    let strength = if let Some(body) = body_b_maybe {
                        cylinder_sphere_strength(
                            pos,
                            explosion.radius,
                            explosion.min_falloff,
                            pos_b.0,
                            *body,
                        )
                    } else {
                        1.0 - dist_sqrd / explosion.radius.powi(2)
                    };

                    // Cast a ray from the explosion to the entity to check visibility
                    if strength > 0.0
                        && (terrain.ray(pos, pos_b.0).until(Block::is_opaque).cast().0 + 0.1)
                            .powi(2)
                            >= dist_sqrd
                    {
                        // See if entities are in the same group
                        let same_group = owner_entity
                            .and_then(|e| groups.get(e))
                            .map(|group_a| Some(group_a) == groups.get(entity_b))
                            .unwrap_or(Some(entity_b) == owner_entity);

                        let target_group = if same_group {
                            GroupTarget::InGroup
                        } else {
                            GroupTarget::OutOfGroup
                        };

                        let dir = Dir::new(
                            (pos_b.0 - pos)
                                .try_normalized()
                                .unwrap_or_else(Vec3::unit_z),
                        );

                        let attacker_info =
                            owner_entity
                                .zip(owner)
                                .map(|(entity, uid)| combat::AttackerInfo {
                                    entity,
                                    uid,
                                    group: groups.get(entity),
                                    energy: energies.get(entity),
                                    combo: combos.get(entity),
                                    inventory: inventories.get(entity),
                                    stats: stats.get(entity),
                                });

                        let target_info = combat::TargetInfo {
                            entity: entity_b,
                            uid: *uid_b,
                            inventory: inventories.get(entity_b),
                            stats: stats.get(entity_b),
                            health: Some(health_b),
                            pos: pos_b.0,
                            ori: ori_b_maybe,
                            char_state: char_state_b_maybe,
                            energy: energies.get(entity_b),
                            buffs: buffs.get(entity_b),
                        };

                        let target_dodging = char_state_b_maybe
                            .and_then(|cs| cs.attack_immunities())
                            .map_or(false, |i| i.explosions);
                        // PvP check
                        let may_harm =
                            combat::may_harm(alignments, players, id_maps, owner_entity, entity_b);
                        // Explosions aren't precise, and thus cannot be a precise strike
                        let precision_mult = None;
                        let attack_options = combat::AttackOptions {
                            target_dodging,
                            may_harm,
                            target_group,
                            precision_mult,
                        };

                        let time = server.state.ecs().read_resource::<Time>();
                        attack.apply_attack(
                            attacker_info,
                            &target_info,
                            dir,
                            attack_options,
                            strength,
                            combat::AttackSource::Explosion,
                            *time,
                            |e| emitter.emit(e),
                            |o| outcomes_emitter.emit(o),
                            &mut rng,
                            0,
                        );
                    }
                }
            },
            RadiusEffect::Entity(mut effect) => {
                let alignments = &ecs.read_storage::<Alignment>();
                let id_maps = &ecs.read_resource::<IdMaps>();
                let players = &ecs.read_storage::<Player>();
                for (entity_b, pos_b, body_b_maybe) in (
                    &ecs.entities(),
                    &ecs.read_storage::<Pos>(),
                    ecs.read_storage::<Body>().maybe(),
                )
                    .join()
                {
                    let strength = if let Some(body) = body_b_maybe {
                        cylinder_sphere_strength(
                            pos,
                            explosion.radius,
                            explosion.min_falloff,
                            pos_b.0,
                            *body,
                        )
                    } else {
                        let distance_squared = pos.distance_squared(pos_b.0);
                        1.0 - distance_squared / explosion.radius.powi(2)
                    };

                    // Player check only accounts for PvP/PvE flag, but bombs
                    // are intented to do friendly fire.
                    //
                    // What exactly is friendly fire is subject to discussion.
                    // As we probably want to minimize possibility of being dick
                    // even to your group members, the only exception is when
                    // you want to harm yourself.
                    //
                    // This can be changed later.
                    let may_harm = || {
                        combat::may_harm(alignments, players, id_maps, owner_entity, entity_b)
                            || owner_entity.map_or(true, |entity_a| entity_a == entity_b)
                    };
                    if strength > 0.0 {
                        let is_alive = ecs
                            .read_storage::<Health>()
                            .get(entity_b)
                            .map_or(true, |h| !h.is_dead);

                        if is_alive {
                            effect.modify_strength(strength);
                            if !effect.is_harm() || may_harm() {
                                server.state().apply_effect(entity_b, effect.clone(), owner);
                            }
                        }
                    }
                }
            },
        }
    }
}

pub fn handle_bonk(server: &mut Server, pos: Vec3<f32>, owner: Option<Uid>, target: Option<Uid>) {
    let ecs = &server.state.ecs();
    let terrain = ecs.read_resource::<TerrainGrid>();
    let mut block_change = ecs.write_resource::<BlockChange>();

    if let Some(_target) = target {
        // TODO: bonk entities but do no damage?
    } else {
        use common::terrain::SpriteKind;
        let pos = pos.map(|e| e.floor() as i32);
        if let Some(block) = terrain.get(pos).ok().copied().filter(|b| b.is_bonkable()) {
            if block_change
                .try_set(pos, block.with_sprite(SpriteKind::Empty))
                .is_some()
            {
                drop(terrain);
                drop(block_change);
                if let Some(items) = comp::Item::try_reclaim_from_block(block) {
                    let msm = &MaterialStatManifest::load().read();
                    let ability_map = &AbilityMap::load().read();
                    for item in flatten_counted_items(&items, ability_map, msm) {
                        server
                            .state
                            .create_object(
                                Pos(pos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0)),
                                match block.get_sprite() {
                                    // Create different containers depending on the original sprite
                                    Some(SpriteKind::Apple) => comp::object::Body::Apple,
                                    Some(SpriteKind::Beehive) => comp::object::Body::Hive,
                                    Some(SpriteKind::Coconut) => comp::object::Body::Coconut,
                                    Some(SpriteKind::Bomb) => comp::object::Body::Bomb,
                                    Some(SpriteKind::Mine) => comp::object::Body::Mine,
                                    _ => comp::object::Body::Pouch,
                                },
                            )
                            .with(item)
                            .maybe_with(match block.get_sprite() {
                                Some(SpriteKind::Bomb) | Some(SpriteKind::Mine) => {
                                    Some(comp::Object::Bomb { owner })
                                },
                                _ => None,
                            })
                            .build();
                    }
                } else if let Some(SpriteKind::Mine) = block.get_sprite() {
                    // Some objects can't be reclaimed as items but still have bonk effects
                    server
                        .state
                        .create_object(
                            Pos(pos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0)),
                            comp::object::Body::Bomb,
                        )
                        .with(comp::Object::Bomb { owner })
                        .build();
                }
            }
        }
    }
}

pub fn handle_aura(server: &mut Server, entity: EcsEntity, aura_change: aura::AuraChange) {
    let ecs = &server.state.ecs();
    let mut auras_all = ecs.write_storage::<Auras>();
    if let Some(mut auras) = auras_all.get_mut(entity) {
        use aura::AuraChange;
        match aura_change {
            AuraChange::Add(new_aura) => {
                auras.insert(new_aura);
            },
            AuraChange::RemoveByKey(keys) => {
                for key in keys {
                    auras.remove(key);
                }
            },
        }
    }
}

pub fn handle_buff(server: &mut Server, entity: EcsEntity, buff_change: buff::BuffChange) {
    let ecs = &server.state.ecs();
    let mut buffs_all = ecs.write_storage::<comp::Buffs>();
    let bodies = ecs.read_storage::<Body>();
    let time = ecs.read_resource::<Time>();
    if let Some(mut buffs) = buffs_all.get_mut(entity) {
        use buff::BuffChange;
        match buff_change {
            BuffChange::Add(new_buff) => {
                if !bodies
                    .get(entity)
                    .map_or(false, |body| body.immune_to(new_buff.kind))
                    && ecs
                        .read_component::<Health>()
                        .get(entity)
                        .map_or(true, |h| !h.is_dead)
                {
                    buffs.insert(new_buff, *time);
                }
            },
            BuffChange::RemoveByKey(keys) => {
                for key in keys {
                    buffs.remove(key);
                }
            },
            BuffChange::RemoveByKind(kind) => {
                buffs.remove_kind(kind);
            },
            BuffChange::RemoveFromController(kind) => {
                if kind.is_buff() {
                    buffs.remove_kind(kind);
                }
            },
            BuffChange::RemoveByCategory {
                all_required,
                any_required,
                none_required,
            } => {
                let mut keys_to_remove = Vec::new();
                for (key, buff) in buffs.buffs.iter() {
                    let mut required_met = true;
                    for required in &all_required {
                        if !buff.cat_ids.iter().any(|cat| cat == required) {
                            required_met = false;
                            break;
                        }
                    }
                    let mut any_met = any_required.is_empty();
                    for any in &any_required {
                        if buff.cat_ids.iter().any(|cat| cat == any) {
                            any_met = true;
                            break;
                        }
                    }
                    let mut none_met = true;
                    for none in &none_required {
                        if buff.cat_ids.iter().any(|cat| cat == none) {
                            none_met = false;
                            break;
                        }
                    }
                    if required_met && any_met && none_met {
                        keys_to_remove.push(key);
                    }
                }
                for key in keys_to_remove {
                    buffs.remove(key);
                }
            },
            BuffChange::Refresh(kind) => {
                buffs
                    .buffs
                    .values_mut()
                    .filter(|b| b.kind == kind)
                    .for_each(|buff| {
                        // Resets buff so that its remaining duration is equal to its original
                        // duration
                        buff.start_time = *time;
                        buff.end_time = buff.data.duration.map(|dur| Time(time.0 + dur.0));
                    })
            },
        }
    }
}

pub fn handle_energy_change(server: &Server, entity: EcsEntity, change: f32) {
    let ecs = &server.state.ecs();
    if let Some(mut energy) = ecs.write_storage::<Energy>().get_mut(entity) {
        energy.change_by(change);
    }
}

fn handle_exp_gain(
    exp_reward: f32,
    inventory: &Inventory,
    skill_set: &mut SkillSet,
    uid: &Uid,
    outcomes: &mut EventBus<Outcome>,
) {
    use comp::inventory::{item::ItemKind, slot::EquipSlot};

    let mut outcomes_emitter = outcomes.emitter();

    // Create hash set of xp pools to consider splitting xp amongst
    let mut xp_pools = HashSet::<SkillGroupKind>::new();
    // Insert general pool since it is always accessible
    xp_pools.insert(SkillGroupKind::General);
    // Closure to add xp pool corresponding to weapon type equipped in a particular
    // EquipSlot
    let mut add_tool_from_slot = |equip_slot| {
        let tool_kind = inventory
            .equipped(equip_slot)
            .and_then(|i| match &*i.kind() {
                ItemKind::Tool(tool) if tool.kind.gains_combat_xp() => Some(tool.kind),
                _ => None,
            });
        if let Some(weapon) = tool_kind {
            // Only adds to xp pools if entity has that skill group available
            if skill_set.skill_group_accessible(SkillGroupKind::Weapon(weapon)) {
                xp_pools.insert(SkillGroupKind::Weapon(weapon));
            }
        }
    };
    // Add weapons to xp pools considered
    add_tool_from_slot(EquipSlot::ActiveMainhand);
    add_tool_from_slot(EquipSlot::ActiveOffhand);
    add_tool_from_slot(EquipSlot::InactiveMainhand);
    add_tool_from_slot(EquipSlot::InactiveOffhand);
    let num_pools = xp_pools.len() as f32;
    for pool in xp_pools.iter() {
        if let Some(level_outcome) =
            skill_set.add_experience(*pool, (exp_reward / num_pools).ceil() as u32)
        {
            outcomes_emitter.emit(Outcome::SkillPointGain {
                uid: *uid,
                skill_tree: *pool,
                total_points: level_outcome,
            });
        }
    }
    outcomes_emitter.emit(Outcome::ExpChange {
        uid: *uid,
        exp: exp_reward as u32,
        xp_pools,
    });
}

pub fn handle_combo_change(server: &Server, entity: EcsEntity, change: i32) {
    let ecs = &server.state.ecs();
    if let Some(mut combo) = ecs.write_storage::<comp::Combo>().get_mut(entity) {
        let time = ecs.read_resource::<Time>();
        let outcome_bus = ecs.read_resource::<EventBus<Outcome>>();
        combo.change_by(change, time.0);
        if let Some(uid) = ecs.read_storage::<Uid>().get(entity) {
            outcome_bus.emit_now(Outcome::ComboChange {
                uid: *uid,
                combo: combo.counter(),
            });
        }
    }
}

pub fn handle_parry_hook(
    server: &Server,
    defender: EcsEntity,
    attacker: Option<EcsEntity>,
    source: AttackSource,
) {
    let ecs = &server.state.ecs();
    let server_eventbus = ecs.read_resource::<EventBus<ServerEvent>>();
    // Reset character state of defender
    if let Some(mut char_state) = ecs
        .write_storage::<comp::CharacterState>()
        .get_mut(defender)
    {
        let return_to_wield = match &mut *char_state {
            CharacterState::RiposteMelee(c) => {
                c.stage_section = StageSection::Action;
                c.timer = Duration::default();
                false
            },
            CharacterState::BasicBlock(c) => {
                // Refund half the energy of entering the block for a successful parry
                server_eventbus.emit_now(ServerEvent::EnergyChange {
                    entity: defender,
                    change: c.static_data.energy_regen,
                });
                true
            },
            _ => false,
        };
        if return_to_wield {
            *char_state =
                CharacterState::Wielding(common::states::wielding::Data { is_sneaking: false });
        }
    };

    if let Some(attacker) = attacker
        && matches!(source, AttackSource::Melee)
    {
        // When attacker is parried, add the parried debuff for 2 seconds, which slows
        // them
        let data = buff::BuffData::new(1.0, Some(Secs(2.0)));
        let source = if let Some(uid) = ecs.read_storage::<Uid>().get(defender) {
            BuffSource::Character { by: *uid }
        } else {
            BuffSource::World
        };
        let time = ecs.read_resource::<Time>();
        let stats = ecs.read_storage::<comp::Stats>();
        let buff = buff::Buff::new(
            BuffKind::Parried,
            data,
            vec![buff::BuffCategory::Physical],
            source,
            *time,
            stats.get(attacker),
        );
        server_eventbus.emit_now(ServerEvent::Buff {
            entity: attacker,
            buff_change: buff::BuffChange::Add(buff),
        });
    }
}

pub fn handle_teleport_to(server: &Server, entity: EcsEntity, target: Uid, max_range: Option<f32>) {
    let ecs = &server.state.ecs();
    let mut positions = ecs.write_storage::<Pos>();

    let target_pos = ecs
        .entity_from_uid(target)
        .and_then(|e| positions.get(e))
        .copied();

    if let (Some(pos), Some(target_pos)) = (positions.get_mut(entity), target_pos) {
        if max_range.map_or(true, |r| pos.0.distance_squared(target_pos.0) < r.powi(2)) {
            *pos = target_pos;
            ecs.write_storage::<comp::ForceUpdate>()
                .get_mut(entity)
                .map(|force_update| force_update.update());
        }
    }
}

/// Intended to handle things that should happen for any successful attack,
/// regardless of the damages and effects specific to that attack
pub fn handle_entity_attacked_hook(
    server: &Server,
    entity: EcsEntity,
    attacker: Option<EcsEntity>,
) {
    let ecs = &server.state.ecs();
    let server_eventbus = ecs.read_resource::<EventBus<ServerEvent>>();

    let time = ecs.read_resource::<Time>();
    if let Some(attacker) = attacker {
        server_eventbus.emit_now(ServerEvent::Buff {
            entity: attacker,
            buff_change: buff::BuffChange::RemoveByCategory {
                all_required: vec![buff::BuffCategory::RemoveOnAttack],
                any_required: vec![],
                none_required: vec![],
            },
        });
    }

    if let (Some(mut char_state), Some(mut poise), Some(pos)) = (
        ecs.write_storage::<CharacterState>().get_mut(entity),
        ecs.write_storage::<Poise>().get_mut(entity),
        ecs.read_storage::<Pos>().get(entity),
    ) {
        // Interrupt sprite interaction and item use if any attack is applied to entity
        if matches!(
            *char_state,
            CharacterState::SpriteInteract(_) | CharacterState::UseItem(_)
        ) {
            let poise_state = comp::poise::PoiseState::Interrupted;
            let was_wielded = char_state.is_wield();
            if let (Some((stunned_state, stunned_duration)), impulse_strength) =
                poise_state.poise_effect(was_wielded)
            {
                // Reset poise if there is some stunned state to apply
                poise.reset(*time, stunned_duration);
                *char_state = stunned_state;
                ecs.read_resource::<EventBus<Outcome>>()
                    .emit_now(Outcome::PoiseChange {
                        pos: pos.0,
                        state: poise_state,
                    });
                if let Some(impulse_strength) = impulse_strength {
                    server_eventbus.emit_now(ServerEvent::Knockback {
                        entity,
                        impulse: impulse_strength * *poise.knockback(),
                    });
                }
            }
        }
    }

    // Remove potion/saturation buff if attacked
    server_eventbus.emit_now(ServerEvent::Buff {
        entity,
        buff_change: buff::BuffChange::RemoveByKind(BuffKind::Potion),
    });
    server_eventbus.emit_now(ServerEvent::Buff {
        entity,
        buff_change: buff::BuffChange::RemoveByKind(BuffKind::Saturation),
    });

    // If entity was in an active trade, cancel it
    let mut trades = ecs.write_resource::<Trades>();
    let uids = ecs.read_storage::<Uid>();
    if let Some(uid) = uids.get(entity) {
        if let Some(trade) = trades.entity_trades.get(uid).copied() {
            trades
                .decline_trade(trade, *uid)
                .and_then(|uid| ecs.entity_from_uid(uid))
                .map(|entity_b| {
                    // Notify both parties that the trade ended
                    let clients = ecs.read_storage::<Client>();
                    let mut agents = ecs.write_storage::<Agent>();
                    let mut notify_trade_party = |entity| {
                        // TODO: Can probably improve UX here for the user that sent the trade
                        // invite, since right now it may seems like their request was
                        // purposefully declined, rather than e.g. being interrupted.
                        if let Some(client) = clients.get(entity) {
                            client
                                .send_fallible(ServerGeneral::FinishedTrade(TradeResult::Declined));
                        }
                        if let Some(agent) = agents.get_mut(entity) {
                            agent
                                .inbox
                                .push_back(AgentEvent::FinishedTrade(TradeResult::Declined));
                        }
                    };
                    notify_trade_party(entity);
                    notify_trade_party(entity_b);
                });
        }
    }

    let stats = ecs.read_storage::<Stats>();
    if let Some(stats) = stats.get(entity) {
        for effect in &stats.effects_on_damaged {
            use combat::DamagedEffect;
            match effect {
                DamagedEffect::Combo(c) => {
                    server_eventbus.emit_now(ServerEvent::ComboChange { entity, change: *c });
                },
            }
        }
    }
}

pub fn handle_change_ability(
    server: &Server,
    entity: EcsEntity,
    slot: usize,
    auxiliary_key: ability::AuxiliaryKey,
    new_ability: ability::AuxiliaryAbility,
) {
    let ecs = &server.state.ecs();
    let inventories = ecs.read_storage::<Inventory>();
    let skill_sets = ecs.read_storage::<SkillSet>();

    if let Some(mut active_abilities) = ecs.write_storage::<comp::ActiveAbilities>().get_mut(entity)
    {
        active_abilities.change_ability(
            slot,
            auxiliary_key,
            new_ability,
            inventories.get(entity),
            skill_sets.get(entity),
        );
    }
}

pub fn handle_update_map_marker(
    server: &mut Server,
    entity: EcsEntity,
    update: comp::MapMarkerChange,
) {
    use comp::{MapMarker, MapMarkerChange::*};
    match update {
        Update(waypoint) => {
            server
                .state
                .write_component_ignore_entity_dead(entity, MapMarker(waypoint));
        },
        Remove => {
            server.state.delete_component::<MapMarker>(entity);
        },
    }
    let ecs = server.state.ecs_mut();
    // Send updated waypoint to group members
    let groups = ecs.read_storage();
    let uids = ecs.read_storage();
    if let Some((group_id, uid)) = groups.get(entity).zip(uids.get(entity)) {
        let clients = ecs.read_storage::<Client>();
        for client in comp::group::members(
            *group_id,
            &groups,
            &ecs.entities(),
            &ecs.read_storage(),
            &uids,
        )
        .filter_map(|(e, _)| if e != entity { clients.get(e) } else { None })
        {
            client.send_fallible(ServerGeneral::MapMarker(
                comp::MapMarkerUpdate::GroupMember(*uid, update),
            ));
        }
    }
}

pub fn handle_make_admin(server: &mut Server, entity: EcsEntity, admin: comp::Admin, uuid: Uuid) {
    if server
        .state
        .read_storage::<Player>()
        .get(entity)
        .map_or(false, |player| player.uuid() == uuid)
    {
        server
            .state
            .write_component_ignore_entity_dead(entity, admin);
    }
}

pub fn handle_stance_change(server: &mut Server, entity: EcsEntity, new_stance: comp::Stance) {
    if let Some(mut stance) = server
        .state
        .ecs_mut()
        .write_storage::<comp::Stance>()
        .get_mut(entity)
    {
        *stance = new_stance;
    }
}

pub fn handle_change_body(server: &mut Server, entity: EcsEntity, new_body: comp::Body) {
    if let Some(mut body) = server
        .state
        .ecs_mut()
        .write_storage::<comp::Body>()
        .get_mut(entity)
    {
        *body = new_body;
    }
}

pub fn handle_remove_light_emitter(server: &mut Server, entity: EcsEntity) {
    server
        .state
        .ecs_mut()
        .write_storage::<comp::LightEmitter>()
        .remove(entity);
}

pub fn handle_teleport_to_position(server: &mut Server, entity: EcsEntity, position: Vec3<f32>) {
    if let Err(error) = server
        .state
        .position_mut(entity, true, |pos| pos.0 = position)
    {
        warn!(?error, "Failed to teleport entity");
    }
}

pub fn handle_start_teleporting(server: &mut Server, entity: EcsEntity, portal: EcsEntity) {
    let ecs = server.state.ecs();
    let positions = ecs.read_storage::<comp::Pos>();
    let mut teleportings = ecs.write_storage::<comp::Teleporting>();
    let now = ecs.read_resource::<Time>().0;

    if let Some(end_time) = (!teleportings.contains(entity))
        .then(|| positions.get(entity))
        .flatten()
        .zip(positions.get(portal))
        .filter(|(entity_pos, portal_pos)| {
            entity_pos.0.distance_squared(portal_pos.0) <= TELEPORTER_RADIUS.powi(2)
        })
        .and_then(|(_, _)| {
            Some(
                now + ecs
                    .read_storage::<comp::Object>()
                    .get(portal)
                    .and_then(|object| {
                        if let Object::Portal { buildup_time, .. } = object {
                            Some(buildup_time.0)
                        } else {
                            None
                        }
                    })?,
            )
        })
    {
        let _ = teleportings.insert(entity, comp::Teleporting {
            portal,
            end_time: Time(end_time),
        });
    }
}
