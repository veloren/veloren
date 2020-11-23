use super::*;
use common::event::{EventBus, ServerEvent};
use specs::{Read, System, WriteExpect};

pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (Read<'a, EventBus<ServerEvent>>, WriteExpect<'a, RtSim>);

    fn run(&mut self, (_server_event_bus, mut rtsim): Self::SystemData) {
        for _chunk in std::mem::take(&mut rtsim.chunks.chunks_to_load) {
            // TODO
        }
    }
}
