pub mod comp;
pub mod sys;

use crate::audio::sfx::SfxEventItem;
use common::event::EventBus;
use specs::{Entity, World, WorldExt};

#[derive(Copy, Clone, Debug)]
pub struct MyEntity(pub Entity);

#[derive(Copy, Clone, Debug)]
pub struct ExpFloater {
    pub exp_change: i32, // Maybe you can loose exp :p
    pub timer: f32,
    // Used to randomly offset position
    pub rand: (f32, f32),
}
#[derive(Clone, Debug, Default)]
pub struct MyExpFloaterList {
    pub floaters: Vec<ExpFloater>,
    pub last_exp: u32,
    pub last_level: u32,
    pub last_exp_max: u32,
}

pub fn init(world: &mut World) {
    world.register::<comp::HpFloaterList>();
    world.register::<comp::Interpolated>();
    world.insert(MyExpFloaterList::default());

    // Voxygen event buses
    world.insert(EventBus::<SfxEventItem>::default());
}
