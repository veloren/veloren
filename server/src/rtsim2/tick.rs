#![allow(dead_code)] // TODO: Remove this when rtsim is fleshed out

use super::*;
use crate::sys::terrain::NpcData;
use common::{
    comp,
    event::{EventBus, ServerEvent},
    generation::{BodyBuilder, EntityConfig, EntityInfo},
    resources::{DeltaTime, Time},
    slowjob::SlowJobPool,
    rtsim::{RtSimEntity, RtSimController},
};
use common_ecs::{Job, Origin, Phase, System};
use rtsim2::data::npc::NpcMode;
use specs::{Join, Read, ReadExpect, ReadStorage, WriteExpect, WriteStorage};
use std::{sync::Arc, time::Duration};

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
        (dt, time, mut server_event_bus, mut rtsim, world, index, slow_jobs, positions, rtsim_entities, mut agents): Self::SystemData,
    ) {
        let mut emitter = server_event_bus.emitter();
        let rtsim = &mut *rtsim;

        rtsim.state.tick(&world, index.as_index_ref(), dt.0);

        if rtsim.last_saved.map_or(true, |ls| ls.elapsed() > Duration::from_secs(60)) {
            rtsim.save(&slow_jobs);
        }

        let chunk_states = rtsim.state.resource::<ChunkStates>();
        for (npc_id, npc) in rtsim.state.data_mut().npcs.iter_mut() {
            let chunk = npc.wpos
                .xy()
                .map2(TerrainChunk::RECT_SIZE, |e, sz| (e as i32).div_euclid(sz as i32));

            // Load the NPC into the world if it's in a loaded chunk and is not already loaded
            if matches!(npc.mode, NpcMode::Simulated) && chunk_states.0.get(chunk).map_or(false, |c| c.is_some()) {
                npc.mode = NpcMode::Loaded;

                let body = npc.get_body();
                emitter.emit(ServerEvent::CreateNpc {
                    pos: comp::Pos(npc.wpos),
                    stats: comp::Stats::new("Rtsim NPC".to_string()),
                    skill_set: comp::SkillSet::default(),
                    health: None,
                    poise: comp::Poise::new(body),
                    inventory: comp::Inventory::with_empty(),
                    body,
                    agent: Some(comp::Agent::from_body(&body)),
                    alignment: comp::Alignment::Wild,
                    scale: comp::Scale(1.0),
                    anchor: None,
                    loot: Default::default(),
                    rtsim_entity: Some(RtSimEntity(npc_id)),
                    projectile: None,
                });
            }
        }

        // Synchronise rtsim NPC with entity data
        for (pos, rtsim_entity, agent) in (&positions, &rtsim_entities, (&mut agents).maybe()).join() {
            rtsim
                .state
                .data_mut()
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
