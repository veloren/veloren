use std::{f32::consts::PI, ops::Mul};

use common::rtsim::DialogueKind;
use common_state::{BlockChange, ScheduledBlockChange};
use specs::{DispatcherBuilder, Join, ReadExpect, ReadStorage, WriteExpect, WriteStorage};
use tracing::error;
use vek::*;

use common::{
    assets::{AssetCombined, AssetHandle, Ron},
    comp::{
        self, InventoryUpdateEvent,
        agent::{AgentEvent, Sound, SoundKind},
        inventory::slot::EquipSlot,
        item::{MaterialStatManifest, flatten_counted_items},
        loot_owner::LootOwnerKind,
        tool::AbilityMap,
    },
    consts::{MAX_INTERACT_RANGE, MAX_NPCINTERACT_RANGE, SOUND_TRAVEL_DIST_PER_VOLUME},
    event::{
        CreateItemDropEvent, CreateSpriteEvent, DialogueEvent, EventBus, MineBlockEvent,
        NpcInteractEvent, SetLanternEvent, SetPetStayEvent, SoundEvent, TamePetEvent,
        ToggleSpriteLightEvent,
    },
    link::Is,
    mounting::Mount,
    outcome::Outcome,
    resources::ProgramTime,
    terrain::{self, Block, SpriteKind, TerrainGrid},
    uid::Uid,
    util::Dir,
    vol::ReadVol,
};

use crate::{Server, ServerGeneral, Time, client::Client};

use crate::pet::tame_pet;
use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;

use super::{ServerEvent, event_dispatch, mounting::within_mounting_range};

pub(super) fn register_event_systems(builder: &mut DispatcherBuilder) {
    event_dispatch::<SetLanternEvent>(builder, &[]);
    event_dispatch::<NpcInteractEvent>(builder, &[]);
    event_dispatch::<DialogueEvent>(builder, &[]);
    event_dispatch::<SetPetStayEvent>(builder, &[]);
    event_dispatch::<MineBlockEvent>(builder, &[]);
    event_dispatch::<SoundEvent>(builder, &[]);
    event_dispatch::<CreateSpriteEvent>(builder, &[]);
    event_dispatch::<ToggleSpriteLightEvent>(builder, &[]);
}

impl ServerEvent for SetLanternEvent {
    type SystemData<'a> = (
        WriteStorage<'a, comp::LightEmitter>,
        ReadStorage<'a, comp::Inventory>,
        ReadStorage<'a, comp::Health>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (mut light_emitters, inventories, healths): Self::SystemData<'_>,
    ) {
        for SetLanternEvent(entity, enable) in events {
            let lantern_exists = light_emitters
                .get(entity)
                .is_some_and(|light| light.strength > 0.0);

            if lantern_exists != enable {
                if !enable {
                    light_emitters.remove(entity);
                }
                // Only enable lantern if entity is alive
                else if healths.get(entity).is_none_or(|h| !h.is_dead) {
                    let lantern_info = inventories
                        .get(entity)
                        .and_then(|inventory| inventory.equipped(EquipSlot::Lantern))
                        .and_then(|item| {
                            if let comp::item::ItemKind::Lantern(l) = &*item.kind() {
                                Some((l.color(), l.strength(), l.flicker()))
                            } else {
                                None
                            }
                        });
                    if let Some((col, strength, flicker)) = lantern_info {
                        let _ = light_emitters.insert(entity, comp::LightEmitter {
                            col,
                            strength,
                            flicker,
                            animated: true,
                        });
                    }
                }
            }
        }
    }
}

impl ServerEvent for NpcInteractEvent {
    type SystemData<'a> = (
        WriteStorage<'a, comp::Agent>,
        ReadStorage<'a, comp::Pos>,
        ReadStorage<'a, Uid>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (mut agents, positions, uids): Self::SystemData<'_>,
    ) {
        for NpcInteractEvent(interactor, npc_entity) in events {
            let within_range = {
                positions
                    .get(interactor)
                    .zip(positions.get(npc_entity))
                    .is_some_and(|(interactor_pos, npc_pos)| {
                        interactor_pos.0.distance_squared(npc_pos.0)
                            <= MAX_NPCINTERACT_RANGE.powi(2)
                    })
            };

            if within_range
                && let Some(agent) = agents.get_mut(npc_entity)
                && agent.target.is_none()
                && let Some(interactor_uid) = uids.get(interactor)
            {
                agent.inbox.push_back(AgentEvent::Talk(*interactor_uid));
            }
        }
    }
}

