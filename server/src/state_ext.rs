use crate::{
    automod::AutoMod,
    chat::ChatExporter,
    client::Client,
    events::{self, update_map_markers},
    persistence::PersistedComponents,
    pet::restore_pet,
    presence::RepositionOnChunkLoad,
    rtsim::RtSim,
    settings::Settings,
    sys::sentinel::DeletedEntities,
    wiring, BattleModeBuffer, SpawnPoint,
};
use common::{
    calendar::Calendar,
    character::CharacterId,
    combat,
    combat::DamageContributor,
    comp::{
        self,
        item::{ItemKind, MaterialStatManifest},
        misc::PortalData,
        object,
        skills::{GeneralSkill, Skill},
        ChatType, Content, Group, Inventory, Item, LootOwner, Object, Player, Poise, Presence,
        PresenceKind, BASE_ABILITY_LIMIT,
    },
    effect::Effect,
    link::{Is, Link, LinkHandle},
    mounting::{Mounting, Rider, VolumeMounting, VolumeRider},
    resources::{Secs, Time, TimeOfDay},
    rtsim::{Actor, RtSimEntity},
    slowjob::SlowJobPool,
    tether::Tethered,
    uid::{IdMaps, Uid},
    util::Dir,
    LoadoutBuilder, ViewDistances,
};
use common_net::{
    msg::{CharacterInfo, PlayerListUpdate, ServerGeneral},
    sync::WorldSyncExt,
};
use common_state::State;
use rand::prelude::*;
use specs::{
    storage::{GenericReadStorage, GenericWriteStorage},
    Builder, Entity as EcsEntity, EntityBuilder as EcsEntityBuilder, Join, WorldExt, WriteStorage,
};
use std::time::{Duration, Instant};
use tracing::{error, trace, warn};
use vek::*;

pub trait StateExt {
    /// Updates a component associated with the entity based on the `Effect`
    fn apply_effect(&self, entity: EcsEntity, effect: Effect, source: Option<Uid>);
    /// Build a non-player character
    fn create_npc(
        &mut self,
        pos: comp::Pos,
        ori: comp::Ori,
        stats: comp::Stats,
        skill_set: comp::SkillSet,
        health: Option<comp::Health>,
        poise: Poise,
        inventory: Inventory,
        body: comp::Body,
    ) -> EcsEntityBuilder;
    /// Build a static object entity
    fn create_object(&mut self, pos: comp::Pos, object: comp::object::Body) -> EcsEntityBuilder;
    /// Create an item drop or merge the item with an existing drop, if a
    /// suitable candidate exists.
    fn create_item_drop(
        &mut self,
        pos: comp::Pos,
        ori: comp::Ori,
        vel: comp::Vel,
        item: Item,
        loot_owner: Option<LootOwner>,
    ) -> Option<EcsEntity>;
    fn create_ship<F: FnOnce(comp::ship::Body) -> comp::Collider>(
        &mut self,
        pos: comp::Pos,
        ori: comp::Ori,
        ship: comp::ship::Body,
        make_collider: F,
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
    /// Creates a teleporter entity, which allows players to teleport to the
    /// `target` position. You might want to require the teleporting entity
    /// to not have agro for teleporting.
    fn create_teleporter(&mut self, pos: comp::Pos, portal: PortalData) -> EcsEntityBuilder;
    /// Insert common/default components for a new character joining the server
    fn initialize_character_data(
        &mut self,
        entity: EcsEntity,
        character_id: CharacterId,
        view_distances: ViewDistances,
    );
    /// Insert common/default components for a new spectator joining the server
    fn initialize_spectator_data(&mut self, entity: EcsEntity, view_distances: ViewDistances);
    /// Update the components associated with the entity's current character.
    /// Performed after loading component data from the database
    fn update_character_data(
        &mut self,
        entity: EcsEntity,
        components: PersistedComponents,
    ) -> Result<(), String>;
    /// Iterates over registered clients and send each `ServerMsg`
    fn validate_chat_msg(
        &self,
        player: EcsEntity,
        chat_type: &comp::ChatType<comp::Group>,
        msg: &str,
    ) -> bool;
    fn send_chat(&self, msg: comp::UnresolvedChatMsg);
    fn notify_players(&self, msg: ServerGeneral);
    fn notify_in_game_clients(&self, msg: ServerGeneral);
    /// Create a new link between entities (see [`common::mounting`] for an
    /// example).
    fn link<L: Link>(&mut self, link: L) -> Result<(), L::Error>;
    /// Maintain active links between entities
    fn maintain_links(&mut self);
    /// Delete an entity, recording the deletion in [`DeletedEntities`]
    fn delete_entity_recorded(
        &mut self,
        entity: EcsEntity,
    ) -> Result<(), specs::error::WrongGeneration>;
    /// Get the given entity as an [`Actor`], if it is one.
    fn entity_as_actor(&self, entity: EcsEntity) -> Option<Actor>;
    /// Mutate the position of an entity or, if the entity is mounted, the
    /// mount.
    ///
    /// If `dismount_volume` is `true`, an entity mounted on a volume entity
    /// (such as an airship) will be dismounted to avoid teleporting the volume
    /// entity.
    fn position_mut<T>(
        &mut self,
        entity: EcsEntity,
        dismount_volume: bool,
        f: impl for<'a> FnOnce(&'a mut comp::Pos) -> T,
    ) -> Result<T, Content>;
}

