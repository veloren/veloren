use crate::{
    data::{npc::SimulationMode, Npc},
    event::{OnDeath, OnSetup, OnTick},
    RtState, Rule, RuleError,
};
use common::{
    comp::{self, Body},
    grid::Grid,
    rtsim::{Actor, NpcAction, NpcActivity, Personality},
    terrain::TerrainChunkSize,
    vol::RectVolSize,
};
use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};
use tracing::warn;
use world::site::SiteKind;

pub struct SimulateNpcs;

impl Rule for SimulateNpcs {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnSetup>(|ctx| {
            let data = &mut *ctx.state.data_mut();
            data.npcs.npc_grid = Grid::new(ctx.world.sim().get_size().as_(), Default::default());

            for (npc_id, npc) in data.npcs.npcs.iter() {
                if let Some(ride) = &npc.riding {
                    if let Some(vehicle) = data.npcs.vehicles.get_mut(ride.vehicle) {
                        let actor = Actor::Npc(npc_id);
                        vehicle.riders.push(actor);
                        if ride.steering && vehicle.driver.replace(actor).is_some() {
                            panic!("Replaced driver");
                        }
                    }
                }

                if let Some(home) = npc.home.and_then(|home| data.sites.get_mut(home)) {
                    home.population.insert(npc_id);
                }
            }
        });

