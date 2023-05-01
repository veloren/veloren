use crate::{
    data::{npc::SimulationMode, Npc},
    event::{EventCtx, OnDeath, OnSetup, OnTick},
    RtState, Rule, RuleError,
};
use common::{
    comp::{self, Body},
    rtsim::{Actor, NpcAction, NpcActivity, Personality, Role},
    terrain::CoordinateConversions,
};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use tracing::{error, warn};
use vek::Vec2;
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

    if let Actor::Npc(npc_id) = ctx.event.actor {
        if let Some(npc) = data.npcs.get(npc_id) {
            let mut rng = ChaChaRng::from_seed(thread_rng().gen::<[u8; 32]>());

            // Respawn dead NPCs
            let details = match npc.body {
                Body::Humanoid(_) => {
                    if let Some((site_id, site)) = data
                        .sites
                        .iter()
                        .filter(|(id, site)| {
                            Some(*id) != npc.home
                                && (npc.faction.is_none() || site.faction == npc.faction)
                                && site.world_site.map_or(false, |s| {
                                    matches!(
                                        ctx.index.sites.get(s).kind,
                                        SiteKind::Refactor(_)
                                            | SiteKind::CliffTown(_)
                                            | SiteKind::SavannahPit(_)
                                            | SiteKind::DesertCity(_)
                                    )
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
                            Npc::new(
                                rng.gen(),
                                rand_wpos(&mut rng),
                                random_humanoid(&mut rng),
                                npc.role.clone(),
                            )
                            .with_personality(Personality::random(&mut rng))
                            .with_home(site_id)
                            .with_faction(npc.faction),
                        );
                        Some((npc_id, site_id))
                    } else {
                        warn!("No site found for respawning humanoid");
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
                                Role::Wild,
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
}

fn on_tick(ctx: EventCtx<SimulateNpcs, OnTick>) {
    let data = &mut *ctx.state.data_mut();
    for (npc_id, npc) in data.npcs.npcs.iter_mut().filter(|(_, npc)| !npc.is_dead) {
        if matches!(npc.mode, SimulationMode::Simulated) {
            // Simulate NPC movement when riding
            if let Some(riding) = &npc.riding {
                if let Some(vehicle) = data.npcs.vehicles.get_mut(riding.vehicle) {
                    match npc.controller.activity {
                        // If steering, the NPC controls the vehicle's motion
                        Some(NpcActivity::Goto(target, speed_factor)) if riding.steering => {
                            let diff = target.xy() - vehicle.wpos.xy();
                            let dist2 = diff.magnitude_squared();

                            if dist2 > 0.5f32.powi(2) {
                                let wpos = vehicle.wpos
                                    + (diff
                                        * (vehicle.get_speed() * speed_factor * ctx.event.dt
                                            / dist2.sqrt())
                                        .min(1.0));

                                let is_valid = match vehicle.body {
                                    common::comp::ship::Body::DefaultAirship
                                    | common::comp::ship::Body::AirBalloon => true,
                                    common::comp::ship::Body::SailBoat
                                    | common::comp::ship::Body::Galleon => {
                                        let chunk_pos = wpos.xy().as_().wpos_to_cpos();
                                        ctx.world
                                            .sim()
                                            .get(chunk_pos)
                                            .map_or(true, |f| f.river.river_kind.is_some())
                                    },
                                    _ => false,
                                };

                                if is_valid {
                                    vehicle.wpos = wpos;
                                }
                                vehicle.dir = (target.xy() - vehicle.wpos.xy())
                                    .try_normalized()
                                    .unwrap_or(Vec2::unit_y());
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
                    Some(
                        NpcActivity::Gather(_) | NpcActivity::HuntAnimals | NpcActivity::Dance,
                    ) => {
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

        // Move home if required
        if let Some(new_home) = npc.controller.new_home.take() {
            // Remove the NPC from their old home population
            if let Some(old_home) = npc.home {
                if let Some(old_home) = data.sites.get_mut(old_home) {
                    old_home.population.remove(&npc_id);
                }
            }
            // Add the NPC to their new home population
            if let Some(new_home) = data.sites.get_mut(new_home) {
                new_home.population.insert(npc_id);
            }
            npc.home = Some(new_home);
        }
    }
}
