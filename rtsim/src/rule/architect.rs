use common::{
    comp::{self, Body},
    resources::TimeOfDay,
    rtsim::{Actor, Personality, Profession, Role},
    terrain::CoordinateConversions,
};
use rand::{
    Rng, rng,
    seq::{IndexedRandom, IteratorRandom},
};
use world::{CONFIG, IndexRef, World, sim::SimChunk, site::SiteKind};

use crate::{
    Data, EventCtx, OnTick, RtState,
    data::{
        Npc,
        architect::{Death, TrackedPopulation},
    },
    event::OnDeath,
};

use super::{Rule, RuleError};

/// How many ticks the architect skips.
///
/// We don't need to run it every tick.
const ARCHITECT_TICK_SKIP: u64 = 32;
/// Min spawn delay, in ingame time.
const MIN_SPAWN_DELAY: f64 = 60.0 * 60.0 * 24.0;
/// For monsters that respawn in chunks, how many chunks should we try each
/// respawn.
const RESPAWN_ATTEMPTS: usize = 30;

pub struct Architect;

impl Rule for Architect {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind(on_death);
        rtstate.bind(architect_tick);

        Ok(Self)
    }
}

fn on_death(ctx: EventCtx<Architect, OnDeath>) {
    let data = &mut *ctx.state.data_mut();

    if let Actor::Npc(npc_id) = ctx.event.actor
        && let Some(npc) = data.npcs.get(npc_id)
    {
        data.architect.on_death(npc, data.time_of_day);
    }
}

