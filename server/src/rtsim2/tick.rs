#![allow(dead_code)] // TODO: Remove this when rtsim is fleshed out

use super::*;
use crate::sys::terrain::NpcData;
use common::{
    comp::{self, inventory::loadout::Loadout, skillset::skills},
    event::{EventBus, ServerEvent},
    generation::{BodyBuilder, EntityConfig, EntityInfo},
    resources::{DeltaTime, Time, TimeOfDay},
    rtsim::{RtSimController, RtSimEntity},
    slowjob::SlowJobPool,
    trade::{Good, SiteInformation},
    LoadoutBuilder, SkillSetBuilder,
};
use common_ecs::{Job, Origin, Phase, System};
use rtsim2::data::{
    npc::{NpcMode, Profession},
    Npc, Sites,
};
use specs::{Join, Read, ReadExpect, ReadStorage, WriteExpect, WriteStorage};
use std::{sync::Arc, time::Duration};
use world::site::settlement::trader_loadout;

fn humanoid_config(profession: &Profession) -> &'static str {
    match profession {
        Profession::Farmer => "common.entity.village.farmer",
        Profession::Hunter => "common.entity.village.hunter",
        Profession::Herbalist => "common.entity.village.herbalist",
        Profession::Merchant => "common.entity.village.merchant",
        Profession::Guard => "common.entity.village.guard",
        Profession::Adventurer(rank) => match rank {
            0 => "common.entity.world.traveler0",
            1 => "common.entity.world.traveler1",
            2 => "common.entity.world.traveler2",
            3 => "common.entity.world.traveler3",
            _ => panic!("Not a valid adventurer rank"),
        },
        Profession::Blacksmith => "common.entity.village.blacksmith",
        Profession::Chef => "common.entity.village.chef",
        Profession::Alchemist => "common.entity.village.alchemist",
        Profession::Pirate => "common.entity.spot.pirate",
        Profession::Cultist => "common.entity.dungeon.tier-5.cultist",
    }
}

fn loadout_default(loadout: LoadoutBuilder, _economy: Option<&SiteInformation>) -> LoadoutBuilder {
    loadout
}

fn merchant_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
) -> LoadoutBuilder {
    trader_loadout(loadout_builder, economy, |_| true)
}

fn farmer_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
) -> LoadoutBuilder {
    trader_loadout(loadout_builder, economy, |good| matches!(good, Good::Food))
}

fn herbalist_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
) -> LoadoutBuilder {
    trader_loadout(loadout_builder, economy, |good| {
        matches!(good, Good::Ingredients)
    })
}

fn chef_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
) -> LoadoutBuilder {
    trader_loadout(loadout_builder, economy, |good| matches!(good, Good::Food))
}

fn blacksmith_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
) -> LoadoutBuilder {
    trader_loadout(loadout_builder, economy, |good| {
        matches!(good, Good::Tools | Good::Armor)
    })
}

fn alchemist_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
) -> LoadoutBuilder {
    trader_loadout(loadout_builder, economy, |good| {
        matches!(good, Good::Potions)
    })
}

fn profession_extra_loadout(
    profession: Option<&Profession>,
) -> fn(LoadoutBuilder, Option<&SiteInformation>) -> LoadoutBuilder {
    match profession {
        Some(Profession::Merchant) => merchant_loadout,
        Some(Profession::Farmer) => farmer_loadout,
        Some(Profession::Herbalist) => herbalist_loadout,
        Some(Profession::Chef) => chef_loadout,
        Some(Profession::Blacksmith) => blacksmith_loadout,
        Some(Profession::Alchemist) => alchemist_loadout,
        _ => loadout_default,
    }
}

fn profession_agent_mark(profession: Option<&Profession>) -> Option<comp::agent::Mark> {
    match profession {
        Some(
            Profession::Merchant
            | Profession::Farmer
            | Profession::Herbalist
            | Profession::Chef
            | Profession::Blacksmith
            | Profession::Alchemist,
        ) => Some(comp::agent::Mark::Merchant),
        Some(Profession::Guard) => Some(comp::agent::Mark::Guard),
        _ => None,
    }
}

