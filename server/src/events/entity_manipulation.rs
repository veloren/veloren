use crate::{
    client::Client,
    comp::{
        agent::{Agent, AgentEvent, Sound, SoundKind},
        biped_large, bird_large, quadruped_low, quadruped_medium, quadruped_small,
        skills::SkillGroupKind,
        theropod, BuffKind, BuffSource, PhysicsState,
    },
    rtsim::RtSim,
    sys::terrain::SAFE_ZONE_RADIUS,
    Server, SpawnPoint, StateExt,
};
use common::{
    assets::AssetExt,
    combat,
    comp::{
        self, aura, buff,
        chat::{KillSource, KillType},
        inventory::item::MaterialStatManifest,
        object, Alignment, Auras, Body, CharacterState, Energy, EnergyChange, EnergySource, Group,
        Health, HealthChange, Inventory, Player, Poise, PoiseChange, PoiseSource, Pos, SkillSet,
        Stats,
    },
    event::{EventBus, ServerEvent},
    lottery::{LootSpec, Lottery},
    outcome::Outcome,
    resources::Time,
    rtsim::RtSimEntity,
    terrain::{Block, BlockKind, TerrainGrid},
    uid::{Uid, UidAllocator},
    util::Dir,
    vol::ReadVol,
    Damage, DamageKind, DamageSource, Explosion, GroupTarget, RadiusEffect,
};
use common_net::{msg::ServerGeneral, sync::WorldSyncExt};
use common_state::BlockChange;
use comp::chat::GenericChatMsg;
use hashbrown::HashSet;
use rand::Rng;
use specs::{join::Join, saveload::MarkerAllocator, Builder, Entity as EcsEntity, WorldExt};
use tracing::error;
use vek::{Vec2, Vec3};

pub fn handle_poise(
    server: &Server,
    entity: EcsEntity,
    change: PoiseChange,
    knockback_dir: Vec3<f32>,
) {
    let ecs = &server.state.ecs();
    if let Some(character_state) = ecs.read_storage::<CharacterState>().get(entity) {
        // Entity is invincible to poise change during stunned/staggered character state
        if !character_state.is_stunned() {
            if let Some(mut poise) = ecs.write_storage::<Poise>().get_mut(entity) {
                poise.change_by(change, knockback_dir);
            }
        }
    }
}

