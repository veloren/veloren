use crate::{
    client::Client,
    comp::{
        agent::{Agent, AgentEvent, Sound, SoundKind},
        loot_owner::LootOwner,
        skillset::SkillGroupKind,
        BuffKind, BuffSource, PhysicsState,
    },
    error,
    rtsim::RtSim,
    state_ext::StateExt,
    sys::terrain::SAFE_ZONE_RADIUS,
    Server, Settings, SpawnPoint,
};
use common::{
    combat,
    combat::{AttackSource, DamageContributor},
    comp::{
        self, aura, buff,
        chat::{KillSource, KillType},
        inventory::item::{AbilityMap, MaterialStatManifest},
        item::flatten_counted_items,
        loot_owner::LootOwnerKind,
        Alignment, Auras, Body, CharacterState, Energy, Group, Health, Inventory, Object, Player,
        Poise, Pos, Presence, PresenceKind, SkillSet, Stats,
    },
    consts::TELEPORTER_RADIUS,
    event::{
        AuraEvent, BonkEvent, BuffEvent, ChangeAbilityEvent, ChangeBodyEvent, ChangeStanceEvent,
        ChatEvent, ComboChangeEvent, CreateItemDropEvent, CreateObjectEvent, DeleteEvent,
        DestroyEvent, EmitExt, Emitter, EnergyChangeEvent, EntityAttackedHookEvent, EventBus,
        ExplosionEvent, HealthChangeEvent, KnockbackEvent, LandOnGroundEvent, MakeAdminEvent,
        ParryHookEvent, PoiseChangeEvent, RemoveLightEmitterEvent, RespawnEvent, SoundEvent,
        StartTeleportingEvent, TeleportToEvent, TeleportToPositionEvent, UpdateMapMarkerEvent,
    },
    event_emitters,
    link::Is,
    lottery::distribute_many,
    mounting::{Rider, VolumeRider},
    outcome::{HealthChangeInfo, Outcome},
    resources::{Secs, Time},
    rtsim::{Actor, RtSimEntity},
    spiral::Spiral2d,
    states::utils::StageSection,
    terrain::{Block, BlockKind, TerrainGrid},
    trade::{TradeResult, Trades},
    uid::{IdMaps, Uid},
    util::Dir,
    vol::ReadVol,
    CachedSpatialGrid, Damage, DamageKind, DamageSource, GroupTarget, RadiusEffect,
};
use common_net::msg::ServerGeneral;
use common_state::{AreasContainer, BlockChange, NoDurabilityArea};
use hashbrown::HashSet;
use rand::Rng;
use specs::{
    shred, DispatcherBuilder, Entities, Entity as EcsEntity, Entity, Join, LendJoin, Read,
    ReadExpect, ReadStorage, SystemData, Write, WriteExpect, WriteStorage,
};
use std::{collections::HashMap, iter, sync::Arc, time::Duration};
use tracing::{debug, warn};
use vek::{Vec2, Vec3};
use world::World;

use super::{event_dispatch, ServerEvent};

pub(super) fn register_event_systems(builder: &mut DispatcherBuilder) {
    event_dispatch::<PoiseChangeEvent>(builder);
    event_dispatch::<HealthChangeEvent>(builder);
    event_dispatch::<KnockbackEvent>(builder);
    event_dispatch::<DestroyEvent>(builder);
    event_dispatch::<LandOnGroundEvent>(builder);
    event_dispatch::<RespawnEvent>(builder);
    event_dispatch::<ExplosionEvent>(builder);
    event_dispatch::<BonkEvent>(builder);
    event_dispatch::<AuraEvent>(builder);
    event_dispatch::<BuffEvent>(builder);
    event_dispatch::<EnergyChangeEvent>(builder);
    event_dispatch::<ComboChangeEvent>(builder);
    event_dispatch::<ParryHookEvent>(builder);
    event_dispatch::<TeleportToEvent>(builder);
    event_dispatch::<EntityAttackedHookEvent>(builder);
    event_dispatch::<ChangeAbilityEvent>(builder);
    event_dispatch::<UpdateMapMarkerEvent>(builder);
    event_dispatch::<MakeAdminEvent>(builder);
    event_dispatch::<ChangeStanceEvent>(builder);
    event_dispatch::<ChangeBodyEvent>(builder);
    event_dispatch::<RemoveLightEmitterEvent>(builder);
    event_dispatch::<TeleportToPositionEvent>(builder);
    event_dispatch::<StartTeleportingEvent>(builder);
}

pub fn handle_delete(server: &mut Server, DeleteEvent(entity): DeleteEvent) {
    let _ = server
        .state_mut()
        .delete_entity_recorded(entity)
        .map_err(|e| error!(?e, ?entity, "Failed to delete destroyed entity"));
}

#[derive(Hash, Eq, PartialEq)]
enum DamageContrib {
    Solo(EcsEntity),
    Group(Group),
    NotFound,
}

impl ServerEvent for PoiseChangeEvent {
    type SystemData<'a> = (
        Entities<'a>,
        ReadStorage<'a, CharacterState>,
        WriteStorage<'a, Poise>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (entities, character_states, mut poises): Self::SystemData<'_>,
    ) {
        for ev in events {
            if let Some((character_state, mut poise)) = (&character_states, &mut poises)
                .lend_join()
                .get(ev.entity, &entities)
            {
                // Entity is invincible to poise change during stunned character state
                if !matches!(character_state, CharacterState::Stunned(_)) {
                    poise.change(ev.change);
                }
            }
        }
    }
}

impl ServerEvent for HealthChangeEvent {
    type SystemData<'a> = (
        Entities<'a>,
        Read<'a, EventBus<Outcome>>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Uid>,
        WriteStorage<'a, Agent>,
        WriteStorage<'a, Health>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (entities, outcomes, positions, uids, mut agents, mut healths): Self::SystemData<'_>,
    ) {
        let mut outcomes_emitter = outcomes.emitter();
        for ev in events {
            if let Some((mut health, pos, uid)) = (&mut healths, positions.maybe(), uids.maybe())
                .lend_join()
                .get(ev.entity, &entities)
            {
                // If the change amount was not zero
                let changed = health.change_by(ev.change);
                if let (Some(pos), Some(uid)) = (pos, uid) {
                    if changed {
                        outcomes_emitter.emit(Outcome::HealthChange {
                            pos: pos.0,
                            info: HealthChangeInfo {
                                amount: ev.change.amount,
                                by: ev.change.by,
                                target: *uid,
                                cause: ev.change.cause,
                                precise: ev.change.precise,
                                instance: ev.change.instance,
                            },
                        });
                    }
                }
            }

            // This if statement filters out anything under 5 damage, for DOT ticks
            // TODO: Find a better way to separate direct damage from DOT here
            let damage = -ev.change.amount;
            if damage > 5.0 {
                if let Some(agent) = agents.get_mut(ev.entity) {
                    agent.inbox.push_back(AgentEvent::Hurt);
                }
            }
        }
    }
}

