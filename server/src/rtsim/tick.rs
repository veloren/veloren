use super::*;
use crate::{ServerConstants, sys::terrain::SpawnEntityData};
use common::{
    LoadoutBuilder,
    calendar::Calendar,
    comp::{
        self, Body, Item, Presence, PresenceKind, inventory::trade_pricing::TradePricing,
        slot::ArmorSlot,
    },
    event::{CreateNpcEvent, CreateShipEvent, DeleteEvent, EventBus, NpcBuilder},
    generation::{BodyBuilder, EntityConfig, EntityInfo},
    resources::{DeltaTime, Time, TimeOfDay},
    rtsim::{Actor, NpcId, RtSimEntity},
    slowjob::SlowJobPool,
    terrain::CoordinateConversions,
    trade::{Good, SiteInformation},
    uid::IdMaps,
    util::Dir,
    weather::WeatherGrid,
};
use common_ecs::{Job, Origin, Phase, System};
use rand::Rng;
use rtsim::{
    ai::NpcSystemData,
    data::{
        Npc, Sites,
        npc::{Profession, SimulationMode},
    },
};
use specs::{Entities, Join, LendJoin, Read, ReadExpect, ReadStorage, WriteExpect, WriteStorage};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tracing::error;

pub fn trader_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    mut permitted: impl FnMut(Good) -> bool,
) -> LoadoutBuilder {
    let rng = &mut rand::rng();
    let mut backpack = Item::new_from_asset_expect("common.items.armor.misc.back.backpack");
    let mut bag1 = Item::new_from_asset_expect("common.items.armor.misc.bag.sturdy_red_backpack");
    let mut bag2 = Item::new_from_asset_expect("common.items.armor.misc.bag.sturdy_red_backpack");
    let mut bag3 = Item::new_from_asset_expect("common.items.armor.misc.bag.sturdy_red_backpack");
    let mut bag4 = Item::new_from_asset_expect("common.items.armor.misc.bag.sturdy_red_backpack");
    let slots = backpack.slots().len() + 4 * bag1.slots().len();
    let mut stockmap: hashbrown::HashMap<Good, f32> = economy
        .map(|e| {
            e.unconsumed_stock
                .clone()
                .into_iter()
                .filter(|(good, _)| permitted(*good))
                .collect()
        })
        .unwrap_or_default();
    // modify stock for better gameplay

    // TODO: currently econsim spends all its food on population, resulting in none
    // for the players to buy; the `.max` is temporary to ensure that there's some
    // food for sale at every site, to be used until we have some solution like NPC
    // houses as a limit on econsim population growth
    if permitted(Good::Food) {
        stockmap
            .entry(Good::Food)
            .and_modify(|e| *e = e.max(10_000.0))
            .or_insert(10_000.0);
    }
    // Reduce amount of potions so merchants do not oversupply potions.
    // TODO: Maybe remove when merchants and their inventories are rtsim?
    // Note: Likely without effect now that potions are counted as food
    if permitted(Good::Potions) {
        stockmap
            .entry(Good::Potions)
            .and_modify(|e| *e = e.powf(0.25));
    }
    // It's safe to truncate here, because coins clamped to 3000 max
    // also we don't really want negative values here
    if permitted(Good::Coin) {
        stockmap
            .entry(Good::Coin)
            .and_modify(|e| *e = e.min(rng.random_range(1000.0..3000.0)));
    }
    // assume roughly 10 merchants sharing a town's stock (other logic for coins)
    stockmap
        .iter_mut()
        .filter(|(good, _amount)| **good != Good::Coin)
        .for_each(|(_good, amount)| *amount *= 0.1);
    // Fill bags with stuff according to unclaimed stock
    let ability_map = &comp::tool::AbilityMap::load().read();
    let msm = &comp::item::MaterialStatManifest::load().read();
    let mut wares: Vec<Item> =
        TradePricing::random_items(&mut stockmap, slots as u32, true, true, 16)
            .iter()
            .filter_map(|(n, a)| {
                let i = Item::new_from_item_definition_id(n.as_ref(), ability_map, msm).ok();
                i.map(|mut i| {
                    i.set_amount(*a)
                        .map_err(|_| tracing::error!("merchant loadout amount failure"))
                        .ok();
                    i
                })
            })
            .collect();
    sort_wares(&mut wares);
    transfer(&mut wares, &mut backpack);
    transfer(&mut wares, &mut bag1);
    transfer(&mut wares, &mut bag2);
    transfer(&mut wares, &mut bag3);
    transfer(&mut wares, &mut bag4);

    loadout_builder
        .back(Some(backpack))
        .bag(ArmorSlot::Bag1, Some(bag1))
        .bag(ArmorSlot::Bag2, Some(bag2))
        .bag(ArmorSlot::Bag3, Some(bag3))
        .bag(ArmorSlot::Bag4, Some(bag4))
}