impl ServerEvent for DialogueEvent {
    type SystemData<'a> = (
        ReadStorage<'a, Uid>,
        ReadStorage<'a, comp::Pos>,
        ReadStorage<'a, Client>,
        WriteStorage<'a, comp::Agent>,
        WriteStorage<'a, comp::Inventory>,
        ReadExpect<'a, AbilityMap>,
        ReadExpect<'a, MaterialStatManifest>,
        WriteStorage<'a, comp::InventoryUpdate>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (
            uids,
            positions,
            clients,
            mut agents,
            mut inventories,
            ability_map,
            msm,
            mut inventory_updates,
        ): Self::SystemData<'_>,
    ) {
        for DialogueEvent(sender, target, dialogue) in events {
            let within_range = positions
                .get(sender)
                .zip(positions.get(target))
                .is_some_and(|(sender_pos, target_pos)| {
                    sender_pos.0.distance_squared(target_pos.0) <= MAX_NPCINTERACT_RANGE.powi(2)
                });

            if within_range && let Some(sender_uid) = uids.get(sender) {
                // Perform item transfer, if required
                let given_item = match &dialogue.kind {
                    DialogueKind::Start
                    | DialogueKind::End
                    | DialogueKind::Question { .. }
                    | DialogueKind::Marker { .. } => None,
                    DialogueKind::Statement { given_item, .. } => given_item.as_ref(),
                    DialogueKind::Response { response, .. } => response.given_item.as_ref(),
                };
                // If the response requires an item to be given, perform exchange (or exit)
                if let Some((item_def, amount)) = given_item {
                    // Check that the target's inventory has enough space for the item
                    if let Some(target_inv) = inventories.get(target)
                        && target_inv.has_space_for(item_def, *amount)
                        // Check that the sender has enough of the item
                        && let Some(mut sender_inv) = inventories.get_mut(sender)
                        && sender_inv.item_count(item_def) >= *amount as u64
                        // First, remove the item from the sender's inventory
                        && let Some(items) = sender_inv.remove_item_amount(item_def, *amount, &ability_map, &msm)
                        && let Some(mut target_inv) = inventories.get_mut(target)
                    {
                        for item in items {
                            let item_event = InventoryUpdateEvent::Collected(
                                item.frontend_item(&ability_map, &msm),
                            );
                            // Push the items to the target's inventory
                            if target_inv.push(item).is_err() {
                                error!(
                                    "Failed to insert dialogue given item despite target \
                                     inventory claiming to have space, dropping remaining items..."
                                );
                                break;
                            } else {
                                inventory_updates
                                    .insert(target, comp::InventoryUpdate::new(item_event));
                            }
                        }
                    } else {
                        // TODO: Respond with error message on failure?
                        continue;
                    }
                }

                let dialogue = dialogue.into_validated_unchecked();

                if let Some(agent) = agents.get_mut(target) {
                    agent
                        .inbox
                        .push_back(AgentEvent::Dialogue(*sender_uid, dialogue.clone()));
                }

                if let Some(client) = clients.get(target) {
                    client.send_fallible(ServerGeneral::Dialogue(*sender_uid, dialogue));
                }
            }
        }
    }
}

