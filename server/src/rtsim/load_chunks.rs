use super::*;
use common::{
    event::{EventBus, ServerEvent},
    vsystem::{Origin, Phase, VJob, VSystem},
};
use specs::{Read, WriteExpect};

#[derive(Default)]
pub struct Sys;
impl<'a> VSystem<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (Read<'a, EventBus<ServerEvent>>, WriteExpect<'a, RtSim>);

    const NAME: &'static str = "rtsim::load_chunks";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut VJob<Self>, (_server_event_bus, mut rtsim): Self::SystemData) {
        for _chunk in std::mem::take(&mut rtsim.chunks.chunks_to_load) {
            // TODO
        }
    }
}