fn sort_wares(bag: &mut [Item]) {
    use common::comp::item::TagExampleInfo;

    bag.sort_by(|a, b| {
        a.quality()
            .cmp(&b.quality())
        // sort by kind
        .then(
            Ord::cmp(
                a.tags().first().map_or("", |tag| tag.name()),
                b.tags().first().map_or("", |tag| tag.name()),
            )
        )
        // sort by name
        .then(#[expect(deprecated)] Ord::cmp(&a.name(), &b.name()))
    });
}

fn transfer(wares: &mut Vec<Item>, bag: &mut Item) {
    let capacity = bag.slots().len();
    for (s, w) in bag
        .slots_mut()
        .iter_mut()
        .zip(wares.drain(0..wares.len().min(capacity)))
    {
        *s = Some(w);
    }
}

fn humanoid_config(profession: &Profession) -> &'static str {
    match profession {
        Profession::Farmer => "common.entity.village.farmer",
        Profession::Hunter => "common.entity.village.hunter",
        Profession::Herbalist => "common.entity.village.herbalist",
        Profession::Captain => "common.entity.village.captain",
        Profession::Merchant => "common.entity.village.merchant",
        Profession::Guard => "common.entity.village.guard",
        Profession::Adventurer(rank) => match rank {
            0 => "common.entity.world.traveler0",
            1 => "common.entity.world.traveler1",
            2 => "common.entity.world.traveler2",
            3 => "common.entity.world.traveler3",
            _ => {
                error!(
                    "Tried to get configuration for invalid adventurer rank {}",
                    rank
                );
                "common.entity.world.traveler3"
            },
        },
        Profession::Blacksmith => "common.entity.village.blacksmith",
        Profession::Chef => "common.entity.village.chef",
        Profession::Alchemist => "common.entity.village.alchemist",
        Profession::Pirate(leader) => match leader {
            false => "common.entity.spot.pirate",
            true => "common.entity.spot.buccaneer",
        },
        Profession::Cultist => "common.entity.dungeon.cultist.cultist",
    }
}

fn loadout_default(
    loadout: LoadoutBuilder,
    _economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
) -> LoadoutBuilder {
    loadout
}

fn merchant_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
) -> LoadoutBuilder {
    trader_loadout(loadout_builder, economy, |_| true)
}

fn farmer_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
) -> LoadoutBuilder {
    trader_loadout(loadout_builder, economy, |good| {
        matches!(good, Good::Food | Good::Coin)
    })
}

fn herbalist_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
) -> LoadoutBuilder {
    trader_loadout(loadout_builder, economy, |good| {
        matches!(good, Good::Ingredients)
    })
}

fn chef_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
) -> LoadoutBuilder {
    trader_loadout(loadout_builder, economy, |good| {
        matches!(good, Good::Food | Good::Coin)
    })
}

fn blacksmith_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
) -> LoadoutBuilder {
    trader_loadout(loadout_builder, economy, |good| {
        matches!(good, Good::Tools | Good::Armor | Good::Coin)
    })
}

fn alchemist_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
) -> LoadoutBuilder {
    trader_loadout(loadout_builder, economy, |good| {
        matches!(good, Good::Potions | Good::Coin)
    })
}

fn profession_extra_loadout(
    profession: Option<&Profession>,
) -> fn(
    LoadoutBuilder,
    Option<&SiteInformation>,
    time: Option<&(TimeOfDay, Calendar)>,
) -> LoadoutBuilder {
    match profession {
        Some(Profession::Merchant) => merchant_loadout,
        Some(Profession::Farmer) => farmer_loadout,
        Some(Profession::Herbalist) => herbalist_loadout,
        Some(Profession::Chef) => chef_loadout,
        Some(Profession::Blacksmith) => blacksmith_loadout,
        Some(Profession::Alchemist) => alchemist_loadout,
        _ => loadout_default,
    }
}

