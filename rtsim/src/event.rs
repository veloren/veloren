use common::resources::Time;
use world::{World, IndexRef};
use super::{Rule, RtState};

pub trait Event: Clone + 'static {}

pub struct EventCtx<'a, R: Rule, E: Event> {
    pub state: &'a RtState,
    pub rule: &'a mut R,
    pub event: &'a E,
    pub world: &'a World,
    pub index: IndexRef<'a>,
}

#[derive(Clone)]
pub struct OnSetup;
impl Event for OnSetup {}

#[derive(Clone)]
pub struct OnTick {
    pub dt: f32,
    pub time: f64,
}
impl Event for OnTick {}
