use specs::{world::WorldExt, Builder, Entity as EcsEntity, Join};
use vek::*;

use common::{
    assets,
    comp::{
        self,
        agent::{AgentEvent, Sound, SoundKind},
        dialogue::Subject,
        inventory::slot::EquipSlot,
        loot_owner::LootOwnerKind,
        tool::ToolKind,
        Inventory, LootOwner, Pos, SkillGroupKind,
    },
    consts::{MAX_MOUNT_RANGE, SOUND_TRAVEL_DIST_PER_VOLUME},
    event::EventBus,
    link::Is,
    mounting::{Mount, Mounting, Rider},
    outcome::Outcome,
    terrain::{Block, SpriteKind},
    uid::Uid,
    vol::ReadVol,
};
use common_net::sync::WorldSyncExt;

use crate::{state_ext::StateExt, Server, Time};

use crate::pet::tame_pet;
use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;
use serde::Deserialize;
use std::iter::FromIterator;

pub fn handle_lantern(server: &mut Server, entity: EcsEntity, enable: bool) {
    let ecs = server.state_mut().ecs();

    let lantern_exists = ecs
        .read_storage::<comp::LightEmitter>()
        .get(entity)
        .map_or(false, |light| light.strength > 0.0);

    if lantern_exists != enable {
        if !enable {
            server
                .state_mut()
                .ecs()
                .write_storage::<comp::LightEmitter>()
                .remove(entity);
        } else if ecs // Only enable lantern if entity is alive
            .read_storage::<comp::Health>()
            .get(entity)
            .map_or(true, |h| !h.is_dead)
        {
            let inventory_storage = ecs.read_storage::<Inventory>();
            let lantern_info = inventory_storage
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
                let _ =
                    ecs.write_storage::<comp::LightEmitter>()
                        .insert(entity, comp::LightEmitter {
                            col,
                            strength,
                            flicker,
                            animated: true,
                        });
            }
        }
    }
}

pub fn handle_npc_interaction(server: &mut Server, interactor: EcsEntity, npc_entity: EcsEntity) {
    let state = server.state_mut();
    if let Some(agent) = state
        .ecs()
        .write_storage::<comp::Agent>()
        .get_mut(npc_entity)
    {
        if agent.target.is_none() {
            if let Some(interactor_uid) = state.ecs().uid_from_entity(interactor) {
                agent
                    .inbox
                    .push_back(AgentEvent::Talk(interactor_uid, Subject::Regular));
            }
        }
    }
}

pub fn handle_mount(server: &mut Server, rider: EcsEntity, mount: EcsEntity) {
    let state = server.state_mut();

    if state.ecs().read_storage::<Is<Rider>>().get(rider).is_none() {
        let not_mounting_yet = state.ecs().read_storage::<Is<Mount>>().get(mount).is_none();

        let within_range = || {
            let positions = state.ecs().read_storage::<Pos>();
            within_mounting_range(positions.get(rider), positions.get(mount))
        };
        let healths = state.ecs().read_storage::<comp::Health>();
        let alive = |e| healths.get(e).map_or(true, |h| !h.is_dead);

        if not_mounting_yet && within_range() && alive(rider) && alive(mount) {
            let uids = state.ecs().read_storage::<Uid>();
            if let (Some(rider_uid), Some(mount_uid)) =
                (uids.get(rider).copied(), uids.get(mount).copied())
            {
                let is_pet = matches!(
                    state
                        .ecs()
                        .read_storage::<comp::Alignment>()
                        .get(mount),
                    Some(comp::Alignment::Owned(owner)) if *owner == rider_uid,
                );

                if is_pet {
                    drop(uids);
                    drop(healths);
                    let _ = state.link(Mounting {
                        mount: mount_uid,
                        rider: rider_uid,
                    });
                }
            }
        }
    }
}

pub fn handle_unmount(server: &mut Server, rider: EcsEntity) {
    let state = server.state_mut();
    state.ecs().write_storage::<Is<Rider>>().remove(rider);
}

fn within_mounting_range(player_position: Option<&Pos>, mount_position: Option<&Pos>) -> bool {
    match (player_position, mount_position) {
        (Some(ppos), Some(ipos)) => ppos.0.distance_squared(ipos.0) < MAX_MOUNT_RANGE.powi(2),
        _ => false,
    }
}

#[derive(Deserialize)]
struct ResourceExperienceManifest(HashMap<String, u32>);