impl ServerEvent for SetPetStayEvent {
    type SystemData<'a> = (
        WriteStorage<'a, comp::Agent>,
        WriteStorage<'a, comp::CharacterActivity>,
        ReadStorage<'a, comp::Pos>,
        ReadStorage<'a, comp::Alignment>,
        ReadStorage<'a, Is<Mount>>,
        ReadStorage<'a, Uid>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (mut agents, mut character_activities, positions, alignments, is_mounts, uids): Self::SystemData<'_>,
    ) {
        for SetPetStayEvent(command_giver, pet, stay) in events {
            let is_owner = uids.get(command_giver).is_some_and(|owner_uid| {
                matches!(
                    alignments.get(pet),
                    Some(comp::Alignment::Owned(pet_owner)) if *pet_owner == *owner_uid,
                )
            });

            let current_pet_position = positions.get(pet).copied();
            let stay = stay && current_pet_position.is_some();
            if is_owner
                && within_mounting_range(positions.get(command_giver), positions.get(pet))
                && is_mounts.get(pet).is_none()
            {
                character_activities
                    .get_mut(pet)
                    .map(|mut activity| activity.is_pet_staying = stay);
                agents
                    .get_mut(pet)
                    .map(|s| s.stay_pos = current_pet_position.filter(|_| stay));
            }
        }
    }
}

lazy_static! {
    static ref RESOURCE_EXPERIENCE_MANIFEST: AssetHandle<Ron<HashMap<String, u32>>> =
        Ron::load_expect_combined_static("server.manifests.resource_experience_manifest");
}

