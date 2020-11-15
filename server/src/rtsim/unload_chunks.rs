use super::*;
use common::{
    comp::Pos,
    event::{EventBus, ServerEvent},
    terrain::TerrainGrid,
};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, Write, WriteExpect};

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
        let chunks = std::mem::take(&mut rtsim.chunks.chunks_to_unload);

        for chunk in chunks {
            // TODO
        }
    }
}
