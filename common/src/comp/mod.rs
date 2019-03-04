pub mod phys;
pub mod uid;

// Reexports
pub use uid::{Uid, UidAllocator};

use specs::World as EcsWorld;

pub fn register_local_components(ecs_world: &mut EcsWorld) {
    ecs_world.register::<Uid>();

    ecs_world.register::<phys::Pos>();
    ecs_world.register::<phys::Vel>();
    ecs_world.register::<phys::Dir>();
}