impl ServerEvent for MineBlockEvent {
    type SystemData<'a> = (
        WriteExpect<'a, BlockChange>,
        ReadExpect<'a, TerrainGrid>,
        ReadExpect<'a, MaterialStatManifest>,
        ReadExpect<'a, AbilityMap>,
        ReadExpect<'a, EventBus<CreateItemDropEvent>>,
        ReadExpect<'a, EventBus<SoundEvent>>,
        ReadExpect<'a, EventBus<Outcome>>,
        ReadExpect<'a, ProgramTime>,
        ReadExpect<'a, Time>,
        WriteStorage<'a, comp::SkillSet>,
        ReadStorage<'a, Uid>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (
            mut block_change,
            terrain,
            msm,
            ability_map,
            create_item_drop_events,
            sound_events,
            outcomes,
            program_time,
            time,
            mut skill_sets,
            uids,
        ): Self::SystemData<'_>,
    ) {
        use rand::Rng;
        let mut rng = rand::rng();
        let mut create_item_drop_emitter = create_item_drop_events.emitter();
        let mut sound_event_emitter = sound_events.emitter();
        let mut outcome_emitter = outcomes.emitter();
        for ev in events {
            if block_change.can_set_block(ev.pos) {
                let block = terrain.get(ev.pos).ok().copied();
                if let Some(mut block) =
                    block.filter(|b| b.mine_tool().is_some_and(|t| Some(t) == ev.tool))
                {
                    // Attempt to increase the resource's damage
                    let damage = if let Ok(damage) = block.get_attr::<terrain::sprite::Damage>() {
                        let updated_damage = damage.0.saturating_add(1);
                        block
                            .set_attr(terrain::sprite::Damage(updated_damage))
                            .expect(
                                "We just read the Damage attribute from the block, writing should \
                                 be possible too",
                            );

                        Some(updated_damage)
                    } else {
                        None
                    };

                    let sprite = block.get_sprite();

                    // Maximum damage has reached, destroy the block
                    let is_broken = damage
                        .and_then(|damage| Some((sprite?.required_mine_damage(), damage)))
                        .is_some_and(|(required_damage, damage)| {
                            required_damage.is_none_or(|required| damage >= required)
                        });

                    // Stage changes happen in damage interval of `mine_drop_intevral`
                    let stage_changed = damage
                        .and_then(|damage| Some((sprite?.mine_drop_interval(), damage)))
                        .is_some_and(|(interval, damage)| damage % interval == 0);

                    let sprite_cfg = terrain.sprite_cfg_at(ev.pos);
                    if (stage_changed || is_broken)
                        && let Some(items) = comp::Item::try_reclaim_from_block(block, sprite_cfg)
                    {
                        let mut items: Vec<_> =
                            flatten_counted_items(&items, &ability_map, &msm).collect();
                        let maybe_uid = uids.get(ev.entity).copied();

                        if let Some(mut skillset) = skill_sets.get_mut(ev.entity) {
                            use common::comp::skills::{MiningSkill, SKILL_MODIFIERS, Skill};

                            if is_broken
                                && let (Some(tool), Some(uid), exp_reward @ 1..) = (
                                    ev.tool,
                                    maybe_uid,
                                    items
                                        .iter()
                                        .filter_map(|item| {
                                            item.item_definition_id().itemdef_id().and_then(|id| {
                                                RESOURCE_EXPERIENCE_MANIFEST
                                                    .read()
                                                    .0
                                                    .get(id)
                                                    .copied()
                                            })
                                        })
                                        .sum(),
                                )
                            {
                                let skill_group = comp::SkillGroupKind::Weapon(tool);
                                if let Some(level_outcome) =
                                    skillset.add_experience(skill_group, exp_reward)
                                {
                                    outcome_emitter.emit(Outcome::SkillPointGain {
                                        uid,
                                        skill_tree: skill_group,
                                        total_points: level_outcome,
                                    });
                                }
                                outcome_emitter.emit(Outcome::ExpChange {
                                    uid,
                                    exp: exp_reward,
                                    xp_pools: HashSet::from([skill_group]),
                                });
                            }

                            let stage_ore_chance = || {
                                let chance_mod = f64::from(SKILL_MODIFIERS.mining_tree.ore_gain);
                                let skill_level = skillset
                                    .skill_level(Skill::Pick(MiningSkill::OreGain))
                                    .unwrap_or(0);

                                chance_mod * f64::from(skill_level)
                            };
                            let stage_gem_chance = || {
                                let chance_mod = f64::from(SKILL_MODIFIERS.mining_tree.gem_gain);
                                let skill_level = skillset
                                    .skill_level(Skill::Pick(MiningSkill::GemGain))
                                    .unwrap_or(0);

                                chance_mod * f64::from(skill_level)
                            };

                            // If the resource hasn't been fully broken, only drop certain resources
                            // with a chance
                            if !is_broken {
                                items.retain(|item| {
                                    rng.random_bool(
                                        0.5 + item
                                            .item_definition_id()
                                            .itemdef_id()
                                            .map(|id| {
                                                if id.contains("mineral.ore.") {
                                                    stage_ore_chance()
                                                } else if id.contains("mineral.gem.") {
                                                    stage_gem_chance()
                                                } else {
                                                    0.0
                                                }
                                            })
                                            .unwrap_or(0.0),
                                    )
                                });
                            }
                        }
                        for item in items {
                            let loot_owner = maybe_uid
                                .map(LootOwnerKind::Player)
                                .map(|owner| comp::LootOwner::new(owner, false));
                            create_item_drop_emitter.emit(CreateItemDropEvent {
                                pos: comp::Pos(ev.pos.map(|e| e as f32) + Vec3::broadcast(0.5)),
                                vel: comp::Vel(
                                    Vec2::unit_x()
                                        .rotated_z(rng.random::<f32>() * PI * 2.0)
                                        .mul(4.0)
                                        .with_z(rng.random_range(5.0..10.0)),
                                ),
                                ori: comp::Ori::from(Dir::random_2d(&mut rng)),
                                item: comp::PickupItem::new(item, *program_time, false),
                                loot_owner,
                            });
                        }
                    }

                    if damage.is_some() && !is_broken {
                        block_change.set(ev.pos, block);
                    } else {
                        block_change.set(ev.pos, block.into_vacant());
                    }
                    outcome_emitter.emit(if is_broken {
                        Outcome::BreakBlock {
                            pos: ev.pos,
                            tool: ev.tool,
                            color: block.get_color(),
                        }
                    } else {
                        Outcome::DamagedBlock {
                            pos: ev.pos,
                            stage_changed,
                            tool: ev.tool,
                        }
                    });

                    // Emit mining sound
                    sound_event_emitter.emit(SoundEvent {
                        sound: Sound::new(SoundKind::Mine, ev.pos.as_(), 20.0, time.0),
                    });
                }
            }
        }
    }
}