impl StateExt for State {
    fn apply_effect(&self, entity: EcsEntity, effects: Effect, source: Option<Uid>) {
        let msm = self.ecs().read_resource::<MaterialStatManifest>();
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
                let groups = self.ecs().read_storage::<Group>();

                let damage_contributor = source.and_then(|uid| {
                    self.ecs().entity_from_uid(uid).map(|attacker_entity| {
                        DamageContributor::new(uid, groups.get(attacker_entity).cloned())
                    })
                });
                let time = self.ecs().read_resource::<Time>();
                let change = damage.calculate_health_change(
                    combat::Damage::compute_damage_reduction(
                        Some(damage),
                        inventories.get(entity),
                        stats.get(entity),
                        &msm,
                    ),
                    damage_contributor,
                    None,
                    0.0,
                    1.0,
                    *time,
                    random(),
                );
                self.ecs()
                    .write_storage::<comp::Health>()
                    .get_mut(entity)
                    .map(|mut health| health.change_by(change));
            },
            Effect::Poise(poise) => {
                let inventories = self.ecs().read_storage::<Inventory>();
                let char_states = self.ecs().read_storage::<comp::CharacterState>();
                let stats = self.ecs().read_storage::<comp::Stats>();

                let change = Poise::apply_poise_reduction(
                    poise,
                    inventories.get(entity),
                    &msm,
                    char_states.get(entity),
                    stats.get(entity),
                );
                // Check to make sure the entity is not already stunned
                if let Some(character_state) = self
                    .ecs()
                    .read_storage::<comp::CharacterState>()
                    .get(entity)
                {
                    if !character_state.is_stunned() {
                        let groups = self.ecs().read_storage::<Group>();
                        let damage_contributor = source.and_then(|uid| {
                            self.ecs().entity_from_uid(uid).map(|attacker_entity| {
                                DamageContributor::new(uid, groups.get(attacker_entity).cloned())
                            })
                        });
                        let time = self.ecs().read_resource::<Time>();
                        let poise_change = comp::PoiseChange {
                            amount: change,
                            impulse: Vec3::zero(),
                            cause: None,
                            by: damage_contributor,
                            time: *time,
                        };
                        self.ecs()
                            .write_storage::<Poise>()
                            .get_mut(entity)
                            .map(|mut poise| poise.change(poise_change));
                    }
                }
            },
            Effect::Buff(buff) => {
                let time = self.ecs().read_resource::<Time>();
                let stats = self.ecs().read_storage::<comp::Stats>();
                self.ecs()
                    .write_storage::<comp::Buffs>()
                    .get_mut(entity)
                    .map(|mut buffs| {
                        buffs.insert(
                            comp::Buff::new(
                                buff.kind,
                                buff.data,
                                buff.cat_ids,
                                comp::BuffSource::Item,
                                *time,
                                stats.get(entity),
                            ),
                            *time,
                        )
                    });
            },
        }
    }

    fn create_npc(
        &mut self,
        pos: comp::Pos,
        ori: comp::Ori,
        stats: comp::Stats,
        skill_set: comp::SkillSet,
        health: Option<comp::Health>,
        poise: Poise,
        inventory: Inventory,
        body: comp::Body,
    ) -> EcsEntityBuilder {
        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::Vel(Vec3::zero()))
            .with(ori)
            .with(body.mass())
            .with(body.density())
            .with(body.collider())
            .with(comp::Controller::default())
            .with(body)
            .with(comp::Energy::new(
                body,
                skill_set
                    .skill_level(Skill::General(GeneralSkill::EnergyIncrease))
                    .unwrap_or(0),
            ))
            .with(stats)
            .with(if body.is_humanoid() {
                comp::ActiveAbilities::default_limited(BASE_ABILITY_LIMIT)
            } else {
                comp::ActiveAbilities::default()
            })
            .with(skill_set)
            .maybe_with(health)
            .with(poise)
            .with(comp::Alignment::Npc)
            .with(comp::CharacterState::default())
            .with(comp::CharacterActivity::default())
            .with(inventory)
            .with(comp::Buffs::default())
            .with(comp::Combo::default())
            .with(comp::Auras::default())
            .with(comp::Stance::default())
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
            .with(body.collider())
            .with(body)
    }

    fn create_item_drop(
        &mut self,
        pos: comp::Pos,
        ori: comp::Ori,
        vel: comp::Vel,
        item: Item,
        loot_owner: Option<LootOwner>,
    ) -> Option<EcsEntity> {
        {
            const MAX_MERGE_DIST: f32 = 1.5;

            // First, try to identify possible candidates for item merging
            // We limit our search to just a few blocks and we prioritise merging with the
            // closest
            let positions = self.ecs().read_storage::<comp::Pos>();
            let loot_owners = self.ecs().read_storage::<LootOwner>();
            let mut items = self.ecs().write_storage::<Item>();
            let mut nearby_items = self
                .ecs()
                .read_resource::<common::CachedSpatialGrid>()
                .0
                .in_circle_aabr(pos.0.xy(), MAX_MERGE_DIST)
                .filter(|entity| items.contains(*entity))
                .filter_map(|entity| {
                    Some((entity, positions.get(entity)?.0.distance_squared(pos.0)))
                })
                .filter(|(_, dist_sqrd)| *dist_sqrd < MAX_MERGE_DIST.powi(2))
                .collect::<Vec<_>>();
            nearby_items.sort_by_key(|(_, dist_sqrd)| (dist_sqrd * 1000.0) as i32);
            for (nearby, _) in nearby_items {
                // Only merge if the loot owner is the same
                if loot_owners.get(nearby).map(|lo| lo.owner()) == loot_owner.map(|lo| lo.owner())
                    && items
                        .get(nearby)
                        .map_or(false, |nearby_item| nearby_item.can_merge(&item))
                {
                    // Merging can occur! Perform the merge:
                    items
                        .get_mut(nearby)
                        .expect("we know that the item exists")
                        .try_merge(item)
                        .expect("`try_merge` should succeed because `can_merge` returned `true`");
                    return None;
                }
            }
            // Only if merging items fails do we give up and create a new item
        }

        let spawned_at = *self.ecs().read_resource::<Time>();

        let item_drop = comp::item_drop::Body::from(&item);
        let body = comp::Body::ItemDrop(item_drop);
        let light_emitter = match &*item.kind() {
            ItemKind::Lantern(lantern) => Some(comp::LightEmitter {
                col: lantern.color(),
                strength: lantern.strength(),
                flicker: lantern.flicker(),
                animated: true,
            }),
            _ => None,
        };
        Some(
            self.ecs_mut()
                .create_entity_synced()
                .with(item)
                .with(pos)
                .with(ori)
                .with(vel)
                .with(item_drop.orientation(&mut thread_rng()))
                .with(item_drop.mass())
                .with(item_drop.density())
                .with(body.collider())
                .with(body)
                .with(Object::DeleteAfter {
                    spawned_at,
                    // Delete the item drop after 5 minutes
                    timeout: Duration::from_secs(300),
                })
                .maybe_with(loot_owner)
                .maybe_with(light_emitter)
                .build(),
        )
    }

    fn create_ship<F: FnOnce(comp::ship::Body) -> comp::Collider>(
        &mut self,
        pos: comp::Pos,
        ori: comp::Ori,
        ship: comp::ship::Body,
        make_collider: F,
    ) -> EcsEntityBuilder {
        let body = comp::Body::Ship(ship);
        let builder = self
            .ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::Vel(Vec3::zero()))
            .with(ori)
            .with(body.mass())
            .with(body.density())
            .with(make_collider(ship))
            .with(body)
            .with(comp::Controller::default())
            .with(Inventory::with_empty())
            .with(comp::CharacterState::default())
            .with(comp::CharacterActivity::default())
            // TODO: some of these are required in order for the character_behavior system to
            // recognize a possesed airship; that system should be refactored to use `.maybe()`
            .with(comp::Energy::new(ship.into(), 0))
            .with(comp::Stats::new("Airship".to_string(), body))
            .with(comp::SkillSet::default())
            .with(comp::ActiveAbilities::default())
            .with(comp::Combo::default());

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
            projectile_base = projectile_base.with(body.collider())
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

    fn create_safezone(&mut self, range: Option<f32>, pos: comp::Pos) -> EcsEntityBuilder {
        use comp::{
            aura::{Aura, AuraKind, AuraTarget, Auras},
            buff::{BuffCategory, BuffData, BuffKind, BuffSource},
        };
        let time = self.get_time();
        // TODO: Consider using the area system for this
        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(Auras::new(vec![Aura::new(
                AuraKind::Buff {
                    kind: BuffKind::Invulnerability,
                    data: BuffData::new(1.0, Some(Secs(1.0))),
                    category: BuffCategory::Natural,
                    source: BuffSource::World,
                },
                range.unwrap_or(100.0),
                None,
                AuraTarget::All,
                Time(time),
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
            .with({
                let body: comp::Body = object.into();
                body.collider()
            })
            .with(comp::Body::Object(object))
            .with(comp::Mass(100.0))
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
            #[cfg(feature = "worldgen")]
            let rtsim = ecs.read_resource::<RtSim>();
            #[cfg(not(feature = "worldgen"))]
            let rtsim = ();
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
                    let time = (*ecs.read_resource::<TimeOfDay>(), (*ecs.read_resource::<Calendar>()).clone());
                    chunk_generator.generate_chunk(None, chunk_key, &slow_jobs, Arc::clone(world), &rtsim, index.clone(), time);
                }
            });
        }

        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(Presence::new(
                ViewDistances {
                    terrain: view_distance,
                    entity: view_distance,
                },
                PresenceKind::Spectator,
            ))
    }

    fn create_teleporter(&mut self, pos: comp::Pos, portal: PortalData) -> EcsEntityBuilder {
        self.create_object(pos, object::Body::Portal)
            .with(comp::Immovable)
            .with(comp::Object::from(portal))
    }

    fn initialize_character_data(
        &mut self,
        entity: EcsEntity,
        character_id: CharacterId,
        view_distances: ViewDistances,
    ) {
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
            self.write_component_ignore_entity_dead(entity, comp::Collider::CapsulePrism {
                p0: Vec2::zero(),
                p1: Vec2::zero(),
                radius: 0.4,
                z_min: 0.0,
                z_max: 1.75,
            });
            self.write_component_ignore_entity_dead(entity, comp::CharacterState::default());
            self.write_component_ignore_entity_dead(entity, comp::CharacterActivity::default());
            self.write_component_ignore_entity_dead(entity, comp::Alignment::Owned(player_uid));
            self.write_component_ignore_entity_dead(entity, comp::Buffs::default());
            self.write_component_ignore_entity_dead(entity, comp::Auras::default());
            self.write_component_ignore_entity_dead(entity, comp::Combo::default());
            self.write_component_ignore_entity_dead(entity, comp::Stance::default());

            // Make sure physics components are updated
            self.write_component_ignore_entity_dead(entity, comp::ForceUpdate::forced());

            self.write_component_ignore_entity_dead(
                entity,
                Presence::new(view_distances, PresenceKind::LoadingCharacter(character_id)),
            );

            // Tell the client its request was successful.
            if let Some(client) = self.ecs().read_storage::<Client>().get(entity) {
                client.send_fallible(ServerGeneral::CharacterSuccess);
            }
        }
    }

    fn initialize_spectator_data(&mut self, entity: EcsEntity, view_distances: ViewDistances) {
        let spawn_point = self.ecs().read_resource::<SpawnPoint>().0;

        if self.read_component_copied::<Uid>(entity).is_some() {
            // NOTE: By fetching the player_uid, we validated that the entity exists, and we
            // call nothing that can delete it in any of the subsequent
            // commands, so we can assume that all of these calls succeed,
            // justifying ignoring the result of insertion.
            self.write_component_ignore_entity_dead(entity, comp::Pos(spawn_point));

            // Make sure physics components are updated
            self.write_component_ignore_entity_dead(entity, comp::ForceUpdate::forced());

            self.write_component_ignore_entity_dead(
                entity,
                Presence::new(view_distances, PresenceKind::Spectator),
            );

            // Tell the client its request was successful.
            if let Some(client) = self.ecs().read_storage::<Client>().get(entity) {
                client.send_fallible(ServerGeneral::SpectatorSuccess(spawn_point));
            }
        }
    }

    /// Returned error intended to be sent to the client.
    fn update_character_data(
        &mut self,
        entity: EcsEntity,
        components: PersistedComponents,
    ) -> Result<(), String> {
        let PersistedComponents {
            body,
            stats,
            skill_set,
            inventory,
            waypoint,
            pets,
            active_abilities,
            map_marker,
        } = components;

        if let Some(player_uid) = self.read_component_copied::<Uid>(entity) {
            let result =
                if let Some(presence) = self.ecs().write_storage::<Presence>().get_mut(entity) {
                    if let PresenceKind::LoadingCharacter(id) = presence.kind {
                        presence.kind = PresenceKind::Character(id);
                        self.ecs()
                            .write_resource::<IdMaps>()
                            .add_character(id, entity);
                        Ok(())
                    } else {
                        Err("PresenceKind is not LoadingCharacter")
                    }
                } else {
                    Err("Presence component missing")
                };
            if let Err(err) = result {
                let err = format!("Unexpected state when applying loaded character info: {err}");
                error!("{err}");
                // TODO: we could produce a `comp::Content` for this to allow localization.
                return Err(err);
            }

            // Notify clients of a player list update
            self.notify_players(ServerGeneral::PlayerListUpdate(
                PlayerListUpdate::SelectedCharacter(player_uid, CharacterInfo {
                    name: String::from(&stats.name),
                    // NOTE: hack, read docs on body::Gender for more
                    gender: stats.original_body.humanoid_gender(),
                }),
            ));

            // NOTE: By fetching the player_uid, we validated that the entity exists,
            // and we call nothing that can delete it in any of the subsequent
            // commands, so we can assume that all of these calls succeed,
            // justifying ignoring the result of insertion.
            self.write_component_ignore_entity_dead(entity, body.collider());
            self.write_component_ignore_entity_dead(entity, body);
            self.write_component_ignore_entity_dead(entity, body.mass());
            self.write_component_ignore_entity_dead(entity, body.density());
            let (health_level, energy_level) = (
                skill_set
                    .skill_level(Skill::General(GeneralSkill::HealthIncrease))
                    .unwrap_or(0),
                skill_set
                    .skill_level(Skill::General(GeneralSkill::EnergyIncrease))
                    .unwrap_or(0),
            );
            self.write_component_ignore_entity_dead(entity, comp::Health::new(body, health_level));
            self.write_component_ignore_entity_dead(entity, comp::Energy::new(body, energy_level));
            self.write_component_ignore_entity_dead(entity, Poise::new(body));
            self.write_component_ignore_entity_dead(entity, stats);
            self.write_component_ignore_entity_dead(entity, active_abilities);
            self.write_component_ignore_entity_dead(entity, skill_set);
            self.write_component_ignore_entity_dead(entity, inventory);
            self.write_component_ignore_entity_dead(
                entity,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::default()),
            );

            if let Some(waypoint) = waypoint {
                self.write_component_ignore_entity_dead(entity, RepositionOnChunkLoad {
                    needs_ground: true,
                });
                self.write_component_ignore_entity_dead(entity, waypoint);
                self.write_component_ignore_entity_dead(entity, comp::Pos(waypoint.get_pos()));
                self.write_component_ignore_entity_dead(entity, comp::Vel(Vec3::zero()));
                // TODO: We probably want to increment the existing force update counter since
                // it is added in initialized_character (to be robust we can also insert it if
                // it doesn't exist)
                self.write_component_ignore_entity_dead(entity, comp::ForceUpdate::forced());
            }

            if let Some(map_marker) = map_marker {
                self.write_component_ignore_entity_dead(entity, map_marker);
            }

            let player_pos = self.ecs().read_storage::<comp::Pos>().get(entity).copied();
            if let Some(player_pos) = player_pos {
                trace!(
                    "Loading {} pets for player at pos {:?}",
                    pets.len(),
                    player_pos
                );
                // This is the same as wild creatures naturally spawned in the world
                const DEFAULT_PET_HEALTH_LEVEL: u16 = 0;

                let mut rng = rand::thread_rng();

                for (pet, body, stats) in pets {
                    let ori = comp::Ori::from(Dir::random_2d(&mut rng));
                    let pet_entity = self
                        .create_npc(
                            player_pos,
                            ori,
                            stats,
                            comp::SkillSet::default(),
                            Some(comp::Health::new(body, DEFAULT_PET_HEALTH_LEVEL)),
                            Poise::new(body),
                            Inventory::with_loadout(
                                LoadoutBuilder::from_default(&body).build(),
                                body,
                            ),
                            body,
                        )
                        .with(comp::Scale(1.0))
                        .with(comp::Vel(Vec3::new(0.0, 0.0, 0.0)))
                        .build();

                    restore_pet(self.ecs(), pet_entity, entity, pet);
                }
            } else {
                error!("Player has no pos, cannot load {} pets", pets.len());
            }

            let presences = self.ecs().read_storage::<Presence>();
            let presence = presences.get(entity);
            if let Some(Presence {
                kind: PresenceKind::Character(char_id),
                ..
            }) = presence
            {
                let battlemode_buffer = self.ecs().fetch::<BattleModeBuffer>();
                let mut players = self.ecs().write_storage::<comp::Player>();
                if let Some((mode, change)) = battlemode_buffer.get(char_id) {
                    if let Some(mut player_info) = players.get_mut(entity) {
                        player_info.battle_mode = *mode;
                        player_info.last_battlemode_change = Some(*change);
                    }
                } else {
                    // TODO: this sounds related to handle_exit_ingame? Actually, sounds like
                    // trying to place character specific info on the `Player` component. TODO
                    // document component better.
                    // FIXME:
                    // ???
                    //
                    // This probably shouldn't exist,
                    // but without this code, character gets battle_mode from
                    // another character on this account.
                    let settings = self.ecs().read_resource::<Settings>();
                    let mode = settings.gameplay.battle_mode.default_mode();
                    if let Some(mut player_info) = players.get_mut(entity) {
                        player_info.battle_mode = mode;
                        player_info.last_battlemode_change = None;
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_chat_msg(
        &self,
        entity: EcsEntity,
        chat_type: &comp::ChatType<comp::Group>,
        msg: &str,
    ) -> bool {
        let mut automod = self.ecs().write_resource::<AutoMod>();
        let client = self.ecs().read_storage::<Client>();
        let player = self.ecs().read_storage::<Player>();
        let Some(client) = client.get(entity) else {
            return true;
        };
        let Some(player) = player.get(entity) else {
            return true;
        };

        match automod.validate_chat_msg(
            player.uuid(),
            self.ecs()
                .read_storage::<comp::Admin>()
                .get(entity)
                .map(|a| a.0),
            Instant::now(),
            chat_type,
            msg,
        ) {
            Ok(note) => {
                if let Some(note) = note {
                    let _ = client.send(ServerGeneral::server_msg(
                        ChatType::CommandInfo,
                        format!("{}", note),
                    ));
                }
                true
            },
            Err(err) => {
                let _ = client.send(ServerGeneral::server_msg(
                    ChatType::CommandError,
                    format!("{}", err),
                ));
                false
            },
        }
    }

    /// Send the chat message to the proper players. Say and region are limited
    /// by location. Faction and group are limited by component.
    fn send_chat(&self, msg: comp::UnresolvedChatMsg) {
        let ecs = self.ecs();
        let is_within =
            |target, a: &comp::Pos, b: &comp::Pos| a.0.distance_squared(b.0) < target * target;

        let group_manager = ecs.read_resource::<comp::group::GroupManager>();
        let chat_exporter = ecs.read_resource::<ChatExporter>();

        let group_info = msg.get_group().and_then(|g| group_manager.group_info(*g));

        if let Some(exported_message) = ChatExporter::generate(&msg, ecs) {
            chat_exporter.send(exported_message);
        }

        let resolved_msg = msg
            .clone()
            .map_group(|_| group_info.map_or_else(|| "???".to_string(), |i| i.name.clone()));

        let id_maps = ecs.read_resource::<IdMaps>();
        let entity_from_uid = |uid| id_maps.uid_entity(uid);

        if msg.chat_type.uid().map_or(true, |sender| {
            entity_from_uid(sender).map_or(false, |e| {
                self.validate_chat_msg(
                    e,
                    &msg.chat_type,
                    msg.content().as_plain().unwrap_or_default(),
                )
            })
        }) {
            match &msg.chat_type {
                comp::ChatType::Offline(_)
                | comp::ChatType::CommandInfo
                | comp::ChatType::CommandError
                | comp::ChatType::Meta
                | comp::ChatType::World(_) => {
                    self.notify_players(ServerGeneral::ChatMsg(resolved_msg))
                },
                comp::ChatType::Online(u) => {
                    for (client, uid) in
                        (&ecs.read_storage::<Client>(), &ecs.read_storage::<Uid>()).join()
                    {
                        if uid != u {
                            client.send_fallible(ServerGeneral::ChatMsg(resolved_msg.clone()));
                        }
                    }
                },
                comp::ChatType::Tell(from, to) => {
                    for (client, uid) in
                        (&ecs.read_storage::<Client>(), &ecs.read_storage::<Uid>()).join()
                    {
                        if uid == from || uid == to {
                            client.send_fallible(ServerGeneral::ChatMsg(resolved_msg.clone()));
                        }
                    }
                },
                comp::ChatType::Kill(kill_source, uid) => {
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
                        let killed_entity = entity_from_uid(*uid);
                        let groups = ecs.read_storage::<Group>();
                        let killed_group = killed_entity.and_then(|e| groups.get(e));
                        if let Some(g) = &killed_group {
                            send_to_group(g, ecs, &resolved_msg);
                        }

                        // Send kill message to nearby players that aren't part of the deceased's
                        // group
                        let positions = ecs.read_storage::<comp::Pos>();
                        if let Some(died_player_pos) = killed_entity.and_then(|e| positions.get(e))
                        {
                            for (ent, client, pos) in
                                (&*ecs.entities(), &clients, &positions).join()
                            {
                                let client_group = groups.get(ent);
                                let is_different_group =
                                    !(killed_group == client_group && client_group.is_some());
                                if is_within(comp::ChatMsg::SAY_DISTANCE, pos, died_player_pos)
                                    && is_different_group
                                {
                                    client.send_fallible(ServerGeneral::ChatMsg(
                                        resolved_msg.clone(),
                                    ));
                                }
                            }
                        }
                    } else {
                        self.notify_players(ServerGeneral::server_msg(
                            comp::ChatType::Kill(kill_source.clone(), *uid),
                            msg.into_content(),
                        ))
                    }
                },
                comp::ChatType::Say(uid) => {
                    let entity_opt = entity_from_uid(*uid);

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
                    let entity_opt = entity_from_uid(*uid);

                    let positions = ecs.read_storage::<comp::Pos>();
                    if let Some(speaker_pos) = entity_opt.and_then(|e| positions.get(e)) {
                        for (client, pos) in (&ecs.read_storage::<Client>(), &positions).join() {
                            if is_within(comp::ChatMsg::REGION_DISTANCE, pos, speaker_pos) {
                                client.send_fallible(ServerGeneral::ChatMsg(resolved_msg.clone()));
                            }
                        }
                    }
                },
                comp::ChatType::Npc(uid) => {
                    let entity_opt = entity_from_uid(*uid);

                    let positions = ecs.read_storage::<comp::Pos>();
                    if let Some(speaker_pos) = entity_opt.and_then(|e| positions.get(e)) {
                        for (client, pos) in (&ecs.read_storage::<Client>(), &positions).join() {
                            if is_within(comp::ChatMsg::NPC_DISTANCE, pos, speaker_pos) {
                                client.send_fallible(ServerGeneral::ChatMsg(resolved_msg.clone()));
                            }
                        }
                    }
                },
                comp::ChatType::NpcSay(uid) => {
                    let entity_opt = entity_from_uid(*uid);

                    let positions = ecs.read_storage::<comp::Pos>();
                    if let Some(speaker_pos) = entity_opt.and_then(|e| positions.get(e)) {
                        for (client, pos) in (&ecs.read_storage::<Client>(), &positions).join() {
                            if is_within(comp::ChatMsg::NPC_SAY_DISTANCE, pos, speaker_pos) {
                                client.send_fallible(ServerGeneral::ChatMsg(resolved_msg.clone()));
                            }
                        }
                    }
                },
                comp::ChatType::NpcTell(from, to) => {
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
                        // Group not found, reply with command error
                        // This should usually NEVER happen since now it is checked whether the
                        // sender is still in the group upon emitting the message (TODO: Can this be
                        // triggered if the message is sent in the same tick as the sender is
                        // removed from the group?)

                        let reply = comp::ChatType::CommandError
                            .into_msg(Content::localized("command-message-group-missing"));

                        let clients = ecs.read_storage::<Client>();
                        if let Some(client) =
                            entity_from_uid(*from).and_then(|entity| clients.get(entity))
                        {
                            client.send_fallible(ServerGeneral::ChatMsg(reply));
                        }
                    } else {
                        send_to_group(g, ecs, &resolved_msg);
                    }
                },
                comp::ChatType::GroupMeta(g) => {
                    send_to_group(g, ecs, &resolved_msg);
                },
            }
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
            lazy_msg.as_ref().map(|msg| client.send_prepared(msg));
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
            lazy_msg.as_ref().map(|msg| client.send_prepared(msg));
        }
    }

    fn link<L: Link>(&mut self, link: L) -> Result<(), L::Error> {
        let linker = LinkHandle::from_link(link);

        L::create(&linker, &mut self.ecs().system_data())?;

        self.ecs_mut()
            .entry::<Vec<LinkHandle<L>>>()
            .or_insert_with(Vec::new)
            .push(linker);

        Ok(())
    }

    fn maintain_links(&mut self) {
        fn maintain_link<L: Link>(state: &State) {
            if let Some(mut handles) = state.ecs().try_fetch_mut::<Vec<LinkHandle<L>>>() {
                let mut persist_data = None;
                handles.retain(|link| {
                    if L::persist(
                        link,
                        persist_data.get_or_insert_with(|| state.ecs().system_data()),
                    ) {
                        true
                    } else {
                        // Make sure to drop persist data before running deletion to avoid potential
                        // access violations
                        persist_data.take();
                        L::delete(link, &mut state.ecs().system_data());
                        false
                    }
                });
            }
        }

        maintain_link::<Mounting>(self);
        maintain_link::<VolumeMounting>(self);
        maintain_link::<Tethered>(self);
    }

    fn delete_entity_recorded(
        &mut self,
        entity: EcsEntity,
    ) -> Result<(), specs::error::WrongGeneration> {
        // NOTE: both this and handle_exit_ingame call delete_entity_common, so cleanup
        // added here may need to be duplicated in handle_exit_ingame (depending
        // on its nature).

        // Remove entity from a group if they are in one.
        {
            let clients = self.ecs().read_storage::<Client>();
            let uids = self.ecs().read_storage::<Uid>();
            let mut group_manager = self.ecs().write_resource::<comp::group::GroupManager>();
            let map_markers = self.ecs().read_storage::<comp::MapMarker>();
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
                                .try_map_ref(|e| uids.get(*e).copied())
                                .map(|g| (g, c))
                        })
                        .map(|(g, c)| {
                            update_map_markers(&map_markers, &uids, c, &group_change);
                            c.send_fallible(ServerGeneral::GroupUpdate(g));
                        });
                },
            );
        }

        // Cancel extant trades
        events::cancel_trades_for(self, entity);

        // NOTE: We expect that these 3 components are never removed from an entity (nor
        // mutated) (at least not without updating the relevant mappings)!
        let maybe_uid = self.read_component_copied::<Uid>(entity);
        let (maybe_character, sync_me) = self
            .read_storage::<Presence>()
            .get(entity)
            .map(|p| (p.kind.character_id(), p.kind.sync_me()))
            .unzip();
        let maybe_rtsim = self.read_component_copied::<RtSimEntity>(entity);

        self.mut_resource::<IdMaps>().remove_entity(
            Some(entity),
            maybe_uid,
            maybe_character.flatten(),
            maybe_rtsim,
        );

        delete_entity_common(self, entity, maybe_uid, sync_me.unwrap_or(true))
    }

    fn entity_as_actor(&self, entity: EcsEntity) -> Option<Actor> {
        if let Some(rtsim_entity) = self
            .ecs()
            .read_storage::<RtSimEntity>()
            .get(entity)
            .copied()
        {
            Some(Actor::Npc(rtsim_entity.0))
        } else if let Some(PresenceKind::Character(character)) = self
            .ecs()
            .read_storage::<Presence>()
            .get(entity)
            .map(|p| p.kind)
        {
            Some(Actor::Character(character))
        } else {
            None
        }
    }

    fn position_mut<T>(
        &mut self,
        entity: EcsEntity,
        dismount_volume: bool,
        f: impl for<'a> FnOnce(&'a mut comp::Pos) -> T,
    ) -> Result<T, Content> {
        let ecs = self.ecs_mut();
        position_mut(
            entity,
            dismount_volume,
            f,
            &ecs.read_resource(),
            &mut ecs.write_storage(),
            ecs.write_storage(),
            ecs.write_storage(),
            ecs.read_storage(),
            ecs.read_storage(),
            ecs.read_storage(),
        )
    }
}

