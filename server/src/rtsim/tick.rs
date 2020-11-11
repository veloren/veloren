use super::*;
use common::{
    event::{EventBus, ServerEvent},
    terrain::TerrainGrid,
    comp,
};
use specs::{Join, Read, ReadStorage, System, Write, WriteExpect, ReadExpect};
use rand_chacha::ChaChaRng;
use std::sync::Arc;

pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, EventBus<ServerEvent>>,
        WriteExpect<'a, RtSim>,
        ReadExpect<'a, TerrainGrid>,
        ReadExpect<'a, Arc<world::World>>,
    );

    fn run(
        &mut self,
        (
            server_event_bus,
            mut rtsim,
            terrain,
            world,
        ): Self::SystemData,
    ) {
        let rtsim = &mut *rtsim;

        // TODO: don't update all of them each tick
        let mut to_reify = Vec::new();
        for (id, entity) in rtsim.entities.iter_mut() {
            if entity.is_loaded {
                continue;
            } else if rtsim.world.chunk_at(entity.pos.xy()).map(|c| c.is_loaded).unwrap_or(false) {
                to_reify.push(id);
            }

            if let Some(chunk) = world.sim().get_wpos(entity.pos.xy().map(|e| e.floor() as i32)) {
                entity.pos.z = chunk.alt;
            }
        }

        let mut server_emitter = server_event_bus.emitter();
        for id in to_reify {
            rtsim.reify_entity(id);
            let entity = &rtsim.entities[id];
            let mut rng = ChaChaRng::from_seed([
                entity.seed.to_le_bytes()[0],
                entity.seed.to_le_bytes()[1],
                entity.seed.to_le_bytes()[2],
                entity.seed.to_le_bytes()[3],
                0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
            ]);
            let species = *(&comp::humanoid::ALL_SPECIES).choose(&mut rng).unwrap();
            let body = comp::humanoid::Body::random_with(&mut rng, &species).into();
            server_emitter.emit(ServerEvent::CreateNpc {
                pos: comp::Pos(terrain.find_space(entity.pos.map(|e| e.floor() as i32)).map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0)),
                stats: comp::Stats::new("Rtsim Entity".to_string(), body),
                health: comp::Health::new(body, 10),
                loadout: comp::Loadout::default(),
                body,
                agent: None,
                alignment: comp::Alignment::Npc,
                scale: comp::Scale(1.0),
                drop_item: None,
                home_chunk: None,
                rtsim_entity: Some(RtSimEntity(id)),
            });
        }
    }
}
