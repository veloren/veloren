use crate::{sys, Server, StateExt};
use common::{
    comp::{
        self, Agent, Alignment, Body, Gravity, LightEmitter, Loadout, Pos, Projectile, Scale,
        Stats, Vel, WaypointArea,
    },
    util::Dir,
};
use specs::{Builder, Entity as EcsEntity, WorldExt};
use vek::{Rgb, Vec3};

pub fn handle_create_character(
    server: &mut Server,
    entity: EcsEntity,
    character_id: i32,
    body: Body,
    main: Option<String>,
    stats: Stats,
) {
    let state = &mut server.state;
    let server_settings = &server.server_settings;

    state.create_player_character(entity, character_id, body, main, stats, server_settings);
    sys::subscription::initialize_region_subscription(state.ecs(), entity);
}

pub fn handle_create_npc(
    server: &mut Server,
    pos: Pos,
    stats: Stats,
    loadout: Loadout,
    body: Body,
    agent: Agent,
    alignment: Alignment,
    scale: Scale,
) {
    server
        .state
        .create_npc(pos, stats, loadout, body)
        .with(agent)
        .with(scale)
        .with(alignment)
        .build();
}

pub fn handle_shoot(
    server: &mut Server,
    entity: EcsEntity,
    dir: Dir,
    body: Body,
    light: Option<LightEmitter>,
    projectile: Projectile,
    gravity: Option<Gravity>,
) {
    let state = server.state_mut();

    let mut pos = state
        .ecs()
        .read_storage::<Pos>()
        .get(entity)
        .expect("Failed to fetch entity")
        .0;

    // TODO: Player height
    pos.z += 1.2;

    let mut builder = state.create_projectile(Pos(pos), Vel(*dir * 100.0), body, projectile);
    if let Some(light) = light {
        builder = builder.with(light)
    }
    if let Some(gravity) = gravity {
        builder = builder.with(gravity)
    }

    builder.build();
}

pub fn handle_create_waypoint(server: &mut Server, pos: Vec3<f32>) {
    server
        .state
        .create_object(Pos(pos), comp::object::Body::CampfireLit)
        .with(LightEmitter {
            col: Rgb::new(1.0, 0.65, 0.2),
            strength: 2.0,
            flicker: 1.0,
            animated: true,
        })
        .with(WaypointArea::default())
        .build();
}