impl ServerEvent for KnockbackEvent {
    type SystemData<'a> = (
        Entities<'a>,
        ReadStorage<'a, Client>,
        ReadStorage<'a, PhysicsState>,
        ReadStorage<'a, comp::Mass>,
        WriteStorage<'a, comp::Vel>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (entities, clients, physic_states, mass, mut velocities): Self::SystemData<'_>,
    ) {
        for ev in events {
            if let Some((physics, mass, vel, client)) = (
                &physic_states,
                mass.maybe(),
                &mut velocities,
                clients.maybe(),
            )
                .lend_join()
                .get(ev.entity, &entities)
            {
                //Check if the entity is on a surface. If it is not, reduce knockback.
                let mut impulse = ev.impulse
                    * if physics.on_surface().is_some() {
                        1.0
                    } else {
                        0.4
                    };

                // we go easy on the little ones (because they fly so far)
                impulse /= mass.map_or(0.0, |m| m.0).max(40.0);

                vel.0 += impulse;
                if let Some(client) = client {
                    client.send_fallible(ServerGeneral::Knockback(impulse));
                }
            }
        }
    }
}

fn handle_exp_gain(
    exp_reward: f32,
    inventory: &Inventory,
    skill_set: &mut SkillSet,
    uid: &Uid,
    outcomes_emitter: &mut Emitter<Outcome>,
) {
    use comp::inventory::{item::ItemKind, slot::EquipSlot};

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

#[derive(SystemData)]
pub struct DestroyEventData<'a> {
    entities: Entities<'a>,
    rtsim: WriteExpect<'a, RtSim>,
    id_maps: Read<'a, IdMaps>,
    msm: ReadExpect<'a, MaterialStatManifest>,
    ability_map: ReadExpect<'a, AbilityMap>,
    time: Read<'a, Time>,
    world: ReadExpect<'a, Arc<World>>,
    index: ReadExpect<'a, world::IndexOwned>,
    areas_container: Read<'a, AreasContainer<NoDurabilityArea>>,
    outcomes: Read<'a, EventBus<Outcome>>,
    create_item_drop: Read<'a, EventBus<CreateItemDropEvent>>,
    delete_event: Read<'a, EventBus<DeleteEvent>>,
    chat_events: Read<'a, EventBus<ChatEvent>>,
    melees: WriteStorage<'a, comp::Melee>,
    beams: WriteStorage<'a, comp::Beam>,
    skill_sets: WriteStorage<'a, SkillSet>,
    inventories: WriteStorage<'a, Inventory>,
    item_drops: WriteStorage<'a, comp::ItemDrops>,
    velocities: WriteStorage<'a, comp::Vel>,
    force_updates: WriteStorage<'a, comp::ForceUpdate>,
    energies: WriteStorage<'a, Energy>,
    character_states: WriteStorage<'a, CharacterState>,
    players: ReadStorage<'a, Player>,
    clients: ReadStorage<'a, Client>,
    uids: ReadStorage<'a, Uid>,
    positions: ReadStorage<'a, Pos>,
    healths: ReadStorage<'a, Health>,
    bodies: ReadStorage<'a, Body>,
    poises: ReadStorage<'a, Poise>,
    groups: ReadStorage<'a, Group>,
    alignments: ReadStorage<'a, Alignment>,
    stats: ReadStorage<'a, Stats>,
    agents: ReadStorage<'a, Agent>,
    rtsim_entities: ReadStorage<'a, RtSimEntity>,
    presences: ReadStorage<'a, Presence>,
}

/// Handle an entity dying. If it is a player, it will send a message to all
/// other players. If the entity that killed it had stats, then give it exp for
/// the kill. Experience given is equal to the level of the entity that was
/// killed times 10.
impl ServerEvent for DestroyEvent {
    type SystemData<'a> = DestroyEventData<'a>;

