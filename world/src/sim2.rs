use crate::{Index, sim::WorldSim, site::economy::simulate_economy};
use common_base::prof_span;

pub fn simulate(index: &mut Index, _world: &mut WorldSim) {
    prof_span!("sim2::simulate");
    simulate_economy(index);
}