fn architect_tick(ctx: EventCtx<Architect, OnTick>) {
    if ctx.event.tick % ARCHITECT_TICK_SKIP != 0 {
        return;
    }

    let tod = ctx.event.time_of_day;

    let data = &mut *ctx.state.data_mut();

    let mut rng = rng();
    let mut count_to_spawn = rng.random_range(1..20);

    let pop = data.architect.population.clone();
    'outer: for (pop, count) in pop
        .iter()
        .zip(data.architect.wanted_population.iter())
        .filter(|((_, current), (_, wanted))| current < wanted)
        .map(|((pop, current), (_, wanted))| (pop, wanted - current))
    {
        for _ in 0..count {
            let (body, role) = match pop {
                TrackedPopulation::Adventurers => (
                    Body::Humanoid(comp::humanoid::Body::random()),
                    Role::Civilised(Some(Profession::Adventurer(rng.random_range(0..=3)))),
                ),
                TrackedPopulation::Merchants => (
                    Body::Humanoid(comp::humanoid::Body::random()),
                    Role::Civilised(Some(Profession::Merchant)),
                ),
                TrackedPopulation::Guards => (
                    Body::Humanoid(comp::humanoid::Body::random()),
                    Role::Civilised(Some(Profession::Guard)),
                ),
                TrackedPopulation::Captains => (
                    Body::Humanoid(comp::humanoid::Body::random()),
                    Role::Civilised(Some(Profession::Captain)),
                ),
                TrackedPopulation::OtherTownNpcs => (
                    Body::Humanoid(comp::humanoid::Body::random()),
                    Role::Civilised(Some(match rng.random_range(0..10) {
                        0 => Profession::Hunter,
                        1 => Profession::Blacksmith,
                        2 => Profession::Chef,
                        3 => Profession::Alchemist,
                        4..=5 => Profession::Herbalist,
                        _ => Profession::Farmer,
                    })),
                ),
                TrackedPopulation::Pirates => (
                    Body::Humanoid(comp::humanoid::Body::random()),
                    Role::Civilised(Some(Profession::Pirate(false))),
                ),
                TrackedPopulation::PirateCaptains => (
                    Body::Humanoid(comp::humanoid::Body::random()),
                    Role::Civilised(Some(Profession::Pirate(true))),
                ),
                TrackedPopulation::Cultists => (
                    Body::Humanoid(comp::humanoid::Body::random()),
                    Role::Civilised(Some(Profession::Cultist)),
                ),
                TrackedPopulation::GigasFrost => (
                    Body::BipedLarge(comp::biped_large::Body::random_with(
                        &mut rng,
                        &comp::biped_large::Species::Gigasfrost,
                    )),
                    Role::Monster,
                ),
                TrackedPopulation::GigasFire => (
                    Body::BipedLarge(comp::biped_large::Body::random_with(
                        &mut rng,
                        &comp::biped_large::Species::Gigasfire,
                    )),
                    Role::Monster,
                ),
                TrackedPopulation::OtherMonsters => {
                    let species = [
                        comp::biped_large::Species::Ogre,
                        comp::biped_large::Species::Cyclops,
                        comp::biped_large::Species::Wendigo,
                        comp::biped_large::Species::Cavetroll,
                        comp::biped_large::Species::Mountaintroll,
                        comp::biped_large::Species::Swamptroll,
                        comp::biped_large::Species::Blueoni,
                        comp::biped_large::Species::Redoni,
                        comp::biped_large::Species::Tursus,
                    ]
                    .choose(&mut rng)
                    .unwrap();

                    (
                        Body::BipedLarge(comp::biped_large::Body::random_with(&mut rng, species)),
                        Role::Monster,
                    )
                },
                TrackedPopulation::CloudWyvern => (
                    Body::BirdLarge(comp::bird_large::Body::random_with(
                        &mut rng,
                        &comp::bird_large::Species::CloudWyvern,
                    )),
                    Role::Wild,
                ),
                TrackedPopulation::FrostWyvern => (
                    Body::BirdLarge(comp::bird_large::Body::random_with(
                        &mut rng,
                        &comp::bird_large::Species::FrostWyvern,
                    )),
                    Role::Wild,
                ),
                TrackedPopulation::SeaWyvern => (
                    Body::BirdLarge(comp::bird_large::Body::random_with(
                        &mut rng,
                        &comp::bird_large::Species::SeaWyvern,
                    )),
                    Role::Wild,
                ),
                TrackedPopulation::FlameWyvern => (
                    Body::BirdLarge(comp::bird_large::Body::random_with(
                        &mut rng,
                        &comp::bird_large::Species::FlameWyvern,
                    )),
                    Role::Wild,
                ),
                TrackedPopulation::WealdWyvern => (
                    Body::BirdLarge(comp::bird_large::Body::random_with(
                        &mut rng,
                        &comp::bird_large::Species::WealdWyvern,
                    )),
                    Role::Wild,
                ),
                TrackedPopulation::Phoenix => (
                    Body::BirdLarge(comp::bird_large::Body::random_with(
                        &mut rng,
                        &comp::bird_large::Species::Phoenix,
                    )),
                    Role::Wild,
                ),
                TrackedPopulation::Roc => (
                    Body::BirdLarge(comp::bird_large::Body::random_with(
                        &mut rng,
                        &comp::bird_large::Species::Roc,
                    )),
                    Role::Wild,
                ),
                TrackedPopulation::Cockatrice => (
                    Body::BirdLarge(comp::bird_large::Body::random_with(
                        &mut rng,
                        &comp::bird_large::Species::Cockatrice,
                    )),
                    Role::Wild,
                ),
                TrackedPopulation::Other => continue 'outer,
            };

            let fake_death = Death {
                time: TimeOfDay(tod.0 - MIN_SPAWN_DELAY),
                body,
                role,
                faction: None,
            };

            data.architect.population.on_spawn(&fake_death);

            data.architect.deaths.push_front(fake_death);
        }

        count_to_spawn += count;
    }

    // @perf: Could reuse previous allocation here.
    let mut failed_spawn = Vec::new();

    while count_to_spawn > 0
        && let Some(death) = data.architect.deaths.pop_front()
    {
        if data.architect.population.of_death(&death)
            > data.architect.wanted_population.of_death(&death)
        {
            data.architect.population.on_death(&death);
            // If we have more than enough of this npc, we skip spawning a new one.
            continue;
        }

        if death.time.0 + MIN_SPAWN_DELAY > tod.0 {
            data.architect.deaths.push_front(death);
            break;
        }

        if spawn_npc(data, ctx.world, ctx.index, &death) {
            count_to_spawn -= 1;
        } else {
            failed_spawn.push(death);
        }
    }

    for death in failed_spawn.into_iter().rev() {
        data.architect.deaths.push_front(death);
    }
}

