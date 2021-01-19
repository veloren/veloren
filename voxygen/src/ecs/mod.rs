pub mod comp;
pub mod sys;

use crate::audio::sfx::SfxEventItem;
use common::{event::EventBus, outcome::Outcome};
use specs::{Entity, World, WorldExt};

#[derive(Copy, Clone, Debug)]
pub struct MyEntity(pub Entity);

pub fn init(world: &mut World) {
    world.register::<comp::HpFloaterList>();
    world.register::<comp::Interpolated>();
    world.insert(Vec::<Outcome>::new());

    // Voxygen event buses
    world.insert(EventBus::<SfxEventItem>::default());
}
