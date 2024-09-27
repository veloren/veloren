#[cfg(not(feature = "worldgen"))]
use crate::test_world::{IndexOwned, World};
#[cfg(feature = "persistent_world")]
use crate::TerrainPersistence;
use tracing::error;
#[cfg(feature = "worldgen")]
use world::{IndexOwned, World};

#[cfg(feature = "worldgen")] use crate::rtsim;
use crate::{
    chunk_generator::ChunkGenerator, chunk_serialize::ChunkSendEntry, client::Client,
    presence::RepositionOnChunkLoad, settings::Settings, ChunkRequest, Tick,
};
use common::{
    calendar::Calendar,
    combat::DeathEffects,
    comp::{
        self, agent, biped_small, bird_medium, BehaviorCapability, ForceUpdate, Pos, Presence,
        Waypoint,
    },
    event::{CreateNpcEvent, CreateSpecialEntityEvent, EmitExt, EventBus, NpcBuilder},
    event_emitters,
    generation::{EntityInfo, SpecialEntity},
    lottery::LootSpec,
    resources::{Time, TimeOfDay},
    slowjob::SlowJobPool,
    terrain::TerrainGrid,
    util::Dir,
    SkillSetBuilder,
};

use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::ServerGeneral;
use common_state::TerrainChanges;
use comp::Behavior;
use core::cmp::Reverse;
use itertools::Itertools;
use rayon::{iter::Either, prelude::*};
use specs::{
    shred, storage::GenericReadStorage, Entities, Entity, Join, LendJoin, ParJoin, Read,
    ReadExpect, ReadStorage, SystemData, Write, WriteExpect, WriteStorage,
};
use std::{f32::consts::TAU, sync::Arc};
use vek::*;

#[cfg(feature = "persistent_world")]
pub type TerrainPersistenceData<'a> = Option<Write<'a, TerrainPersistence>>;
#[cfg(not(feature = "persistent_world"))]
pub type TerrainPersistenceData<'a> = ();

pub const SAFE_ZONE_RADIUS: f32 = 200.0;

#[cfg(feature = "worldgen")]
type RtSimData<'a> = WriteExpect<'a, rtsim::RtSim>;
#[cfg(not(feature = "worldgen"))]
type RtSimData<'a> = ();

event_emitters! {
    struct Events[Emitters] {
        create_npc: CreateNpcEvent,
        create_waypoint: CreateSpecialEntityEvent,
    }
}

#[derive(SystemData)]
pub struct Data<'a> {
    events: Events<'a>,
    tick: Read<'a, Tick>,
    server_settings: Read<'a, Settings>,
    time_of_day: Read<'a, TimeOfDay>,
    calendar: Read<'a, Calendar>,
    slow_jobs: ReadExpect<'a, SlowJobPool>,
    index: ReadExpect<'a, IndexOwned>,
    world: ReadExpect<'a, Arc<World>>,
    chunk_send_bus: ReadExpect<'a, EventBus<ChunkSendEntry>>,
    chunk_generator: WriteExpect<'a, ChunkGenerator>,
    terrain: WriteExpect<'a, TerrainGrid>,
    terrain_changes: Write<'a, TerrainChanges>,
    chunk_requests: Write<'a, Vec<ChunkRequest>>,
    rtsim: RtSimData<'a>,
    #[cfg(feature = "persistent_world")]
    terrain_persistence: TerrainPersistenceData<'a>,
    positions: WriteStorage<'a, Pos>,
    presences: ReadStorage<'a, Presence>,
    clients: ReadStorage<'a, Client>,
    entities: Entities<'a>,
    reposition_on_load: WriteStorage<'a, RepositionOnChunkLoad>,
    forced_updates: WriteStorage<'a, ForceUpdate>,
    waypoints: WriteStorage<'a, Waypoint>,
    time: ReadExpect<'a, Time>,
}

