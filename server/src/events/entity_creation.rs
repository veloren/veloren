use crate::{
    CharacterUpdater, Server, StateExt, client::Client, events::player::handle_exit_ingame,
    persistence::PersistedComponents, pet::tame_pet, presence::RepositionOnChunkLoad, sys,
};
use common::{
    CachedSpatialGrid,
    combat::AttackTarget,
    comp::{
        self, Alignment, BehaviorCapability, Body, Inventory, ItemDrops, LightEmitter, Object, Ori,
        Pos, ThrownItem, TradingBehavior, Vel, WaypointArea,
        aura::{Aura, AuraKind, AuraTarget},
        body,
        buff::{BuffCategory, BuffData, BuffKind, BuffSource},
        item::MaterialStatManifest,
        ship::figuredata::VOXEL_COLLIDER_MANIFEST,
        tool::AbilityMap,
    },
    consts::MAX_CAMPFIRE_RANGE,
    event::{
        CreateAuraEntityEvent, CreateItemDropEvent, CreateNpcEvent, CreateObjectEvent,
        CreateShipEvent, CreateSpecialEntityEvent, EventBus, InitializeCharacterEvent,
        InitializeSpectatorEvent, NpcBuilder, ShockwaveEvent, ShootEvent, SummonBeamPillarsEvent,
        ThrowEvent, UpdateCharacterDataEvent,
    },
    generation::SpecialEntity,
    mounting::{Mounting, Volume, VolumeMounting, VolumePos},
    outcome::Outcome,
    resources::{Secs, Time},
    terrain::TerrainGrid,
    uid::{IdMaps, Uid},
    util::Dir,
    vol::IntoFullVolIterator,
};
use common_net::{msg::ServerGeneral, sync::WorldSyncExt};
use specs::{Builder, Entity as EcsEntity, WorldExt};
use std::time::Duration;
use vek::{Rgb, Vec3};

use super::group_manip::update_map_markers;

pub fn handle_initialize_character(server: &mut Server, ev: InitializeCharacterEvent) {
    let updater = server.state.ecs().fetch::<CharacterUpdater>();
    let pending_database_action = updater.has_pending_database_action(ev.character_id);
    drop(updater);

    if !pending_database_action {
        let clamped_vds = ev
            .requested_view_distances
            .clamp(server.settings().max_view_distance);
        server
            .state
            .initialize_character_data(ev.entity, ev.character_id, clamped_vds);
        // Correct client if its requested VD is too high.
        if ev.requested_view_distances.terrain != clamped_vds.terrain {
            server.notify_client(
                ev.entity,
                ServerGeneral::SetViewDistance(clamped_vds.terrain),
            );
        }
    } else {
        // A character delete or update was somehow initiated after the login commenced,
        // so kick the client out of "ingame" without saving any data and abort
        // the character loading process.
        handle_exit_ingame(server, ev.entity, true);
    }
}

pub fn handle_initialize_spectator(server: &mut Server, ev: InitializeSpectatorEvent) {
    let clamped_vds = ev.1.clamp(server.settings().max_view_distance);
    server.state.initialize_spectator_data(ev.0, clamped_vds);
    // Correct client if its requested VD is too high.
    if ev.1.terrain != clamped_vds.terrain {
        server.notify_client(ev.0, ServerGeneral::SetViewDistance(clamped_vds.terrain));
    }
    sys::subscription::initialize_region_subscription(server.state.ecs(), ev.0);
}

pub fn handle_loaded_character_data(server: &mut Server, ev: UpdateCharacterDataEvent) {
    let loaded_components = PersistedComponents {
        body: ev.components.0,
        hardcore: ev.components.1,
        stats: ev.components.2,
        skill_set: ev.components.3,
        inventory: ev.components.4,
        waypoint: ev.components.5,
        pets: ev.components.6,
        active_abilities: ev.components.7,
        map_marker: ev.components.8,
    };
    if let Some(marker) = loaded_components.map_marker {
        server.notify_client(
            ev.entity,
            ServerGeneral::MapMarker(comp::MapMarkerUpdate::Owned(comp::MapMarkerChange::Update(
                marker.0,
            ))),
        );
    }

    let result_msg = if let Err(err) = server
        .state
        .update_character_data(ev.entity, loaded_components)
    {
        handle_exit_ingame(server, ev.entity, false); // remove client from in-game state
        ServerGeneral::CharacterDataLoadResult(Err(err))
    } else {
        sys::subscription::initialize_region_subscription(server.state.ecs(), ev.entity);
        // We notify the client with the metadata result from the operation.
        ServerGeneral::CharacterDataLoadResult(Ok(ev.metadata))
    };
    server.notify_client(ev.entity, result_msg);
}