fn profession_agent_mark(profession: Option<&Profession>) -> Option<comp::agent::Mark> {
    match profession {
        Some(
            Profession::Merchant
            | Profession::Farmer
            | Profession::Herbalist
            | Profession::Chef
            | Profession::Blacksmith
            | Profession::Alchemist,
        ) => Some(comp::agent::Mark::Merchant),
        Some(Profession::Guard) => Some(comp::agent::Mark::Guard),
        _ => None,
    }
}

fn get_npc_entity_info(
    npc: &Npc,
    sites: &Sites,
    index: IndexRef,
    time: Option<&(TimeOfDay, Calendar)>,
) -> EntityInfo {
    let pos = comp::Pos(npc.wpos);

    let mut rng = npc.rng(Npc::PERM_ENTITY_CONFIG);
    if let Some(profession) = npc.profession() {
        let economy = npc.home.and_then(|home| {
            let site = sites.get(home)?.world_site?;
            index.sites.get(site).trade_information(site)
        });

        let config_asset = humanoid_config(&profession);

        let entity_config = EntityConfig::from_asset_expect_owned(config_asset)
            .with_body(BodyBuilder::Exact(npc.body));
        EntityInfo::at(pos.0)
            .with_entity_config(entity_config, Some(config_asset), &mut rng, time)
            .with_alignment(
                if matches!(profession, Profession::Cultist | Profession::Pirate(_)) {
                    comp::Alignment::Enemy
                } else {
                    comp::Alignment::Npc
                },
            )
            .with_economy(economy.as_ref())
            .with_lazy_loadout(profession_extra_loadout(Some(&profession)))
            .with_alias(npc.get_name())
            .with_agent_mark(profession_agent_mark(Some(&profession)))
    } else {
        let config_asset = match npc.body {
            Body::BirdLarge(body) => match body.species {
                comp::bird_large::Species::Phoenix => "common.entity.wild.aggressive.phoenix",
                comp::bird_large::Species::Cockatrice => "common.entity.wild.aggressive.cockatrice",
                comp::bird_large::Species::Roc => "common.entity.wild.aggressive.roc",
                comp::bird_large::Species::CloudWyvern => {
                    "common.entity.wild.aggressive.cloudwyvern"
                },
                comp::bird_large::Species::FlameWyvern => {
                    "common.entity.wild.aggressive.flamewyvern"
                },
                comp::bird_large::Species::FrostWyvern => {
                    "common.entity.wild.aggressive.frostwyvern"
                },
                comp::bird_large::Species::SeaWyvern => "common.entity.wild.aggressive.seawyvern",
                comp::bird_large::Species::WealdWyvern => {
                    "common.entity.wild.aggressive.wealdwyvern"
                },
            },
            Body::BipedLarge(body) => match body.species {
                comp::biped_large::Species::Ogre => "common.entity.wild.aggressive.ogre",
                comp::biped_large::Species::Cyclops => "common.entity.wild.aggressive.cyclops",
                comp::biped_large::Species::Wendigo => "common.entity.wild.aggressive.wendigo",
                comp::biped_large::Species::Werewolf => "common.entity.wild.aggressive.werewolf",
                comp::biped_large::Species::Cavetroll => "common.entity.wild.aggressive.cave_troll",
                comp::biped_large::Species::Mountaintroll => {
                    "common.entity.wild.aggressive.mountain_troll"
                },
                comp::biped_large::Species::Swamptroll => {
                    "common.entity.wild.aggressive.swamp_troll"
                },
                comp::biped_large::Species::Blueoni => "common.entity.wild.aggressive.blue_oni",
                comp::biped_large::Species::Redoni => "common.entity.wild.aggressive.red_oni",
                comp::biped_large::Species::Tursus => "common.entity.wild.aggressive.tursus",
                comp::biped_large::Species::Gigasfrost => {
                    "common.entity.world.world_bosses.gigas_frost"
                },
                comp::biped_large::Species::Gigasfire => {
                    "common.entity.world.world_bosses.gigas_fire"
                },
                species => unimplemented!("rtsim spawning for {:?}", species),
            },
            body => unimplemented!("rtsim spawning for {:?}", body),
        };
        let entity_config = EntityConfig::from_asset_expect_owned(config_asset)
            .with_body(BodyBuilder::Exact(npc.body));

        EntityInfo::at(pos.0).with_entity_config(entity_config, Some(config_asset), &mut rng, time)
    }
}

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        Read<'a, Time>,
        Read<'a, TimeOfDay>,
        Read<'a, EventBus<CreateShipEvent>>,
        Read<'a, EventBus<CreateNpcEvent>>,
        Read<'a, EventBus<DeleteEvent>>,
        WriteExpect<'a, RtSim>,
        ReadExpect<'a, Arc<world::World>>,
        ReadExpect<'a, world::IndexOwned>,
        ReadExpect<'a, SlowJobPool>,
        ReadStorage<'a, comp::Pos>,
        ReadStorage<'a, RtSimEntity>,
        WriteStorage<'a, comp::Agent>,
        ReadStorage<'a, Presence>,
        ReadExpect<'a, Calendar>,
        Read<'a, IdMaps>,
        ReadExpect<'a, ServerConstants>,
        ReadExpect<'a, WeatherGrid>,
        WriteStorage<'a, comp::Inventory>,
        WriteExpect<'a, comp::gizmos::RtsimGizmos>,
        ReadExpect<'a, comp::tool::AbilityMap>,
        ReadExpect<'a, comp::item::MaterialStatManifest>,
    );

    const NAME: &'static str = "rtsim::tick";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            entities,
            dt,
            time,
            time_of_day,
            create_ship_events,
            create_npc_events,
            delete_events,
            mut rtsim,
            world,
            index,
            slow_jobs,
            positions,
            rtsim_entities,
            mut agents,
            presences,
            calendar,
            id_maps,
            server_constants,
            weather_grid,
            inventories,
            rtsim_gizmos,
            ability_map,
            msm,
        ): Self::SystemData,
    ) {
        let mut create_ship_emitter = create_ship_events.emitter();
        let mut create_npc_emitter = create_npc_events.emitter();
        let mut delete_emitter = delete_events.emitter();
        let rtsim = &mut *rtsim;
        let calendar_data = (*time_of_day, (*calendar).clone());

        // Set up rtsim inputs
        {
            let mut data = rtsim.state.data_mut();

            // Update time of day
            data.time_of_day = *time_of_day;

            // Update character map (i.e: so that rtsim knows where players are)
            // TODO: Other entities too like animals? Or do we now care about that?
            data.npcs.character_map.clear();
            for (presence, wpos) in (&presences, &positions).join() {
                if let PresenceKind::Character(character) = &presence.kind {
                    let chunk_pos = wpos.0.xy().as_().wpos_to_cpos();
                    data.npcs
                        .character_map
                        .entry(chunk_pos)
                        .or_default()
                        .push((*character, wpos.0));
                }
            }
        }

        // Tick rtsim
        rtsim.state.tick(
            &mut NpcSystemData {
                positions: positions.clone(),
                id_maps,
                server_constants,
                weather_grid,
                inventories: Mutex::new(inventories),
                rtsim_gizmos,
                ability_map,
                msm,
            },
            &world,
            index.as_index_ref(),
            *time_of_day,
            *time,
            dt.0,
        );

        // Perform a save if required
        if rtsim
            .last_saved
            .is_none_or(|ls| ls.elapsed() > Duration::from_secs(60))
        {
            // TODO: Use slow jobs
            let _ = slow_jobs;
            rtsim.save(/* &slow_jobs, */ false);
        }

        let chunk_states = rtsim.state.resource::<ChunkStates>();
        let data = &mut *rtsim.state.data_mut();

        let mut create_event = |id: NpcId, npc: &Npc, steering: Option<NpcBuilder>| match npc.body {
            Body::Ship(body) => {
                create_ship_emitter.emit(CreateShipEvent {
                    pos: comp::Pos(npc.wpos),
                    ori: comp::Ori::from(Dir::new(npc.dir.with_z(0.0))),
                    ship: body,
                    rtsim_entity: Some(id),
                    driver: steering,
                });
            },
            _ => {
                let entity_info = get_npc_entity_info(
                    npc,
                    &data.sites,
                    index.as_index_ref(),
                    Some(&calendar_data),
                );

                let (mut npc_builder, pos) = SpawnEntityData::from_entity_info(entity_info)
                    .into_npc_data_inner()
                    .expect("Entity loaded from assets cannot be special")
                    .to_npc_builder();

                if let Some(agent) = &mut npc_builder.agent {
                    agent.rtsim_outbox = Some(Default::default());
                }

                if let Some(health) = &mut npc_builder.health {
                    health.set_fraction(npc.health_fraction);
                }

                create_npc_emitter.emit(CreateNpcEvent {
                    pos,
                    ori: comp::Ori::from(Dir::new(npc.dir.with_z(0.0))),
                    npc: npc_builder.with_rtsim(id).with_rider(steering),
                });
            },
        };

        // Load in mounted npcs and their riders
        for mount in data.npcs.mounts.iter_mounts() {
            let mount_npc = data.npcs.npcs.get_mut(mount).expect("This should exist");
            let chunk = mount_npc.wpos.xy().as_::<i32>().wpos_to_cpos();

            if matches!(mount_npc.mode, SimulationMode::Simulated)
                && chunk_states.0.get(chunk).is_some_and(|c| c.is_some())
            {
                mount_npc.mode = SimulationMode::Loaded;

                let mut actor_info = |actor: Actor| {
                    let npc_id = actor.npc()?;
                    let npc = data.npcs.npcs.get_mut(npc_id)?;
                    if matches!(npc.mode, SimulationMode::Simulated) {
                        npc.mode = SimulationMode::Loaded;
                        let entity_info = get_npc_entity_info(
                            npc,
                            &data.sites,
                            index.as_index_ref(),
                            Some(&calendar_data),
                        );

                        let mut npc_builder = SpawnEntityData::from_entity_info(entity_info)
                            .into_npc_data_inner()
                            // EntityConfig can't represent Waypoints at all
                            // as of now, and if someone will try to spawn
                            // rtsim waypoint it is definitely error.
                            .expect("Entity loaded from assets cannot be special")
                            .to_npc_builder()
                            .0
                            .with_rtsim(npc_id);

                        if let Some(agent) = &mut npc_builder.agent {
                            agent.rtsim_outbox = Some(Default::default());
                        }

                        Some(npc_builder)
                    } else {
                        error!("Npc is loaded but vehicle is unloaded");
                        None
                    }
                };

                let steerer = data
                    .npcs
                    .mounts
                    .get_steerer_link(mount)
                    .and_then(|link| actor_info(link.rider));

                let mount_npc = data.npcs.npcs.get(mount).expect("This should exist");
                create_event(mount, mount_npc, steerer);
            }
        }

        // Load in NPCs
        for (npc_id, npc) in data.npcs.npcs.iter_mut() {
            let chunk = npc.wpos.xy().as_::<i32>().wpos_to_cpos();

            // Load the NPC into the world if it's in a loaded chunk and is not already
            // loaded
            if matches!(npc.mode, SimulationMode::Simulated)
                && chunk_states.0.get(chunk).is_some_and(|c| c.is_some())
                // Riding npcs will be spawned by the vehicle.
                && data.npcs.mounts.get_mount_link(npc_id).is_none()
            {
                npc.mode = SimulationMode::Loaded;
                create_event(npc_id, npc, None);
            }
        }

        // Synchronise rtsim NPC with entity data
        for (entity, pos, rtsim_entity, agent) in (
            &entities,
            &positions,
            &rtsim_entities,
            (&mut agents).maybe(),
        )
            .join()
        {
            if let Some(npc) = data.npcs.get_mut(*rtsim_entity) {
                match npc.mode {
                    SimulationMode::Loaded => {
                        // Update rtsim NPC state
                        npc.wpos = pos.0;

                        // Update entity state
                        if let Some(agent) = agent {
                            agent.rtsim_controller.personality = npc.personality;
                            agent.rtsim_controller.look_dir = npc.controller.look_dir;
                            agent.rtsim_controller.activity = npc.controller.activity;
                            agent
                                .rtsim_controller
                                .actions
                                .extend(std::mem::take(&mut npc.controller.actions));
                            if let Some(rtsim_outbox) = &mut agent.rtsim_outbox {
                                npc.inbox.append(rtsim_outbox);
                            }
                        }
                    },
                    SimulationMode::Simulated => {
                        delete_emitter.emit(DeleteEvent(entity));
                    },
                }
            }
        }
    }
}
