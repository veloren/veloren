use crate::{
    client::Client, persistence::PersistedComponents, presence::Presence, settings::Settings,
    sys::sentinel::DeletedEntities, wiring, SpawnPoint,
};
use common::{
    character::CharacterId,
    combat,
    comp::{
        self,
        skills::{GeneralSkill, Skill},
        Group, Inventory,
    },
    effect::Effect,
    resources::TimeOfDay,
    slowjob::SlowJobPool,
    uid::{Uid, UidAllocator},
};
use common_net::{
    msg::{CharacterInfo, PlayerListUpdate, PresenceKind, ServerGeneral},
    sync::WorldSyncExt,
};
use common_state::State;
use rand::prelude::*;
use specs::{
    saveload::MarkerAllocator, Builder, Entity as EcsEntity, EntityBuilder as EcsEntityBuilder,
    Join, WorldExt,
};
use std::time::Duration;
use tracing::warn;
use vek::*;

pub trait StateExt {
    /// Updates a component associated with the entity based on the `Effect`
    fn apply_effect(&self, entity: EcsEntity, effect: Effect, source: Option<Uid>);
    /// Build a non-player character
    #[allow(clippy::too_many_arguments)]
    fn create_npc(
        &mut self,
        pos: comp::Pos,
        stats: comp::Stats,
        skill_set: comp::SkillSet,
        health: Option<comp::Health>,
        poise: comp::Poise,
        inventory: comp::Inventory,
        body: comp::Body,
    ) -> EcsEntityBuilder;
    /// Build a static object entity
    fn create_object(&mut self, pos: comp::Pos, object: comp::object::Body) -> EcsEntityBuilder;
    fn create_ship(
        &mut self,
        pos: comp::Pos,
        ship: comp::ship::Body,
        mountable: bool,
    ) -> EcsEntityBuilder;
    /// Build a projectile
    fn create_projectile(
        &mut self,
        pos: comp::Pos,
        vel: comp::Vel,
        body: comp::Body,
        projectile: comp::Projectile,
    ) -> EcsEntityBuilder;
    /// Build a shockwave entity
    fn create_shockwave(
        &mut self,
        properties: comp::shockwave::Properties,
        pos: comp::Pos,
        ori: comp::Ori,
    ) -> EcsEntityBuilder;
    /// Build a beam entity
    fn create_beam(
        &mut self,
        properties: comp::beam::Properties,
        pos: comp::Pos,
        ori: comp::Ori,
    ) -> EcsEntityBuilder;
    /// Creates a safezone
    fn create_safezone(&mut self, range: Option<f32>, pos: comp::Pos) -> EcsEntityBuilder;
    fn create_wiring(
        &mut self,
        pos: comp::Pos,
        object: comp::object::Body,
        wiring_element: wiring::WiringElement,
    ) -> EcsEntityBuilder;
    // NOTE: currently only used for testing
    /// Queues chunk generation in the view distance of the persister, this
    /// entity must be built before those chunks are received (the builder
    /// borrows the ecs world so that is kind of impossible in practice)
    fn create_persister(
        &mut self,
        pos: comp::Pos,
        view_distance: u32,
        world: &std::sync::Arc<world::World>,
        index: &world::IndexOwned,
    ) -> EcsEntityBuilder;
    /// Insert common/default components for a new character joining the server
    fn initialize_character_data(&mut self, entity: EcsEntity, character_id: CharacterId);
    /// Update the components associated with the entity's current character.
    /// Performed after loading component data from the database
    fn update_character_data(&mut self, entity: EcsEntity, components: PersistedComponents);
    /// Iterates over registered clients and send each `ServerMsg`
    fn send_chat(&self, msg: comp::UnresolvedChatMsg);
    fn notify_players(&self, msg: ServerGeneral);
    fn notify_in_game_clients(&self, msg: ServerGeneral);
    /// Delete an entity, recording the deletion in [`DeletedEntities`]
    fn delete_entity_recorded(
        &mut self,
        entity: EcsEntity,
    ) -> Result<(), specs::error::WrongGeneration>;
}