    fn handle(events: impl ExactSizeIterator<Item = Self>, mut data: Self::SystemData<'_>) {
        let mut chat_emitter = data.chat_events.emitter();
        let mut create_item_drop = data.create_item_drop.emitter();
        let mut delete_emitter = data.delete_event.emitter();
        let mut outcomes_emitter = data.outcomes.emitter();
        for ev in events {
            // TODO: Investigate duplicate `Destroy` events (but don't remove this).
            // If the entity was already deleted, it can't be destroyed again.
            if !data.entities.is_alive(ev.entity) {
                continue;
            }
            let mut outcomes = data.outcomes.emitter();

            // Remove components that should not persist across death
            data.melees.remove(ev.entity);
            data.beams.remove(ev.entity);

            let get_attacker_name = |cause_of_death: KillType, by: Uid| -> KillSource {
                // Get attacker entity
                if let Some(char_entity) = data.id_maps.uid_entity(by) {
                    // Check if attacker is another player or entity with stats (npc)
                    if data.players.contains(char_entity) {
                        KillSource::Player(by, cause_of_death)
                    } else if let Some(stats) = data.stats.get(char_entity) {
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
            if let Some((pos, _)) = (&data.positions, &data.character_states)
                .lend_join()
                .get(ev.entity, &data.entities)
            {
                outcomes_emitter.emit(Outcome::Death { pos: pos.0 });
            }

            // Chat message
            // If it was a player that died
            if let Some((uid, _player)) = (&data.uids, &data.players)
                .lend_join()
                .get(ev.entity, &data.entities)
            {
                let kill_source = match (ev.cause.cause, ev.cause.by.map(|x| x.uid())) {
                    (Some(DamageSource::Melee), Some(by)) => get_attacker_name(KillType::Melee, by),
                    (Some(DamageSource::Projectile), Some(by)) => {
                        get_attacker_name(KillType::Projectile, by)
                    },
                    (Some(DamageSource::Explosion), Some(by)) => {
                        get_attacker_name(KillType::Explosion, by)
                    },
                    (Some(DamageSource::Energy), Some(by)) => {
                        get_attacker_name(KillType::Energy, by)
                    },
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

                chat_emitter.emit(ChatEvent(comp::UnresolvedChatMsg::death(kill_source, *uid)));
            }

            let mut exp_awards = Vec::<(Entity, f32, Option<Group>)>::new();
            // Award EXP to damage contributors
            //
            // NOTE: Debug logging is disabled by default for this module - to enable it add
            // veloren_server::events::entity_manipulation=debug to RUST_LOG
            'xp: {
                let Some((
                    entity_skill_set,
                    entity_health,
                    entity_energy,
                    entity_inventory,
                    entity_body,
                    entity_poise,
                    entity_pos,
                )) = (
                    &data.skill_sets,
                    &data.healths,
                    &data.energies,
                    &data.inventories,
                    &data.bodies,
                    &data.poises,
                    &data.positions,
                )
                    .lend_join()
                    .get(ev.entity, &data.entities)
                else {
                    break 'xp;
                };

                // Calculate the total EXP award for the kill
                let exp_reward = combat::combat_rating(
                    entity_inventory,
                    entity_health,
                    entity_energy,
                    entity_poise,
                    entity_skill_set,
                    *entity_body,
                    &data.msm,
                ) * 20.0;

                let mut damage_contributors = HashMap::<DamageContrib, (u64, f32)>::new();
                for (damage_contributor, damage) in entity_health.damage_contributions() {
                    match damage_contributor {
                        DamageContributor::Solo(uid) => {
                            if let Some(attacker) = data.id_maps.uid_entity(*uid) {
                                damage_contributors
                                    .insert(DamageContrib::Solo(attacker), (*damage, 0.0));
                            } else {
                                // An entity who was not in a group contributed damage but is now
                                // either dead or offline. Add a
                                // placeholder to ensure that the contributor's
                                // exp is discarded, not distributed between
                                // the other contributors
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

                let destroyed_group = data.groups.get(ev.entity);

                let within_range = |attacker_pos: &Pos| {
                    // Maximum distance that an attacker must be from an entity at the time of its
                    // death to receive EXP for the kill
                    const MAX_EXP_DIST: f32 = 150.0;
                    entity_pos.0.distance_squared(attacker_pos.0) < MAX_EXP_DIST.powi(2)
                };

                let is_pvp_kill = |attacker: Entity| {
                    data.players.contains(ev.entity) && data.players.contains(attacker)
                };

                // Iterate through all contributors of damage for the killed entity, calculating
                // how much EXP each contributor should be awarded based on their
                // percentage of damage contribution
                exp_awards = damage_contributors.iter().filter_map(|(damage_contributor, (_, damage_percent))| {
                let contributor_exp = exp_reward * damage_percent;
                match damage_contributor {
                    DamageContrib::Solo(attacker) => {
                        // No exp for self kills or PvP
                        if *attacker == ev.entity || is_pvp_kill(*attacker) { return None; }

                        // Only give EXP to the attacker if they are within EXP range of the killed entity
                        data.positions.get(*attacker).and_then(|attacker_pos| {
                            if within_range(attacker_pos) {
                                debug!("Awarding {} exp to individual {:?} who contributed {}% damage to the kill of {:?}", contributor_exp, attacker, *damage_percent * 100.0, ev.entity);
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
                            &data.entities,
                            &data.groups,
                            &data.positions,
                            data.alignments.maybe(),
                            &data.uids,
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

                        debug!("Awarding {} exp per member of group ID {:?} with {} members which contributed {}% damage to the kill of {:?}", exp_per_member, group, members_in_range.len(), *damage_percent * 100.0, ev.entity);
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
                    if let Some((mut attacker_skill_set, attacker_uid, attacker_inventory)) =
                        (&mut data.skill_sets, &data.uids, &data.inventories)
                            .lend_join()
                            .get(*attacker, &data.entities)
                    {
                        handle_exp_gain(
                            *exp_reward,
                            attacker_inventory,
                            &mut attacker_skill_set,
                            attacker_uid,
                            &mut outcomes,
                        );
                    }
                });
            };

            let should_delete = if data.clients.contains(ev.entity) {
                if let Some(vel) = data.velocities.get_mut(ev.entity) {
                    vel.0 = Vec3::zero();
                }
                if let Some(force_update) = data.force_updates.get_mut(ev.entity) {
                    force_update.update();
                }
                if let Some(mut energy) = data.energies.get_mut(ev.entity) {
                    energy.refresh();
                }
                if let Some(mut character_state) = data.character_states.get_mut(ev.entity) {
                    *character_state = CharacterState::default();
                }

                false
            } else {
                if let Some((_agent, uid, pos, alignment, vel, body)) = (
                    &data.agents,
                    &data.uids,
                    &data.positions,
                    data.alignments.maybe(),
                    data.velocities.maybe(),
                    data.bodies.maybe(),
                )
                    .lend_join()
                    .get(ev.entity, &data.entities)
                {
                    // Only drop loot if entity has agency (not a player),
                    // and if it is not owned by another entity (not a pet)
                    if !matches!(alignment, Some(Alignment::Owned(_)))
                        && let Some(items) = data
                            .item_drops
                            .remove(ev.entity)
                            .map(|comp::ItemDrops(item)| item)
                    {
                        // Remove entries where zero exp was awarded - this happens because some
                        // entities like Object bodies don't give EXP.
                        let mut item_receivers = HashMap::new();
                        for (_entity, exp, group) in exp_awards {
                            if exp >= f32::EPSILON {
                                let loot_owner = if let Some(group) = group {
                                    Some(LootOwnerKind::Group(group))
                                } else {
                                    let uid = body.and_then(|body| {
                                        // Only humanoids are awarded loot ownership - if the winner
                                        // was a non-humanoid NPC the loot will be free-for-all
                                        if matches!(body, Body::Humanoid(_)) {
                                            Some(*uid)
                                        } else {
                                            None
                                        }
                                    });

                                    uid.map(LootOwnerKind::Player)
                                };

                                *item_receivers.entry(loot_owner).or_insert(0.0) += exp;
                            }
                        }

                        let mut item_offset_spiral =
                            Spiral2d::new().map(|offset| offset.as_::<f32>() * 0.5);

                        let mut rng = rand::thread_rng();
                        let mut spawn_item = |item, loot_owner| {
                            let offset = item_offset_spiral.next().unwrap_or_default();
                            create_item_drop.emit(CreateItemDropEvent {
                                pos: Pos(pos.0 + Vec3::unit_z() * 0.25 + offset),
                                vel: vel.copied().unwrap_or(comp::Vel(Vec3::zero())),
                                // TODO: Random
                                ori: comp::Ori::from(Dir::random_2d(&mut rng)),
                                item,
                                loot_owner: if let Some(loot_owner) = loot_owner {
                                    debug!(
                                        "Assigned UID {loot_owner:?} as the winner for the loot \
                                         drop"
                                    );
                                    Some(LootOwner::new(loot_owner, false))
                                } else {
                                    None
                                },
                            })
                        };

                        if item_receivers.is_empty() {
                            for item in flatten_counted_items(&items, &data.ability_map, &data.msm)
                            {
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
                                    for item in
                                        item.stacked_duplicates(&data.ability_map, &data.msm, count)
                                    {
                                        spawn_item(item, loot_owner)
                                    }
                                },
                            );
                        }
                    }
                }
                true
            };
            if !should_delete {
                let resists_durability =
                    data.positions
                        .get(ev.entity)
                        .cloned()
                        .map_or(false, |our_pos| {
                            let our_pos = our_pos.0.map(|i| i as i32);

                            let is_in_area = data
                                .areas_container
                                .areas()
                                .iter()
                                .any(|(_, area)| area.contains_point(our_pos));

                            is_in_area
                        });

                // Modify durability on all equipped items
                if !resists_durability
                    && let Some(mut inventory) = data.inventories.get_mut(ev.entity)
                {
                    inventory.damage_items(&data.ability_map, &data.msm, *data.time);
                }
            }

            let entity_as_actor = |entity| {
                if let Some(rtsim_entity) = data.rtsim_entities.get(entity).copied() {
                    Some(Actor::Npc(rtsim_entity.0))
                } else if let Some(PresenceKind::Character(character)) =
                    data.presences.get(entity).map(|p| p.kind)
                {
                    Some(Actor::Character(character))
                } else {
                    None
                }
            };
            let actor = entity_as_actor(ev.entity);

            if let Some(actor) = actor {
                data.rtsim.hook_rtsim_actor_death(
                &data.world,
                data.index.as_index_ref(),
                actor,
                data.positions.get(ev.entity).map(|p| p.0),
                ev.cause
                    .by
                    .as_ref()
                    .and_then(
                        |(DamageContributor::Solo(entity_uid)
                         | DamageContributor::Group { entity_uid, .. })| {
                            data.id_maps.uid_entity(*entity_uid)
                        },
                    )
                    .and_then(entity_as_actor),
            );
            }

            if should_delete {
                delete_emitter.emit(DeleteEvent(ev.entity));
            }
        }
    }
}

impl ServerEvent for LandOnGroundEvent {
    type SystemData<'a> = (
        Read<'a, Time>,
        ReadExpect<'a, MaterialStatManifest>,
        Read<'a, EventBus<HealthChangeEvent>>,
        Read<'a, EventBus<PoiseChangeEvent>>,
        ReadStorage<'a, PhysicsState>,
        ReadStorage<'a, CharacterState>,
        ReadStorage<'a, comp::Mass>,
        ReadStorage<'a, Inventory>,
        ReadStorage<'a, Stats>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (
            time,
            msm,
            health_change_events,
            poise_change_events,
            physic_states,
            character_states,
            masses,
            inventories,
            stats,
        ): Self::SystemData<'_>,
    ) {
        let mut health_change_emitter = health_change_events.emitter();
        let mut poise_change_emitter = poise_change_events.emitter();
        for ev in events {
            // HACK: Certain ability movements currently take us above the fall damage
            // threshold in the horizontal axis. This factor dampens velocity in the
            // horizontal axis when applying fall damage.
            let horizontal_damp = 0.5
                + ev.vel
                    .try_normalized()
                    .unwrap_or_default()
                    .dot(Vec3::unit_z())
                    .abs()
                    * 0.5;

            let relative_vel = ev.vel.dot(-ev.surface_normal) * horizontal_damp;
            // The second part of this if statement disables all fall damage when in the
            // water. This was added as a *temporary* fix a bug that causes you to take
            // fall damage while swimming downwards. FIXME: Fix the actual bug and
            // remove the following relevant part of the if statement.
            if relative_vel >= 30.0
                && physic_states
                    .get(ev.entity)
                    .map_or(true, |ps| ps.in_liquid().is_none())
            {
                let reduced_vel =
                    if let Some(CharacterState::DiveMelee(c)) = character_states.get(ev.entity) {
                        (relative_vel + c.static_data.vertical_speed).min(0.0)
                    } else {
                        relative_vel
                    };

                let mass = masses.get(ev.entity).copied().unwrap_or_default();
                let impact_energy = mass.0 * reduced_vel.powi(2) / 2.0;
                let falldmg = impact_energy / 1000.0;

                // Emit health change
                let damage = Damage {
                    source: DamageSource::Falling,
                    kind: DamageKind::Crushing,
                    value: falldmg,
                };
                let damage_reduction = Damage::compute_damage_reduction(
                    Some(damage),
                    inventories.get(ev.entity),
                    stats.get(ev.entity),
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

                health_change_emitter.emit(HealthChangeEvent {
                    entity: ev.entity,
                    change,
                });

                // Emit poise change
                let poise_damage = -(mass.0 * reduced_vel.powi(2) / 1500.0);
                let poise_change = Poise::apply_poise_reduction(
                    poise_damage,
                    inventories.get(ev.entity),
                    &msm,
                    character_states.get(ev.entity),
                    stats.get(ev.entity),
                );
                let poise_change = comp::PoiseChange {
                    amount: poise_change,
                    impulse: Vec3::unit_z(),
                    by: None,
                    cause: None,
                    time: *time,
                };
                poise_change_emitter.emit(PoiseChangeEvent {
                    entity: ev.entity,
                    change: poise_change,
                });
            }
        }
    }
}

impl ServerEvent for RespawnEvent {
    type SystemData<'a> = (
        Read<'a, SpawnPoint>,
        WriteStorage<'a, Health>,
        WriteStorage<'a, comp::Combo>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, comp::PhysicsState>,
        WriteStorage<'a, comp::ForceUpdate>,
        ReadStorage<'a, Client>,
        ReadStorage<'a, comp::Waypoint>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (
            spawn_point,
            mut healths,
            mut combos,
            mut positions,
            mut physic_states,
            mut force_updates,
            clients,
            waypoints,
        ): Self::SystemData<'_>,
    ) {
        for RespawnEvent(entity) in events {
            if clients.contains(entity) {
                let respawn_point = waypoints
                    .get(entity)
                    .map(|wp| wp.get_pos())
                    .unwrap_or(spawn_point.0);

                healths.get_mut(entity).map(|mut health| health.revive());
                combos.get_mut(entity).map(|mut combo| combo.reset());
                positions.get_mut(entity).map(|pos| pos.0 = respawn_point);
                physic_states
                    .get_mut(entity)
                    .map(|phys_state| phys_state.reset());
                force_updates
                    .get_mut(entity)
                    .map(|force_update| force_update.update());
            }
        }
    }
}

event_emitters! {
    struct ReadExplosionEvents[ExplosionEmitters] {
        health_change: HealthChangeEvent,
        energy_change: EnergyChangeEvent,
        poise_change: PoiseChangeEvent,
        sound: SoundEvent,
        parry_hook: ParryHookEvent,
        kockback: KnockbackEvent,
        entity_attack_hoow: EntityAttackedHookEvent,
        combo_change: ComboChangeEvent,
        buff: BuffEvent,
        bonk: BonkEvent,
    }
}

impl ServerEvent for ExplosionEvent {
    type SystemData<'a> = (
        Entities<'a>,
        Write<'a, BlockChange>,
        Read<'a, Settings>,
        Read<'a, Time>,
        Read<'a, IdMaps>,
        Read<'a, CachedSpatialGrid>,
        ReadExpect<'a, TerrainGrid>,
        ReadExpect<'a, MaterialStatManifest>,
        ReadExplosionEvents<'a>,
        Read<'a, EventBus<Outcome>>,
        ReadStorage<'a, Group>,
        ReadStorage<'a, Auras>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, Energy>,
        ReadStorage<'a, comp::Combo>,
        ReadStorage<'a, Inventory>,
        ReadStorage<'a, Alignment>,
        ReadStorage<'a, comp::Buffs>,
        ReadStorage<'a, comp::Stats>,
        ReadStorage<'a, Health>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, comp::Ori>,
        ReadStorage<'a, CharacterState>,
        ReadStorage<'a, Uid>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (
            entities,
            mut block_change,
            settings,
            time,
            id_maps,
            spatial_grid,
            terrain,
            msm,
            event_busses,
            outcomes,
            groups,
            auras,
            positions,
            players,
            energies,
            combos,
            inventories,
            alignments,
            buffs,
            stats,
            healths,
            bodies,
            orientations,
            character_states,
            uids,
        ): Self::SystemData<'_>,
    ) {
        let mut emitters = event_busses.get_emitters();
        let mut outcome_emitter = outcomes.emitter();

        // TODO: Faster RNG?
        let mut rng = rand::thread_rng();

        for ev in events {
            let owner_entity = ev.owner.and_then(|uid| id_maps.uid_entity(uid));

            let explosion_volume = 6.25 * ev.explosion.radius;

            emitters.emit(SoundEvent {
                sound: Sound::new(SoundKind::Explosion, ev.pos, explosion_volume, time.0),
            });

            let outcome_power = ev.explosion.radius;
            outcome_emitter.emit(Outcome::Explosion {
                pos: ev.pos,
                power: outcome_power,
                radius: ev.explosion.radius,
                is_attack: ev
                    .explosion
                    .effects
                    .iter()
                    .any(|e| matches!(e, RadiusEffect::Attack(_))),
                reagent: ev.explosion.reagent,
            });

            /// Used to get strength of explosion effects as they falloff over
            /// distance
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

            // TODO: Process terrain destruction first so that entities don't get protected
            // by terrain that gets destroyed?
            'effects: for effect in ev.explosion.effects {
                match effect {
                    RadiusEffect::TerrainDestruction(power, new_color) => {
                        const RAYS: usize = 500;

                        // Prevent block colour changes within the radius of a safe zone aura
                        if spatial_grid
                            .0
                            .in_circle_aabr(ev.pos.xy(), SAFE_ZONE_RADIUS)
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
                                ev.pos.distance_squared(aura_pos.0) < aura_radius.powi(2)
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

                            let _ = terrain
                                .ray(ev.pos, ev.pos + dir * color_range)
                                .until(|_| rng.gen::<f32>() < 0.05)
                                .for_each(|_: &Block, pos| touched_blocks.push(pos))
                                .cast();
                        }

                        for block_pos in touched_blocks {
                            if let Ok(block) = terrain.get(block_pos) {
                                if !matches!(block.kind(), BlockKind::Lava | BlockKind::GlowingRock)
                                    && (
                                        // Check that owner is not player or explosion_burn_marks by
                                        // players
                                        // is enabled
                                        owner_entity.map_or(true, |e| players.get(e).is_none())
                                            || settings.gameplay.explosion_burn_marks
                                    )
                                {
                                    let diff2 =
                                        block_pos.map(|b| b as f32).distance_squared(ev.pos);
                                    let fade = (1.0 - diff2 / color_range.powi(2)).max(0.0);
                                    if let Some(mut color) = block.get_color() {
                                        let r = color[0] as f32
                                            + (fade
                                                * (color[0] as f32 * 0.5 - color[0] as f32
                                                    + new_color[0]));
                                        let g = color[1] as f32
                                            + (fade
                                                * (color[1] as f32 * 0.3 - color[1] as f32
                                                    + new_color[1]));
                                        let b = color[2] as f32
                                            + (fade
                                                * (color[2] as f32 * 0.3 - color[2] as f32
                                                    + new_color[2]));
                                        // Darken blocks, but not too much
                                        color[0] = (r as u8).max(30);
                                        color[1] = (g as u8).max(30);
                                        color[2] = (b as u8).max(30);
                                        block_change
                                            .set(block_pos, Block::new(block.kind(), color));
                                    }
                                }

                                if block.is_bonkable() {
                                    emitters.emit(BonkEvent {
                                        pos: block_pos.map(|e| e as f32 + 0.5),
                                        owner: ev.owner,
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

                            let from = ev.pos;
                            let to = ev.pos + dir * power;
                            let _ = terrain
                                .ray(from, to)
                                .while_(|block: &Block| {
                                    ray_energy -= block.explode_power().unwrap_or(0.0)
                                        + rng.gen::<f32>() * 0.1;

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
                        for (
                            entity_b,
                            pos_b,
                            health_b,
                            (body_b_maybe, ori_b_maybe, char_state_b_maybe, uid_b),
                        ) in (
                            &entities,
                            &positions,
                            &healths,
                            (
                                bodies.maybe(),
                                orientations.maybe(),
                                character_states.maybe(),
                                &uids,
                            ),
                        )
                            .join()
                            .filter(|(_, _, h, _)| !h.is_dead)
                        {
                            let dist_sqrd = ev.pos.distance_squared(pos_b.0);

                            // Check if it is a hit
                            let strength = if let Some(body) = body_b_maybe {
                                cylinder_sphere_strength(
                                    ev.pos,
                                    ev.explosion.radius,
                                    ev.explosion.min_falloff,
                                    pos_b.0,
                                    *body,
                                )
                            } else {
                                1.0 - dist_sqrd / ev.explosion.radius.powi(2)
                            };

                            // Cast a ray from the explosion to the entity to check visibility
                            if strength > 0.0
                                && (terrain
                                    .ray(ev.pos, pos_b.0)
                                    .until(Block::is_opaque)
                                    .cast()
                                    .0
                                    + 0.1)
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
                                    (pos_b.0 - ev.pos)
                                        .try_normalized()
                                        .unwrap_or_else(Vec3::unit_z),
                                );

                                let attacker_info =
                                    owner_entity.zip(ev.owner).map(|(entity, uid)| {
                                        combat::AttackerInfo {
                                            entity,
                                            uid,
                                            group: groups.get(entity),
                                            energy: energies.get(entity),
                                            combo: combos.get(entity),
                                            inventory: inventories.get(entity),
                                            stats: stats.get(entity),
                                        }
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
                                let may_harm = combat::may_harm(
                                    &alignments,
                                    &players,
                                    &id_maps,
                                    owner_entity,
                                    entity_b,
                                );
                                let attack_options = combat::AttackOptions {
                                    target_dodging,
                                    may_harm,
                                    target_group,
                                    precision_mult: None,
                                };

                                attack.apply_attack(
                                    attacker_info,
                                    &target_info,
                                    dir,
                                    attack_options,
                                    strength,
                                    combat::AttackSource::Explosion,
                                    *time,
                                    &mut emitters,
                                    |o| outcome_emitter.emit(o),
                                    &mut rng,
                                    0,
                                );
                            }
                        }
                    },
                    RadiusEffect::Entity(mut effect) => {
                        for (entity_b, pos_b, body_b_maybe) in
                            (&entities, &positions, bodies.maybe()).join()
                        {
                            let strength = if let Some(body) = body_b_maybe {
                                cylinder_sphere_strength(
                                    ev.pos,
                                    ev.explosion.radius,
                                    ev.explosion.min_falloff,
                                    pos_b.0,
                                    *body,
                                )
                            } else {
                                let distance_squared = ev.pos.distance_squared(pos_b.0);
                                1.0 - distance_squared / ev.explosion.radius.powi(2)
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
                                combat::may_harm(
                                    &alignments,
                                    &players,
                                    &id_maps,
                                    owner_entity,
                                    entity_b,
                                ) || owner_entity.map_or(true, |entity_a| entity_a == entity_b)
                            };
                            if strength > 0.0 {
                                let is_alive = healths.get(entity_b).map_or(true, |h| !h.is_dead);

                                if is_alive {
                                    effect.modify_strength(strength);
                                    if !effect.is_harm() || may_harm() {
                                        emit_effect_events(
                                            &mut emitters,
                                            *time,
                                            entity_b,
                                            effect.clone(),
                                            ev.owner.map(|owner| {
                                                (
                                                    owner,
                                                    id_maps
                                                        .uid_entity(owner)
                                                        .and_then(|e| groups.get(e))
                                                        .copied(),
                                                )
                                            }),
                                            inventories.get(entity_b),
                                            &msm,
                                            character_states.get(entity_b),
                                            stats.get(entity_b),
                                        );
                                    }
                                }
                            }
                        }
                    },
                }
            }
        }
    }
}

pub fn emit_effect_events(
    emitters: &mut (impl EmitExt<HealthChangeEvent> + EmitExt<PoiseChangeEvent> + EmitExt<BuffEvent>),
    time: Time,
    entity: EcsEntity,
    effect: common::effect::Effect,
    source: Option<(Uid, Option<Group>)>,
    inventory: Option<&Inventory>,
    msm: &MaterialStatManifest,
    char_state: Option<&CharacterState>,
    stats: Option<&Stats>,
) {
    let damage_contributor = source.map(|(uid, group)| DamageContributor::new(uid, group));
    match effect {
        common::effect::Effect::Health(change) => {
            emitters.emit(HealthChangeEvent { entity, change })
        },
        common::effect::Effect::Poise(amount) => {
            let amount = Poise::apply_poise_reduction(amount, inventory, msm, char_state, stats);
            emitters.emit(PoiseChangeEvent {
                entity,
                change: comp::PoiseChange {
                    amount,
                    impulse: Vec3::zero(),
                    by: damage_contributor,
                    cause: None,
                    time,
                },
            })
        },
        common::effect::Effect::Damage(damage) => {
            let change = damage.calculate_health_change(
                combat::Damage::compute_damage_reduction(Some(damage), inventory, stats, msm),
                damage_contributor,
                None,
                0.0,
                1.0,
                time,
                rand::random(),
            );
            emitters.emit(HealthChangeEvent { entity, change })
        },
        common::effect::Effect::Buff(buff) => emitters.emit(BuffEvent {
            entity,
            buff_change: comp::BuffChange::Add(comp::Buff::new(
                buff.kind,
                buff.data,
                buff.cat_ids,
                comp::BuffSource::Item,
                time,
                stats,
            )),
        }),
    }
}

impl ServerEvent for BonkEvent {
    type SystemData<'a> = (
        Write<'a, BlockChange>,
        ReadExpect<'a, TerrainGrid>,
        Read<'a, EventBus<CreateObjectEvent>>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (mut block_change, terrain, create_object_events): Self::SystemData<'_>,
    ) {
        let mut create_object_emitter = create_object_events.emitter();
        for ev in events {
            if let Some(_target) = ev.target {
                // TODO: bonk entities but do no damage?
            } else {
                use common::terrain::SpriteKind;
                let pos = ev.pos.map(|e| e.floor() as i32);
                if let Some(block) = terrain.get(pos).ok().copied().filter(|b| b.is_bonkable()) {
                    if block_change
                        .try_set(pos, block.with_sprite(SpriteKind::Empty))
                        .is_some()
                    {
                        if let Some(items) = comp::Item::try_reclaim_from_block(block) {
                            let msm = &MaterialStatManifest::load().read();
                            let ability_map = &AbilityMap::load().read();
                            for item in flatten_counted_items(&items, ability_map, msm) {
                                create_object_emitter.emit(CreateObjectEvent {
                                    pos: Pos(pos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0)),
                                    vel: comp::Vel::default(),
                                    body: match block.get_sprite() {
                                        // Create different containers depending on the original
                                        // sprite
                                        Some(SpriteKind::Apple) => comp::object::Body::Apple,
                                        Some(SpriteKind::Beehive) => comp::object::Body::Hive,
                                        Some(SpriteKind::Coconut) => comp::object::Body::Coconut,
                                        Some(SpriteKind::Bomb) => comp::object::Body::Bomb,
                                        Some(SpriteKind::Mine) => comp::object::Body::Mine,
                                        _ => comp::object::Body::Pouch,
                                    },
                                    object: match block.get_sprite() {
                                        Some(SpriteKind::Bomb) | Some(SpriteKind::Mine) => {
                                            Some(comp::Object::Bomb { owner: ev.owner })
                                        },
                                        _ => None,
                                    },
                                    item: Some(item),
                                    light_emitter: None,
                                    stats: None,
                                });
                            }
                        } else if let Some(SpriteKind::Mine) = block.get_sprite() {
                            // Some objects can't be reclaimed as items but still have bonk effects
                            create_object_emitter.emit(CreateObjectEvent {
                                pos: Pos(pos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0)),
                                vel: comp::Vel::default(),
                                body: comp::object::Body::Bomb,
                                object: Some(comp::Object::Bomb { owner: ev.owner }),
                                item: None,
                                light_emitter: None,
                                stats: None,
                            });
                        }
                    }
                }
            }
        }
    }
}

impl ServerEvent for AuraEvent {
    type SystemData<'a> = WriteStorage<'a, Auras>;

    fn handle(events: impl ExactSizeIterator<Item = Self>, mut auras: Self::SystemData<'_>) {
        for ev in events {
            if let Some(mut auras) = auras.get_mut(ev.entity) {
                use aura::AuraChange;
                match ev.aura_change {
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
    }
}

impl ServerEvent for BuffEvent {
    type SystemData<'a> = (
        Read<'a, Time>,
        WriteStorage<'a, comp::Buffs>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, Health>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (time, mut buffs, bodies, healths): Self::SystemData<'_>,
    ) {
        for ev in events {
            if let Some(mut buffs) = buffs.get_mut(ev.entity) {
                use buff::BuffChange;
                match ev.buff_change {
                    BuffChange::Add(new_buff) => {
                        if !bodies
                            .get(ev.entity)
                            .map_or(false, |body| body.immune_to(new_buff.kind))
                            && healths.get(ev.entity).map_or(true, |h| !h.is_dead)
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
                                // Resets buff so that its remaining duration is equal to its
                                // original duration
                                buff.start_time = *time;
                                buff.end_time = buff.data.duration.map(|dur| Time(time.0 + dur.0));
                            })
                    },
                }
            }
        }
    }
}

impl ServerEvent for EnergyChangeEvent {
    type SystemData<'a> = WriteStorage<'a, Energy>;

    fn handle(events: impl ExactSizeIterator<Item = Self>, mut energies: Self::SystemData<'_>) {
        for ev in events {
            if let Some(mut energy) = energies.get_mut(ev.entity) {
                energy.change_by(ev.change);
            }
        }
    }
}

impl ServerEvent for ComboChangeEvent {
    type SystemData<'a> = (
        Read<'a, Time>,
        Read<'a, EventBus<Outcome>>,
        WriteStorage<'a, comp::Combo>,
        ReadStorage<'a, Uid>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (time, outcomes, mut combos, uids): Self::SystemData<'_>,
    ) {
        let mut outcome_emitter = outcomes.emitter();
        for ev in events {
            if let Some(mut combo) = combos.get_mut(ev.entity) {
                combo.change_by(ev.change, time.0);
                if let Some(uid) = uids.get(ev.entity) {
                    outcome_emitter.emit(Outcome::ComboChange {
                        uid: *uid,
                        combo: combo.counter(),
                    });
                }
            }
        }
    }
}

impl ServerEvent for ParryHookEvent {
    type SystemData<'a> = (
        Read<'a, Time>,
        Read<'a, EventBus<EnergyChangeEvent>>,
        Read<'a, EventBus<BuffEvent>>,
        WriteStorage<'a, CharacterState>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Stats>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (time, energy_change_events, buff_events, mut character_states, uids, stats): Self::SystemData<'_>,
    ) {
        let mut energy_change_emitter = energy_change_events.emitter();
        let mut buff_emitter = buff_events.emitter();
        for ev in events {
            if let Some(mut char_state) = character_states.get_mut(ev.defender) {
                let return_to_wield = match &mut *char_state {
                    CharacterState::RiposteMelee(c) => {
                        c.stage_section = StageSection::Action;
                        c.timer = Duration::default();
                        false
                    },
                    CharacterState::BasicBlock(c) => {
                        // Refund half the energy of entering the block for a successful parry
                        energy_change_emitter.emit(EnergyChangeEvent {
                            entity: ev.defender,
                            change: c.static_data.energy_regen,
                        });
                        true
                    },
                    _ => false,
                };
                if return_to_wield {
                    *char_state = CharacterState::Wielding(common::states::wielding::Data {
                        is_sneaking: false,
                    });
                }
            };

            if let Some(attacker) = ev.attacker
                && matches!(ev.source, AttackSource::Melee)
            {
                // When attacker is parried, add the parried debuff for 2 seconds, which slows
                // them
                let data = buff::BuffData::new(1.0, Some(Secs(2.0)));
                let source = if let Some(uid) = uids.get(ev.defender) {
                    BuffSource::Character { by: *uid }
                } else {
                    BuffSource::World
                };
                let buff = buff::Buff::new(
                    BuffKind::Parried,
                    data,
                    vec![buff::BuffCategory::Physical],
                    source,
                    *time,
                    stats.get(attacker),
                );
                buff_emitter.emit(BuffEvent {
                    entity: attacker,
                    buff_change: buff::BuffChange::Add(buff),
                });
            }
        }
    }
}

impl ServerEvent for TeleportToEvent {
    type SystemData<'a> = (
        Read<'a, IdMaps>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, comp::ForceUpdate>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (id_maps, mut positions, mut force_updates): Self::SystemData<'_>,
    ) {
        for ev in events {
            let target_pos = id_maps
                .uid_entity(ev.target)
                .and_then(|e| positions.get(e))
                .copied();

            if let (Some(pos), Some(target_pos)) = (positions.get_mut(ev.entity), target_pos) {
                if ev
                    .max_range
                    .map_or(true, |r| pos.0.distance_squared(target_pos.0) < r.powi(2))
                {
                    *pos = target_pos;
                    force_updates
                        .get_mut(ev.entity)
                        .map(|force_update| force_update.update());
                }
            }
        }
    }
}

impl ServerEvent for EntityAttackedHookEvent {
    type SystemData<'a> = (
        Entities<'a>,
        Write<'a, Trades>,
        Read<'a, IdMaps>,
        Read<'a, Time>,
        Read<'a, EventBus<BuffEvent>>,
        Read<'a, EventBus<ComboChangeEvent>>,
        Read<'a, EventBus<KnockbackEvent>>,
        Read<'a, EventBus<Outcome>>,
        WriteStorage<'a, CharacterState>,
        WriteStorage<'a, Poise>,
        WriteStorage<'a, Agent>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Client>,
        ReadStorage<'a, Stats>,
    );

    /// Intended to handle things that should happen for any successful attack,
    /// regardless of the damages and effects specific to that attack
    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (
            entities,
            mut trades,
            id_maps,
            time,
            buff_events,
            combo_change_events,
            knockback_events,
            outcomes,
            mut character_states,
            mut poises,
            mut agents,
            positions,
            uids,
            clients,
            stats,
        ): Self::SystemData<'_>,
    ) {
        let mut buff_emitter = buff_events.emitter();
        let mut combo_change_emitter = combo_change_events.emitter();
        let mut knockback_emitter = knockback_events.emitter();

        let mut outcomes = outcomes.emitter();
        for ev in events {
            if let Some(attacker) = ev.attacker {
                buff_emitter.emit(BuffEvent {
                    entity: attacker,
                    buff_change: buff::BuffChange::RemoveByCategory {
                        all_required: vec![buff::BuffCategory::RemoveOnAttack],
                        any_required: vec![],
                        none_required: vec![],
                    },
                });
            }

            if let Some((mut char_state, mut poise, pos)) =
                (&mut character_states, &mut poises, &positions)
                    .lend_join()
                    .get(ev.entity, &entities)
            {
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
                        outcomes.emit(Outcome::PoiseChange {
                            pos: pos.0,
                            state: poise_state,
                        });
                        if let Some(impulse_strength) = impulse_strength {
                            knockback_emitter.emit(KnockbackEvent {
                                entity: ev.entity,
                                impulse: impulse_strength * *poise.knockback(),
                            });
                        }
                    }
                }
            }

            // Remove potion/saturation buff if attacked
            buff_emitter.emit(BuffEvent {
                entity: ev.entity,
                buff_change: buff::BuffChange::RemoveByKind(BuffKind::Potion),
            });
            buff_emitter.emit(BuffEvent {
                entity: ev.entity,
                buff_change: buff::BuffChange::RemoveByKind(BuffKind::Saturation),
            });

            // If entity was in an active trade, cancel it
            if let Some(uid) = uids.get(ev.entity) {
                if let Some(trade) = trades.entity_trades.get(uid).copied() {
                    trades
                        .decline_trade(trade, *uid)
                        .and_then(|uid| id_maps.uid_entity(uid))
                        .map(|entity_b| {
                            // Notify both parties that the trade ended
                            let mut notify_trade_party = |entity| {
                                // TODO: Can probably improve UX here for the user that sent the
                                // trade invite, since right now it
                                // may seems like their request was
                                // purposefully declined, rather than e.g. being interrupted.
                                if let Some(client) = clients.get(entity) {
                                    client.send_fallible(ServerGeneral::FinishedTrade(
                                        TradeResult::Declined,
                                    ));
                                }
                                if let Some(agent) = agents.get_mut(entity) {
                                    agent.inbox.push_back(AgentEvent::FinishedTrade(
                                        TradeResult::Declined,
                                    ));
                                }
                            };
                            notify_trade_party(ev.entity);
                            notify_trade_party(entity_b);
                        });
                }
            }

            if let Some(stats) = stats.get(ev.entity) {
                for effect in &stats.effects_on_damaged {
                    use combat::DamagedEffect;
                    match effect {
                        DamagedEffect::Combo(c) => {
                            combo_change_emitter.emit(ComboChangeEvent {
                                entity: ev.entity,
                                change: *c,
                            });
                        },
                    }
                }
            }
        }
    }
}

impl ServerEvent for ChangeAbilityEvent {
    type SystemData<'a> = (
        WriteStorage<'a, comp::ActiveAbilities>,
        ReadStorage<'a, Inventory>,
        ReadStorage<'a, SkillSet>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (mut active_abilities, inventories, skill_sets): Self::SystemData<'_>,
    ) {
        for ev in events {
            if let Some(mut active_abilities) = active_abilities.get_mut(ev.entity) {
                active_abilities.change_ability(
                    ev.slot,
                    ev.auxiliary_key,
                    ev.new_ability,
                    inventories.get(ev.entity),
                    skill_sets.get(ev.entity),
                );
            }
        }
    }
}

