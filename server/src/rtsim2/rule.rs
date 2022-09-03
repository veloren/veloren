pub mod deplete_resources;

use rtsim2::RtState;
use tracing::info;

pub fn start_rules(rtstate: &mut RtState) {
    info!("Starting server rtsim rules...");
    rtstate.start_rule::<deplete_resources::DepleteResources>();
}