impl StateExt for State {
    fn apply_effect(&self, entity: EcsEntity, effects: Effect, source: Option<Uid>) {
        match effects {
            Effect::Health(change) => {
                self.ecs()
                    .write_storage::<comp::Health>()
                    .get_mut(entity)
                    .map(|mut health| health.change_by(change));
            },
            Effect::Damage(damage) => {
                let inventories = self.ecs().read_storage::<Inventory>();
                let stats = self.ecs().read_storage::<comp::Stats>();
                let change = damage.calculate_health_change(
                    combat::Damage::compute_damage_reduction(
                        inventories.get(entity),
                        stats.get(entity),
                        Some(damage.kind),
                    ),
                    source,
                    false,
                    0.0,
                    1.0,
                );
                self.ecs()
                    .write_storage::<comp::Health>()
                    .get_mut(entity)
                    .map(|mut health| health.change_by(change));
            },
            Effect::PoiseChange(poise_damage) => {
                let inventories = self.ecs().read_storage::<Inventory>();
                let change = poise_damage.modify_poise_damage(inventories.get(entity));
                // Check to make sure the entity is not already stunned
                if let Some(character_state) = self
                    .ecs()
                    .read_storage::<comp::CharacterState>()
                    .get(entity)
                {
                    if !character_state.is_stunned() {
                        self.ecs()
                            .write_storage::<comp::Poise>()
                            .get_mut(entity)
                            .map(|mut poise| poise.change_by(change, Vec3::zero()));
                    }
                }
            },
            Effect::Buff(buff) => {
                self.ecs()
                    .write_storage::<comp::Buffs>()
                    .get_mut(entity)
                    .map(|mut buffs| {
                        buffs.insert(comp::Buff::new(
                            buff.kind,
                            buff.data,
                            buff.cat_ids,
                            comp::BuffSource::Item,
                        ))
                    });
            },
        }
    }