pub fn handle_health_change(server: &Server, entity: EcsEntity, change: HealthChange) {
    let ecs = &server.state.ecs();
    if let Some(mut health) = ecs.write_storage::<Health>().get_mut(entity) {
        health.change_by(change);
    }
    // This if statement filters out anything under 5 damage, for DOT ticks
    // TODO: Find a better way to separate direct damage from DOT here
    let damage = -change.amount;
    if damage > -5.0 {
        if let Some(agent) = ecs.write_storage::<Agent>().get_mut(entity) {
            agent.inbox.push_front(AgentEvent::Hurt);
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
// NOTE: Clippy incorrectly warns about a needless collect here because it does not
// understand that the pet count (which is computed during the first iteration over the
// members in range) is actually used by the second iteration over the members in range;
// since we have no way of knowing the pet count before the first loop finishes, we
// definitely need at least two loops.   Then (currently) our only options are to store
// the member list in temporary space (e.g. by collecting to a vector), or to repeat
// the loop; but repeating the loop would currently be very inefficient since it has to
// rescan every entity on the server again.
#[allow(clippy::needless_collect)]
pub fn handle_destroy(server: &mut Server, entity: EcsEntity, last_change: HealthChange) {
    let state = server.state_mut();

    // TODO: Investigate duplicate `Destroy` events (but don't remove this).
    // If the entity was already deleted, it can't be destroyed again.
    if !state.ecs().is_alive(entity) {
        return;
    }

    let get_attacker_name = |cause_of_death: KillType, by: Uid| -> KillSource {
        // Get attacker entity
        if let Some(char_entity) = state.ecs().entity_from_uid(by.into()) {
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
        .read_storage::<comp::CharacterState>()
        .get(entity)
        .is_some()
    {
        if let Some(pos) = state.ecs().read_storage::<Pos>().get(entity) {
            state
                .ecs()
                .write_resource::<Vec<Outcome>>()
                .push(Outcome::Death { pos: pos.0 });
        }
    }

    // Chat message
    // If it was a player that died
    if let Some(_player) = state.ecs().read_storage::<Player>().get(entity) {
        if let Some(uid) = state.ecs().read_storage::<Uid>().get(entity) {
            let kill_source = match (last_change.cause, last_change.by) {
                (Some(DamageSource::Melee), Some(by)) => get_attacker_name(KillType::Melee, by),
                (Some(DamageSource::Projectile), Some(by)) => {
                    get_attacker_name(KillType::Projectile, by)
                },
                (Some(DamageSource::Explosion), Some(by)) => {
                    get_attacker_name(KillType::Explosion, by)
                },
                (Some(DamageSource::Energy), Some(by)) => get_attacker_name(KillType::Energy, by),
                (Some(DamageSource::Buff(buff_kind)), Some(by)) => {
                    get_attacker_name(KillType::Buff(buff_kind), by)
                },
                (Some(DamageSource::Other), Some(by)) => get_attacker_name(KillType::Other, by),
                (Some(DamageSource::Falling), _) => KillSource::FallDamage,
                // HealthSource::Suicide => KillSource::Suicide,
                _ => KillSource::Other,
            };

            state.send_chat(GenericChatMsg {
                chat_type: comp::ChatType::Kill(kill_source, *uid),
                message: "".to_string(),
            });
        }
    }

    // Give EXP to the killer if entity had stats
    (|| {
        let mut skill_set = state.ecs().write_storage::<SkillSet>();
        let healths = state.ecs().read_storage::<Health>();
        let energies = state.ecs().read_storage::<Energy>();
        let inventories = state.ecs().read_storage::<Inventory>();
        let players = state.ecs().read_storage::<Player>();
        let bodies = state.ecs().read_storage::<Body>();
        let by = if let Some(by) = last_change.by {
            by
        } else {
            return;
        };
        let attacker = if let Some(attacker) = state.ecs().entity_from_uid(by.into()) {
            attacker
        } else {
            return;
        };
        let (entity_skill_set, entity_health, entity_energy, entity_inventory, entity_body) =
            if let (
                Some(entity_skill_set),
                Some(entity_health),
                Some(entity_energy),
                Some(entity_inventory),
                Some(entity_body),
            ) = (
                skill_set.get(entity),
                healths.get(entity),
                energies.get(entity),
                inventories.get(entity),
                bodies.get(entity),
            ) {
                (
                    entity_skill_set,
                    entity_health,
                    entity_energy,
                    entity_inventory,
                    entity_body,
                )
            } else {
                return;
            };

        let groups = state.ecs().read_storage::<Group>();
        let attacker_group = groups.get(attacker);
        let destroyed_group = groups.get(entity);
        // Don't give exp if attacker destroyed themselves or one of their group
        // members, or a pvp kill
        if (attacker_group.is_some() && attacker_group == destroyed_group)
            || attacker == entity
            || (players.get(entity).is_some() && players.get(attacker).is_some())
        {
            return;
        }
        // Maximum distance for other group members to receive exp
        const MAX_EXP_DIST: f32 = 150.0;
        // TODO: Scale xp from skillset rather than health, when NPCs have their own
        // skillsets
        let msm = state.ecs().read_resource::<MaterialStatManifest>();
        let mut exp_reward = combat::combat_rating(
            entity_inventory,
            entity_health,
            entity_energy,
            entity_skill_set,
            *entity_body,
            &msm,
        ) * 2.5;

        // Distribute EXP to group
        let positions = state.ecs().read_storage::<Pos>();
        let alignments = state.ecs().read_storage::<Alignment>();
        let uids = state.ecs().read_storage::<Uid>();
        let mut outcomes = state.ecs().write_resource::<Vec<Outcome>>();
        let inventories = state.ecs().read_storage::<comp::Inventory>();
        if let (Some(attacker_group), Some(pos)) = (attacker_group, positions.get(entity)) {
            // TODO: rework if change to groups makes it easier to iterate entities in a
            // group
            let mut non_pet_group_members_in_range = 1;
            let members_in_range = (
                &state.ecs().entities(),
                &groups,
                &positions,
                alignments.maybe(),
                &uids,
            )
                .join()
                .filter(|(entity, group, member_pos, _, _)| {
                    // Check if: in group, not main attacker, and in range
                    *group == attacker_group
                        && *entity != attacker
                        && pos.0.distance_squared(member_pos.0) < MAX_EXP_DIST.powi(2)
                })
                .map(|(entity, _, _, alignment, uid)| {
                    if !matches!(alignment, Some(Alignment::Owned(owner)) if owner != uid) {
                        non_pet_group_members_in_range += 1;
                    }

                    (entity, uid)
                })
                .collect::<Vec<_>>();
            // Divides exp reward by square root of number of people in group
            exp_reward /= (non_pet_group_members_in_range as f32).sqrt();
            members_in_range.into_iter().for_each(|(e, uid)| {
                if let (Some(inventory), Some(mut skill_set)) =
                    (inventories.get(e), skill_set.get_mut(e))
                {
                    handle_exp_gain(exp_reward, inventory, &mut skill_set, uid, &mut outcomes);
                }
            });
        }

        if let (Some(mut attacker_skill_set), Some(attacker_uid), Some(attacker_inventory)) = (
            skill_set.get_mut(attacker),
            uids.get(attacker),
            inventories.get(attacker),
        ) {
            // TODO: Discuss whether we should give EXP by Player Killing or not.
            handle_exp_gain(
                exp_reward,
                attacker_inventory,
                &mut attacker_skill_set,
                attacker_uid,
                &mut outcomes,
            );
        }
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
            .write_storage()
            .insert(entity, comp::ForceUpdate)
            .err()
            .map(|e| error!(?e, ?entity, "Failed to insert ForceUpdate on dead client"));
        state
            .ecs()
            .write_storage::<comp::LightEmitter>()
            .remove(entity);
        state
            .ecs()
            .write_storage::<comp::Energy>()
            .get_mut(entity)
            .map(|mut energy| {
                let energy = &mut *energy;
                energy.set_to(energy.maximum(), comp::EnergySource::Revive)
            });
        let _ = state
            .ecs()
            .write_storage::<comp::CharacterState>()
            .insert(entity, comp::CharacterState::default());

        false
    } else if state.ecs().read_storage::<comp::Agent>().contains(entity)
        && !matches!(
            state.ecs().read_storage::<comp::Alignment>().get(entity),
            Some(comp::Alignment::Owned(_))
        )
    {
        // Only drop loot if entity has agency (not a player), and if it is not owned by
        // another entity (not a pet)

        // Decide for a loot drop before turning into a lootbag
        let old_body = state.ecs().write_storage::<Body>().remove(entity);
        let lottery = || {
            Lottery::<LootSpec<String>>::load_expect(match old_body {
                Some(common::comp::Body::Humanoid(_)) => "common.loot_tables.creature.humanoid",
                Some(common::comp::Body::QuadrupedSmall(quadruped_small)) => {
                    match quadruped_small.species {
                        quadruped_small::Species::Dodarock => {
                            "common.loot_tables.creature.quad_small.dodarock"
                        },
                        quadruped_small::Species::Truffler | quadruped_small::Species::Fungome => {
                            "common.loot_tables.creature.quad_small.mushroom"
                        },
                        quadruped_small::Species::Sheep | quadruped_small::Species::Goat => {
                            "common.loot_tables.creature.quad_small.wool"
                        },
                        quadruped_small::Species::Skunk
                        | quadruped_small::Species::Quokka
                        | quadruped_small::Species::Beaver
                        | quadruped_small::Species::Jackalope
                        | quadruped_small::Species::Hare => {
                            "common.loot_tables.creature.quad_small.fur"
                        },
                        quadruped_small::Species::Frog
                        | quadruped_small::Species::Axolotl
                        | quadruped_small::Species::Gecko
                        | quadruped_small::Species::Turtle => {
                            "common.loot_tables.creature.quad_small.ooze"
                        },
                        _ => "common.loot_tables.creature.quad_small.generic",
                    }
                },
                Some(Body::QuadrupedMedium(quadruped_medium)) => match quadruped_medium.species {
                    quadruped_medium::Species::Frostfang | quadruped_medium::Species::Roshwalr => {
                        "common.loot_tables.creature.quad_medium.ice"
                    },
                    quadruped_medium::Species::Catoblepas => {
                        "common.loot_tables.creature.quad_medium.catoblepas"
                    },
                    quadruped_medium::Species::Bear
                    | quadruped_medium::Species::Snowleopard
                    | quadruped_medium::Species::Tiger
                    | quadruped_medium::Species::Lion => {
                        "common.loot_tables.creature.quad_medium.clawed"
                    },
                    quadruped_medium::Species::Tarasque
                    | quadruped_medium::Species::Bonerattler => {
                        "common.loot_tables.creature.quad_medium.carapace"
                    },
                    quadruped_medium::Species::Dreadhorn => {
                        "common.loot_tables.creature.quad_medium.dreadhorn"
                    },
                    quadruped_medium::Species::Camel
                    | quadruped_medium::Species::Deer
                    | quadruped_medium::Species::Hirdrasil
                    | quadruped_medium::Species::Horse
                    | quadruped_medium::Species::Highland
                    | quadruped_medium::Species::Zebra
                    | quadruped_medium::Species::Donkey
                    | quadruped_medium::Species::Antelope
                    | quadruped_medium::Species::Kelpie
                    | quadruped_medium::Species::Cattle
                    | quadruped_medium::Species::Yak => {
                        "common.loot_tables.creature.quad_medium.gentle"
                    },
                    quadruped_medium::Species::Mouflon
                    | quadruped_medium::Species::Llama
                    | quadruped_medium::Species::Alpaca => {
                        "common.loot_tables.creature.quad_medium.wool"
                    },
                    quadruped_medium::Species::Ngoubou => {
                        "common.loot_tables.creature.quad_medium.horned"
                    },
                    quadruped_medium::Species::Mammoth => {
                        "common.loot_tables.creature.quad_medium.mammoth"
                    },
                    _ => "common.loot_tables.creature.quad_medium.fanged",
                },
                Some(common::comp::Body::BirdMedium(_)) => {
                    "common.loot_tables.creature.bird_medium"
                },
                Some(common::comp::Body::BirdLarge(bird_large)) => match bird_large.species {
                    bird_large::Species::Cockatrice => {
                        "common.loot_tables.creature.bird_large.cockatrice"
                    },
                    bird_large::Species::Roc => "common.loot_tables.creature.bird_large.roc",
                    _ => "common.loot_tables.creature.bird_large.phoenix",
                },
                Some(common::comp::Body::FishMedium(_)) => "common.loot_tables.creature.fish",
                Some(common::comp::Body::FishSmall(_)) => "common.loot_tables.creature.fish",
                Some(common::comp::Body::BipedLarge(biped_large)) => match biped_large.species {
                    biped_large::Species::Wendigo => {
                        "common.loot_tables.creature.biped_large.wendigo"
                    },
                    biped_large::Species::Cavetroll
                    | biped_large::Species::Mountaintroll
                    | biped_large::Species::Swamptroll => {
                        "common.loot_tables.creature.biped_large.troll"
                    },
                    biped_large::Species::Occultsaurok
                    | biped_large::Species::Mightysaurok
                    | biped_large::Species::Slysaurok => {
                        "common.loot_tables.creature.biped_large.saurok"
                    },
                    _ => "common.loot_tables.creature.biped_large.default",
                },
                Some(common::comp::Body::Golem(_)) => "common.loot_tables.creature.golem",
                Some(common::comp::Body::Theropod(theropod)) => match theropod.species {
                    theropod::Species::Sandraptor
                    | theropod::Species::Snowraptor
                    | theropod::Species::Woodraptor
                    | theropod::Species::Sunlizard => "common.loot_tables.creature.theropod.raptor",
                    theropod::Species::Archaeos
                    | theropod::Species::Ntouka
                    | theropod::Species::Yale => "common.loot_tables.creature.theropod.horned",
                    _ => "common.loot_tables.creature.theropod.generic",
                },
                Some(common::comp::Body::Dragon(_)) => "common.loot_tables.creature.dragon",
                Some(common::comp::Body::QuadrupedLow(quadruped_low)) => {
                    match quadruped_low.species {
                        quadruped_low::Species::Maneater => {
                            "common.loot_tables.creature.quad_low.maneater"
                        },
                        quadruped_low::Species::Lavadrake
                        | quadruped_low::Species::Rocksnapper
                        | quadruped_low::Species::Sandshark => {
                            "common.loot_tables.creature.quad_low.carapace"
                        },
                        quadruped_low::Species::Asp => {
                            "common.loot_tables.creature.quad_low.venemous"
                        },
                        quadruped_low::Species::Hakulaq => {
                            "common.loot_tables.creature.quad_low.fanged"
                        },
                        quadruped_low::Species::Deadwood => {
                            "common.loot_tables.creature.quad_low.deadwood"
                        },
                        quadruped_low::Species::Basilisk => {
                            "common.loot_tables.creature.quad_low.basilisk"
                        },
                        quadruped_low::Species::Salamander => {
                            "common.loot_tables.creature.quad_low.salamander"
                        },
                        _ => "common.loot_tables.creature.quad_low.generic",
                    }
                },
                Some(common::comp::Body::BipedSmall(_)) => "common.loot_tables.nothing",
                _ => "common.loot_tables.fallback",
            })
        };

        if let Some(item) = {
            let mut item_drops = state.ecs().write_storage::<comp::ItemDrop>();
            item_drops.remove(entity).map_or_else(
                || lottery().read().choose().to_item(),
                |item_drop| Some(item_drop.0),
            )
        } {
            let pos = state.ecs().read_storage::<comp::Pos>().get(entity).cloned();
            let vel = state.ecs().read_storage::<comp::Vel>().get(entity).cloned();
            if let Some(pos) = pos {
                let _ = state
                    .create_object(comp::Pos(pos.0 + Vec3::unit_z() * 0.25), match old_body {
                        Some(common::comp::Body::Humanoid(_)) => object::Body::Pouch,
                        Some(common::comp::Body::BipedSmall(_)) => object::Body::Pouch,
                        Some(common::comp::Body::Golem(_)) => object::Body::Chest,
                        Some(common::comp::Body::QuadrupedSmall(_)) => object::Body::SmallMeat,
                        Some(common::comp::Body::FishMedium(_))
                        | Some(common::comp::Body::FishSmall(_)) => object::Body::FishMeat,
                        Some(common::comp::Body::QuadrupedMedium(_)) => object::Body::BeastMeat,
                        Some(common::comp::Body::BipedLarge(_))
                        | Some(common::comp::Body::QuadrupedLow(_)) => object::Body::ToughMeat,
                        Some(common::comp::Body::BirdLarge(_))
                        | Some(common::comp::Body::BirdMedium(_)) => object::Body::BirdMeat,

                        _ => object::Body::BeastMeat,
                    })
                    .maybe_with(vel)
                    .with(item)
                    .build();
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

    if should_delete {
        if let Some(rtsim_entity) = state
            .ecs()
            .read_storage::<RtSimEntity>()
            .get(entity)
            .copied()
        {
            state
                .ecs()
                .write_resource::<RtSim>()
                .destroy_entity(rtsim_entity.0);
        }

        let _ = state
            .delete_entity_recorded(entity)
            .map_err(|e| error!(?e, ?entity, "Failed to delete destroyed entity"));
    }

    // TODO: Add Delete(time_left: Duration) component
    /*
    // If not a player delete the entity
    if let Err(err) = state.delete_entity_recorded(entity) {
        error!(?e, "Failed to delete destroyed entity");
    }
    */
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

pub fn handle_land_on_ground(server: &Server, entity: EcsEntity, vel: Vec3<f32>) {
    let ecs = server.state.ecs();

    if vel.z <= -30.0 {
        let mass = ecs
            .read_storage::<comp::Mass>()
            .get(entity)
            .copied()
            .unwrap_or_default();
        let impact_energy = mass.0 * vel.z.powi(2) / 2.0;
        let falldmg = impact_energy / 1000.0;

        let inventories = ecs.read_storage::<Inventory>();
        let stats = ecs.read_storage::<Stats>();

        // Handle health change
        if let Some(mut health) = ecs.write_storage::<comp::Health>().get_mut(entity) {
            let damage = Damage {
                source: DamageSource::Falling,
                kind: DamageKind::Crushing,
                value: falldmg,
            };
            let damage_reduction = Damage::compute_damage_reduction(
                inventories.get(entity),
                stats.get(entity),
                Some(DamageKind::Crushing),
            );
            // 45 - 75 是地牢楼梯摔落伤害区间
            // if !(45.0 >= falldmg || falldmg <= 95.0) {
            //     let change =
            //         damage.calculate_health_change(damage_reduction, None, false, 0.0, 1.0);
            //     health.change_by(change);
            // }
        }
        // Handle poise change
        if let Some(mut poise) = ecs.write_storage::<comp::Poise>().get_mut(entity) {
            let poise_damage = PoiseChange {
                amount: -(mass.0 * vel.magnitude_squared() / 1500.0) as i32,
                source: PoiseSource::Falling,
            };
            let poise_change = poise_damage.modify_poise_damage(inventories.get(entity));
            poise.change_by(poise_change, Vec3::unit_z());
        }
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
            .write_storage::<comp::Health>()
            .get_mut(entity)
            .map(|mut health| health.revive());
        state
            .ecs()
            .write_storage::<comp::Combo>()
            .get_mut(entity)
            .map(|mut combo| combo.reset());
        state
            .ecs()
            .write_storage::<comp::Pos>()
            .get_mut(entity)
            .map(|pos| pos.0 = respawn_point);
        state
            .ecs()
            .write_storage()
            .insert(entity, comp::ForceUpdate)
            .err()
            .map(|e| {
                error!(
                    ?e,
                    "Error inserting ForceUpdate component when respawning client"
                )
            });
    }
}

#[allow(clippy::blocks_in_if_conditions)]
pub fn handle_explosion(server: &Server, pos: Vec3<f32>, explosion: Explosion, owner: Option<Uid>) {
    // Go through all other entities
    let ecs = &server.state.ecs();
    let server_eventbus = ecs.read_resource::<EventBus<ServerEvent>>();
    let time = ecs.read_resource::<Time>();
    let owner_entity = owner.and_then(|uid| {
        ecs.read_resource::<UidAllocator>()
            .retrieve_entity_internal(uid.into())
    });

    let explosion_volume = 2.5 * explosion.radius;
    server_eventbus.emit_now(ServerEvent::Sound {
        sound: Sound::new(SoundKind::Explosion, pos, explosion_volume, time.0),
    });

    // Add an outcome
    // Uses radius as outcome power for now
    let outcome_power = explosion.radius;
    let mut outcomes = ecs.write_resource::<Vec<Outcome>>();
    outcomes.push(Outcome::Explosion {
        pos,
        power: outcome_power,
        radius: explosion.radius,
        is_attack: explosion
            .effects
            .iter()
            .any(|e| matches!(e, RadiusEffect::Attack(_))),
        reagent: explosion.reagent,
    });
    let groups = ecs.read_storage::<comp::Group>();

    // Used to get strength of explosion effects as they falloff over distance
    fn cylinder_sphere_strength(
        sphere_pos: Vec3<f32>,
        radius: f32,
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

        // Compare both checks, take whichever gives weaker effect, sets minimum of 0 so
        // that explosions reach a max strength on edge of entity
        ((horiz_dist.max(vert_distance).max(0.0) / radius).min(1.0) - 1.0).abs()
    }

    // TODO: Faster RNG?
    let mut rng = rand::thread_rng();
    'effects: for effect in explosion.effects {
        match effect {
            RadiusEffect::TerrainDestruction(power) => {
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
                        if !matches!(block.kind(), BlockKind::Lava | BlockKind::GlowingRock) {
                            let diff2 = block_pos.map(|b| b as f32).distance_squared(pos);
                            let fade = (1.0 - diff2 / color_range.powi(2)).max(0.0);
                            if let Some(mut color) = block.get_color() {
                                let r = color[0] as f32
                                    + (fade * (color[0] as f32 * 0.5 - color[0] as f32));
                                let g = color[1] as f32
                                    + (fade * (color[1] as f32 * 0.3 - color[1] as f32));
                                let b = color[2] as f32
                                    + (fade * (color[2] as f32 * 0.3 - color[2] as f32));
                                // Darken blocks, but not too much
                                color[0] = (r as u8).max(30);
                                color[1] = (g as u8).max(30);
                                color[2] = (b as u8).max(30);
                                block_change.set(block_pos, Block::new(block.kind(), color));
                            }
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
                        .until(|block: &Block| {
                            // Stop if:
                            // 1) Block is liquid
                            // 2) Consumed all energy
                            // 3) Can't explode block (for example we hit stone wall)
                            let stop = block.is_liquid()
                                || block.explode_power().is_none()
                                || ray_energy <= 0.0;

                            ray_energy -=
                                block.explode_power().unwrap_or(0.0) + rng.gen::<f32>() * 0.1;

                            stop
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
                let energies = &ecs.read_storage::<comp::Energy>();
                let combos = &ecs.read_storage::<comp::Combo>();
                let inventories = &ecs.read_storage::<comp::Inventory>();
                let alignments = &ecs.read_storage::<Alignment>();
                let uid_allocator = &ecs.read_resource::<UidAllocator>();
                let players = &ecs.read_storage::<comp::Player>();
                for (
                    entity_b,
                    pos_b,
                    health_b,
                    (body_b_maybe, stats_b_maybe, ori_b_maybe, char_state_b_maybe, uid_b),
                ) in (
                    &ecs.entities(),
                    &ecs.read_storage::<comp::Pos>(),
                    &ecs.read_storage::<comp::Health>(),
                    (
                        ecs.read_storage::<comp::Body>().maybe(),
                        ecs.read_storage::<comp::Stats>().maybe(),
                        ecs.read_storage::<comp::Ori>().maybe(),
                        ecs.read_storage::<comp::CharacterState>().maybe(),
                        &ecs.read_storage::<Uid>(),
                    ),
                )
                    .join()
                    .filter(|(_, _, h, _)| !h.is_dead)
                {
                    // Check if it is a hit
                    let strength = if let Some(body) = body_b_maybe {
                        cylinder_sphere_strength(pos, explosion.radius, pos_b.0, *body)
                    } else {
                        let distance_squared = pos.distance_squared(pos_b.0);
                        1.0 - distance_squared / explosion.radius.powi(2)
                    };
                    if strength > 0.0 {
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
                                    energy: energies.get(entity),
                                    combo: combos.get(entity),
                                    inventory: inventories.get(entity),
                                });

                        let target_info = combat::TargetInfo {
                            entity: entity_b,
                            uid: *uid_b,
                            inventory: inventories.get(entity_b),
                            stats: stats_b_maybe,
                            health: Some(health_b),
                            pos: pos_b.0,
                            ori: ori_b_maybe,
                            char_state: char_state_b_maybe,
                        };

                        // PvP check
                        let may_harm = combat::may_harm(
                            alignments,
                            players,
                            uid_allocator,
                            owner_entity,
                            entity_b,
                        );
                        let attack_options = combat::AttackOptions {
                            // cool guyz maybe don't look at explosions
                            // but they still got hurt, it's not Hollywood
                            target_dodging: false,
                            may_harm,
                            target_group,
                        };

                        attack.apply_attack(
                            attacker_info,
                            target_info,
                            dir,
                            attack_options,
                            strength,
                            combat::AttackSource::Explosion,
                            |e| server_eventbus.emit_now(e),
                            |o| outcomes.push(o),
                        );
                    }
                }
            },
            RadiusEffect::Entity(mut effect) => {
                let alignments = &ecs.read_storage::<Alignment>();
                let uid_allocator = &ecs.read_resource::<UidAllocator>();
                let players = &ecs.read_storage::<comp::Player>();
                for (entity_b, pos_b, body_b_maybe) in (
                    &ecs.entities(),
                    &ecs.read_storage::<comp::Pos>(),
                    ecs.read_storage::<comp::Body>().maybe(),
                )
                    .join()
                {
                    let strength = if let Some(body) = body_b_maybe {
                        cylinder_sphere_strength(pos, explosion.radius, pos_b.0, *body)
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
                        combat::may_harm(alignments, players, uid_allocator, owner_entity, entity_b)
                            || owner_entity.map_or(true, |entity_a| entity_a == entity_b)
                    };
                    if strength > 0.0 {
                        let is_alive = ecs
                            .read_storage::<comp::Health>()
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

pub fn handle_bonk(server: &mut Server, pos: Vec3<f32>, _owner: Option<Uid>, target: Option<Uid>) {
    let ecs = &server.state.ecs();
    let terrain = ecs.read_resource::<TerrainGrid>();
    let mut block_change = ecs.write_resource::<BlockChange>();

    if let Some(_target) = target {
        // TODO: bonk entities but do no damage?
    } else {
        use common::terrain::SpriteKind;
        let pos = pos.map(|e| e.floor() as i32);
        if let Some(block) = terrain.get(pos).ok().copied().filter(|b| b.is_bonkable()) {
            if let Some(item) = comp::Item::try_reclaim_from_block(block) {
                if block_change
                    .try_set(pos, block.with_sprite(SpriteKind::Empty))
                    .is_some()
                {
                    drop(terrain);
                    drop(block_change);
                    server
                        .state
                        .create_object(Default::default(), match block.get_sprite() {
                            // Create different containers depending on the original sprite
                            Some(SpriteKind::Apple) => comp::object::Body::Apple,
                            Some(SpriteKind::Beehive) => comp::object::Body::Hive,
                            Some(SpriteKind::Coconut) => comp::object::Body::Coconut,
                            _ => comp::object::Body::Pouch,
                        })
                        .with(comp::Pos(pos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0)))
                        .with(item)
                        .build();
                }
            };
        }
    }
}

pub fn handle_aura(server: &mut Server, entity: EcsEntity, aura_change: aura::AuraChange) {
    let ecs = &server.state.ecs();
    let mut auras_all = ecs.write_storage::<comp::Auras>();
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
    let bodies = ecs.read_storage::<comp::Body>();
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
                    buffs.insert(new_buff);
                }
            },
            BuffChange::RemoveById(ids) => {
                for id in ids {
                    buffs.remove(id);
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
                let mut ids_to_remove = Vec::new();
                for (id, buff) in buffs.buffs.iter() {
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
                        ids_to_remove.push(*id);
                    }
                }
                for id in ids_to_remove {
                    buffs.remove(id);
                }
            },
        }
    }
}

pub fn handle_energy_change(server: &Server, entity: EcsEntity, change: EnergyChange) {
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
    outcomes: &mut Vec<Outcome>,
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
            .and_then(|i| match &i.kind() {
                ItemKind::Tool(tool) if tool.kind.gains_combat_xp() => Some(tool.kind),
                _ => None,
            });
        if let Some(weapon) = tool_kind {
            // Only adds to xp pools if entity has that skill group available
            if skill_set.contains_skill_group(SkillGroupKind::Weapon(weapon)) {
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
        skill_set.change_experience(*pool, (exp_reward / num_pools).ceil() as i32);
    }
    outcomes.push(Outcome::ExpChange {
        uid: *uid,
        exp: exp_reward as i32,
        xp_pools,
    });
}

pub fn handle_combo_change(server: &Server, entity: EcsEntity, change: i32) {
    let ecs = &server.state.ecs();
    if let Some(mut combo) = ecs.write_storage::<comp::Combo>().get_mut(entity) {
        let time = ecs.read_resource::<Time>();
        let mut outcomes = ecs.write_resource::<Vec<Outcome>>();
        combo.change_by(change, time.0);
        if let Some(uid) = ecs.read_storage::<Uid>().get(entity) {
            outcomes.push(Outcome::ComboChange {
                uid: *uid,
                combo: combo.counter(),
            });
        }
    }
}

pub fn handle_parry(server: &Server, entity: EcsEntity, energy_cost: i32) {
    let ecs = &server.state.ecs();
    if let Some(mut character) = ecs.write_storage::<comp::CharacterState>().get_mut(entity) {
        *character = CharacterState::Wielding;
    };
    if let Some(mut energy) = ecs.write_storage::<Energy>().get_mut(entity) {
        energy.change_by(EnergyChange {
            amount: energy_cost,
            source: EnergySource::Ability,
        });
    }
}

pub fn handle_teleport_to(server: &Server, entity: EcsEntity, target: Uid, max_range: Option<f32>) {
    let ecs = &server.state.ecs();
    let mut positions = ecs.write_storage::<Pos>();

    let target_pos = ecs
        .entity_from_uid(target.into())
        .and_then(|e| positions.get(e))
        .copied();

    if let (Some(pos), Some(target_pos)) = (positions.get_mut(entity), target_pos) {
        if max_range.map_or(true, |r| pos.0.distance_squared(target_pos.0) < r.powi(2)) {
            *pos = target_pos;
            ecs.write_storage()
                .insert(entity, comp::ForceUpdate)
                .err()
                .map(|e| {
                    error!(
                        ?e,
                        "Error inserting ForceUpdate component when teleporting client"
                    )
                });
        }
    }
}

/// Intended to handle things that should happen for any successful attack,
/// regardless of the damages and effects specific to that attack
pub fn handle_entity_attacked_hook(server: &Server, entity: EcsEntity) {
    let ecs = &server.state.ecs();
    let server_eventbus = ecs.read_resource::<EventBus<ServerEvent>>();

    if let Some(mut char_state) = ecs.write_storage::<CharacterState>().get_mut(entity) {
        // Interrupt sprite interaction and item use if any attack is applied to entity
        if matches!(
            *char_state,
            CharacterState::SpriteInteract(_) | CharacterState::UseItem(_)
        ) {
            *char_state = CharacterState::Idle;
        }
    }

    // Remove potion/saturation buff if attacked
    server_eventbus.emit_now(ServerEvent::Buff {
        entity,
        buff_change: buff::BuffChange::RemoveByKind(buff::BuffKind::Potion),
    });
    server_eventbus.emit_now(ServerEvent::Buff {
        entity,
        buff_change: buff::BuffChange::RemoveByKind(buff::BuffKind::Saturation),
    });
}
