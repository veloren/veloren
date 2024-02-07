use common_state::{BlockChange, ScheduledBlockChange};
use specs::{DispatcherBuilder, Join, ReadExpect, ReadStorage, WriteExpect, WriteStorage};
use vek::*;

use common::{
    assets::{self, Concatenate},
    comp::{
        self,
        agent::{AgentEvent, SoundKind},
        inventory::slot::EquipSlot,
        item::{flatten_counted_items, MaterialStatManifest},
        loot_owner::LootOwnerKind,
        tool::AbilityMap,
    },
    consts::{MAX_INTERACT_RANGE, MAX_NPCINTERACT_RANGE, SOUND_TRAVEL_DIST_PER_VOLUME},
    event::{
        CreateItemDropEvent, CreateSpriteEvent, EventBus, MineBlockEvent, NpcInteractEvent,
        SetLanternEvent, SetPetStayEvent, SoundEvent, TamePetEvent, ToggleSpriteLightEvent,
    },
    link::Is,
    mounting::Mount,
    outcome::Outcome,
    terrain::{Block, SpriteKind, TerrainGrid},
    uid::Uid,
    util::Dir,
    vol::ReadVol,
};

use crate::{Server, Time};

use crate::pet::tame_pet;
use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;
use serde::Deserialize;
use std::iter::FromIterator;

use super::{event_dispatch, mounting::within_mounting_range, ServerEvent};

