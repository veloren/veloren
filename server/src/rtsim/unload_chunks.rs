use super::*;
use common::{
    event::{EventBus, ServerEvent},
    terrain::TerrainGrid,
    comp::Pos,
};
use specs::{Join, Read, ReadStorage, System, Write, ReadExpect, WriteExpect, Entities};

pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, EventBus<ServerEvent>>,
        WriteExpect<'a, RtSim>,
        ReadExpect<'a, TerrainGrid>,
        Entities<'a>,
        ReadStorage<'a, RtSimEntity>,
        ReadStorage<'a, Pos>,
    );

    fn run(
        &mut self,
        (
            server_event_bus,
            mut rtsim,
            terrain_grid,
            entities,
            rtsim_entities,
            positions,
        ): Self::SystemData,
    ) {
        let chunks = std::mem::take(&mut rtsim.chunks_to_unload);

        for (entity, rtsim_entity, pos) in (
            &entities,
            &rtsim_entities,
            &positions,
        ).join() {
            let key = terrain_grid.pos_key(pos.0.map(|e| e.floor() as i32));

            if terrain_grid.get_key(key).is_some() {
                break;
            } else if chunks.contains(&key) {
                // Assimilate the entity back into the simulation
                rtsim.assimilate_entity(rtsim_entity.0);
            }

            rtsim.update_entity(rtsim_entity.0, pos.0);
        }

        for chunk in chunks {

        }
    }
}
