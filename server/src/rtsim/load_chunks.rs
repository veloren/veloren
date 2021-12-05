use super::*;
use common::event::{EventBus, ServerEvent};
use common_ecs::{Job, Origin, Phase, System};
use specs::{Read, WriteExpect};

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (Read<'a, EventBus<ServerEvent>>, WriteExpect<'a, RtSim>);

    const NAME: &'static str = "rtsim::load_chunks";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, (_server_event_bus, mut rtsim): Self::SystemData) {
        for _chunk in std::mem::take(&mut rtsim.chunks.chunks_to_load) {
            // TODO
        }
    }
}