fn get_npc_entity_info(npc: &Npc, sites: &Sites, index: IndexRef) -> EntityInfo {
    let body = npc.get_body();
    let pos = comp::Pos(npc.wpos);

    if let Some(ref profession) = npc.profession {
        let economy = npc.home.and_then(|home| {
            let site = sites.get(home)?.world_site?;
            index.sites.get(site).trade_information(site.id())
        });

        let config_asset = humanoid_config(profession);

        let entity_config =
            EntityConfig::from_asset_expect_owned(config_asset).with_body(BodyBuilder::Exact(body));
        let mut rng = npc.rng(3);
        EntityInfo::at(pos.0)
            .with_entity_config(entity_config, Some(config_asset), &mut rng)
            .with_alignment(if matches!(profession, Profession::Cultist) {
                comp::Alignment::Enemy
            } else {
                comp::Alignment::Npc
            })
            .with_maybe_economy(economy.as_ref())
            .with_lazy_loadout(profession_extra_loadout(npc.profession.as_ref()))
            .with_maybe_agent_mark(profession_agent_mark(npc.profession.as_ref()))
    } else {
        EntityInfo::at(pos.0)
            .with_body(body)
            .with_alignment(comp::Alignment::Wild)
            .with_name("Rtsim NPC")
    }
}

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, DeltaTime>,
        Read<'a, Time>,
        Read<'a, TimeOfDay>,
        Read<'a, EventBus<ServerEvent>>,
        WriteExpect<'a, RtSim>,
        ReadExpect<'a, Arc<world::World>>,
        ReadExpect<'a, world::IndexOwned>,
        ReadExpect<'a, SlowJobPool>,
        ReadStorage<'a, comp::Pos>,
        ReadStorage<'a, RtSimEntity>,
        WriteStorage<'a, comp::Agent>,
    );

    const NAME: &'static str = "rtsim::tick";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            dt,
            time,
            time_of_day,
            server_event_bus,
            mut rtsim,
            world,
            index,
            slow_jobs,
            positions,
            rtsim_entities,
            mut agents,
        ): Self::SystemData,
    ) {
        let mut emitter = server_event_bus.emitter();
        let rtsim = &mut *rtsim;

        rtsim.state.data_mut().time_of_day = *time_of_day;
        rtsim
            .state
            .tick(&world, index.as_index_ref(), *time_of_day, *time, dt.0);

        if rtsim
            .last_saved
            .map_or(true, |ls| ls.elapsed() > Duration::from_secs(60))
        {
            rtsim.save(&slow_jobs);
        }

        let chunk_states = rtsim.state.resource::<ChunkStates>();
        let data = &mut *rtsim.state.data_mut();
        for (npc_id, npc) in data.npcs.iter_mut() {
            let chunk = npc.wpos.xy().map2(TerrainChunk::RECT_SIZE, |e, sz| {
                (e as i32).div_euclid(sz as i32)
            });

            // Load the NPC into the world if it's in a loaded chunk and is not already
            // loaded
            if matches!(npc.mode, NpcMode::Simulated)
                && chunk_states.0.get(chunk).map_or(false, |c| c.is_some())
            {
                npc.mode = NpcMode::Loaded;

                let entity_info = get_npc_entity_info(npc, &data.sites, index.as_index_ref());

                emitter.emit(match NpcData::from_entity_info(entity_info) {
                    NpcData::Data {
                        pos,
                        stats,
                        skill_set,
                        health,
                        poise,
                        inventory,
                        agent,
                        body,
                        alignment,
                        scale,
                        loot,
                    } => ServerEvent::CreateNpc {
                        pos,
                        stats,
                        skill_set,
                        health,
                        poise,
                        inventory,
                        agent,
                        body,
                        alignment,
                        scale,
                        anchor: None,
                        loot,
                        rtsim_entity: Some(RtSimEntity(npc_id)),
                        projectile: None,
                    },
                    // EntityConfig can't represent Waypoints at all
                    // as of now, and if someone will try to spawn
                    // rtsim waypoint it is definitely error.
                    NpcData::Waypoint(_) => unimplemented!(),
                });
            }
        }

        // Synchronise rtsim NPC with entity data
        for (pos, rtsim_entity, agent) in
            (&positions, &rtsim_entities, (&mut agents).maybe()).join()
        {
            data.npcs
                .get_mut(rtsim_entity.0)
                .filter(|npc| matches!(npc.mode, NpcMode::Loaded))
                .map(|npc| {
                    // Update rtsim NPC state
                    npc.wpos = pos.0;

                    // Update entity state
                    if let Some(agent) = agent {
                        agent.rtsim_controller.travel_to = npc.goto.map(|(wpos, _)| wpos);
                        agent.rtsim_controller.speed_factor = npc.goto.map_or(1.0, |(_, sf)| sf);
                    }
                });
        }
    }
}
