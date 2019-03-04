pub mod phys;
pub mod uid;

// External
use specs::World as EcsWorld;

pub fn register_local_components(ecs_world: &mut EcsWorld) {
    ecs_world.register::<uid::Uid>();

    ecs_world.register::<phys::Pos>();
    ecs_world.register::<phys::Vel>();
    ecs_world.register::<phys::Dir>();
}
