pub mod phys;
pub mod uid;
pub mod util;

// Reexports
pub use uid::{Uid, UidAllocator};

use specs::World as EcsWorld;

pub fn register_local_components(ecs_world: &mut EcsWorld) {
    ecs_world.register::<Uid>();
    ecs_world.add_resource(UidAllocator::new());

    ecs_world.register::<util::New>();

    ecs_world.register::<phys::Pos>();
    ecs_world.register::<phys::Vel>();
    ecs_world.register::<phys::Dir>();
    ecs_world.register::<phys::UpdateKind>();
}