impl ServerEvent for SoundEvent {
    type SystemData<'a> = (
        ReadExpect<'a, EventBus<Outcome>>,
        WriteStorage<'a, comp::Agent>,
        ReadStorage<'a, comp::Pos>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (outcomes, mut agents, positions): Self::SystemData<'_>,
    ) {
        let mut outcome_emitter = outcomes.emitter();
        for SoundEvent { sound } in events {
            // TODO: Reduce the complexity of this problem by using spatial partitioning
            // system
            for (agent, agent_pos) in (&mut agents, &positions).join() {
                // TODO: Use pathfinding for more dropoff around obstacles
                let agent_dist_sqrd = agent_pos.0.distance_squared(sound.pos);
                let sound_travel_dist_sqrd = (sound.vol * SOUND_TRAVEL_DIST_PER_VOLUME).powi(2);

                let vol_dropoff = agent_dist_sqrd / sound_travel_dist_sqrd * sound.vol;
                let propagated_sound = sound.with_new_vol(sound.vol - vol_dropoff);

                let can_hear_sound = propagated_sound.vol > 0.00;
                let should_hear_sound = agent_dist_sqrd < agent.psyche.listen_dist.powi(2);

                if can_hear_sound && should_hear_sound {
                    agent
                        .inbox
                        .push_back(AgentEvent::ServerSound(propagated_sound));
                }
            }

            // Attempt to turn this sound into an outcome to be received by frontends.
            if let Some(outcome) = match sound.kind {
                SoundKind::Utterance(kind, body) => Some(Outcome::Utterance {
                    kind,
                    pos: sound.pos,
                    body,
                }),
                _ => None,
            } {
                outcome_emitter.emit(outcome);
            }
        }
    }
}

impl ServerEvent for CreateSpriteEvent {
    type SystemData<'a> = (
        WriteExpect<'a, BlockChange>,
        WriteExpect<'a, ScheduledBlockChange>,
        ReadExpect<'a, TerrainGrid>,
        ReadExpect<'a, Time>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (mut block_change, mut scheduled_block_change, terrain, time): Self::SystemData<'_>,
    ) {
        for ev in events {
            if block_change.can_set_block(ev.pos) {
                let block = terrain.get(ev.pos).ok().copied();
                if block.is_some_and(|b| (*b).is_fluid()) {
                    let old_block = block.unwrap_or_else(|| Block::air(SpriteKind::Empty));
                    let new_block = old_block.with_sprite(ev.sprite);
                    block_change.set(ev.pos, new_block);
                    // Remove sprite after del_timeout and offset if specified
                    if let Some((timeout, del_offset)) = ev.del_timeout {
                        use rand::Rng;
                        let mut rng = rand::rng();
                        let offset = rng.random_range(0.0..del_offset);
                        let current_time: f64 = time.0;
                        let replace_time = current_time + (timeout + offset) as f64;
                        if old_block != new_block {
                            scheduled_block_change.set(ev.pos, old_block, replace_time);
                            scheduled_block_change.outcome_set(ev.pos, new_block, replace_time);
                        }
                    }
                }
            }
        }
    }
}

impl ServerEvent for ToggleSpriteLightEvent {
    type SystemData<'a> = (
        WriteExpect<'a, BlockChange>,
        ReadExpect<'a, TerrainGrid>,
        ReadStorage<'a, comp::Pos>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (mut block_change, terrain, positions): Self::SystemData<'_>,
    ) {
        for ev in events.into_iter() {
            if let Some(entity_pos) = positions.get(ev.entity)
                && entity_pos.0.distance_squared(ev.pos.as_()) < MAX_INTERACT_RANGE.powi(2)
                && block_change.can_set_block(ev.pos)
                && let Some(new_block) = terrain
                    .get(ev.pos)
                    .ok()
                    .and_then(|block| block.with_toggle_light(ev.enable))
            {
                block_change.set(ev.pos, new_block);
                // TODO: Emit outcome
            }
        }
    }
}

pub fn handle_tame_pet(server: &mut Server, ev: TamePetEvent) {
    // TODO: Raise outcome to send to clients to play sound/render an indicator
    // showing taming success?
    tame_pet(server.state.ecs(), ev.pet_entity, ev.owner_entity);
}
