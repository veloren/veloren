use crate::{
    client::Client, events::player::handle_exit_ingame, persistence::PersistedComponents, sys,
    CharacterUpdater, Server, StateExt,
};
use common::{
    character::CharacterId,
    comp::{
        self,
        agent::pid_coefficients,
        aura::{Aura, AuraKind, AuraTarget},
        beam,
        buff::{BuffCategory, BuffData, BuffKind, BuffSource},
        shockwave, Agent, Alignment, Anchor, BehaviorCapability, Body, Health, Inventory, ItemDrop,
        LightEmitter, Object, Ori, PidController, Poise, Pos, Projectile, Scale, SkillSet, Stats,
        TradingBehavior, Vel, WaypointArea,
    },
    event::{EventBus, UpdateCharacterMetadata},
    lottery::LootSpec,
    outcome::Outcome,
    resources::{Secs, Time},
    rtsim::RtSimEntity,
    uid::Uid,
    util::Dir,
    ViewDistances,
};
use common_net::{msg::ServerGeneral, sync::WorldSyncExt};
use specs::{Builder, Entity as EcsEntity, WorldExt};
use vek::{Rgb, Vec3};

use super::group_manip::update_map_markers;

pub fn handle_initialize_character(
    server: &mut Server,
    entity: EcsEntity,
    character_id: CharacterId,
    requested_view_distances: ViewDistances,
) {
    let updater = server.state.ecs().fetch::<CharacterUpdater>();
    let pending_database_action = updater.has_pending_database_action(character_id);
    drop(updater);

    if !pending_database_action {
        let clamped_vds = requested_view_distances.clamp(server.settings().max_view_distance);
        server
            .state
            .initialize_character_data(entity, character_id, clamped_vds);
        // Correct client if its requested VD is too high.
        if requested_view_distances.terrain != clamped_vds.terrain {
            server.notify_client(entity, ServerGeneral::SetViewDistance(clamped_vds.terrain));
        }
    } else {
        // A character delete or update was somehow initiated after the login commenced,
        // so disconnect the client without saving any data and abort the login process.
        handle_exit_ingame(server, entity, true);
    }
}

pub fn handle_initialize_spectator(
    server: &mut Server,
    entity: EcsEntity,
    requested_view_distances: ViewDistances,
) {
    let clamped_vds = requested_view_distances.clamp(server.settings().max_view_distance);
    server.state.initialize_spectator_data(entity, clamped_vds);
    // Correct client if its requested VD is too high.
    if requested_view_distances.terrain != clamped_vds.terrain {
        server.notify_client(entity, ServerGeneral::SetViewDistance(clamped_vds.terrain));
    }
    sys::subscription::initialize_region_subscription(server.state.ecs(), entity);
}

pub fn handle_loaded_character_data(
    server: &mut Server,
    entity: EcsEntity,
    loaded_components: PersistedComponents,
    metadata: UpdateCharacterMetadata,
) {
    if let Some(marker) = loaded_components.map_marker {
        server.notify_client(
            entity,
            ServerGeneral::MapMarker(comp::MapMarkerUpdate::Owned(comp::MapMarkerChange::Update(
                marker.0,
            ))),
        );
    }
    server
        .state
        .update_character_data(entity, loaded_components);
    sys::subscription::initialize_region_subscription(server.state.ecs(), entity);
    // We notify the client with the metadata result from the operation.
    server.notify_client(entity, ServerGeneral::CharacterDataLoadResult(Ok(metadata)));
}