pub(super) fn register_event_systems(builder: &mut DispatcherBuilder) {
    event_dispatch::<SetLanternEvent>(builder);
    event_dispatch::<NpcInteractEvent>(builder);
    event_dispatch::<SetPetStayEvent>(builder);
    event_dispatch::<MineBlockEvent>(builder);
    event_dispatch::<SoundEvent>(builder);
    event_dispatch::<CreateSpriteEvent>(builder);
    event_dispatch::<ToggleSpriteLightEvent>(builder);
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
                .map_or(false, |light| light.strength > 0.0);

            if lantern_exists != enable {
                if !enable {
                    light_emitters.remove(entity);
                }
                // Only enable lantern if entity is alive
                else if healths.get(entity).map_or(true, |h| !h.is_dead) {
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
        for NpcInteractEvent(interactor, npc_entity, subject) in events {
            let within_range = {
                positions
                    .get(interactor)
                    .zip(positions.get(npc_entity))
                    .map_or(false, |(interactor_pos, npc_pos)| {
                        interactor_pos.0.distance_squared(npc_pos.0)
                            <= MAX_NPCINTERACT_RANGE.powi(2)
                    })
            };

            if within_range
                && let Some(agent) = agents.get_mut(npc_entity)
                && agent.target.is_none()
            {
                if let Some(interactor_uid) = uids.get(interactor) {
                    agent
                        .inbox
                        .push_back(AgentEvent::Talk(*interactor_uid, subject));
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
            let is_owner = uids.get(command_giver).map_or(false, |owner_uid| {
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

#[derive(Deserialize)]
struct ResourceExperienceManifest(HashMap<String, u32>);

impl assets::Asset for ResourceExperienceManifest {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}
impl Concatenate for ResourceExperienceManifest {
    fn concatenate(self, b: Self) -> Self { Self(self.0.concatenate(b.0)) }
}

lazy_static! {
    static ref RESOURCE_EXPERIENCE_MANIFEST: assets::AssetHandle<ResourceExperienceManifest> =
        assets::AssetCombined::load_expect_combined_static(
            "server.manifests.resource_experience_manifest"
        );
}

impl ServerEvent for MineBlockEvent {
    type SystemData<'a> = (
        WriteExpect<'a, BlockChange>,
        ReadExpect<'a, TerrainGrid>,
        ReadExpect<'a, MaterialStatManifest>,
        ReadExpect<'a, AbilityMap>,
        ReadExpect<'a, EventBus<CreateItemDropEvent>>,
        ReadExpect<'a, EventBus<Outcome>>,
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
            outcomes,
            mut skill_sets,
            uids,
        ): Self::SystemData<'_>,
    ) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut create_item_drop_emitter = create_item_drop_events.emitter();
        let mut outcome_emitter = outcomes.emitter();
        for ev in events {
            if block_change.can_set_block(ev.pos) {
                let block = terrain.get(ev.pos).ok().copied();
                if let Some(block) =
                    block.filter(|b| b.mine_tool().map_or(false, |t| Some(t) == ev.tool))
                {
                    // Drop item if one is recoverable from the block
                    if let Some(items) = comp::Item::try_reclaim_from_block(block) {
                        let mut items: Vec<_> =
                            flatten_counted_items(&items, &ability_map, &msm).collect();
                        let maybe_uid = uids.get(ev.entity).copied();

                        if let Some(mut skillset) = skill_sets.get_mut(ev.entity) {
                            if let (Some(tool), Some(uid), exp_reward @ 1..) = (
                                ev.tool,
                                maybe_uid,
                                items
                                    .iter()
                                    .filter_map(|item| {
                                        item.item_definition_id().itemdef_id().and_then(|id| {
                                            RESOURCE_EXPERIENCE_MANIFEST.read().0.get(id).copied()
                                        })
                                    })
                                    .sum(),
                            ) {
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
                                    xp_pools: HashSet::from_iter(vec![skill_group]),
                                });
                            }
                            use common::comp::skills::{MiningSkill, Skill, SKILL_MODIFIERS};

                            let need_double_ore = |rng: &mut rand::rngs::ThreadRng| {
                                let chance_mod = f64::from(SKILL_MODIFIERS.mining_tree.ore_gain);
                                let skill_level = skillset
                                    .skill_level(Skill::Pick(MiningSkill::OreGain))
                                    .unwrap_or(0);

                                rng.gen_bool(chance_mod * f64::from(skill_level))
                            };
                            let need_double_gem = |rng: &mut rand::rngs::ThreadRng| {
                                let chance_mod = f64::from(SKILL_MODIFIERS.mining_tree.gem_gain);
                                let skill_level = skillset
                                    .skill_level(Skill::Pick(MiningSkill::GemGain))
                                    .unwrap_or(0);

                                rng.gen_bool(chance_mod * f64::from(skill_level))
                            };
                            for item in items.iter_mut() {
                                let double_gain =
                                    item.item_definition_id().itemdef_id().map_or(false, |id| {
                                        (id.contains("mineral.ore.") && need_double_ore(&mut rng))
                                            || (id.contains("mineral.gem.")
                                                && need_double_gem(&mut rng))
                                    });

                                if double_gain {
                                    // Ignore non-stackable errors
                                    let _ = item.increase_amount(1);
                                }
                            }
                        }
                        for item in items {
                            let loot_owner = maybe_uid
                                .map(LootOwnerKind::Player)
                                .map(|owner| comp::LootOwner::new(owner, false));
                            create_item_drop_emitter.emit(CreateItemDropEvent {
                                pos: comp::Pos(ev.pos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0)),
                                vel: comp::Vel(Vec3::zero()),
                                ori: comp::Ori::from(Dir::random_2d(&mut rng)),
                                item,
                                loot_owner,
                            });
                        }
                    }

                    block_change.set(ev.pos, block.into_vacant());
                    outcome_emitter.emit(Outcome::BreakBlock {
                        pos: ev.pos,
                        color: block.get_color(),
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
                if block.map_or(false, |b| (*b).is_fluid()) {
                    let old_block = block.unwrap_or_else(|| Block::air(SpriteKind::Empty));
                    let new_block = old_block.with_sprite(ev.sprite);
                    block_change.set(ev.pos, new_block);
                    // Remove sprite after del_timeout and offset if specified
                    if let Some((timeout, del_offset)) = ev.del_timeout {
                        use rand::Rng;
                        let mut rng = rand::thread_rng();
                        let offset = rng.gen_range(0.0..del_offset);
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
            {
                if let Some(new_block) = terrain
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
}

pub fn handle_tame_pet(server: &mut Server, ev: TamePetEvent) {
    // TODO: Raise outcome to send to clients to play sound/render an indicator
    // showing taming success?
    tame_pet(server.state.ecs(), ev.pet_entity, ev.owner_entity);
}