fn randomize_body(body: Body, rng: &mut impl Rng) -> Body {
    let mut random_humanoid = || {
        let species = comp::humanoid::ALL_SPECIES.choose(rng).unwrap();
        Body::Humanoid(comp::humanoid::Body::random_with(rng, species))
    };
    match body {
        Body::Humanoid(_) => random_humanoid(),
        body => body,
    }
}

fn role_personality(rng: &mut impl Rng, role: &Role) -> Personality {
    match role {
        Role::Civilised(profession) => match profession {
            Some(Profession::Guard | Profession::Merchant | Profession::Captain) => {
                Personality::random_good(rng)
            },
            Some(Profession::Cultist | Profession::Pirate(_)) => Personality::random_evil(rng),
            None
            | Some(
                Profession::Farmer
                | Profession::Chef
                | Profession::Hunter
                | Profession::Blacksmith
                | Profession::Alchemist
                | Profession::Herbalist
                | Profession::Adventurer(_),
            ) => Personality::random(rng),
        },
        Role::Wild => Personality::random(rng),
        Role::Monster => Personality::random_evil(rng),
        Role::Vehicle => Personality::default(),
    }
}

fn spawn_anywhere(
    data: &mut Data,
    world: &World,
    death: &Death,
    rng: &mut impl Rng,
    body: Body,
    personality: Personality,
) {
    let mut attempt = |check: bool| {
        let cpos = world
            .sim()
            .map_size_lg()
            .chunks()
            .map(|s| rng.random_range(0..s as i32));

        // TODO: If we had access to `ChunkStates` here we could make sure
        // these aren't getting respawned in loaded chunks.
        if let Some(chunk) = world.sim().get(cpos)
            && (!check || !chunk.is_underwater())
        {
            let wpos = cpos.cpos_to_wpos_center();
            let wpos = wpos.as_().with_z(world.sim().get_surface_alt_approx(wpos));

            data.spawn_npc(
                Npc::new(rng.random(), wpos, body, death.role.clone())
                    .with_personality(personality),
            );
            return true;
        }

        false
    };
    for _ in 0..RESPAWN_ATTEMPTS {
        if attempt(true) {
            return;
        }
    }
    attempt(false);
}

fn spawn_at_plot(
    data: &mut Data,
    world: &World,
    index: IndexRef,
    death: &Death,
    rng: &mut impl Rng,
    body: Body,
    personality: Personality,
    match_plot: impl Fn(&Data, common::rtsim::SiteId, &world::site::Plot) -> bool,
) -> bool {
    let sites = &index.sites;
    let data_ref = &*data;
    let match_plot = &match_plot;
    if let Some((id, site, plot)) = data
        .sites
        .iter()
        .filter(|(_, site)| !site.is_loaded())
        .filter_map(|(id, site)| Some((id, site.world_site?)))
        .flat_map(|(id, world_site)| {
            let world_site = sites.get(world_site);
            world_site
                .filter_plots(move |plot| match_plot(data_ref, id, plot))
                .map(move |plot| (id, world_site, plot))
        })
        .choose(rng)
    {
        let wpos = site.tile_center_wpos(plot.root_tile());
        let wpos = wpos
            .as_()
            .with_z(world.sim().get_alt_approx(wpos).unwrap_or(0.0));
        let mut npc = Npc::new(rng.random(), wpos, body, death.role.clone())
            .with_personality(personality)
            .with_home(id);
        if let Some(faction) = data.sites[id].faction {
            npc = npc.with_faction(faction);
        }
        data.spawn_npc(npc);

        true
    } else {
        false
    }
}

