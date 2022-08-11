pub mod deplete_resources;

use tracing::info;
use rtsim2::RtState;

pub fn start_rules(rtstate: &mut RtState) {
    info!("Starting server rtsim rules...");
    rtstate.start_rule::<deplete_resources::DepleteResources>();
}
