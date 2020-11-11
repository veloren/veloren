use super::*;
use common::event::{EventBus, ServerEvent};
use specs::{Join, Read, ReadStorage, System, Write, WriteExpect};

pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, EventBus<ServerEvent>>,
        WriteExpect<'a, RtSim>,
    );

    fn run(&mut self, (server_event_bus, mut rtsim): Self::SystemData) {
        for chunk in std::mem::take(&mut rtsim.world.chunks_to_load) {
            // TODO
        }
    }
}