pub fn position_mut<T>(
    entity: EcsEntity,
    dismount_volume: bool,
    f: impl for<'a> FnOnce(&'a mut comp::Pos) -> T,
    id_maps: &IdMaps,
    is_volume_riders: &mut WriteStorage<Is<VolumeRider>>,
    mut positions: impl GenericWriteStorage<Component = comp::Pos>,
    mut force_updates: impl GenericWriteStorage<Component = comp::ForceUpdate>,
    is_riders: impl GenericReadStorage<Component = Is<Rider>>,
    presences: impl GenericReadStorage<Component = Presence>,
    clients: impl GenericReadStorage<Component = Client>,
) -> Result<T, Content> {
    if dismount_volume {
        is_volume_riders.remove(entity);
    }

    let entity = is_riders
        .get(entity)
        .and_then(|is_rider| id_maps.uid_entity(is_rider.mount))
        .map(Ok)
        .or_else(|| {
            is_volume_riders.get(entity).and_then(|volume_rider| {
                Some(match volume_rider.pos.kind {
                    common::mounting::Volume::Terrain => Err("Tried to move the world."),
                    common::mounting::Volume::Entity(uid) => Ok(id_maps.uid_entity(uid)?),
                })
            })
        })
        .unwrap_or(Ok(entity))?;

    let mut maybe_pos = None;

    let res = positions
        .get_mut(entity)
        .map(|pos| {
            let res = f(pos);
            maybe_pos = Some(pos.0);
            res
        })
        .ok_or(Content::localized_with_args(
            "command-position-unavailable",
            [("target", "entity")],
        ));

    if let Some(pos) = maybe_pos {
        if presences
            .get(entity)
            .map(|presence| presence.kind == PresenceKind::Spectator)
            .unwrap_or(false)
        {
            clients.get(entity).map(|client| {
                client.send_fallible(ServerGeneral::SpectatePosition(pos));
            });
        } else {
            force_updates
                .get_mut(entity)
                .map(|force_update| force_update.update());
        }
    }

    res
}

