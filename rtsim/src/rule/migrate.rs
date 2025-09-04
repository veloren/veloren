use crate::{
    RtState, Rule, RuleError,
    data::{
        Site,
        architect::{Population, TrackedPopulation},
        npc::Profession,
    },
    event::OnSetup,
    gen::wanted_population,
};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use tracing::warn;
use world::site::plot::PlotKindMeta;

/// This rule runs at rtsim startup and broadly acts to perform some primitive
/// migration/sanitisation in order to ensure that the state of rtsim is mostly
/// sensible.
pub struct Migrate;

impl Rule for Migrate {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnSetup>(|ctx| {
            let data = &mut *ctx.state.data_mut();

            let mut rng = ChaChaRng::from_seed(rand::rng().random::<[u8; 32]>());

            // Delete rtsim sites that don't correspond to a world site
            data.sites.sites.retain(|site_id, site| {
                if let Some((world_site_id, _)) = ctx
                    .index
                    .sites
                    .iter()
                    .find(|(_, world_site)| world_site.origin == site.wpos)
                {
                    site.world_site = Some(world_site_id);
                    data.sites.world_site_map.insert(world_site_id, site_id);
                    true
                } else {
                    warn!(
                        "{:?} is no longer valid because the site it was derived from no longer \
                         exists. It will now be deleted.",
                        site_id
                    );
                    false
                }
            });

            // Generate rtsim sites for world sites that don't have a corresponding rtsim
            // site yet
            for (world_site_id, _) in ctx.index.sites.iter() {
                if !data.sites.values().any(|site| {
                    site.world_site
                        .expect("Rtsim site not assigned to world site")
                        == world_site_id
                }) {
                    warn!(
                        "{:?} is new and does not have a corresponding rtsim site. One will now \
                         be generated afresh.",
                        world_site_id
                    );
                    data.sites.create(Site::generate(
                        world_site_id,
                        ctx.world,
                        ctx.index,
                        &[],
                        &data.factions,
                        &mut rng,
                    ));
                }
            }

            // Reassign NPCs to sites if their old one was deleted. If they were already
            // homeless, no need to do anything.
            // Keep track of airship captains separately, as they need to be handled
            // differently.
            let mut airship_captains = Vec::new();
            for (key, npc) in data.npcs.iter_mut() {
                // For airships, just collect the captains for now
                if matches!(npc.profession(), Some(Profession::Captain)) {
                    airship_captains.push(key);
                } else if let Some(home) = npc.home
                    && !data.sites.contains_key(home)
                {
                    // Choose the closest habitable site as the new home for the NPC
                    npc.home = data
                        .sites
                        .sites
                        .iter()
                        .filter(|(_, site)| {
                            let ally_faction = match (
                                npc.faction.and_then(|f| data.factions.get(f)),
                                site.faction.and_then(|f| data.factions.get(f)),
                            ) {
                                (None, None) => true,
                                (None, Some(_)) => true,
                                (Some(_), None) => true,
                                (Some(npc_faction), Some(site_faction)) => {
                                    npc_faction.good_or_evil == site_faction.good_or_evil
                                },
                            };

                            // See if there is at least one house in this site.
                            let has_house = site.world_site.is_some_and(|ws| {
                                ctx.index.sites.get(ws).any_plot(|p| {
                                    matches!(p.meta(), Some(PlotKindMeta::House { .. }))
                                })
                            });

                            ally_faction && has_house
                        })
                        .min_by_key(|(_, site)| {
                            site.wpos.as_().distance_squared(npc.wpos.xy()) as i32
                        })
                        .map(|(site_id, _)| site_id);
                }
            }

            /*
               First, get all the location where airships can spawn. All available spawning points for airships must be used.
               It does not matter that site ids may be moved around. A captain may be assigned to any site, and
               it does not have to be the site that was previously assigned to the captain.

               First, use all existing captains:
               For each captain
                   If captain is not assigned to a route
                       if a spawning point is available
                           Register the captain for the route that uses the spawning point.
                           Remove the spawning point from the list of available spawning points.
                       else
                           Delete the captain (& airship) pair
                       end
                   End
               End

               Then use all remaining spawning points:
               while there are available spawning points
                   spawn a new captain/airship pair (there won't be existing captains for these)
                   Register the captain for the route that uses the spawning point.
                   Remove the spawning point from the list of available spawning points
               End
            */

            // get all the places to spawn an airship
            let mut spawning_locations = data.airship_spawning_locations(ctx.world);

            // The captains can't be registered inline with this code because it requires
            // mutable access to data.
            let mut captains_to_register = Vec::new();
            for captain_id in airship_captains.iter() {
                if let Some(mount_link) = data.npcs.mounts.get_mount_link(*captain_id) {
                    let airship_id = mount_link.mount;
                    assert!(data.airship_sim.assigned_routes.get(captain_id).is_none());
                    if let Some(spawning_location) = spawning_locations.pop() {
                        captains_to_register.push((*captain_id, airship_id, spawning_location));
                    } else {
                        // delete the captain (& airship) pair
                        data.npcs.remove(*captain_id);
                        data.npcs.remove(airship_id);
                    }
                }
            }
            // All spawning points must be filled, so spawn new airships for any remaining
            // points.
            while let Some(spawning_location) = spawning_locations.pop() {
                let (captain_id, airship_id) = data.spawn_airship(&spawning_location, &mut rng);
                captains_to_register.push((captain_id, airship_id, spawning_location));
            }

            // Register all of the airship captains with airship operations. This can't be
            // done inside the previous loop because this requires mutable
            // access to this (data).
            for (captain_id, airship_id, spawning_location) in captains_to_register.iter() {
                data.airship_sim.register_airship_captain(
                    spawning_location,
                    *captain_id,
                    *airship_id,
                    ctx.world,
                    &mut data.npcs,
                );
            }

            // Group the airship captains by route
            data.airship_sim
                .configure_route_pilots(&ctx.world.civs().airships, &data.npcs);

            // Calculate architect populations
            data.architect.wanted_population = wanted_population(ctx.world, ctx.index);

            data.architect.population = Population::default();

            for npc in data.npcs.values() {
                let pop = TrackedPopulation::from_body_and_role(&npc.body, &npc.role);
                data.architect.population.add(pop, 1);
            }

            for death in data.architect.deaths.iter() {
                data.architect.population.on_spawn(death);
            }
        });

        Ok(Self)
    }
}