pub fn handle_create_npc(server: &mut Server, ev: CreateNpcEvent) -> EcsEntity {
    // Destruct the builder to ensure all fields are exhaustive
    let NpcBuilder {
        stats,
        skill_set,
        health,
        poise,
        inventory,
        body,
        mut agent,
        alignment,
        scale,
        anchor,
        loot,
        pets,
        rtsim_entity,
        projectile,
        heads,
        death_effects,
        rider_effects,
        rider,
    } = ev.npc;
    let entity = server
      .state
      .create_npc(
            ev.pos, ev.ori, stats, skill_set, health, poise, inventory, body, scale,
        )
      .maybe_with(heads)
      .maybe_with(death_effects)
      .maybe_with(rider_effects);

    if let Some(agent) = &mut agent
        && let Alignment::Owned(_) = &alignment
    {
        agent.behavior.allow(BehaviorCapability::TRADE);
        agent.behavior.trading_behavior = TradingBehavior::AcceptFood;
    }

    let entity = entity.with(alignment);

    let entity = if let Some(agent) = agent {
        entity.with(agent)
    } else {
        entity
    };

    let entity = if let Some(drop_items) = loot.to_items() {
        entity.with(ItemDrops(drop_items))
    } else {
        entity
    };

    let entity = if let Some(home_chunk) = anchor {
        entity.with(home_chunk)
    } else {
        entity
    };

    // Rtsim entity added to IdMaps below.
    let entity = if let Some(rtsim_entity) = rtsim_entity {
        entity.with(rtsim_entity).with(RepositionOnChunkLoad {
            needs_ground: false,
        })
    } else {
        entity
    };

    let entity = if let Some(projectile) = projectile {
        entity.with(projectile)
    } else {
        entity
    };

    let new_entity = entity.build();

    if let Some(rtsim_entity) = rtsim_entity {
        server
          .state()
          .ecs()
          .write_resource::<IdMaps>()
          .add_rtsim(rtsim_entity, new_entity);
    }

    // Add to group system if a pet
    if let comp::Alignment::Owned(owner_uid) = alignment {
        // --- START: CUSTOM LOGIC FOR EPHEMERAL MOUNT ---
        if let Some(owner_entity) = server.state.ecs().entity_from_uid(owner_uid) {
            let ecs = server.state.ecs();
            let inventories = ecs.read_storage::<comp::Inventory>();
            let is_our_summon = if let Some(inventory) = inventories.get(owner_entity) {
                // Check if the summoner is holding the flute
                let is_holding_flute = inventory
                  .equipped(comp::inventory::slot::EquipSlot::ActiveMainhand)
                  .map_or(false, |item| {
                        item.ability_spec() == Some(&comp::inventory::item::tool::AbilitySpec::Custom("SummonEphemeralAntelope".to_string()))
                    });

                // Check if the NPC being created is an Antelope
                let is_antelope = matches!(body, comp::Body::QuadrupedMedium(
                    comp::quadruped_medium::Body {
                        species: comp::quadruped_medium::Species::Antelope,
                      ..
                    }
                ));

                is_holding_flute && is_antelope
            } else {
                false
            };

            if is_our_summon {
                // 1. TAG IT
                // Note: You will need to add `EphemeralMount` to the world's components
                // and potentially add `WriteStorage<EphemeralMount>` to the system's data dependencies.
                let mut ephemeral_mounts = ecs.write_storage::<comp::EphemeralMount>();
                ephemeral_mounts.insert(new_entity, comp::EphemeralMount).unwrap();

                // 2. MOUNT IT (from `/mount` command logic)
                let uids = ecs.read_storage::<Uid>();
                if let Some(mount_uid) = uids.get(new_entity) {
                    server.state.link(common::mounting::Mounting { mount: *mount_uid, rider: owner_uid }).unwrap();
                }
            }
        }
        // --- END: CUSTOM LOGIC ---

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
    } else if let Some(group) = alignment.group() {
        let _ = server.state.ecs().write_storage().insert(new_entity, group);
    }

    if let Some(rider) = rider {
        let rider_entity = handle_create_npc(server, CreateNpcEvent {
            pos: ev.pos,
            ori: Ori::default(),
            npc: *rider,
        });
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

    for (pet, offset) in pets {
        let pet_entity = handle_create_npc(server, CreateNpcEvent {
            pos: comp::Pos(ev.pos.0 + offset),
            ori: Ori::from_unnormalized_vec(offset).unwrap_or_default(),
            npc: pet,
        });

        tame_pet(server.state.ecs(), pet_entity, new_entity);
    }

    new_entity
}

pub fn handle_create_ship(server: &mut Server, ev: CreateShipEvent) {
    let collider = ev.ship.make_collider();
    let voxel_colliders_manifest = VOXEL_COLLIDER_MANIFEST.read();

    // TODO: Find better solution for this, maybe something like a serverside block
    // of interests.
    let (mut steering, mut _seats) = {
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

    let mut entity = server
        .state
        .create_ship(ev.pos, ev.ori, ev.ship, |_| collider);
    /*
    if let Some(mut agent) = agent {
        let (kp, ki, kd) = pid_coefficients(&Body::Ship(ship));
        fn pure_z(sp: Vec3<f32>, pv: Vec3<f32>) -> f32 { (sp - pv).z }
        agent =
            agent.with_position_pid_controller(PidController::new(kp, ki, kd, pos.0, 0.0, pure_z));
        entity = entity.with(agent);
    }
    */
    if let Some(rtsim_vehicle) = ev.rtsim_entity {
        entity = entity.with(rtsim_vehicle);
    }
    let entity = entity.build();

    if let Some(rtsim_entity) = ev.rtsim_entity {
        server
            .state()
            .ecs()
            .write_resource::<IdMaps>()
            .add_rtsim(rtsim_entity, entity);
    }

    if let Some(driver) = ev.driver {
        let npc_entity = handle_create_npc(server, CreateNpcEvent {
            pos: ev.pos,
            ori: ev.ori,
            npc: driver,
        });

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

    /*
    for passenger in ev.passengers {
        let npc_entity = handle_create_npc(server, CreateNpcEvent {
            pos: Pos(ev.pos.0 + Vec3::unit_z() * 5.0),
            ori: ev.ori,
            npc: passenger,
            rider: None,
        });
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
    */
}

pub fn handle_shoot(server: &mut Server, ev: ShootEvent) {
    let state = server.state_mut();

    let pos = ev.pos.0;

    let vel = *ev.dir * ev.speed
        + ev.entity
            .and_then(|entity| state.ecs().read_storage::<Vel>().get(entity).map(|v| v.0))
            .unwrap_or(Vec3::zero());

    // Add an outcome
    state
        .ecs()
        .read_resource::<EventBus<Outcome>>()
        .emit_now(Outcome::ProjectileShot {
            pos,
            body: ev.body,
            vel,
        });

    state
        .create_projectile(Pos(pos), Vel(vel), ev.body, ev.projectile)
        .maybe_with(ev.light)
        .maybe_with(ev.object)
        .build();
}

pub fn handle_throw(server: &mut Server, ev: ThrowEvent) {
    let state = server.state_mut();

    let thrown_item = state
        .ecs()
        .write_storage::<Inventory>()
        .get_mut(ev.entity)
        .and_then(|mut inv| {
            if let Some(thrown_item) = inv.equipped(ev.equip_slot) {
                let ability_map = state.ecs().read_resource::<AbilityMap>();
                let msm = state.ecs().read_resource::<MaterialStatManifest>();
                let time = state.ecs().read_resource::<Time>();

                // If stackable, try to remove the throwable from inv stacks before
                // removing the equipped one to avoid having to reequip after each throw
                if let Some(inv_slot) = inv.get_slot_of_item(thrown_item)
                    && thrown_item.is_stackable()
                {
                    inv.take(inv_slot, &ability_map, &msm)
                } else {
                    inv.replace_loadout_item(ev.equip_slot, None, *time)
                }
            } else {
                None
            }
        })
        .map(|mut thrown_item| {
            thrown_item.put_in_world();
            ThrownItem(thrown_item)
        });

    if let Some(thrown_item) = thrown_item {
        let body = Body::Item(body::item::Body::from(&thrown_item));

        let pos = ev.pos.0;

        let vel = *ev.dir * ev.speed
            + state
                .ecs()
                .read_storage::<Vel>()
                .get(ev.entity)
                .map_or(Vec3::zero(), |v| v.0);

        // Add an outcome
        state
            .ecs()
            .read_resource::<EventBus<Outcome>>()
            .emit_now(Outcome::ProjectileShot { pos, body, vel });

        state
            .create_projectile(Pos(pos), Vel(vel), body, ev.projectile)
            .with(thrown_item)
            .maybe_with(ev.light)
            .maybe_with(ev.object)
            .build();
    }
}

pub fn handle_shockwave(server: &mut Server, ev: ShockwaveEvent) {
    let state = server.state_mut();
    state
        .create_shockwave(ev.properties, ev.pos, ev.ori)
        .build();
}

pub fn handle_create_special_entity(server: &mut Server, ev: CreateSpecialEntityEvent) {
    let time = server.state.get_time();

    match ev.entity {
        SpecialEntity::Waypoint => {
            server
                .state
                .create_object(Pos(ev.pos), comp::object::Body::CampfireLit)
                .with(LightEmitter {
                    col: Rgb::new(1.0, 0.3, 0.1),
                    strength: 5.0,
                    flicker: 1.0,
                    animated: true,
                })
                .with(WaypointArea::default())
                .with(comp::Immovable)
                .with(comp::EnteredAuras::default())
                .with(comp::Auras::new(vec![
                    Aura::new(
                        AuraKind::Buff {
                            kind: BuffKind::RestingHeal,
                            data: BuffData::new(0.02, Some(Secs(1.0))),
                            category: BuffCategory::Natural,
                            source: BuffSource::World,
                        },
                        MAX_CAMPFIRE_RANGE,
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
        },
        SpecialEntity::Teleporter(portal) => {
            server
                .state
                .create_teleporter(comp::Pos(ev.pos), portal)
                .build();
        },
        SpecialEntity::ArenaTotem { range } => {
            server
                .state
                .create_object(Pos(ev.pos), comp::object::Body::GnarlingTotemGreen)
                .with(comp::Immovable)
                .with(comp::EnteredAuras::default())
                .with(comp::Auras::new(vec![
                    Aura::new(
                        AuraKind::FriendlyFire,
                        range,
                        None,
                        AuraTarget::All,
                        Time(time),
                    ),
                    Aura::new(AuraKind::ForcePvP, range, None, AuraTarget::All, Time(time)),
                ]))
                .build();
        },
    }
}

pub fn handle_create_item_drop(server: &mut Server, ev: CreateItemDropEvent) {
    server
        .state
        .create_item_drop(ev.pos, ev.ori, ev.vel, ev.item, ev.loot_owner);
}

pub fn handle_create_object(
    server: &mut Server,
    CreateObjectEvent {
        pos,
        vel,
        body,
        object,
        item,
        light_emitter,
        stats,
    }: CreateObjectEvent,
) {
    match object {
        Some(
            object @ Object::Crux {
                owner,
                scale,
                range,
                strength,
                duration,
                ..
            },
        ) => {
            let state = server.state_mut();
            let time = *state.ecs().read_resource::<Time>();

            // HACK: Spawn slightly damaged so that the health bar is visible and players
            // are aware it is a killable entity
            let mut health = comp::Health::new(Body::Object(body));
            health.set_fraction(0.99996);

            let crux = state
                .create_object(pos, body)
                .with(object)
                .maybe_with(light_emitter)
                .maybe_with(stats)
                .with(comp::Scale(scale))
                .with(health)
                .with(comp::Energy::new(Body::Object(body)))
                .with(comp::Poise::new(Body::Object(body)))
                .with(comp::SkillSet::default())
                .with(comp::Buffs::default())
                .with(comp::Inventory::with_empty())
                .with(comp::Immovable)
                .with(comp::Auras::new(vec![Aura::new(
                    AuraKind::Buff {
                        kind: BuffKind::Heatstroke,
                        data: BuffData {
                            strength,
                            duration: Some(duration),
                            delay: None,
                            secondary_duration: None,
                            misc_data: None,
                        },
                        category: BuffCategory::Magical,
                        source: BuffSource::World,
                    },
                    range,
                    None,
                    AuraTarget::NotGroupOf(owner),
                    time,
                )]))
                .build();

            if let Some(owner) = state.ecs().read_resource::<IdMaps>().uid_entity(owner) {
                let mut group_manager = state.ecs().write_resource::<comp::group::GroupManager>();
                group_manager.new_pet(
                    crux,
                    owner,
                    &mut state.ecs().write_storage(),
                    &state.ecs().entities(),
                    &state.ecs().read_storage(),
                    &state.ecs().read_storage::<Uid>(),
                    &mut |_, _| {},
                );
            }
        },
        _ => {
            server
                .state
                .create_object(pos, body)
                .with(vel)
                .maybe_with(object)
                .maybe_with(item)
                .maybe_with(light_emitter)
                .maybe_with(stats)
                .build();
        },
    }
}

pub fn handle_create_aura_entity(server: &mut Server, ev: CreateAuraEntityEvent) {
    let time = *server.state.ecs().read_resource::<Time>();
    let mut entity = server
        .state
        .ecs_mut()
        .create_entity_synced()
        .with(ev.pos)
        .with(comp::Vel(Vec3::zero()))
        .with(comp::Ori::default())
        .with(ev.auras)
        .with(comp::Alignment::Owned(ev.creator_uid));

    // If a duration is specified, create a projectile component for the entity
    if let Some(dur) = ev.duration {
        let object = comp::Object::DeleteAfter {
            spawned_at: time,
            timeout: Duration::from_secs_f64(dur.0),
        };
        entity = entity.with(object);
    }
    entity.build();
}

pub fn handle_summon_beam_pillars(server: &mut Server, ev: SummonBeamPillarsEvent) {
    let ecs = server.state().ecs();

    let Some((&Pos(center), &summoner_alignment)) = ecs
        .read_storage::<Pos>()
        .get(ev.summoner)
        .zip(ecs.read_storage::<Alignment>().get(ev.summoner))
    else {
        return;
    };

    let summon_pillar = |server: &mut Server, pos: Vec3<f32>, spawned_at| {
        let integer_pos = pos.map(|x| x as i32);
        let ground_height = server
            .state()
            .ecs()
            .read_resource::<TerrainGrid>()
            .find_ground(integer_pos)
            .z as f32;

        // If the distance from the attempted spawn position and the nearest valid
        // position is too far, avoid spawning the fire pillar to prevent
        // ability usage in a cave from spawning pillars on the surface or other
        // edge cases
        if (ground_height - pos.z).abs() <= 16.0 {
            let ecs = server.state_mut().ecs_mut();

            let pillar = ecs
                .create_entity_synced()
                .with(Pos(pos.with_z(ground_height)))
                .with(Ori::from(Dir::up()))
                .with(comp::Object::BeamPillar {
                    spawned_at,
                    buildup_duration: ev.buildup_duration,
                    attack_duration: ev.attack_duration,
                    beam_duration: ev.beam_duration,
                    radius: ev.radius,
                    height: ev.height,
                    damage: ev.damage,
                    damage_effect: ev.damage_effect,
                    dodgeable: ev.dodgeable,
                    tick_rate: ev.tick_rate,
                    specifier: ev.specifier,
                    indicator_specifier: ev.indicator_specifier,
                })
                .build();

            let mut group_manager = ecs.write_resource::<comp::group::GroupManager>();
            group_manager.new_pet(
                pillar,
                ev.summoner,
                &mut ecs.write_storage(),
                &ecs.entities(),
                &ecs.read_storage(),
                &ecs.read_storage::<Uid>(),
                &mut |_, _| {},
            );
        }
    };

    let spawned_at = *ecs.read_resource::<Time>();
    match ev.target {
        AttackTarget::AllInRange(range) => {
            let enemy_positions = ecs
                .read_resource::<CachedSpatialGrid>()
                .0
                .in_circle_aabr(center.xy(), range)
                .filter(|entity| {
                    ecs.read_storage::<Alignment>()
                        .get(*entity)
                        .is_some_and(|alignment| summoner_alignment.hostile_towards(*alignment))
                })
                .filter(|entity| {
                    ecs.read_storage::<comp::Group>()
                        .get(ev.summoner)
                        .is_none_or(|summoner_group| {
                            ecs.read_storage::<comp::Group>()
                                .get(*entity)
                                .is_none_or(|entity_group| summoner_group != entity_group)
                        })
                })
                .filter_map(|nearby_enemy| {
                    ecs.read_storage::<Pos>()
                        .get(nearby_enemy)
                        .map(|Pos(pos)| *pos)
                })
                .collect::<Vec<_>>();

            for enemy_pos in enemy_positions.into_iter() {
                summon_pillar(server, enemy_pos, spawned_at);
            }
        },
        AttackTarget::Pos(pos) => {
            summon_pillar(server, pos, spawned_at);
        },
        AttackTarget::Entity(entity) => {
            let pos = ecs.read_storage::<Pos>().get(entity).map(|pos| pos.0);
            if let Some(pos) = pos {
                summon_pillar(server, pos, spawned_at);
            }
        },
    }
}