fn send_to_group(g: &Group, ecs: &specs::World, msg: &comp::ChatMsg) {
    for (client, group) in (&ecs.read_storage::<Client>(), &ecs.read_storage::<Group>()).join() {
        if g == group {
            client.send_fallible(ServerGeneral::ChatMsg(msg.clone()));
        }
    }
}

/// This should only be called from `handle_exit_ingame` and
/// `delete_entity_recorded`!!!!!!!
pub(crate) fn delete_entity_common(
    state: &mut State,
    entity: EcsEntity,
    maybe_uid: Option<Uid>,
    sync_me: bool,
) -> Result<(), specs::error::WrongGeneration> {
    if maybe_uid.is_none() {
        // For now we expect all entities have a Uid component.
        error!("Deleting entity without Uid component");
    }
    let maybe_pos = state.read_component_copied::<comp::Pos>(entity);

    // TODO: workaround for https://github.com/amethyst/specs/pull/766
    let actual_gen = state.ecs().entities().entity(entity.id()).gen();
    let res = if actual_gen == entity.gen() {
        state.ecs_mut().delete_entity(entity)
    } else {
        Err(specs::error::WrongGeneration {
            action: "delete",
            actual_gen,
            entity,
        })
    };

    if res.is_ok() {
        let region_map = state.mut_resource::<common::region::RegionMap>();
        let uid_pos_region_key = maybe_uid
            .zip(maybe_pos)
            .map(|(uid, pos)| (uid, pos, region_map.find_region(entity, pos.0)));
        region_map.entity_deleted(entity);
        // Note: Adding the `Uid` to the deleted list when exiting "in-game" relies on
        // the client not being able to immediately re-enter the game in the
        // same tick (since we could then mix up the ordering of things and
        // tell other clients to forget the new entity).
        //
        // The client will ignore requests to delete its own entity that are triggered
        // by this.
        if let Some((uid, pos, region_key)) = uid_pos_region_key {
            if let Some(region_key) = region_key {
                state
                    .mut_resource::<DeletedEntities>()
                    .record_deleted_entity(uid, region_key);
            // If there is a position and sync_me is true, but the entity is not
            // in a region, something might be wrong.
            } else if sync_me {
                // Don't panic if the entity wasn't found in a region, maybe it was just created
                // and then deleted before the region manager had a chance to assign it a region
                warn!(
                    ?uid,
                    ?pos,
                    "Failed to find region containing entity during entity deletion, assuming it \
                     wasn't sent to any clients and so deletion doesn't need to be recorded for \
                     sync purposes"
                );
            }
        }
    }
    res
}
