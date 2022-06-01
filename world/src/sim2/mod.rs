use crate::{sim::WorldSim, site::economy::simulate_economy, Index};

pub fn simulate(index: &mut Index, _world: &mut WorldSim) { simulate_economy(index); }
