use crate::{
    automod::AutoMod,
    client::Client,
    events::{self, update_map_markers},
    persistence::PersistedComponents,
    pet::restore_pet,
    presence::{Presence, RepositionOnChunkLoad},
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
        skills::{GeneralSkill, Skill},
        ChatType, Group, Inventory, Item, Player, Poise,
    },
    effect::Effect,
    link::{Link, LinkHandle},
    mounting::Mounting,
    resources::{Secs, Time, TimeOfDay},
    slowjob::SlowJobPool,
    uid::{Uid, UidAllocator},
    LoadoutBuilder, ViewDistances,
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
use std::time::Instant;
use tracing::{trace, warn};
use vek::*;

pub trait StateExt {
    /// Updates a component associated with the entity based on the `Effect`
    fn apply_effect(&self, entity: EcsEntity, effect: Effect, source: Option<Uid>);
    /// Build a non-player character
    fn create_npc(
        &mut self,
        pos: comp::Pos,
        stats: comp::Stats,
        skill_set: comp::SkillSet,
        health: Option<comp::Health>,
        poise: Poise,
        inventory: Inventory,
        body: comp::Body,
    ) -> EcsEntityBuilder;
    /// Build a static object entity
    fn create_object(&mut self, pos: comp::Pos, object: comp::object::Body) -> EcsEntityBuilder;
    fn create_item_drop(&mut self, pos: comp::Pos, item: Item) -> EcsEntityBuilder;
    fn create_ship<F: FnOnce(comp::ship::Body) -> comp::Collider>(
        &mut self,
        pos: comp::Pos,
        ship: comp::ship::Body,
        make_collider: F,
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
    fn update_character_data(&mut self, entity: EcsEntity, components: PersistedComponents);
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
                    self.ecs().entity_from_uid(uid.0).map(|attacker_entity| {
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
                    false,
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
                            self.ecs().entity_from_uid(uid.0).map(|attacker_entity| {
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
                let healths = self.ecs().read_storage::<comp::Health>();
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
                                healths.get(entity),
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
            .with(comp::ActiveAbilities::default())
            .with(skill_set)
            .maybe_with(health)
            .with(poise)
            .with(comp::Alignment::Npc)
            .with(comp::CharacterState::default())
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

    fn create_item_drop(&mut self, pos: comp::Pos, item: Item) -> EcsEntityBuilder {
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
        self.ecs_mut()
            .create_entity_synced()
            .with(item)
            .with(pos)
            .with(comp::Vel(Vec3::zero()))
            .with(item_drop.orientation(&mut thread_rng()))
            .with(item_drop.mass())
            .with(item_drop.density())
            .with(body.collider())
            .with(body)
            .maybe_with(light_emitter)
    }

    fn create_ship<F: FnOnce(comp::ship::Body) -> comp::Collider>(
        &mut self,
        pos: comp::Pos,
        ship: comp::ship::Body,
        make_collider: F,
        mountable: bool,
    ) -> EcsEntityBuilder {
        let body = comp::Body::Ship(ship);
        let builder = self
            .ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::Vel(Vec3::zero()))
            .with(comp::Ori::default())
            .with(body.mass())
            .with(body.density())
            .with(make_collider(ship))
            .with(body)
            .with(comp::Scale(comp::ship::AIRSHIP_SCALE))
            .with(comp::Controller::default())
            .with(Inventory::with_empty())
            .with(comp::CharacterState::default())
            // TODO: some of these are required in order for the character_behavior system to
            // recognize a possesed airship; that system should be refactored to use `.maybe()`
            .with(comp::Energy::new(ship.into(), 0))
            .with(comp::Stats::new("Airship".to_string(), body))
            .with(comp::SkillSet::default())
            .with(comp::ActiveAbilities::default())
            .with(comp::Combo::default());

        if mountable {
            // TODO: Re-add mounting check
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
        let time = self.get_time();
        self.ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(Auras::new(vec![Aura::new(
                AuraKind::Buff {
                    kind: BuffKind::Invulnerability,
                    data: BuffData::new(1.0, Some(Secs(1.0)), None),
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
                    chunk_generator.generate_chunk(None, chunk_key, &slow_jobs, Arc::clone(world), index.clone(), time);
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
            self.write_component_ignore_entity_dead(entity, comp::Alignment::Owned(player_uid));
            self.write_component_ignore_entity_dead(entity, comp::Buffs::default());
            self.write_component_ignore_entity_dead(entity, comp::Auras::default());
            self.write_component_ignore_entity_dead(entity, comp::Combo::default());
            self.write_component_ignore_entity_dead(entity, comp::Stance::default());

            // Make sure physics components are updated
            self.write_component_ignore_entity_dead(entity, comp::ForceUpdate::forced());

            self.write_component_ignore_entity_dead(
                entity,
                Presence::new(view_distances, PresenceKind::Character(character_id)),
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

    fn update_character_data(&mut self, entity: EcsEntity, components: PersistedComponents) {
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
            // Notify clients of a player list update
            self.notify_players(ServerGeneral::PlayerListUpdate(
                PlayerListUpdate::SelectedCharacter(player_uid, CharacterInfo {
                    name: String::from(&stats.name),
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
                self.write_component_ignore_entity_dead(entity, RepositionOnChunkLoad);
                self.write_component_ignore_entity_dead(entity, waypoint);
                self.write_component_ignore_entity_dead(entity, comp::Pos(waypoint.get_pos()));
                self.write_component_ignore_entity_dead(entity, comp::Vel(Vec3::zero()));
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

                for (pet, body, stats) in pets {
                    let pet_entity = self
                        .create_npc(
                            player_pos,
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
                warn!("Player has no pos, cannot load {} pets", pets.len());
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
        let Some(client) = client.get(entity) else { return true };
        let Some(player) = player.get(entity) else { return true };

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

        let group_info = msg.get_group().and_then(|g| group_manager.group_info(*g));

        let resolved_msg = msg
            .clone()
            .map_group(|_| group_info.map_or_else(|| "???".to_string(), |i| i.name.clone()));

        if msg.chat_type.uid().map_or(true, |sender| {
            (*ecs.read_resource::<UidAllocator>())
                .retrieve_entity_internal(sender.0)
                .map_or(false, |e| {
                    self.validate_chat_msg(e, &msg.chat_type, &msg.message)
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

        L::create(&linker, self.ecs().system_data())?;

        self.ecs_mut()
            .entry::<Vec<LinkHandle<L>>>()
            .or_insert_with(Vec::new)
            .push(linker);

        Ok(())
    }

    fn maintain_links(&mut self) {
        fn maintain_link<L: Link>(state: &State) {
            if let Some(mut handles) = state.ecs().try_fetch_mut::<Vec<LinkHandle<L>>>() {
                handles.retain(|link| {
                    if L::persist(link, state.ecs().system_data()) {
                        true
                    } else {
                        L::delete(link, state.ecs().system_data());
                        false
                    }
                });
            }
        }

        maintain_link::<Mounting>(self);
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

fn send_to_group(g: &Group, ecs: &specs::World, msg: &comp::ChatMsg) {
    for (client, group) in (&ecs.read_storage::<Client>(), &ecs.read_storage::<Group>()).join() {
        if g == group {
            client.send_fallible(ServerGeneral::ChatMsg(msg.clone()));
        }
    }
}
