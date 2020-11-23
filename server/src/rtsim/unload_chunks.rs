use super::*;
use common::{
    comp::Pos,
    event::{EventBus, ServerEvent},
    terrain::TerrainGrid,
};
use specs::{Entities, Read, ReadExpect, ReadStorage, System, WriteExpect};

pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
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
            _server_event_bus,
            mut rtsim,
            _terrain_grid,
            _entities,
            _rtsim_entities,
            _positions,
        ): Self::SystemData,
    ) {
        let chunks = std::mem::take(&mut rtsim.chunks.chunks_to_unload);

        for _chunk in chunks {
            // TODO
        }
    }
}
