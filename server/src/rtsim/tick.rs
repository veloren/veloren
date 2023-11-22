#![allow(dead_code)] // TODO: Remove this when rtsim is fleshed out

use super::*;
use crate::sys::terrain::NpcData;
use common::{
    calendar::Calendar,
    comp::{self, Agent, Body, Presence, PresenceKind},
    event::{CreateNpcEvent, CreateShipEvent, DeleteEvent, EventBus, NpcBuilder},
    generation::{BodyBuilder, EntityConfig, EntityInfo},
    resources::{DeltaTime, Time, TimeOfDay},
    rtsim::{Actor, NpcId, RtSimEntity},
    slowjob::SlowJobPool,
    terrain::CoordinateConversions,
    trade::{Good, SiteInformation},
    util::Dir,
    LoadoutBuilder,
};
use common_ecs::{Job, Origin, Phase, System};
use rtsim::data::{
    npc::{Profession, SimulationMode},
    Npc, Sites,
};
use specs::{Entities, Join, LendJoin, Read, ReadExpect, ReadStorage, WriteExpect, WriteStorage};
use std::{sync::Arc, time::Duration};
use tracing::error;
use world::site::settlement::trader_loadout;

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
        Profession::Pirate => "common.entity.spot.pirate",
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
    trader_loadout(loadout_builder, economy, |good| matches!(good, Good::Food))
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
    trader_loadout(loadout_builder, economy, |good| matches!(good, Good::Food))
}

fn blacksmith_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
) -> LoadoutBuilder {
    trader_loadout(loadout_builder, economy, |good| {
        matches!(good, Good::Tools | Good::Armor)
    })
}

fn alchemist_loadout(
    loadout_builder: LoadoutBuilder,
    economy: Option<&SiteInformation>,
    _time: Option<&(TimeOfDay, Calendar)>,
) -> LoadoutBuilder {
    trader_loadout(loadout_builder, economy, |good| {
        matches!(good, Good::Potions)
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
            index.sites.get(site).trade_information(site.id())
        });

        let config_asset = humanoid_config(&profession);

        let entity_config = EntityConfig::from_asset_expect_owned(config_asset)
            .with_body(BodyBuilder::Exact(npc.body));
        EntityInfo::at(pos.0)
            .with_entity_config(entity_config, Some(config_asset), &mut rng, time)
            .with_alignment(if matches!(profession, Profession::Cultist) {
                comp::Alignment::Enemy
            } else {
                comp::Alignment::Npc
            })
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
        rtsim
            .state
            .tick(&world, index.as_index_ref(), *time_of_day, *time, dt.0);

        // Perform a save if required
        if rtsim
            .last_saved
            .map_or(true, |ls| ls.elapsed() > Duration::from_secs(60))
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
                    rtsim_entity: Some(RtSimEntity(id)),
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

                create_npc_emitter.emit(match NpcData::from_entity_info(entity_info) {
                    NpcData::Data {
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
                    } => CreateNpcEvent {
                        pos,
                        ori: comp::Ori::from(Dir::new(npc.dir.with_z(0.0))),
                        npc: NpcBuilder::new(stats, body, alignment)
                            .with_skill_set(skill_set)
                            .with_health(health)
                            .with_poise(poise)
                            .with_inventory(inventory)
                            .with_agent(agent.map(|agent| Agent {
                                rtsim_outbox: Some(Default::default()),
                                ..agent
                            }))
                            .with_scale(scale)
                            .with_loot(loot)
                            .with_rtsim(RtSimEntity(id)),
                        rider: steering,
                    },
                    // EntityConfig can't represent Waypoints at all
                    // as of now, and if someone will try to spawn
                    // rtsim waypoint it is definitely error.
                    NpcData::Waypoint(_) => unimplemented!(),
                    NpcData::Teleporter(_, _) => unimplemented!(),
                });
            },
        };

        // Load in mounted npcs and their riders
        for mount in data.npcs.mounts.iter_mounts() {
            let mount_npc = data.npcs.npcs.get_mut(mount).expect("This should exist");
            let chunk = mount_npc.wpos.xy().as_::<i32>().wpos_to_cpos();

            if matches!(mount_npc.mode, SimulationMode::Simulated)
                && chunk_states.0.get(chunk).map_or(false, |c| c.is_some())
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

                        Some(match NpcData::from_entity_info(entity_info) {
                            NpcData::Data {
                                pos: _,
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
                            } => NpcBuilder::new(stats, body, alignment)
                                .with_skill_set(skill_set)
                                .with_health(health)
                                .with_poise(poise)
                                .with_inventory(inventory)
                                .with_agent(agent.map(|agent| Agent {
                                    rtsim_outbox: Some(Default::default()),
                                    ..agent
                                }))
                                .with_scale(scale)
                                .with_loot(loot)
                                .with_rtsim(RtSimEntity(npc_id)),
                            // EntityConfig can't represent Waypoints at all
                            // as of now, and if someone will try to spawn
                            // rtsim waypoint it is definitely error.
                            NpcData::Waypoint(_) => unimplemented!(),
                            NpcData::Teleporter(_, _) => unimplemented!(),
                        })
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
                && chunk_states.0.get(chunk).map_or(false, |c| c.is_some())
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
            if let Some(npc) = data.npcs.get_mut(rtsim_entity.0) {
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
