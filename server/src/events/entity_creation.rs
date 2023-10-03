use crate::{
    client::Client, events::player::handle_exit_ingame, persistence::PersistedComponents,
    presence::RepositionOnChunkLoad, sys, CharacterUpdater, Server, StateExt,
};
use common::{
    character::CharacterId,
    comp::{
        self,
        aura::{Aura, AuraKind, AuraTarget},
        buff::{BuffCategory, BuffData, BuffKind, BuffSource},
        misc::PortalData,
        ship::figuredata::VOXEL_COLLIDER_MANIFEST,
        shockwave, Alignment, BehaviorCapability, Body, ItemDrops, LightEmitter, Object, Ori, Pos,
        Projectile, TradingBehavior, Vel, WaypointArea,
    },
    event::{EventBus, NpcBuilder, UpdateCharacterMetadata},
    mounting::{Mounting, Volume, VolumeMounting, VolumePos},
    outcome::Outcome,
    resources::{Secs, Time},
    rtsim::RtSimEntity,
    uid::{IdMaps, Uid},
    util::Dir,
    vol::IntoFullVolIterator,
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
        // so kick the client out of "ingame" without saving any data and abort
        // the character loading process.
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

    let result_msg = if let Err(err) = server
        .state
        .update_character_data(entity, loaded_components)
    {
        handle_exit_ingame(server, entity, false); // remove client from in-game state
        ServerGeneral::CharacterDataLoadResult(Err(err))
    } else {
        sys::subscription::initialize_region_subscription(server.state.ecs(), entity);
        // We notify the client with the metadata result from the operation.
        ServerGeneral::CharacterDataLoadResult(Ok(metadata))
    };
    server.notify_client(entity, result_msg);
}