/// This system will handle loading generated chunks and unloading
/// unneeded chunks.
///     1. Inserts newly generated chunks into the TerrainGrid
///     2. Sends new chunks to nearby clients
///     3. Handles the chunk's supplement (e.g. npcs)
///     4. Removes chunks outside the range of players
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = Data<'a>;

    const NAME: &'static str = "terrain";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, mut data: Self::SystemData) {
        let mut emitters = data.events.get_emitters();

        // Generate requested chunks
        //
        // Submit requests for chunks right before receiving finished chunks so that we
        // don't create duplicate work for chunks that just finished but are not
        // yet added to the terrain.
        data.chunk_requests.drain(..).for_each(|request| {
            data.chunk_generator.generate_chunk(
                Some(request.entity),
                request.key,
                &data.slow_jobs,
                Arc::clone(&data.world),
                &data.rtsim,
                data.index.clone(),
                (*data.time_of_day, data.calendar.clone()),
            )
        });

        let mut rng = rand::thread_rng();
        // Fetch any generated `TerrainChunk`s and insert them into the terrain.
        // Also, send the chunk data to anybody that is close by.
        let mut new_chunks = Vec::new();
        'insert_terrain_chunks: while let Some((key, res)) = data.chunk_generator.recv_new_chunk() {
            #[allow(unused_mut)]
            let (mut chunk, supplement) = match res {
                Ok((chunk, supplement)) => (chunk, supplement),
                Err(Some(entity)) => {
                    if let Some(client) = data.clients.get(entity) {
                        client.send_fallible(ServerGeneral::TerrainChunkUpdate {
                            key,
                            chunk: Err(()),
                        });
                    }
                    continue 'insert_terrain_chunks;
                },
                Err(None) => {
                    continue 'insert_terrain_chunks;
                },
            };

            // Apply changes from terrain persistence to this chunk
            #[cfg(feature = "persistent_world")]
            if let Some(terrain_persistence) = data.terrain_persistence.as_mut() {
                terrain_persistence.apply_changes(key, &mut chunk);
            }

            // Arcify the chunk
            let chunk = Arc::new(chunk);

            // Add to list of chunks to send to nearby players.
            new_chunks.push(key);

            // TODO: code duplication for chunk insertion between here and state.rs
            // Insert the chunk into terrain changes
            if data.terrain.insert(key, chunk).is_some() {
                data.terrain_changes.modified_chunks.insert(key);
            } else {
                data.terrain_changes.new_chunks.insert(key);
                #[cfg(feature = "worldgen")]
                data.rtsim
                    .hook_load_chunk(key, supplement.rtsim_max_resources);
            }

            // Handle chunk supplement
            for entity in supplement.entities {
                // Check this because it's a common source of weird bugs
                assert!(
                    data.terrain
                        .pos_key(entity.pos.map(|e| e.floor() as i32))
                        .map2(key, |e, tgt| (e - tgt).abs() <= 1)
                        .reduce_and(),
                    "Chunk spawned entity that wasn't nearby",
                );

                let data = SpawnEntityData::from_entity_info(entity);
                match data {
                    SpawnEntityData::Special(pos, entity) => {
                        emitters.emit(CreateSpecialEntityEvent { pos, entity });
                    },
                    SpawnEntityData::Npc(data) => {
                        let (npc_builder, pos) = data.to_npc_builder();

                        emitters.emit(CreateNpcEvent {
                            pos,
                            ori: comp::Ori::from(Dir::random_2d(&mut rng)),
                            npc: npc_builder.with_anchor(comp::Anchor::Chunk(key)),
                            rider: None,
                        });
                    },
                }
            }
        }

        // TODO: Consider putting this in another system since this forces us to take
        // positions by write rather than read access.
        let repositioned = (&data.entities, &mut data.positions, (&mut data.forced_updates).maybe(), &data.reposition_on_load)
            // TODO: Consider using par_bridge() because Rayon has very poor work splitting for
            // sparse joins.
            .par_join()
            .filter_map(|(entity, pos, force_update, reposition)| {
                // NOTE: We use regular as casts rather than as_ because we want to saturate on
                // overflow.
                let entity_pos = pos.0.map(|x| x as i32);
                // If an entity is marked as needing repositioning once the chunk loads (e.g.
                // from having just logged in), reposition them.
                let chunk_pos = TerrainGrid::chunk_key(entity_pos);
                let chunk = data.terrain.get_key(chunk_pos)?;
                let new_pos = if reposition.needs_ground {
                    data.terrain.try_find_ground(entity_pos)
                } else {
                    data.terrain.try_find_space(entity_pos)
                }.map(|x| x.as_::<f32>()).unwrap_or_else(|| chunk.find_accessible_pos(entity_pos.xy(), false));
                pos.0 = new_pos;
                force_update.map(|force_update| force_update.update());
                Some((entity, new_pos))
            })
            .collect::<Vec<_>>();

        for (entity, new_pos) in repositioned {
            if let Some(waypoint) = data.waypoints.get_mut(entity) {
                *waypoint = Waypoint::new(new_pos, *data.time);
            }
            data.reposition_on_load.remove(entity);
        }

        let max_view_distance = data.server_settings.max_view_distance.unwrap_or(u32::MAX);
        #[cfg(feature = "worldgen")]
        let world_size = data.world.sim().get_size();
        #[cfg(not(feature = "worldgen"))]
        let world_size = data.world.map_size_lg().chunks().map(u32::from);
        let (presences_position_entities, presences_positions) = prepare_player_presences(
            world_size,
            max_view_distance,
            &data.entities,
            &data.positions,
            &data.presences,
            &data.clients,
        );
        let real_max_view_distance = convert_to_loaded_vd(u32::MAX, max_view_distance);

        // Send the chunks to all nearby players.
        new_chunks.par_iter().for_each_init(
            || data.chunk_send_bus.emitter(),
            |chunk_send_emitter, chunk_key| {
                // We only have to check players inside the maximum view distance of the server
                // of our own position.
                //
                // We start by partitioning by X, finding only entities in chunks within the X
                // range of us.  These are guaranteed in bounds due to restrictions on max view
                // distance (namely: the square of any chunk coordinate plus the max view
                // distance along both axes must fit in an i32).
                let min_chunk_x = chunk_key.x - real_max_view_distance;
                let max_chunk_x = chunk_key.x + real_max_view_distance;
                let start = presences_position_entities
                    .partition_point(|((pos, _), _)| i32::from(pos.x) < min_chunk_x);
                // NOTE: We *could* just scan forward until we hit the end, but this way we save
                // a comparison in the inner loop, since also needs to check the
                // list length.  We could also save some time by starting from
                // start rather than end, but the hope is that this way the
                // compiler (and machine) can reorder things so both ends are
                // fetched in parallel; since the vast majority of the time both fetched
                // elements should already be in cache, this should not use any
                // extra memory bandwidth.
                //
                // TODO: Benchmark and figure out whether this is better in practice than just
                // scanning forward.
                let end = presences_position_entities
                    .partition_point(|((pos, _), _)| i32::from(pos.x) < max_chunk_x);
                let interior = &presences_position_entities[start..end];
                interior
                    .iter()
                    .filter(|((player_chunk_pos, player_vd_sqr), _)| {
                        chunk_in_vd(*player_chunk_pos, *player_vd_sqr, *chunk_key)
                    })
                    .for_each(|(_, entity)| {
                        chunk_send_emitter.emit(ChunkSendEntry {
                            entity: *entity,
                            chunk_key: *chunk_key,
                        });
                    });
            },
        );

        let tick = (data.tick.0 % 16) as i32;

        // Remove chunks that are too far from players.
        //
        // Note that all chunks involved here (both terrain chunks and pending chunks)
        // are guaranteed in bounds.  This simplifies the rest of the logic
        // here.
        let chunks_to_remove = data.terrain
            .par_keys()
            .copied()
            // There may be lots of pending chunks, so don't check them all.  This should be okay
            // as long as we're maintaining a reasonable tick rate.
            .chain(data.chunk_generator.par_pending_chunks())
            // Don't check every chunk every tick (spread over 16 ticks)
            //
            // TODO: Investigate whether we can add support for performing this filtering directly
            // within hashbrown (basically, specify we want to iterate through just buckets with
            // hashes in a particular range).  This could provide significiant speedups since we
            // could avoid having to iterate through a bunch of buckets we don't care about.
            //
            // TODO: Make the percentage of the buckets that we go through adjust dynamically
            // depending on the current number of chunks.  In the worst case, we might want to scan
            // just 1/256 of the chunks each tick, for example.
            .filter(|k| k.x % 4 + (k.y % 4) * 4 == tick)
            .filter(|&chunk_key| {
                // We only have to check players inside the maximum view distance of the server of
                // our own position.
                //
                // We start by partitioning by X, finding only entities in chunks within the X
                // range of us.  These are guaranteed in bounds due to restrictions on max view
                // distance (namely: the square of any chunk coordinate plus the max view distance
                // along both axes must fit in an i32).
                let min_chunk_x = chunk_key.x - real_max_view_distance;
                let max_chunk_x = chunk_key.x + real_max_view_distance;
                let start = presences_positions
                    .partition_point(|(pos, _)| i32::from(pos.x) < min_chunk_x);
                // NOTE: We *could* just scan forward until we hit the end, but this way we save a
                // comparison in the inner loop, since also needs to check the list length.  We
                // could also save some time by starting from start rather than end, but the hope
                // is that this way the compiler (and machine) can reorder things so both ends are
                // fetched in parallel; since the vast majority of the time both fetched elements
                // should already be in cache, this should not use any extra memory bandwidth.
                //
                // TODO: Benchmark and figure out whether this is better in practice than just
                // scanning forward.
                let end = presences_positions
                    .partition_point(|(pos, _)| i32::from(pos.x) < max_chunk_x);
                let interior = &presences_positions[start..end];
                !interior.iter().any(|&(player_chunk_pos, player_vd_sqr)| {
                    chunk_in_vd(player_chunk_pos, player_vd_sqr, chunk_key)
                })
            })
            .collect::<Vec<_>>();

        let chunks_to_remove = chunks_to_remove
            .into_iter()
            .filter_map(|key| {
                // Register the unloading of this chunk from terrain persistence
                #[cfg(feature = "persistent_world")]
                if let Some(terrain_persistence) = data.terrain_persistence.as_mut() {
                    terrain_persistence.unload_chunk(key);
                }

                data.chunk_generator.cancel_if_pending(key);

                // If you want to trigger any behaivour on unload, do it in `Server::tick` by
                // reading `TerrainChanges::removed_chunks` since chunks can also be removed
                // using eg. /reload_chunks

                // TODO: code duplication for chunk insertion between here and state.rs
                data.terrain.remove(key).inspect(|_| {
                    data.terrain_changes.removed_chunks.insert(key);
                })
            })
            .collect::<Vec<_>>();
        if !chunks_to_remove.is_empty() {
            // Drop chunks in a background thread.
            data.slow_jobs.spawn("CHUNK_DROP", move || {
                drop(chunks_to_remove);
            });
        }
    }
}