    fn create_npc(
        &mut self,
        pos: comp::Pos,
        stats: comp::Stats,
        skill_set: comp::SkillSet,
        health: Option<comp::Health>,
        poise: comp::Poise,
        inventory: comp::Inventory,
        body: comp::Body,
    ) -> EcsEntityBuilder {
        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::Vel(Vec3::zero()))
            .with(
                comp::Ori::from_unnormalized_vec(Vec3::new(
                    thread_rng().gen_range(-1.0..1.0),
                    thread_rng().gen_range(-1.0..1.0),
                    0.0,
                ))
                .unwrap_or_default(),
            )
            .with(body.mass())
            .with(body.density())
            .with(match body {
                comp::Body::Ship(ship) => comp::Collider::Voxel {
                    id: ship.manifest_entry().to_string(),
                },
                _ => comp::Collider::Box {
                    radius: body.radius(),
                    z_min: 0.0,
                    z_max: body.height(),
                },
            })
            .with(comp::Controller::default())
            .with(body)
            .with(comp::Energy::new(
                body,
                skill_set
                    .skill_level(Skill::General(GeneralSkill::EnergyIncrease))
                    .unwrap_or(None)
                    .unwrap_or(0),
            ))
            .with(stats)
            .with(skill_set)
            .maybe_with(health)
            .with(poise)
            .with(comp::Alignment::Npc)
            .with(comp::CharacterState::default())
            .with(inventory)
            .with(comp::Buffs::default())
            .with(comp::Combo::default())
            .with(comp::Auras::default())
    }

    fn create_object(&mut self, pos: comp::Pos, object: comp::object::Body) -> EcsEntityBuilder {
        let body = comp::Body::Object(object);
        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::Vel(Vec3::zero()))
            .with(comp::Ori::default())
            .with(body.mass())
            .with(body.density())
            .with(comp::Collider::Box {
                radius: body.radius(),
                z_min: 0.0,
                z_max: body.height(),
            })
            .with(body)
    }

    fn create_ship(
        &mut self,
        pos: comp::Pos,
        ship: comp::ship::Body,
        mountable: bool,
    ) -> EcsEntityBuilder {
        let body = comp::Body::Ship(ship);
        let mut builder = self
            .ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::Vel(Vec3::zero()))
            .with(comp::Ori::default())
            .with(body.mass())
            .with(body.density())
            .with(comp::Collider::Voxel {
                id: ship.manifest_entry().to_string(),
            })
            .with(body)
            .with(comp::Scale(comp::ship::AIRSHIP_SCALE))
            .with(comp::Controller::default())
            .with(comp::inventory::Inventory::new_empty())
            .with(comp::CharacterState::default())
            // TODO: some of these are required in order for the character_behavior system to
            // recognize a possesed airship; that system should be refactored to use `.maybe()`
            .with(comp::Energy::new(ship.into(), 0))
            .with(comp::Stats::new("Airship".to_string()))
            .with(comp::SkillSet::default())
            .with(comp::Combo::default());

        if mountable {
            builder = builder.with(comp::MountState::Unmounted);
        }
        builder
    }

    fn create_projectile(
        &mut self,
        pos: comp::Pos,
        vel: comp::Vel,
        body: comp::Body,
        projectile: comp::Projectile,
    ) -> EcsEntityBuilder {
        let mut projectile_base = self
            .ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(vel)
            .with(comp::Ori::from_unnormalized_vec(vel.0).unwrap_or_default())
            .with(body.mass())
            .with(body.density());

        if projectile.is_sticky {
            projectile_base = projectile_base.with(comp::Sticky)
        }
        if projectile.is_point {
            projectile_base = projectile_base.with(comp::Collider::Point)
        } else {
            projectile_base = projectile_base.with(comp::Collider::Box {
                radius: body.radius(),
                z_min: 0.0,
                z_max: body.height(),
            })
        }

        projectile_base.with(projectile).with(body)
    }

    fn create_shockwave(
        &mut self,
        properties: comp::shockwave::Properties,
        pos: comp::Pos,
        ori: comp::Ori,
    ) -> EcsEntityBuilder {
        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(ori)
            .with(comp::Shockwave {
                properties,
                creation: None,
            })
            .with(comp::ShockwaveHitEntities {
                hit_entities: Vec::<Uid>::new(),
            })
    }

    fn create_beam(
        &mut self,
        properties: comp::beam::Properties,
        pos: comp::Pos,
        ori: comp::Ori,
    ) -> EcsEntityBuilder {
        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(ori)
            .with(comp::BeamSegment {
                properties,
                creation: None,
            })
    }

    fn create_safezone(&mut self, range: Option<f32>, pos: comp::Pos) -> EcsEntityBuilder {
        use comp::{
            aura::{Aura, AuraKind, AuraTarget, Auras},
            buff::{BuffCategory, BuffData, BuffKind, BuffSource},
        };
        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(Auras::new(vec![Aura::new(
                AuraKind::Buff {
                    kind: BuffKind::Invulnerability,
                    data: BuffData::new(1.0, Some(Duration::from_secs(1))),
                    category: BuffCategory::Natural,
                    source: BuffSource::World,
                },
                range.unwrap_or(100.0),
                None,
                AuraTarget::All,
            )]))
    }

    fn create_wiring(
        &mut self,
        pos: comp::Pos,
        object: comp::object::Body,
        wiring_element: wiring::WiringElement,
    ) -> EcsEntityBuilder {
        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::Vel(Vec3::zero()))
            .with(comp::Ori::default())
            .with(comp::Collider::Box {
                radius: comp::Body::Object(object).radius(),
                z_min: 0.0,
                z_max: comp::Body::Object(object).height()
            })
            .with(comp::Body::Object(object))
            .with(comp::Mass(10.0))
            // .with(comp::Sticky)
            .with(wiring_element)
            .with(comp::LightEmitter {
                col: Rgb::new(0.0, 0.0, 0.0),
                strength: 2.0,
                flicker: 1.0,
                animated: true,
            })
    }

    // NOTE: currently only used for testing
    /// Queues chunk generation in the view distance of the persister, this
    /// entity must be built before those chunks are received (the builder
    /// borrows the ecs world so that is kind of impossible in practice)
    fn create_persister(
        &mut self,
        pos: comp::Pos,
        view_distance: u32,
        world: &std::sync::Arc<world::World>,
        index: &world::IndexOwned,
    ) -> EcsEntityBuilder {
        use common::{terrain::TerrainChunkSize, vol::RectVolSize};
        use std::sync::Arc;
        // Request chunks
        {
            let ecs = self.ecs();
            let slow_jobs = ecs.write_resource::<SlowJobPool>();
            let mut chunk_generator =
                ecs.write_resource::<crate::chunk_generator::ChunkGenerator>();
            let chunk_pos = self.terrain().pos_key(pos.0.map(|e| e as i32));
            (-(view_distance as i32)..view_distance as i32 + 1)
            .flat_map(|x| {
                (-(view_distance as i32)..view_distance as i32 + 1).map(move |y| Vec2::new(x, y))
            })
            .map(|offset| offset + chunk_pos)
            // Filter chunks outside the view distance
            // Note: calculation from client chunk request filtering
            .filter(|chunk_key| {
                pos.0.xy().map(|e| e as f64).distance(
                    chunk_key.map(|e| e as f64 + 0.5) * TerrainChunkSize::RECT_SIZE.map(|e| e as f64),
                ) < (view_distance as f64 - 1.0 + 2.5 * 2.0_f64.sqrt())
                    * TerrainChunkSize::RECT_SIZE.x as f64
            })
            .for_each(|chunk_key| {
                #[cfg(feature = "worldgen")]
                {
                    chunk_generator.generate_chunk(None, chunk_key, &slow_jobs, Arc::clone(world), index.clone(), *ecs.read_resource::<TimeOfDay>());
                }
            });
        }

        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(Presence::new(view_distance, PresenceKind::Spectator))
    }

    fn initialize_character_data(&mut self, entity: EcsEntity, character_id: CharacterId) {
        let spawn_point = self.ecs().read_resource::<SpawnPoint>().0;

        if let Some(player_uid) = self.read_component_copied::<Uid>(entity) {
            // NOTE: By fetching the player_uid, we validated that the entity exists, and we
            // call nothing that can delete it in any of the subsequent
            // commands, so we can assume that all of these calls succeed,
            // justifying ignoring the result of insertion.
            self.write_component_ignore_entity_dead(entity, comp::Controller::default());
            self.write_component_ignore_entity_dead(entity, comp::Pos(spawn_point));
            self.write_component_ignore_entity_dead(entity, comp::Vel(Vec3::zero()));
            self.write_component_ignore_entity_dead(entity, comp::Ori::default());
            self.write_component_ignore_entity_dead(entity, comp::Collider::Box {
                radius: 0.4,
                z_min: 0.0,
                z_max: 1.75,
            });
            self.write_component_ignore_entity_dead(entity, comp::CharacterState::default());
            self.write_component_ignore_entity_dead(entity, comp::Alignment::Owned(player_uid));
            self.write_component_ignore_entity_dead(entity, comp::Buffs::default());
            self.write_component_ignore_entity_dead(entity, comp::Auras::default());
            self.write_component_ignore_entity_dead(entity, comp::Combo::default());

            // Make sure physics components are updated
            self.write_component_ignore_entity_dead(entity, comp::ForceUpdate);

            const INITIAL_VD: u32 = 5; //will be changed after login
            self.write_component_ignore_entity_dead(
                entity,
                Presence::new(INITIAL_VD, PresenceKind::Character(character_id)),
            );

            // Tell the client its request was successful.
            if let Some(client) = self.ecs().read_storage::<Client>().get(entity) {
                client.send_fallible(ServerGeneral::CharacterSuccess);
            }
        }
    }

    fn update_character_data(&mut self, entity: EcsEntity, components: PersistedComponents) {
        let (body, stats, skill_set, inventory, waypoint) = components;

        if let Some(player_uid) = self.read_component_copied::<Uid>(entity) {
            // Notify clients of a player list update
            self.notify_players(ServerGeneral::PlayerListUpdate(
                PlayerListUpdate::SelectedCharacter(player_uid, CharacterInfo {
                    name: String::from(&stats.name),
                }),
            ));

            // NOTE: By fetching the player_uid, we validated that the entity exists, and we
            // call nothing that can delete it in any of the subsequent
            // commands, so we can assume that all of these calls succeed,
            // justifying ignoring the result of insertion.
            self.write_component_ignore_entity_dead(entity, comp::Collider::Box {
                radius: body.radius(),
                z_min: 0.0,
                z_max: body.height(),
            });
            self.write_component_ignore_entity_dead(entity, body);
            self.write_component_ignore_entity_dead(entity, body.mass());
            self.write_component_ignore_entity_dead(entity, body.density());
            let (health_level, energy_level) = (
                skill_set
                    .skill_level(Skill::General(GeneralSkill::HealthIncrease))
                    .unwrap_or(None)
                    .unwrap_or(0),
                skill_set
                    .skill_level(Skill::General(GeneralSkill::EnergyIncrease))
                    .unwrap_or(None)
                    .unwrap_or(0),
            );
            self.write_component_ignore_entity_dead(entity, comp::Health::new(body, health_level));
            self.write_component_ignore_entity_dead(entity, comp::Energy::new(body, energy_level));
            self.write_component_ignore_entity_dead(entity, comp::Poise::new(body));
            self.write_component_ignore_entity_dead(entity, stats);
            self.write_component_ignore_entity_dead(entity, skill_set);
            self.write_component_ignore_entity_dead(entity, inventory);
            self.write_component_ignore_entity_dead(
                entity,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::default()),
            );

            if let Some(waypoint) = waypoint {
                self.write_component_ignore_entity_dead(entity, waypoint);
                self.write_component_ignore_entity_dead(entity, comp::Pos(waypoint.get_pos()));
                self.write_component_ignore_entity_dead(entity, comp::Vel(Vec3::zero()));
                self.write_component_ignore_entity_dead(entity, comp::ForceUpdate);
            }
        }
    }

    /// Send the chat message to the proper players. Say and region are limited
    /// by location. Faction and group are limited by component.
    fn send_chat(&self, msg: comp::UnresolvedChatMsg) {
        let ecs = self.ecs();
        let is_within =
            |target, a: &comp::Pos, b: &comp::Pos| a.0.distance_squared(b.0) < target * target;

        let group_manager = ecs.read_resource::<comp::group::GroupManager>();

        let group_info = msg
            .get_group()
            .map(|g| group_manager.group_info(*g))
            .flatten();

        let resolved_msg = msg
            .clone()
            .map_group(|_| group_info.map_or_else(|| "???".to_string(), |i| i.name.clone()));

        match &msg.chat_type {
            comp::ChatType::Offline(_)
            | comp::ChatType::CommandInfo
            | comp::ChatType::CommandError
            | comp::ChatType::Meta
            | comp::ChatType::World(_) => self.notify_players(ServerGeneral::ChatMsg(resolved_msg)),
            comp::ChatType::Online(u) => {
                for (client, uid) in
                    (&ecs.read_storage::<Client>(), &ecs.read_storage::<Uid>()).join()
                {
                    if uid != u {
                        client.send_fallible(ServerGeneral::ChatMsg(resolved_msg.clone()));
                    }
                }
            },
            comp::ChatType::Tell(u, t) => {
                for (client, uid) in
                    (&ecs.read_storage::<Client>(), &ecs.read_storage::<Uid>()).join()
                {
                    if uid == u || uid == t {
                        client.send_fallible(ServerGeneral::ChatMsg(resolved_msg.clone()));
                    }
                }
            },
            comp::ChatType::Kill(kill_source, uid) => {
                let comp::chat::GenericChatMsg { message, .. } = msg;
                let clients = ecs.read_storage::<Client>();
                let clients_count = clients.count();
                // Avoid chat spam, send kill message only to group or nearby players if a
                // certain amount of clients are online
                if clients_count
                    > ecs
                        .fetch::<Settings>()
                        .max_player_for_kill_broadcast
                        .unwrap_or_default()
                {
                    // Send kill message to the dead player's group
                    let killed_entity =
                        (*ecs.read_resource::<UidAllocator>()).retrieve_entity_internal(uid.0);
                    let groups = ecs.read_storage::<Group>();
                    let killed_group = killed_entity.and_then(|e| groups.get(e));
                    if let Some(g) = &killed_group {
                        send_to_group(g, ecs, &resolved_msg);
                    }

                    // Send kill message to nearby players that aren't part of the deceased's group
                    let positions = ecs.read_storage::<comp::Pos>();
                    if let Some(died_player_pos) = killed_entity.and_then(|e| positions.get(e)) {
                        for (ent, client, pos) in (&*ecs.entities(), &clients, &positions).join() {
                            let client_group = groups.get(ent);
                            let is_different_group =
                                !(killed_group == client_group && client_group.is_some());
                            if is_within(comp::ChatMsg::SAY_DISTANCE, pos, died_player_pos)
                                && is_different_group
                            {
                                client.send_fallible(ServerGeneral::ChatMsg(resolved_msg.clone()));
                            }
                        }
                    }
                } else {
                    self.notify_players(ServerGeneral::server_msg(
                        comp::ChatType::Kill(kill_source.clone(), *uid),
                        message,
                    ))
                }
            },
            comp::ChatType::Say(uid) => {
                let entity_opt =
                    (*ecs.read_resource::<UidAllocator>()).retrieve_entity_internal(uid.0);
                let positions = ecs.read_storage::<comp::Pos>();
                if let Some(speaker_pos) = entity_opt.and_then(|e| positions.get(e)) {
                    for (client, pos) in (&ecs.read_storage::<Client>(), &positions).join() {
                        if is_within(comp::ChatMsg::SAY_DISTANCE, pos, speaker_pos) {
                            client.send_fallible(ServerGeneral::ChatMsg(resolved_msg.clone()));
                        }
                    }
                }
            },
            comp::ChatType::Region(uid) => {
                let entity_opt =
                    (*ecs.read_resource::<UidAllocator>()).retrieve_entity_internal(uid.0);
                let positions = ecs.read_storage::<comp::Pos>();
                if let Some(speaker_pos) = entity_opt.and_then(|e| positions.get(e)) {
                    for (client, pos) in (&ecs.read_storage::<Client>(), &positions).join() {
                        if is_within(comp::ChatMsg::REGION_DISTANCE, pos, speaker_pos) {
                            client.send_fallible(ServerGeneral::ChatMsg(resolved_msg.clone()));
                        }
                    }
                }
            },
            comp::ChatType::Npc(uid, _r) => {
                let entity_opt =
                    (*ecs.read_resource::<UidAllocator>()).retrieve_entity_internal(uid.0);
                let positions = ecs.read_storage::<comp::Pos>();
                if let Some(speaker_pos) = entity_opt.and_then(|e| positions.get(e)) {
                    for (client, pos) in (&ecs.read_storage::<Client>(), &positions).join() {
                        if is_within(comp::ChatMsg::NPC_DISTANCE, pos, speaker_pos) {
                            client.send_fallible(ServerGeneral::ChatMsg(resolved_msg.clone()));
                        }
                    }
                }
            },
            comp::ChatType::NpcSay(uid, _r) => {
                let entity_opt =
                    (*ecs.read_resource::<UidAllocator>()).retrieve_entity_internal(uid.0);
                let positions = ecs.read_storage::<comp::Pos>();
                if let Some(speaker_pos) = entity_opt.and_then(|e| positions.get(e)) {
                    for (client, pos) in (&ecs.read_storage::<Client>(), &positions).join() {
                        if is_within(comp::ChatMsg::NPC_SAY_DISTANCE, pos, speaker_pos) {
                            client.send_fallible(ServerGeneral::ChatMsg(resolved_msg.clone()));
                        }
                    }
                }
            },
            comp::ChatType::NpcTell(from, to, _r) => {
                for (client, uid) in
                    (&ecs.read_storage::<Client>(), &ecs.read_storage::<Uid>()).join()
                {
                    if uid == from || uid == to {
                        client.send_fallible(ServerGeneral::ChatMsg(resolved_msg.clone()));
                    }
                }
            },
            comp::ChatType::FactionMeta(s) | comp::ChatType::Faction(_, s) => {
                for (client, faction) in (
                    &ecs.read_storage::<Client>(),
                    &ecs.read_storage::<comp::Faction>(),
                )
                    .join()
                {
                    if s == &faction.0 {
                        client.send_fallible(ServerGeneral::ChatMsg(resolved_msg.clone()));
                    }
                }
            },
            comp::ChatType::Group(from, g) => {
                if group_info.is_none() {
                    // group not found, reply with command error
                    let reply = comp::ChatMsg {
                        chat_type: comp::ChatType::CommandError,
                        message: "You are using group chat but do not belong to a group. Use \
                                  /world or /region to change chat."
                            .into(),
                    };

                    if let Some((client, _)) =
                        (&ecs.read_storage::<Client>(), &ecs.read_storage::<Uid>())
                            .join()
                            .find(|(_, uid)| *uid == from)
                    {
                        client.send_fallible(ServerGeneral::ChatMsg(reply));
                    }
                    return;
                }
                send_to_group(g, ecs, &resolved_msg);
            },
            comp::ChatType::GroupMeta(g) => {
                send_to_group(g, ecs, &resolved_msg);
            },
        }
    }

    /// Sends the message to all connected clients
    fn notify_players(&self, msg: ServerGeneral) {
        let mut msg = Some(msg);
        let mut lazy_msg = None;
        for (client, _) in (
            &self.ecs().read_storage::<Client>(),
            &self.ecs().read_storage::<comp::Player>(),
        )
            .join()
        {
            if let Some(msg) = msg.take() {
                lazy_msg = Some(client.prepare(msg));
            }
            lazy_msg.as_ref().map(|ref msg| client.send_prepared(&msg));
        }
    }

    /// Sends the message to all clients playing in game
    fn notify_in_game_clients(&self, msg: ServerGeneral) {
        let mut msg = Some(msg);
        let mut lazy_msg = None;
        for (client, _) in (
            &mut self.ecs().write_storage::<Client>(),
            &self.ecs().read_storage::<Presence>(),
        )
            .join()
        {
            if let Some(msg) = msg.take() {
                lazy_msg = Some(client.prepare(msg));
            }
            lazy_msg.as_ref().map(|ref msg| client.send_prepared(&msg));
        }
    }

    fn delete_entity_recorded(
        &mut self,
        entity: EcsEntity,
    ) -> Result<(), specs::error::WrongGeneration> {
        // Remove entity from a group if they are in one
        {
            let clients = self.ecs().read_storage::<Client>();
            let uids = self.ecs().read_storage::<Uid>();
            let mut group_manager = self.ecs().write_resource::<comp::group::GroupManager>();
            group_manager.entity_deleted(
                entity,
                &mut self.ecs().write_storage(),
                &self.ecs().read_storage(),
                &uids,
                &self.ecs().entities(),
                &mut |entity, group_change| {
                    clients
                        .get(entity)
                        .and_then(|c| {
                            group_change
                                .try_map(|e| uids.get(e).copied())
                                .map(|g| (g, c))
                        })
                        .map(|(g, c)| c.send(ServerGeneral::GroupUpdate(g)));
                },
            );
        }

        let (maybe_uid, maybe_pos) = (
            self.ecs().read_storage::<Uid>().get(entity).copied(),
            self.ecs().read_storage::<comp::Pos>().get(entity).copied(),
        );
        let res = self.ecs_mut().delete_entity(entity);
        if res.is_ok() {
            if let (Some(uid), Some(pos)) = (maybe_uid, maybe_pos) {
                if let Some(region_key) = self
                    .ecs()
                    .read_resource::<common::region::RegionMap>()
                    .find_region(entity, pos.0)
                {
                    self.ecs()
                        .write_resource::<DeletedEntities>()
                        .record_deleted_entity(uid, region_key);
                } else {
                    // Don't panic if the entity wasn't found in a region maybe it was just created
                    // and then deleted before the region manager had a chance to assign it a
                    // region
                    warn!(
                        ?uid,
                        ?pos,
                        "Failed to find region containing entity during entity deletion, assuming \
                         it wasn't sent to any clients and so deletion doesn't need to be \
                         recorded for sync purposes"
                    );
                }
            }
        }
        res
    }
}

fn send_to_group(g: &comp::Group, ecs: &specs::World, msg: &comp::ChatMsg) {
    for (client, group) in (
        &ecs.read_storage::<Client>(),
        &ecs.read_storage::<comp::Group>(),
    )
        .join()
    {
        if g == group {
            client.send_fallible(ServerGeneral::ChatMsg(msg.clone()));
        }
    }
}
