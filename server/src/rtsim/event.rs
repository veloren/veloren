use common_state::BlockDiff;
use rtsim::Event;

#[derive(Clone)]
pub struct OnBlockChange {
    pub changes: Vec<BlockDiff>,
}

impl Event for OnBlockChange {
    type SystemData<'a> = ();
}