// TODO: better name
#[derive(Debug)]
pub struct NpcData {
    pub pos: Pos,
    pub stats: comp::Stats,
    pub skill_set: comp::SkillSet,
    pub health: Option<comp::Health>,
    pub poise: comp::Poise,
    pub inventory: comp::inventory::Inventory,
    pub agent: Option<comp::Agent>,
    pub body: comp::Body,
    pub alignment: comp::Alignment,
    pub scale: comp::Scale,
    pub loot: LootSpec<String>,
    pub pets: Vec<(NpcData, Vec3<f32>)>,
    pub death_effects: Option<DeathEffects>,
}

/// Convinient structure to use when you need to create new npc
/// from EntityInfo
// TODO: better name?
// TODO: if this is send around network, optimize the large_enum_variant
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum SpawnEntityData {
    Npc(NpcData),
    Special(Vec3<f32>, SpecialEntity),
}

impl SpawnEntityData {
    pub fn from_entity_info(entity: EntityInfo) -> Self {
        let EntityInfo {
            // flags
            special_entity,
            has_agency,
            agent_mark,
            alignment,
            no_flee,
            idle_wander_factor,
            aggro_range_multiplier,
            // stats
            body,
            name,
            scale,
            pos,
            loot,
            // tools and skills
            skillset_asset,
            loadout: mut loadout_builder,
            inventory: items,
            make_loadout,
            trading_information: economy,
            pets,
            death_effects,
        } = entity;

        if let Some(special) = special_entity {
            return Self::Special(pos, special);
        }

        let name = name.unwrap_or_else(|| "Unnamed".to_string());
        let stats = comp::Stats::new(name, body);

        let skill_set = {
            let skillset_builder = SkillSetBuilder::default();
            if let Some(skillset_asset) = skillset_asset {
                skillset_builder.with_asset_expect(&skillset_asset).build()
            } else {
                skillset_builder.build()
            }
        };

        let inventory = {
            // Evaluate lazy function for loadout creation
            if let Some(make_loadout) = make_loadout {
                loadout_builder =
                    loadout_builder.with_creator(make_loadout, economy.as_ref(), None);
            }
            let loadout = loadout_builder.build();
            let mut inventory = comp::inventory::Inventory::with_loadout(loadout, body);
            for (num, mut item) in items {
                if let Err(e) = item.set_amount(num) {
                    tracing::warn!(
                        "error during creating inventory for {name} at {pos}: {e:?}",
                        name = &stats.name,
                    );
                }
                if let Err(e) = inventory.push(item) {
                    tracing::warn!(
                        "error during creating inventory for {name} at {pos}: {e:?}",
                        name = &stats.name,
                    );
                }
            }

            inventory
        };

        let health = Some(comp::Health::new(body));
        let poise = comp::Poise::new(body);

        // Allow Humanoid, BirdMedium, and Parrot to speak
        let can_speak = match body {
            comp::Body::Humanoid(_) => true,
            comp::Body::BipedSmall(biped_small) => {
                matches!(biped_small.species, biped_small::Species::Flamekeeper)
            },
            comp::Body::BirdMedium(bird_medium) => match bird_medium.species {
                bird_medium::Species::Parrot => alignment == comp::Alignment::Npc,
                _ => false,
            },
            _ => false,
        };

        let trade_for_site = if matches!(agent_mark, Some(agent::Mark::Merchant)) {
            economy.map(|e| e.id)
        } else {
            None
        };

        let agent = has_agency.then(|| {
            let mut agent = comp::Agent::from_body(&body).with_behavior(
                Behavior::default()
                    .maybe_with_capabilities(can_speak.then_some(BehaviorCapability::SPEAK))
                    .maybe_with_capabilities(trade_for_site.map(|_| BehaviorCapability::TRADE))
                    .with_trade_site(trade_for_site),
            );

            // Non-humanoids get a patrol origin to stop them moving too far
            if !matches!(body, comp::Body::Humanoid(_)) {
                agent = agent.with_patrol_origin(pos);
            }

            agent
                .with_no_flee_if(matches!(agent_mark, Some(agent::Mark::Guard)) || no_flee)
                .with_idle_wander_factor(idle_wander_factor)
                .with_aggro_range_multiplier(aggro_range_multiplier)
        });

        let agent = if matches!(alignment, comp::Alignment::Enemy)
            && matches!(body, comp::Body::Humanoid(_))
        {
            agent.map(|a| a.with_aggro_no_warn().with_no_flee_if(true))
        } else {
            agent
        };

        SpawnEntityData::Npc(NpcData {
            pos: Pos(pos),
            stats,
            skill_set,
            health,
            poise,
            inventory,
            agent,
            body,
            alignment,
            scale: comp::Scale(scale),
            loot,
            pets: {
                let pet_count = pets.len() as f32;
                pets.into_iter()
                    .enumerate()
                    .flat_map(|(i, pet)| {
                        Some((
                            SpawnEntityData::from_entity_info(pet)
                                .into_npc_data_inner()
                                .inspect_err(|data| {
                                    error!("Pets must be SpawnEntityData::Npc, but found: {data:?}")
                                })
                                .ok()?,
                            Vec2::one()
                                .rotated_z(TAU * (i as f32 / pet_count))
                                .with_z(0.0)
                                * ((pet_count * 3.0) / TAU),
                        ))
                    })
                    .collect()
            },
            death_effects,
        })
    }