        rtstate.bind::<Self, OnDeath>(|ctx| {
            let data = &mut *ctx.state.data_mut();
            let npc_id = ctx.event.npc_id;
            let Some(npc) = data.npcs.get(npc_id) else {
                return;
            };
            if let Some(home) = npc.home.and_then(|home| data.sites.get_mut(home)) {
                home.population.remove(&npc_id);
            }
            let mut rng = rand::thread_rng();
            match npc.body {
                Body::Humanoid(_) => {
                    if let Some((site_id, site)) = data
                        .sites
                        .iter()
                        .filter(|(id, site)| {
                            Some(*id) != npc.home
                                && site.faction == npc.faction
                                && site.world_site.map_or(false, |s| {
                                    matches!(ctx.index.sites.get(s).kind, SiteKind::Refactor(_))
                                })
                        })
                        .min_by_key(|(_, site)| site.population.len())
                    {
                        let rand_wpos = |rng: &mut ThreadRng| {
                            let wpos2d = site.wpos.map(|e| e + rng.gen_range(-10..10));
                            wpos2d
                                .map(|e| e as f32 + 0.5)
                                .with_z(ctx.world.sim().get_alt_approx(wpos2d).unwrap_or(0.0))
                        };
                        let random_humanoid = |rng: &mut ThreadRng| {
                            let species = comp::humanoid::ALL_SPECIES.choose(&mut *rng).unwrap();
                            Body::Humanoid(comp::humanoid::Body::random_with(rng, species))
                        };
                        data.spawn_npc(
                            Npc::new(rng.gen(), rand_wpos(&mut rng), random_humanoid(&mut rng))
                                .with_personality(Personality::random(&mut rng))
                                .with_home(site_id)
                                .with_faction(npc.faction)
                                .with_profession(npc.profession.clone()),
                        );
                    } else {
                        warn!("No site found for respawning humaniod");
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
                        let rand_wpos = |rng: &mut ThreadRng| {
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
                        data.npcs.create_npc(
                            Npc::new(
                                rng.gen(),
                                rand_wpos(&mut rng),
                                Body::BirdLarge(comp::body::bird_large::Body::random_with(
                                    &mut rng, species,
                                )),
                            )
                            .with_home(site_id),
                        );
                    } else {
                        warn!("No site found for respawning bird");
                    }
                },
                _ => unimplemented!(),
            }
        });

        rtstate.bind::<Self, OnTick>(|ctx| {
            let data = &mut *ctx.state.data_mut();
            for (vehicle_id, vehicle) in data.npcs.vehicles.iter_mut() {
                let chunk_pos =
                    vehicle.wpos.xy().as_::<i32>() / TerrainChunkSize::RECT_SIZE.as_::<i32>();
                if vehicle.chunk_pos != Some(chunk_pos) {
                    if let Some(cell) = vehicle
                        .chunk_pos
                        .and_then(|chunk_pos| data.npcs.npc_grid.get_mut(chunk_pos))
                    {
                        if let Some(index) = cell.vehicles.iter().position(|id| *id == vehicle_id) {
                            cell.vehicles.swap_remove(index);
                        }
                    }
                    vehicle.chunk_pos = Some(chunk_pos);
                    if let Some(cell) = data.npcs.npc_grid.get_mut(chunk_pos) {
                        cell.vehicles.push(vehicle_id);
                    }
                }
            }
            for (npc_id, npc) in data.npcs.npcs.iter_mut() {
                // Update the NPC's current site, if any
                npc.current_site = ctx
                    .world
                    .sim()
                    .get(npc.wpos.xy().as_::<i32>() / TerrainChunkSize::RECT_SIZE.as_())
                    .and_then(|chunk| {
                        chunk
                            .sites
                            .iter()
                            .find_map(|site| data.sites.world_site_map.get(site).copied())
                    });

                let chunk_pos =
                    npc.wpos.xy().as_::<i32>() / TerrainChunkSize::RECT_SIZE.as_::<i32>();
                if npc.chunk_pos != Some(chunk_pos) {
                    if let Some(cell) = npc
                        .chunk_pos
                        .and_then(|chunk_pos| data.npcs.npc_grid.get_mut(chunk_pos))
                    {
                        if let Some(index) = cell.npcs.iter().position(|id| *id == npc_id) {
                            cell.npcs.swap_remove(index);
                        }
                    }
                    npc.chunk_pos = Some(chunk_pos);
                    if let Some(cell) = data.npcs.npc_grid.get_mut(chunk_pos) {
                        cell.npcs.push(npc_id);
                    }
                }

                // Simulate the NPC's movement and interactions
                if matches!(npc.mode, SimulationMode::Simulated) {
                    // Simulate NPC movement when riding
                    if let Some(riding) = &npc.riding {
                        if let Some(vehicle) = data.npcs.vehicles.get_mut(riding.vehicle) {
                            match npc.controller.activity {
                                // If steering, the NPC controls the vehicle's motion
                                Some(NpcActivity::Goto(target, speed_factor))
                                    if riding.steering =>
                                {
                                    let diff = target.xy() - vehicle.wpos.xy();
                                    let dist2 = diff.magnitude_squared();

                                    if dist2 > 0.5f32.powi(2) {
                                        let mut wpos = vehicle.wpos
                                            + (diff
                                                * (vehicle.get_speed()
                                                    * speed_factor
                                                    * ctx.event.dt
                                                    / dist2.sqrt())
                                                .min(1.0))
                                            .with_z(0.0);

                                        let is_valid = match vehicle.body {
                                            common::comp::ship::Body::DefaultAirship
                                            | common::comp::ship::Body::AirBalloon => true,
                                            common::comp::ship::Body::SailBoat
                                            | common::comp::ship::Body::Galleon => {
                                                let chunk_pos = wpos.xy().as_::<i32>()
                                                    / TerrainChunkSize::RECT_SIZE.as_::<i32>();
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
                                        * (npc.body.max_speed_approx()
                                            * speed_factor
                                            * ctx.event.dt
                                            / dist2.sqrt())
                                        .min(1.0))
                                    .with_z(0.0);
                                }
                            },
                            Some(
                                NpcActivity::Gather(_)
                                | NpcActivity::HuntAnimals
                                | NpcActivity::Dance,
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
                            NpcAction::Greet(_) | NpcAction::Say(_) => {}, // Currently, just swallow interactions
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
        });

        Ok(Self)
    }
}
