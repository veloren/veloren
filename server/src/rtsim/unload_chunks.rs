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
        let chunks = std::mem::take(&mut rtsim.world.chunks_to_unload);

        for chunk in chunks {
            // TODO
        }
    }
}