    pub fn into_npc_data_inner(self) -> Result<NpcData, Self> {
        match self {
            SpawnEntityData::Npc(inner) => Ok(inner),
            other => Err(other),
        }
    }
}

impl NpcData {
    pub fn to_npc_builder(self) -> (NpcBuilder, comp::Pos) {
        let NpcData {
            pos,
            stats,
            skill_set,
            health,
            poise,
            inventory,
            agent,
            body,
            alignment,
            scale,
            loot,
            pets,
            death_effects,
        } = self;

        (
            NpcBuilder::new(stats, body, alignment)
                .with_skill_set(skill_set)
                .with_health(health)
                .with_poise(poise)
                .with_inventory(inventory)
                .with_agent(agent)
                .with_scale(scale)
                .with_loot(loot)
                .with_pets(
                    pets.into_iter()
                        .map(|(pet, offset)| (pet.to_npc_builder().0, offset))
                        .collect::<Vec<_>>(),
                )
                .with_death_effects(death_effects),
            pos,
        )
    }
}

pub fn convert_to_loaded_vd(vd: u32, max_view_distance: u32) -> i32 {
    // Hardcoded max VD to prevent stupid view distances from creating overflows.
    // This must be a value ≤
    // √(i32::MAX - 2 * ((1 << (MAX_WORLD_BLOCKS_LG - TERRAIN_CHUNK_BLOCKS_LG) - 1)²
    // - 1)) / 2
    //
    // since otherwise we could end up overflowing.  Since it is a requirement that
    // each dimension (in chunks) has to fit in a i16, we can derive √((1<<31)-1
    // - 2*((1<<15)-1)^2) / 2 ≥ 1 << 7 as the absolute limit.
    //
    // TODO: Make this more official and use it elsewhere.
    const MAX_VD: u32 = 1 << 7;

    // This fuzzy threshold prevents chunks rapidly unloading and reloading when
    // players move over a chunk border.
    const UNLOAD_THRESHOLD: u32 = 2;

    // NOTE: This cast is safe for the reasons mentioned above.
    (vd.clamp(crate::MIN_VD, max_view_distance)
        .saturating_add(UNLOAD_THRESHOLD))
    .min(MAX_VD) as i32
}