impl ServerEvent for UpdateMapMarkerEvent {
    type SystemData<'a> = (
        Entities<'a>,
        WriteStorage<'a, comp::MapMarker>,
        ReadStorage<'a, Group>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Client>,
        ReadStorage<'a, Alignment>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (entities, mut map_markers, groups, uids, clients, alignments): Self::SystemData<'_>,
    ) {
        for ev in events {
            match ev.update {
                comp::MapMarkerChange::Update(waypoint) => {
                    let _ = map_markers.insert(ev.entity, comp::MapMarker(waypoint));
                },
                comp::MapMarkerChange::Remove => {
                    map_markers.remove(ev.entity);
                },
            }
            // Send updated waypoint to group members
            if let Some((group_id, uid)) = (&groups, &uids).lend_join().get(ev.entity, &entities) {
                for client in
                    comp::group::members(*group_id, &groups, &entities, &alignments, &uids)
                        .filter_map(|(e, _)| if e != ev.entity { clients.get(e) } else { None })
                {
                    client.send_fallible(ServerGeneral::MapMarker(
                        comp::MapMarkerUpdate::GroupMember(*uid, ev.update),
                    ));
                }
            }
        }
    }
}

impl ServerEvent for MakeAdminEvent {
    type SystemData<'a> = (WriteStorage<'a, comp::Admin>, ReadStorage<'a, Player>);

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (mut admins, players): Self::SystemData<'_>,
    ) {
        for ev in events {
            if players
                .get(ev.entity)
                .map_or(false, |player| player.uuid() == ev.uuid)
            {
                let _ = admins.insert(ev.entity, ev.admin);
            }
        }
    }
}

