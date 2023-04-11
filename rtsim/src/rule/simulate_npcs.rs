use crate::{
    data::{npc::SimulationMode, Npc},
    event::{EventCtx, OnDeath, OnSetup, OnTick},
    RtState, Rule, RuleError,
};
use common::{
    comp::{self, Body},
    rtsim::{Actor, NpcAction, NpcActivity, Personality},
    terrain::TerrainChunkSize,
    vol::RectVolSize,
};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use tracing::{error, warn};
use world::site::SiteKind;

pub struct SimulateNpcs;

impl Rule for SimulateNpcs {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnSetup>(on_setup);
        rtstate.bind::<Self, OnDeath>(on_death);
        rtstate.bind::<Self, OnTick>(on_tick);

        Ok(Self)
    }
}

fn on_setup(ctx: EventCtx<SimulateNpcs, OnSetup>) {
    let data = &mut *ctx.state.data_mut();

    // Add riders to vehicles
    for (npc_id, npc) in data.npcs.npcs.iter_mut() {
        if let Some(ride) = &npc.riding {
            if let Some(vehicle) = data.npcs.vehicles.get_mut(ride.vehicle) {
                let actor = Actor::Npc(npc_id);
                if ride.steering && vehicle.driver.replace(actor).is_some() {
                    error!("Replaced driver");
                    npc.riding = None;
                }
            }
        }
    }
}

fn on_death(ctx: EventCtx<SimulateNpcs, OnDeath>) {
    let data = &mut *ctx.state.data_mut();

    if let Actor::Npc(npc_id) = ctx.event.actor
        && let Some(npc) = data.npcs.get(npc_id)
    {
        let mut rng = ChaChaRng::from_seed(thread_rng().gen::<[u8; 32]>());

        // Respawn dead NPCs
        let details = match npc.body {
            Body::Humanoid(_) => {
                if let Some((site_id, site)) = data
                    .sites
                    .iter()
                    .filter(|(id, site)| {
                        Some(*id) != npc.home
                            && site.faction == npc.faction
                            && site.world_site.map_or(false, |s| {
                                matches!(ctx.index.sites.get(s).kind, SiteKind::Refactor(_)
                | SiteKind::CliffTown(_)
                | SiteKind::SavannahPit(_)
                | SiteKind::DesertCity(_))
                            })
                    })
                    .min_by_key(|(_, site)| site.population.len())
                {
                    let rand_wpos = |rng: &mut ChaChaRng| {
                        let wpos2d = site.wpos.map(|e| e + rng.gen_range(-10..10));
                        wpos2d
                            .map(|e| e as f32 + 0.5)
                            .with_z(ctx.world.sim().get_alt_approx(wpos2d).unwrap_or(0.0))
                    };
                    let random_humanoid = |rng: &mut ChaChaRng| {
                        let species = comp::humanoid::ALL_SPECIES.choose(&mut *rng).unwrap();
                        Body::Humanoid(comp::humanoid::Body::random_with(rng, species))
                    };
                    let npc_id = data.spawn_npc(
                        Npc::new(rng.gen(), rand_wpos(&mut rng), random_humanoid(&mut rng))
                            .with_personality(Personality::random(&mut rng))
                            .with_home(site_id)
                            .with_faction(npc.faction)
                            .with_profession(npc.profession.clone()),
                    );
                    Some((npc_id, site_id))
                } else {
                    warn!("No site found for respawning humaniod");
                    None
                }
            },
            Body::BirdLarge(_) => {
                if let Some((site_id, site)) = data
                    .sites
                    .iter()
                    .filter(|(id, site)| {
                        Some(*id) != npc.home
                            && site.world_site.map_or(false, |s| {
                                matches!(ctx.index.sites.get(s).kind, SiteKind::Dungeon(_))
                            })
                    })
                    .min_by_key(|(_, site)| site.population.len())
                {
                    let rand_wpos = |rng: &mut ChaChaRng| {
                        let wpos2d = site.wpos.map(|e| e + rng.gen_range(-10..10));
                        wpos2d
                            .map(|e| e as f32 + 0.5)
                            .with_z(ctx.world.sim().get_alt_approx(wpos2d).unwrap_or(0.0))
                    };
                    let species = [
                        comp::body::bird_large::Species::Phoenix,
                        comp::body::bird_large::Species::Cockatrice,
                        comp::body::bird_large::Species::Roc,
                    ]
                    .choose(&mut rng)
                    .unwrap();
                    let npc_id = data.npcs.create_npc(
                        Npc::new(
                            rng.gen(),
                            rand_wpos(&mut rng),
                            Body::BirdLarge(comp::body::bird_large::Body::random_with(
                                &mut rng, species,
                            )),
                        )
                        .with_home(site_id),
                    );
                    Some((npc_id, site_id))
                } else {
                    warn!("No site found for respawning bird");
                    None
                }
            },
            body => {
                error!("Tried to respawn rtsim NPC with invalid body: {:?}", body);
                None
            },
        };

        // Add the NPC to their home site
        if let Some((npc_id, home_site)) = details {
            if let Some(home) = data.sites.get_mut(home_site) {
                home.population.insert(npc_id);
            }
        }
    } else {
        error!("Trying to respawn non-existent NPC");
    }
}