pub fn handle_create_npc(
    server: &mut Server,
    pos: Pos,
    ori: Ori,
    mut npc: NpcBuilder,
    rider: Option<NpcBuilder>,
) -> EcsEntity {
    let entity = server
        .state
        .create_npc(
            pos,
            ori,
            npc.stats,
            npc.skill_set,
            npc.health,
            npc.poise,
            npc.inventory,
            npc.body,
        )
        .with(npc.scale);

    if let Some(agent) = &mut npc.agent {
        if let Alignment::Owned(_) = &npc.alignment {
            agent.behavior.allow(BehaviorCapability::TRADE);
            agent.behavior.trading_behavior = TradingBehavior::AcceptFood;
        }
    }

    let entity = entity.with(npc.alignment);

    let entity = if let Some(agent) = npc.agent {
        entity.with(agent)
    } else {
        entity
    };

    let entity = if let Some(drop_items) = npc.loot.to_items() {
        entity.with(ItemDrops(drop_items))
    } else {
        entity
    };

    let entity = if let Some(home_chunk) = npc.anchor {
        entity.with(home_chunk)
    } else {
        entity
    };

    // Rtsim entity added to IdMaps below.
    let entity = if let Some(rtsim_entity) = npc.rtsim_entity {
        entity.with(rtsim_entity).with(RepositionOnChunkLoad {
            needs_ground: false,
        })
    } else {
        entity
    };

    let entity = if let Some(projectile) = npc.projectile {
        entity.with(projectile)
    } else {
        entity
    };

    let new_entity = entity.build();

    if let Some(rtsim_entity) = npc.rtsim_entity {
        server
            .state()
            .ecs()
            .write_resource::<IdMaps>()
            .add_rtsim(rtsim_entity, new_entity);
    }

    // Add to group system if a pet
    if let comp::Alignment::Owned(owner_uid) = npc.alignment {
        let state = server.state();
        let uids = state.ecs().read_storage::<Uid>();
        let clients = state.ecs().read_storage::<Client>();
        let mut group_manager = state.ecs().write_resource::<comp::group::GroupManager>();
        if let Some(owner) = state.ecs().entity_from_uid(owner_uid) {
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
    } else if let Some(group) = match npc.alignment {
        Alignment::Wild => None,
        Alignment::Passive => None,
        Alignment::Enemy => Some(comp::group::ENEMY),
        Alignment::Npc | Alignment::Tame => Some(comp::group::NPC),
        comp::Alignment::Owned(_) => unreachable!(),
    } {
        let _ = server.state.ecs().write_storage().insert(new_entity, group);
    }

    if let Some(rider) = rider {
        let rider_entity = handle_create_npc(server, pos, Ori::default(), rider, None);
        let uids = server.state().ecs().read_storage::<Uid>();
        let link = Mounting {
            mount: *uids.get(new_entity).expect("We just created this entity"),
            rider: *uids.get(rider_entity).expect("We just created this entity"),
        };
        drop(uids);
        server
            .state
            .link(link)
            .expect("We just created these entities");
    }

    new_entity
}

pub fn handle_create_ship(
    server: &mut Server,
    pos: Pos,
    ori: Ori,
    ship: comp::ship::Body,
    rtsim_entity: Option<RtSimEntity>,
    driver: Option<NpcBuilder>,
    passengers: Vec<NpcBuilder>,
) {
    let collider = ship.make_collider();
    let voxel_colliders_manifest = VOXEL_COLLIDER_MANIFEST.read();

    // TODO: Find better solution for this, maybe something like a serverside block
    // of interests.
    let (mut steering, mut seats) = {
        let mut steering = Vec::new();
        let mut seats = Vec::new();

        for (pos, block) in collider
            .get_vol(&voxel_colliders_manifest)
            .iter()
            .flat_map(|voxel_collider| voxel_collider.volume().full_vol_iter())
        {
            match (block.is_controller(), block.is_mountable()) {
                (true, true) => steering.push((pos, *block)),
                (false, true) => seats.push((pos, *block)),
                _ => {},
            }
        }
        (steering.into_iter(), seats.into_iter())
    };

    let mut entity = server.state.create_ship(pos, ori, ship, |_| collider);
    /*
    if let Some(mut agent) = agent {
        let (kp, ki, kd) = pid_coefficients(&Body::Ship(ship));
        fn pure_z(sp: Vec3<f32>, pv: Vec3<f32>) -> f32 { (sp - pv).z }
        agent =
            agent.with_position_pid_controller(PidController::new(kp, ki, kd, pos.0, 0.0, pure_z));
        entity = entity.with(agent);
    }
    */
    if let Some(rtsim_vehicle) = rtsim_entity {
        entity = entity.with(rtsim_vehicle);
    }
    let entity = entity.build();

    if let Some(driver) = driver {
        let npc_entity = handle_create_npc(server, pos, ori, driver, None);

        let uids = server.state.ecs().read_storage::<Uid>();
        let (rider_uid, mount_uid) = uids
            .get(npc_entity)
            .copied()
            .zip(uids.get(entity).copied())
            .expect("Couldn't get Uid from newly created ship and npc");
        drop(uids);

        if let Some((steering_pos, steering_block)) = steering.next() {
            server
                .state
                .link(VolumeMounting {
                    pos: VolumePos {
                        kind: Volume::Entity(mount_uid),
                        pos: steering_pos,
                    },
                    block: steering_block,
                    rider: rider_uid,
                })
                .expect("Failed to link driver to ship");
        } else {
            server
                .state
                .link(Mounting {
                    mount: mount_uid,
                    rider: rider_uid,
                })
                .expect("Failed to link driver to ship");
        }
    }

    for passenger in passengers {
        let npc_entity = handle_create_npc(
            server,
            Pos(pos.0 + Vec3::unit_z() * 5.0),
            ori,
            passenger,
            None,
        );
        if let Some((rider_pos, rider_block)) = seats.next() {
            let uids = server.state.ecs().read_storage::<Uid>();
            let (rider_uid, mount_uid) = uids
                .get(npc_entity)
                .copied()
                .zip(uids.get(entity).copied())
                .expect("Couldn't get Uid from newly created ship and npc");
            drop(uids);

            server
                .state
                .link(VolumeMounting {
                    pos: VolumePos {
                        kind: Volume::Entity(mount_uid),
                        pos: rider_pos,
                    },
                    block: rider_block,
                    rider: rider_uid,
                })
                .expect("Failed to link passanger to ship");
        }
    }
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
                    data: BuffData::new(0.02, Some(Secs(1.0))),
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
                    data: BuffData::new(2.0, Some(Secs(10.0))),
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

pub fn handle_create_teleporter(server: &mut Server, pos: Vec3<f32>, portal: PortalData) {
    server
        .state
        .create_teleporter(comp::Pos(pos), portal)
        .build();
}