fn spawn_profession(
    data: &mut Data,
    world: &World,
    index: IndexRef,
    death: &Death,
    rng: &mut impl Rng,
    body: Body,
    personality: Personality,
    profession: Option<Profession>,
) -> bool {
    match profession {
        Some(Profession::Pirate(captain)) => {
            spawn_at_plot(
                data,
                world,
                index,
                death,
                rng,
                body,
                personality,
                |data, s, p| {
                    // Don't spawn multiple captains at the same site.
                    if captain
                        && data.sites[s].population.iter().any(|npc| {
                            data.npcs.get(*npc).is_some_and(|npc| {
                                matches!(npc.profession(), Some(Profession::Pirate(true)))
                            })
                        })
                    {
                        return false;
                    }
                    matches!(p.kind(), world::site::PlotKind::PirateHideout(_))
                },
            )
        },
        _ => spawn_at_plot(
            data,
            world,
            index,
            death,
            rng,
            body,
            personality,
            |_, _, p| {
                matches!(
                    p.kind().meta(),
                    Some(world::site::plot::PlotKindMeta::House { .. })
                )
            },
        ),
    }
}

fn spawn_npc(data: &mut Data, world: &World, index: IndexRef, death: &Death) -> bool {
    let mut rng = rng();
    let body = randomize_body(death.body, &mut rng);
    let personality = role_personality(&mut rng, &death.role);
    // First try and respawn in the same faction.
    let did_spawn = if let Some(faction_id) = death.faction
        && data.factions.get(faction_id).is_some()
    {
        if let Some((id, site)) = data
            .sites
            .iter()
            .filter(|(_, site)| site.faction == Some(faction_id) && !site.is_loaded())
            .choose(&mut rng)
        {
            let wpos = site.wpos;
            let wpos = wpos
                .as_()
                .with_z(world.sim().get_alt_approx(wpos).unwrap_or(0.0));
            data.spawn_npc(
                Npc::new(rng.random(), wpos, body, death.role.clone())
                    .with_personality(personality)
                    .with_home(id)
                    .with_faction(faction_id),
            );

            true
        } else {
            false
        }
    } else {
        match &death.role {
            Role::Civilised(profession) => spawn_profession(
                data,
                world,
                index,
                death,
                &mut rng,
                body,
                personality,
                *profession,
            ),
            Role::Wild => {
                let site_filter: fn(&SiteKind) -> bool = match body {
                    Body::BirdLarge(body) => match body.species {
                        comp::bird_large::Species::Phoenix => {
                            |site| matches!(site, SiteKind::DwarvenMine)
                        },
                        comp::bird_large::Species::Cockatrice => {
                            |site| matches!(site, SiteKind::Myrmidon)
                        },
                        comp::bird_large::Species::Roc => |site| matches!(site, SiteKind::Haniwa),
                        comp::bird_large::Species::FlameWyvern => {
                            |site| matches!(site, SiteKind::Terracotta)
                        },
                        comp::bird_large::Species::CloudWyvern => {
                            |site| matches!(site, SiteKind::Sahagin)
                        },
                        comp::bird_large::Species::FrostWyvern => {
                            |site| matches!(site, SiteKind::Adlet)
                        },
                        comp::bird_large::Species::SeaWyvern => {
                            |site| matches!(site, SiteKind::ChapelSite)
                        },
                        comp::bird_large::Species::WealdWyvern => {
                            |site| matches!(site, SiteKind::GiantTree)
                        },
                    },
                    _ => |_| true,
                };

                if let Some((id, site)) = data
                    .sites
                    .iter()
                    .filter(|(_, site)| {
                        !site.is_loaded()
                            && site
                                .world_site
                                .and_then(|s| index.sites.get(s).kind)
                                .is_some_and(|s| site_filter(&s))
                    })
                    .choose(&mut rng)
                {
                    let wpos = site.wpos;
                    let wpos = wpos
                        .as_()
                        .with_z(world.sim().get_alt_approx(wpos).unwrap_or(0.0));
                    data.spawn_npc(
                        Npc::new(rng.random(), wpos, body, death.role.clone())
                            .with_personality(personality)
                            .with_home(id),
                    );
                    true
                } else {
                    false
                }
            },
            Role::Monster => {
                let chunk_filter: fn(&SimChunk) -> bool = match body {
                    Body::BipedLarge(body) => match body.species {
                        comp::biped_large::Species::Tursus
                        | comp::biped_large::Species::Gigasfrost
                        | comp::biped_large::Species::Wendigo => {
                            |chunk| !chunk.is_underwater() && chunk.temp < CONFIG.snow_temp
                        },
                        comp::biped_large::Species::Gigasfire => |chunk| {
                            !chunk.is_underwater()
                                && chunk.temp > CONFIG.desert_temp
                                && chunk.humidity < CONFIG.desert_hum
                        },
                        comp::biped_large::Species::Mountaintroll => {
                            |chunk| !chunk.is_underwater() && chunk.alt > 500.0
                        },
                        comp::biped_large::Species::Swamptroll => {
                            |chunk| !chunk.is_underwater() && chunk.humidity > CONFIG.jungle_hum
                        },
                        _ => |chunk| !chunk.is_underwater(),
                    },
                    Body::Arthropod(_)
                    | Body::Humanoid(_)
                    | Body::QuadrupedSmall(_)
                    | Body::BipedSmall(_)
                    | Body::QuadrupedMedium(_)
                    | Body::Golem(_)
                    | Body::Theropod(_)
                    | Body::QuadrupedLow(_) => |chunk| !chunk.is_underwater(),
                    Body::Dragon(_) | Body::BirdLarge(_) | Body::BirdMedium(_) => |_| true,
                    Body::Crustacean(_) | Body::FishSmall(_) | Body::FishMedium(_) => {
                        |chunk| chunk.is_underwater()
                    },
                    Body::Object(_) | Body::Ship(_) | Body::Item(_) | Body::Plugin(_) => |_| true,
                };

                for _ in 0..RESPAWN_ATTEMPTS {
                    let cpos = world
                        .sim()
                        .map_size_lg()
                        .chunks()
                        .map(|s| rng.random_range(0..s as i32));

                    // TODO: If we had access to `ChunkStates` here we could make sure
                    // these aren't getting respawned in loaded chunks.
                    if let Some(chunk) = world.sim().get(cpos)
                        && chunk_filter(chunk)
                    {
                        let wpos = cpos.cpos_to_wpos_center();
                        let wpos = wpos.as_().with_z(world.sim().get_surface_alt_approx(wpos));

                        data.spawn_npc(
                            Npc::new(rng.random(), wpos, body, death.role.clone())
                                .with_personality(personality),
                        );
                        return true;
                    }
                }

                false
            },
            Role::Vehicle => {
                // Vehicles don't die as of now.
                unimplemented!()
            },
        }
    };

    // If enough time has passed, try spawning anyway.
    if !did_spawn && death.time.0 + MIN_SPAWN_DELAY * 5.0 < data.time_of_day.0 {
        match death.role {
            Role::Civilised(profession) => {
                if !spawn_profession(
                    data,
                    world,
                    index,
                    death,
                    &mut rng,
                    body,
                    personality,
                    profession,
                ) {
                    spawn_anywhere(data, world, death, &mut rng, body, personality)
                }
            },
            Role::Wild | Role::Monster => {
                spawn_anywhere(data, world, death, &mut rng, body, personality)
            },
            Role::Vehicle => {
                // Vehicles don't die as of now.
                unimplemented!()
            },
        }

        true
    } else {
        did_spawn
    }
}