/// Returns: ((player_chunk_pos, player_vd_squared), entity, is_client)
fn prepare_for_vd_check(
    world_aabr_in_chunks: &Aabr<i32>,
    max_view_distance: u32,
    entity: Entity,
    presence: &Presence,
    pos: &Pos,
    client: Option<u32>,
) -> Option<((Vec2<i16>, i32), Entity, bool)> {
    let is_client = client.is_some();
    let pos = pos.0;
    let vd = presence.terrain_view_distance.current();

    // NOTE: We use regular as casts rather than as_ because we want to saturate on
    // overflow.
    let player_pos = pos.map(|x| x as i32);
    let player_chunk_pos = TerrainGrid::chunk_key(player_pos);
    let player_vd = convert_to_loaded_vd(vd, max_view_distance);

    // We filter out positions that are *clearly* way out of range from
    // consideration. This is pretty easy to do, and means we don't have to
    // perform expensive overflow checks elsewhere (otherwise, a player
    // sufficiently far off the map could cause chunks they were nowhere near to
    // stay loaded, parallel universes style).
    //
    // One could also imagine snapping a player to the part of the map nearest to
    // them. We don't currently do this in case we rely elsewhere on players
    // always being near the chunks they're keeping loaded, but it would allow
    // us to use u32 exclusively so it's tempting.
    let player_aabr_in_chunks = Aabr {
        min: player_chunk_pos - player_vd,
        max: player_chunk_pos + player_vd,
    };

    (world_aabr_in_chunks.max.x >= player_aabr_in_chunks.min.x &&
     world_aabr_in_chunks.min.x <= player_aabr_in_chunks.max.x &&
     world_aabr_in_chunks.max.y >= player_aabr_in_chunks.min.y &&
     world_aabr_in_chunks.min.y <= player_aabr_in_chunks.max.y)
        // The cast to i32 here is definitely safe thanks to MAX_VD limiting us to fit
        // within i32^2.
        //
        // The cast from each coordinate to i16 should also be correct here.  This is because valid
        // world chunk coordinates are no greater than 1 << 14 - 1; since we verified that the
        // player is within world bounds modulo player_vd, which is guaranteed to never let us
        // overflow an i16 when added to a u14, safety of the cast follows.
        .then(|| ((player_chunk_pos.as_::<i16>(), player_vd.pow(2)), entity, is_client))
}