fn on_tick(ctx: EventCtx<SimulateNpcs, OnTick>) {
    let data = &mut *ctx.state.data_mut();
    for npc in data
        .npcs
        .npcs
        .values_mut()
        .filter(|npc| matches!(npc.mode, SimulationMode::Simulated) && !npc.is_dead)
    {
        // Simulate NPC movement when riding
        if let Some(riding) = &npc.riding {
            if let Some(vehicle) = data.npcs.vehicles.get_mut(riding.vehicle) {
                match npc.controller.activity {
                    // If steering, the NPC controls the vehicle's motion
                    Some(NpcActivity::Goto(target, speed_factor)) if riding.steering => {
                        let diff = target.xy() - vehicle.wpos.xy();
                        let dist2 = diff.magnitude_squared();

                        if dist2 > 0.5f32.powi(2) {
                            let mut wpos = vehicle.wpos
                                + (diff
                                    * (vehicle.get_speed() * speed_factor * ctx.event.dt
                                        / dist2.sqrt())
                                    .min(1.0))
                                .with_z(0.0);

                            let is_valid = match vehicle.body {
                                common::comp::ship::Body::DefaultAirship
                                | common::comp::ship::Body::AirBalloon => true,
                                common::comp::ship::Body::SailBoat
                                | common::comp::ship::Body::Galleon => {
                                    let chunk_pos =
                                        wpos.xy().as_::<i32>().map2(
                                            TerrainChunkSize::RECT_SIZE.as_::<i32>(),
                                            |e, sz| e.div_euclid(sz),
                                        );
                                    ctx.world
                                        .sim()
                                        .get(chunk_pos)
                                        .map_or(true, |f| f.river.river_kind.is_some())
                                },
                                _ => false,
                            };

                            if is_valid {
                                match vehicle.body {
                                    common::comp::ship::Body::DefaultAirship
                                    | common::comp::ship::Body::AirBalloon => {
                                        if let Some(alt) = ctx
                                            .world
                                            .sim()
                                            .get_alt_approx(wpos.xy().as_())
                                            .filter(|alt| wpos.z < *alt)
                                        {
                                            wpos.z = alt;
                                        }
                                    },
                                    common::comp::ship::Body::SailBoat
                                    | common::comp::ship::Body::Galleon => {
                                        wpos.z = ctx
                                            .world
                                            .sim()
                                            .get_interpolated(
                                                wpos.xy().map(|e| e as i32),
                                                |chunk| chunk.water_alt,
                                            )
                                            .unwrap_or(0.0);
                                    },
                                    _ => {},
                                }
                                vehicle.wpos = wpos;
                            }
                        }
                    },
                    // When riding, other actions are disabled
                    Some(
                        NpcActivity::Goto(_, _)
                        | NpcActivity::Gather(_)
                        | NpcActivity::HuntAnimals
                        | NpcActivity::Dance,
                    ) => {},
                    None => {},
                }
                npc.wpos = vehicle.wpos;
            } else {
                // Vehicle doens't exist anymore
                npc.riding = None;
            }
        // If not riding, we assume they're just walking
        } else {
            match npc.controller.activity {
                // Move NPCs if they have a target destination
                Some(NpcActivity::Goto(target, speed_factor)) => {
                    let diff = target.xy() - npc.wpos.xy();
                    let dist2 = diff.magnitude_squared();

                    if dist2 > 0.5f32.powi(2) {
                        npc.wpos += (diff
                            * (npc.body.max_speed_approx() * speed_factor * ctx.event.dt
                                / dist2.sqrt())
                            .min(1.0))
                        .with_z(0.0);
                    }
                },
                Some(NpcActivity::Gather(_) | NpcActivity::HuntAnimals | NpcActivity::Dance) => {
                    // TODO: Maybe they should walk around randomly
                    // when gathering resources?
                },
                None => {},
            }
        }

        // Consume NPC actions
        for action in std::mem::take(&mut npc.controller.actions) {
            match action {
                NpcAction::Say(_, _) => {}, // Currently, just swallow interactions
                NpcAction::Attack(_) => {}, // TODO: Implement simulated combat
            }
        }

        // Make sure NPCs remain on the surface
        npc.wpos.z = ctx
            .world
            .sim()
            .get_surface_alt_approx(npc.wpos.xy().map(|e| e as i32))
            .unwrap_or(0.0)
            + npc.body.flying_height();
    }
}
