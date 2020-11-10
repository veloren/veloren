pub mod state;

use common::state::State;

pub fn init(state: &mut State, world: &world::World) {
    state
        .ecs_mut()
        .insert(state::SimState::new(world.sim().get_size()));
    tracing::info!("Initiated real-time world simulation");
}

pub fn tick(state: &mut State) {
    // TODO
}
