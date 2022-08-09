use common::resources::Time;

pub trait Event: Clone + 'static {}

#[derive(Clone)]
pub struct OnTick { pub dt: f32 }

impl Event for OnTick {}