pub fn prepare_player_presences<'a, P>(
    world_size: Vec2<u32>,
    max_view_distance: u32,
    entities: &Entities<'a>,
    positions: P,
    presences: &ReadStorage<'a, Presence>,
    clients: &ReadStorage<'a, Client>,
) -> (Vec<((Vec2<i16>, i32), Entity)>, Vec<(Vec2<i16>, i32)>)
where
    P: GenericReadStorage<Component = Pos> + Join<Type = &'a Pos>,
{
    // We start by collecting presences and positions from players, because they are
    // very sparse in the entity list and therefore iterating over them for each
    // chunk can be quite slow.
    let world_aabr_in_chunks = Aabr {
        min: Vec2::zero(),
        // NOTE: Cast is correct because chunk coordinates must fit in an i32 (actually, i16).
        max: world_size.map(|x| x.saturating_sub(1)).as_::<i32>(),
    };

    let (mut presences_positions_entities, mut presences_positions): (Vec<_>, Vec<_>) =
        (entities, presences, positions, clients.mask().maybe())
            .join()
            .filter_map(|(entity, presence, position, client)| {
                prepare_for_vd_check(
                    &world_aabr_in_chunks,
                    max_view_distance,
                    entity,
                    presence,
                    position,
                    client,
                )
            })
            .partition_map(|(player_data, entity, is_client)| {
                // For chunks with clients, we need to record their entity, because they might
                // be used for insertion.  These elements fit in 8 bytes, so
                // this should be pretty cache-friendly.
                if is_client {
                    Either::Left((player_data, entity))
                } else {
                    // For chunks without clients, we only need to record the position and view
                    // distance.  These elements fit in 4 bytes, which is even cache-friendlier.
                    Either::Right(player_data)
                }
            });

    // We sort the presence lists by X position, so we can efficiently filter out
    // players nowhere near the chunk.  This is basically a poor substitute for
    // the effects of a proper KDTree, but a proper KDTree has too much overhead
    // to be worth using for such a short list (~ 1000 players at most).  We
    // also sort by y and reverse view distance; this will become important later.
    presences_positions_entities
        .sort_unstable_by_key(|&((pos, vd2), _)| (pos.x, pos.y, Reverse(vd2)));
    presences_positions.sort_unstable_by_key(|&(pos, vd2)| (pos.x, pos.y, Reverse(vd2)));
    // For the vast majority of chunks (present and pending ones), we'll only ever
    // need the position and view distance.  So we extend it with these from the
    // list of client chunks, and then do some further work to improve
    // performance (taking advantage of the fact that they don't require
    // entities).
    presences_positions.extend(
        presences_positions_entities
            .iter()
            .map(|&(player_data, _)| player_data),
    );
    // Since both lists were previously sorted, we use stable sort over unstable
    // sort, as it's faster in that case (theoretically a proper merge operation
    // would be ideal, but it's not worth pulling in a library for).
    presences_positions.sort_by_key(|&(pos, vd2)| (pos.x, pos.y, Reverse(vd2)));
    // Now that the list is sorted, we deduplicate players in the same chunk (this
    // is why we need to sort y as well as x; dedup only works if the list is
    // sorted by the element we use to dedup).  Importantly, we can then use
    // only the *first* element as a substitute for all the players in the
    // chunk, because we *also* sorted from greatest to lowest view
    // distance, and dedup_by removes all but the first matching element.  In the
    // common case where a few chunks are very crowded, this further reduces the
    // work required per chunk.
    presences_positions.dedup_by_key(|&mut (pos, _)| pos);

    (presences_positions_entities, presences_positions)
}

pub fn chunk_in_vd(player_chunk_pos: Vec2<i16>, player_vd_sqr: i32, chunk_pos: Vec2<i32>) -> bool {
    // NOTE: Guaranteed in bounds as long as prepare_player_presences prepared the
    // player_chunk_pos and player_vd_sqr.
    let adjusted_dist_sqr = (player_chunk_pos.as_::<i32>() - chunk_pos).magnitude_squared();

    adjusted_dist_sqr <= player_vd_sqr
}
