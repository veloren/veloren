#![allow(dead_code)] // TODO: Remove this when rtsim is fleshed out

use super::*;
use crate::sys::terrain::NpcData;
use common::{
    comp::{self, inventory::loadout::Loadout, skillset::skills},
    event::{EventBus, ServerEvent},
    generation::{BodyBuilder, EntityConfig, EntityInfo},
    resources::{DeltaTime, Time},
    rtsim::{RtSimController, RtSimEntity},
    slowjob::SlowJobPool,
    LoadoutBuilder,
    SkillSetBuilder,
};
use common_ecs::{Job, Origin, Phase, System};
use rtsim2::data::npc::{NpcMode, Profession};
use specs::{Join, Read, ReadExpect, ReadStorage, WriteExpect, WriteStorage};
use std::{sync::Arc, time::Duration};
use world::site::settlement::merchant_loadout;

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, DeltaTime>,
        Read<'a, Time>,
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
            mut server_event_bus,
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

        rtsim.state.tick(&world, index.as_index_ref(), dt.0);

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
                let body = npc.get_body();
                let mut loadout_builder = LoadoutBuilder::from_default(&body);
                let mut rng = npc.rng(3);

                if let Some(ref profession) = npc.profession {
                    loadout_builder = match profession {
                        Profession::Guard => loadout_builder
                            .with_asset_expect("common.loadout.village.guard", &mut rng),

                        Profession::Merchant => {
                            merchant_loadout(
                                loadout_builder,
                                npc.home
                                    .and_then(|home| {
                                        let site = data.sites.get(home)?.world_site?;
                                        index.sites.get(site).trade_information(site.id())
                                    }).as_ref(),
                            )
                        }

                        Profession::Farmer | Profession::Hunter => loadout_builder
                            .with_asset_expect("common.loadout.village.villager", &mut rng),

                        Profession::Adventurer(level) => todo!(),
                    };
                }

                let can_speak = npc.profession.is_some(); // TODO: not this

                let trade_for_site = if let Some(Profession::Merchant) = npc.profession {
                    npc.home.and_then(|home| Some(data.sites.get(home)?.world_site?.id()))
                } else {
                    None
                };

                let skill_set = SkillSetBuilder::default().build();
                let health_level = skill_set
                    .skill_level(skills::Skill::General(skills::GeneralSkill::HealthIncrease))
                    .unwrap_or(0);
                emitter.emit(ServerEvent::CreateNpc {
                    pos: comp::Pos(npc.wpos),
                    stats: comp::Stats::new("Rtsim NPC".to_string()),
                    skill_set: skill_set,
                    health: Some(comp::Health::new(body, health_level)),
                    poise: comp::Poise::new(body),
                    inventory: comp::Inventory::with_loadout(loadout_builder.build(), body),
                    body,
                    agent: Some(comp::Agent::from_body(&body)
                        .with_behavior(
                            comp::Behavior::default()
                                .maybe_with_capabilities(can_speak.then_some(comp::BehaviorCapability::SPEAK))
                                .with_trade_site(trade_for_site),
                        )),
                    alignment: if can_speak {
                        comp::Alignment::Npc
                    } else {
                        comp::Alignment::Wild
                    },
                    scale: comp::Scale(1.0),
                    anchor: None,
                    loot: Default::default(),
                    rtsim_entity: Some(RtSimEntity(npc_id)),
                    projectile: None,
                });
            }
        }

        // Synchronise rtsim NPC with entity data
        for (pos, rtsim_entity, agent) in
            (&positions, &rtsim_entities, (&mut agents).maybe()).join()
        {
            data
                .npcs
                .get_mut(rtsim_entity.0)
                .filter(|npc| matches!(npc.mode, NpcMode::Loaded))
                .map(|npc| {
                    // Update rtsim NPC state
                    npc.wpos = pos.0;

                    // Update entity state
                    if let Some(agent) = agent {
                        agent.rtsim_controller.travel_to = npc.target.map(|(wpos, _)| wpos);
                        agent.rtsim_controller.speed_factor = npc.target.map_or(1.0, |(_, sf)| sf);
                    }
                });
        }
    }
}
