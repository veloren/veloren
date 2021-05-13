pub mod comp;
pub mod sys;

use crate::audio::sfx::SfxEventItem;
use common::{event::EventBus, slowjob::SlowJobPool};
use specs::{World, WorldExt};

pub fn init(world: &mut World) {
    world.register::<comp::HpFloaterList>();
    world.register::<comp::Interpolated>();

    {
        let pool = world.read_resource::<SlowJobPool>();
        pool.configure("IMAGE_PROCESSING", |_| 1);
        pool.configure("FIGURE_MESHING", |n| n / 2);
        pool.configure("TERRAIN_MESHING", |n| n / 2);
    }

    // Voxygen event buses
    world.insert(EventBus::<SfxEventItem>::default());
}
