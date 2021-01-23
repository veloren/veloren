pub mod comp;
pub mod sys;

use crate::audio::sfx::SfxEventItem;
use common::event::EventBus;
use specs::{Entity, World, WorldExt};

#[derive(Copy, Clone, Debug)]
pub struct MyEntity(pub Entity);

pub fn init(world: &mut World) {
    world.register::<comp::HpFloaterList>();
    world.register::<comp::Interpolated>();

    // Voxygen event buses
    world.insert(EventBus::<SfxEventItem>::default());
}