pub fn handle_create_npc(
    server: &mut Server,
    pos: Pos,
    stats: Stats,
    skill_set: SkillSet,
    health: Option<Health>,
    poise: Poise,
    inventory: Inventory,
    body: Body,
    agent: impl Into<Option<Agent>>,
    alignment: Alignment,
    scale: Scale,
    loot: LootSpec<String>,
    home_chunk: Option<Anchor>,
    rtsim_entity: Option<RtSimEntity>,
    projectile: Option<Projectile>,
) {
    let entity = server
        .state
        .create_npc(pos, stats, skill_set, health, poise, inventory, body)
        .with(scale);

    let mut agent = agent.into();
    if let Some(agent) = &mut agent {
        if let Alignment::Owned(_) = &alignment {
            agent.behavior.allow(BehaviorCapability::TRADE);
            agent.behavior.trading_behavior = TradingBehavior::AcceptFood;
        }
    }

    let entity = entity.with(alignment);

    let entity = if let Some(agent) = agent {
        entity.with(agent)
    } else {
        entity
    };

    let entity = if let Some(drop_item) = loot.to_item() {
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

    let entity = if let Some(projectile) = projectile {
        entity.with(projectile)
    } else {
        entity
    };

    let new_entity = entity.build();

    // Add to group system if a pet
    if let comp::Alignment::Owned(owner_uid) = alignment {
        let state = server.state();
        let clients = state.ecs().read_storage::<Client>();
        let uids = state.ecs().read_storage::<Uid>();
        let mut group_manager = state.ecs().write_resource::<comp::group::GroupManager>();
        if let Some(owner) = state.ecs().entity_from_uid(owner_uid.into()) {
            let map_markers = state.ecs().read_storage::<comp::MapMarker>();
            group_manager.new_pet(
                new_entity,
                owner,
                &mut state.ecs().write_storage(),
                &state.ecs().entities(),
                &state.ecs().read_storage(),
                &uids,
                &mut |entity, group_change| {
                    clients
                        .get(entity)
                        .and_then(|c| {
                            group_change
                                .try_map_ref(|e| uids.get(*e).copied())
                                .map(|g| (g, c))
                        })
                        .map(|(g, c)| {
                            // Might be unnecessary, but maybe pets can somehow have map
                            // markers in the future
                            update_map_markers(&map_markers, &uids, c, &group_change);
                            c.send_fallible(ServerGeneral::GroupUpdate(g));
                        });
                },
            );
        }
    } else if let Some(group) = match alignment {
        Alignment::Wild => None,
        Alignment::Passive => None,
        Alignment::Enemy => Some(comp::group::ENEMY),
        Alignment::Npc | Alignment::Tame => Some(comp::group::NPC),
        comp::Alignment::Owned(_) => unreachable!(),
    } {
        let _ = server.state.ecs().write_storage().insert(new_entity, group);
    }
}

pub fn handle_create_ship(
    server: &mut Server,
    pos: Pos,
    ship: comp::ship::Body,
    mountable: bool,
    agent: Option<Agent>,
    rtsim_entity: Option<RtSimEntity>,
) {
    let mut entity = server
        .state
        .create_ship(pos, ship, |ship| ship.make_collider(), mountable);
    if let Some(mut agent) = agent {
        let (kp, ki, kd) = pid_coefficients(&Body::Ship(ship));
        fn pure_z(sp: Vec3<f32>, pv: Vec3<f32>) -> f32 { (sp - pv).z }
        agent =
            agent.with_position_pid_controller(PidController::new(kp, ki, kd, pos.0, 0.0, pure_z));
        entity = entity.with(agent);
    }
    if let Some(rtsim_entity) = rtsim_entity {
        entity = entity.with(rtsim_entity);
    }
    entity.build();
}

pub fn handle_shoot(
    server: &mut Server,
    entity: EcsEntity,
    pos: Pos,
    dir: Dir,
    body: Body,
    light: Option<LightEmitter>,
    projectile: Projectile,
    speed: f32,
    object: Option<Object>,
) {
    let state = server.state_mut();

    let pos = pos.0;

    let vel = *dir * speed
        + state
            .ecs()
            .read_storage::<Vel>()
            .get(entity)
            .map_or(Vec3::zero(), |v| v.0);

    // Add an outcome
    state
        .ecs()
        .read_resource::<EventBus<Outcome>>()
        .emit_now(Outcome::ProjectileShot { pos, body, vel });

    let mut builder = state.create_projectile(Pos(pos), Vel(vel), body, projectile);
    if let Some(light) = light {
        builder = builder.with(light)
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
    ecs.read_resource::<EventBus<Outcome>>()
        .emit_now(Outcome::Beam {
            pos: pos.0,
            specifier: properties.specifier,
        });
    state.create_beam(properties, pos, ori).build();
}

pub fn handle_create_waypoint(server: &mut Server, pos: Vec3<f32>) {
    let time = server.state.get_time();
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
        .with(comp::Immovable)
        .with(comp::Auras::new(vec![
            Aura::new(
                AuraKind::Buff {
                    kind: BuffKind::CampfireHeal,
                    data: BuffData::new(0.02, Some(Secs(1.0)), None),
                    category: BuffCategory::Natural,
                    source: BuffSource::World,
                },
                5.0,
                None,
                AuraTarget::All,
                Time(time),
            ),
            Aura::new(
                AuraKind::Buff {
                    kind: BuffKind::Burning,
                    data: BuffData::new(2.0, Some(Secs(10.0)), None),
                    category: BuffCategory::Natural,
                    source: BuffSource::World,
                },
                0.7,
                None,
                AuraTarget::All,
                Time(time),
            ),
        ]))
        .build();
}