impl ServerEvent for ChangeStanceEvent {
    type SystemData<'a> = WriteStorage<'a, comp::Stance>;

    fn handle(events: impl ExactSizeIterator<Item = Self>, mut stances: Self::SystemData<'_>) {
        for ev in events {
            if let Some(mut stance) = stances.get_mut(ev.entity) {
                *stance = ev.stance;
            }
        }
    }
}

impl ServerEvent for ChangeBodyEvent {
    type SystemData<'a> = WriteStorage<'a, comp::Body>;

    fn handle(events: impl ExactSizeIterator<Item = Self>, mut bodies: Self::SystemData<'_>) {
        for ev in events {
            if let Some(mut body) = bodies.get_mut(ev.entity) {
                *body = ev.new_body;
            }
        }
    }
}

impl ServerEvent for RemoveLightEmitterEvent {
    type SystemData<'a> = WriteStorage<'a, comp::LightEmitter>;

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        mut light_emitters: Self::SystemData<'_>,
    ) {
        for ev in events {
            light_emitters.remove(ev.entity);
        }
    }
}

impl ServerEvent for TeleportToPositionEvent {
    type SystemData<'a> = (
        Read<'a, IdMaps>,
        WriteStorage<'a, Is<VolumeRider>>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, comp::ForceUpdate>,
        ReadStorage<'a, Is<Rider>>,
        ReadStorage<'a, Presence>,
        ReadStorage<'a, Client>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (
            id_maps,
            mut is_volume_riders,
            mut positions,
            mut force_updates,
            is_riders,
            presences,
            clients,
        ): Self::SystemData<'_>,
    ) {
        for ev in events {
            if let Err(error) = crate::state_ext::position_mut(
                ev.entity,
                true,
                |pos| pos.0 = ev.position,
                &id_maps,
                &mut is_volume_riders,
                &mut positions,
                &mut force_updates,
                &is_riders,
                &presences,
                &clients,
            ) {
                warn!(?error, "Failed to teleport entity");
            }
        }
    }
}

impl ServerEvent for StartTeleportingEvent {
    type SystemData<'a> = (
        Read<'a, Time>,
        WriteStorage<'a, comp::Teleporting>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, comp::Object>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (time, mut teleportings, positions, objects): Self::SystemData<'_>,
    ) {
        for ev in events {
            if let Some(end_time) = (!teleportings.contains(ev.entity))
                .then(|| positions.get(ev.entity))
                .flatten()
                .zip(positions.get(ev.portal))
                .filter(|(entity_pos, portal_pos)| {
                    entity_pos.0.distance_squared(portal_pos.0) <= TELEPORTER_RADIUS.powi(2)
                })
                .and_then(|(_, _)| {
                    Some(
                        time.0
                            + objects.get(ev.portal).and_then(|object| {
                                if let Object::Portal { buildup_time, .. } = object {
                                    Some(buildup_time.0)
                                } else {
                                    None
                                }
                            })?,
                    )
                })
            {
                let _ = teleportings.insert(ev.entity, comp::Teleporting {
                    portal: ev.portal,
                    end_time: Time(end_time),
                });
            }
        }
    }
}
