use crate::{sys, Server, StateExt};
use common::{
    character::CharacterId,
    comp::{
        self,
        aura::{Aura, AuraKind, AuraTarget},
        beam,
        buff::{BuffCategory, BuffData, BuffKind, BuffSource},
        group,
        inventory::loadout::Loadout,
        shockwave, Agent, Alignment, Body, Gravity, Health, HomeChunk, Inventory, Item, ItemDrop,
        LightEmitter, Object, Ori, Poise, Pos, Projectile, Scale, Stats, Vel, WaypointArea,
    },
    outcome::Outcome,
    rtsim::RtSimEntity,
    util::Dir,
};
use specs::{Builder, Entity as EcsEntity, WorldExt};
use std::time::Duration;
use vek::{Rgb, Vec3};

pub fn handle_initialize_character(
    server: &mut Server,
    entity: EcsEntity,
    character_id: CharacterId,
) {
    server.state.initialize_character_data(entity, character_id);
}

pub fn handle_loaded_character_data(
    server: &mut Server,
    entity: EcsEntity,
    loaded_components: (
        comp::Body,
        comp::Stats,
        comp::Inventory,
        Option<comp::Waypoint>,
    ),
) {
    server
        .state
        .update_character_data(entity, loaded_components);
    sys::subscription::initialize_region_subscription(server.state.ecs(), entity);
}

#[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
pub fn handle_create_npc(
    server: &mut Server,
    pos: Pos,
    stats: Stats,
    health: Health,
    poise: Poise,
    loadout: Loadout,
    body: Body,
    agent: impl Into<Option<Agent>>,
    alignment: Alignment,
    scale: Scale,
    drop_item: Option<Item>,
    home_chunk: Option<HomeChunk>,
    rtsim_entity: Option<RtSimEntity>,
) {
    let group = match alignment {
        Alignment::Wild => None,
        Alignment::Passive => None,
        Alignment::Enemy => Some(group::ENEMY),
        Alignment::Npc | Alignment::Tame => Some(group::NPC),
        // TODO: handle
        Alignment::Owned(_) => None,
    };

    let inventory = Inventory::new_with_loadout(loadout);

    let entity = server
        .state
        .create_npc(pos, stats, health, poise, inventory, body)
        .with(scale)
        .with(alignment);

    let entity = if let Some(group) = group {
        entity.with(group)
    } else {
        entity
    };

    let entity = if let Some(agent) = agent.into() {
        entity.with(agent)
    } else {
        entity
    };

    let entity = if let Some(drop_item) = drop_item {
        entity.with(ItemDrop(drop_item))
    } else {
        entity
    };

    let entity = if let Some(home_chunk) = home_chunk {
        entity.with(home_chunk)
    } else {
        entity
    };

    let entity = if let Some(rtsim_entity) = rtsim_entity {
        entity.with(rtsim_entity)
    } else {
        entity
    };

    entity.build();
}

#[allow(clippy::too_many_arguments)]
pub fn handle_shoot(
    server: &mut Server,
    entity: EcsEntity,
    dir: Dir,
    body: Body,
    light: Option<LightEmitter>,
    projectile: Projectile,
    gravity: Option<Gravity>,
    speed: f32,
    object: Option<Object>,
) {
    let state = server.state_mut();

    let mut pos = if let Some(pos) = state.ecs().read_storage::<Pos>().get(entity) {
        pos.0
    } else {
        return;
    };

    let vel = *dir * speed;

    // Add an outcome
    state
        .ecs()
        .write_resource::<Vec<Outcome>>()
        .push(Outcome::ProjectileShot { pos, body, vel });

    let eye_height = state
        .ecs()
        .read_storage::<comp::Body>()
        .get(entity)
        .map_or(0.0, |b| b.eye_height());

    pos.z += eye_height;

    let mut builder = state.create_projectile(Pos(pos), Vel(vel), body, projectile);
    if let Some(light) = light {
        builder = builder.with(light)
    }
    if let Some(gravity) = gravity {
        builder = builder.with(gravity)
    }
    if let Some(object) = object {
        builder = builder.with(object)
    }

    builder.build();
}

pub fn handle_shockwave(
    server: &mut Server,
    properties: shockwave::Properties,
    pos: Pos,
    ori: Ori,
) {
    let state = server.state_mut();
    state.create_shockwave(properties, pos, ori).build();
}

pub fn handle_beam(server: &mut Server, properties: beam::Properties, pos: Pos, ori: Ori) {
    let state = server.state_mut();
    let ecs = state.ecs();
    ecs.write_resource::<Vec<Outcome>>().push(Outcome::Beam {
        pos: pos.0,
        specifier: properties.specifier,
    });
    state.create_beam(properties, pos, ori).build();
}

pub fn handle_create_waypoint(server: &mut Server, pos: Vec3<f32>) {
    server
        .state
        .create_object(Pos(pos), comp::object::Body::CampfireLit)
        .with(LightEmitter {
            col: Rgb::new(1.0, 0.3, 0.1),
            strength: 5.0,
            flicker: 1.0,
            animated: true,
        })
        .with(WaypointArea::default())
        .with(comp::Mass(10_f32.powi(10)))
        .with(comp::Auras::new(vec![Aura::new(
            AuraKind::Buff {
                kind: BuffKind::CampfireHeal,
                data: BuffData::new(0.02, Some(Duration::from_secs(1))),
                category: BuffCategory::Natural,
                source: BuffSource::World,
            },
            5.0,
            None,
            AuraTarget::All,
        )]))
        .build();
}