impl assets::Asset for ResourceExperienceManifest {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

lazy_static! {
    static ref RESOURCE_EXPERIENCE_MANIFEST: assets::AssetHandle<ResourceExperienceManifest> =
        assets::AssetExt::load_expect("server.manifests.resource_experience_manifest");
}

pub fn handle_mine_block(
    server: &mut Server,
    entity: EcsEntity,
    pos: Vec3<i32>,
    tool: Option<ToolKind>,
) {
    let state = server.state_mut();
    if state.can_set_block(pos) {
        let block = state.terrain().get(pos).ok().copied();
        if let Some(block) = block.filter(|b| b.mine_tool().map_or(false, |t| Some(t) == tool)) {
            // Drop item if one is recoverable from the block
            if let Some(mut item) = comp::Item::try_reclaim_from_block(block) {
                let maybe_uid = state.ecs().uid_from_entity(entity);

                if let Some(mut skillset) = state
                    .ecs()
                    .write_storage::<comp::SkillSet>()
                    .get_mut(entity)
                {
                    if let (Some(tool), Some(uid), Some(exp_reward)) = (
                        tool,
                        maybe_uid,
                        item.item_definition_id()
                            .itemdef_id()
                            .and_then(|id| RESOURCE_EXPERIENCE_MANIFEST.read().0.get(id).copied()),
                    ) {
                        let skill_group = SkillGroupKind::Weapon(tool);
                        let outcome_bus = state.ecs().read_resource::<EventBus<Outcome>>();
                        if let Some(level_outcome) =
                            skillset.add_experience(skill_group, exp_reward)
                        {
                            outcome_bus.emit_now(Outcome::SkillPointGain {
                                uid,
                                skill_tree: skill_group,
                                total_points: level_outcome,
                            });
                        }
                        outcome_bus.emit_now(Outcome::ExpChange {
                            uid,
                            exp: exp_reward,
                            xp_pools: HashSet::from_iter(vec![skill_group]),
                        });
                    }
                    use common::comp::skills::{MiningSkill, Skill, SKILL_MODIFIERS};
                    use rand::Rng;
                    let mut rng = rand::thread_rng();

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

                    let double_gain = item.item_definition_id().itemdef_id().map_or(false, |id| {
                        (id.contains("mineral.ore.") && need_double_ore(&mut rng))
                            || (id.contains("mineral.gem.") && need_double_gem(&mut rng))
                    });

                    if double_gain {
                        // Ignore non-stackable errors
                        let _ = item.increase_amount(1);
                    }
                }
                let item_drop = state
                    .create_item_drop(Default::default(), item)
                    .with(Pos(pos.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0)));
                if let Some(uid) = maybe_uid {
                    item_drop.with(LootOwner::new(LootOwnerKind::Player(uid)))
                } else {
                    item_drop
                }
                .build();
            }

            state.set_block(pos, block.into_vacant());
            state
                .ecs()
                .read_resource::<EventBus<Outcome>>()
                .emit_now(Outcome::BreakBlock {
                    pos,
                    color: block.get_color(),
                });
        }
    }
}

pub fn handle_sound(server: &mut Server, sound: &Sound) {
    let ecs = &server.state.ecs();
    let positions = &ecs.read_storage::<Pos>();
    let agents = &mut ecs.write_storage::<comp::Agent>();

    // TODO: Reduce the complexity of this problem by using spatial partitioning
    // system
    for (agent, agent_pos) in (agents, positions).join() {
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
        ecs.read_resource::<EventBus<Outcome>>().emit_now(outcome);
    }
}

pub fn handle_create_sprite(
    server: &mut Server,
    pos: Vec3<i32>,
    sprite: SpriteKind,
    del_timeout: Option<(f32, f32)>,
) {
    let state = server.state_mut();
    if state.can_set_block(pos) {
        let block = state.terrain().get(pos).ok().copied();
        if block.map_or(false, |b| (*b).is_air()) {
            let old_block = block.unwrap_or_else(|| Block::air(SpriteKind::Empty));
            let new_block = old_block.with_sprite(sprite);
            state.set_block(pos, new_block);
            // Remove sprite after del_timeout and offset if specified
            if let Some((timeout, del_offset)) = del_timeout {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let offset = rng.gen_range(0.0..del_offset);
                let current_time: f64 = state.ecs().read_resource::<Time>().0;
                let replace_time = current_time + (timeout + offset) as f64;
                if old_block != new_block {
                    server
                        .state
                        .schedule_set_block(pos, old_block, new_block, replace_time)
                }
            }
        }
    }
}

pub fn handle_tame_pet(server: &mut Server, pet_entity: EcsEntity, owner_entity: EcsEntity) {
    // TODO: Raise outcome to send to clients to play sound/render an indicator
    // showing taming success?
    tame_pet(server.state.ecs(), pet_entity, owner_entity);
}
