use crate::{
    data::{npc::SimulationMode, Npc},
    event::{EventCtx, OnDeath, OnMountVolume, OnTick},
    RtState, Rule, RuleError,
};
use common::{
    comp::{self, Body},
    mounting::{Volume, VolumePos},
    rtsim::{Actor, NpcAction, NpcActivity, Personality},
    terrain::{CoordinateConversions, TerrainChunkSize},
    vol::RectVolSize,
};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use slotmap::SecondaryMap;
use tracing::{error, warn};
use vek::{Clamp, Vec2};
use world::{site::SiteKind, CONFIG};

pub struct SimulateNpcs;

impl Rule for SimulateNpcs {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind(on_death);
        rtstate.bind(on_tick);
        rtstate.bind(on_mount_volume);

        Ok(Self)
    }
}

fn on_mount_volume(ctx: EventCtx<SimulateNpcs, OnMountVolume>) {
    let data = &mut *ctx.state.data_mut();

    // TODO: Add actor to riders.
    if let VolumePos {
        kind: Volume::Entity(vehicle),
        ..
    } = ctx.event.pos
        && let Some(link) = data.npcs.mounts.get_steerer_link(vehicle)
        && let Actor::Npc(driver) = link.rider
        && let Some(driver) = data.npcs.get_mut(driver)
    {
        driver.controller.actions.push(NpcAction::Say(
            Some(ctx.event.actor),
            comp::Content::localized("npc-speech-welcome-aboard"),
        ))
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
                            // Don't respawn in the same town
                            Some(*id) != npc.home
                                && site.world_site.map_or(false, |s| {
                                    matches!(
                                        ctx.index.sites.get(s).kind,
                                        SiteKind::Refactor(_)
                                            | SiteKind::CliffTown(_)
                                            | SiteKind::SavannahPit(_)
                                            | SiteKind::CoastalTown(_)
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
                        Some((npc_id, Some(site_id)))
                    } else {
                        warn!("No site found for respawning humanoid");
                        None
                    }
                },
                body => {
                    let home = npc.home.and_then(|_| {
                        data.sites
                            .iter()
                            .filter(|(id, site)| {
                                Some(*id) != npc.home
                                    && site.world_site.map_or(false, |s| {
                                        matches!(ctx.index.sites.get(s).kind, SiteKind::Dungeon(_))
                                    })
                            })
                            .min_by_key(|(_, site)| site.population.len())
                    });

                    let wpos = if let Some((_, home)) = home {
                        let wpos2d = home.wpos.map(|e| e + rng.gen_range(-10..10));
                        wpos2d
                            .map(|e| e as f32 + 0.5)
                            .with_z(ctx.world.sim().get_alt_approx(wpos2d).unwrap_or(0.0))
                    } else {
                        let is_gigas = matches!(body, Body::BipedLarge(body) if body.species == comp::body::biped_large::Species::Gigasfrost);

                        let pos = (0..(if is_gigas {
                            /* More attempts for gigas */
                            100
                        } else {
                            10
                        }))
                            .map(|_| {
                                ctx.world
                                    .sim()
                                    .get_size()
                                    .map(|sz| rng.gen_range(0..sz as i32))
                            })
                            .find(|pos| {
                                ctx.world.sim().get(*pos).map_or(false, |c| {
                                    !c.is_underwater() && (!is_gigas || c.temp < CONFIG.snow_temp)
                                })
                            })
                            .unwrap_or(ctx.world.sim().get_size().as_() / 2);
                        let wpos2d = pos.cpos_to_wpos_center();
                        wpos2d
                            .map(|e| e as f32 + 0.5)
                            .with_z(ctx.world.sim().get_alt_approx(wpos2d).unwrap_or(0.0))
                    };

                    let home = home.map(|(site_id, _)| site_id);

                    let npc_id = data.npcs.create_npc(
                        Npc::new(rng.gen(), wpos, body, npc.role.clone()).with_home(home),
                    );
                    Some((npc_id, home))
                },
            };

            // Add the NPC to their home site
            if let Some((npc_id, Some(home_site))) = details {
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

    // Maintain links
    let ids = data.npcs.mounts.ids().collect::<Vec<_>>();
    let mut mount_activity = SecondaryMap::new();
    for link_id in ids {
        if let Some(link) = data.npcs.mounts.get(link_id) {
            if let Some(mount) = data
                .npcs
                .npcs
                .get(link.mount)
                .filter(|mount| !mount.is_dead)
            {
                let wpos = mount.wpos;
                if let Actor::Npc(rider) = link.rider {
                    if let Some(rider) =
                        data.npcs.npcs.get_mut(rider).filter(|rider| !rider.is_dead)
                    {
                        rider.wpos = wpos;
                        mount_activity.insert(link.mount, rider.controller.activity);
                    } else {
                        data.npcs.mounts.dismount(link.rider)
                    }
                }
            } else {
                data.npcs.mounts.remove_mount(link.mount)
            }
        }
    }

    for (npc_id, npc) in data.npcs.npcs.iter_mut().filter(|(_, npc)| !npc.is_dead) {
        if matches!(npc.mode, SimulationMode::Simulated) {
            // Consume NPC actions
            for action in std::mem::take(&mut npc.controller.actions) {
                match action {
                    NpcAction::Say(_, _) => {}, // Currently, just swallow interactions
                    NpcAction::Attack(_) => {}, // TODO: Implement simulated combat
                }
            }

            let activity = if data.npcs.mounts.get_mount_link(npc_id).is_some() {
                // We are riding, nothing to do.
                continue;
            } else if let Some(activity) = mount_activity.get(npc_id) {
                *activity
            } else {
                npc.controller.activity
            };

            match activity {
                // Move NPCs if they have a target destination
                Some(NpcActivity::Goto(target, speed_factor)) => {
                    let diff = target - npc.wpos;
                    let dist2 = diff.magnitude_squared();

                    if dist2 > 0.5f32.powi(2) {
                        let offset = diff
                            * (npc.body.max_speed_approx() * speed_factor * ctx.event.dt
                                / dist2.sqrt())
                            .min(1.0);
                        let new_wpos = npc.wpos + offset;

                        let is_valid = match npc.body {
                            // Don't move water bound bodies outside of water.
                            Body::Ship(comp::ship::Body::SailBoat | comp::ship::Body::Galleon)
                            | Body::FishMedium(_)
                            | Body::FishSmall(_) => {
                                let chunk_pos = new_wpos.xy().as_().wpos_to_cpos();
                                ctx.world
                                    .sim()
                                    .get(chunk_pos)
                                    .map_or(true, |f| f.river.river_kind.is_some())
                            },
                            _ => true,
                        };

                        if is_valid {
                            npc.wpos = new_wpos;
                        }

                        npc.dir = (target.xy() - npc.wpos.xy())
                            .try_normalized()
                            .unwrap_or(npc.dir);
                    }
                },
                Some(
                    NpcActivity::Gather(_)
                    | NpcActivity::HuntAnimals
                    | NpcActivity::Dance(_)
                    | NpcActivity::Cheer(_)
                    | NpcActivity::Sit(..),
                ) => {
                    // TODO: Maybe they should walk around randomly
                    // when gathering resources?
                },
                None => {},
            }

            // Make sure NPCs remain in a valid location
            let clamped_wpos = npc.wpos.xy().clamped(
                Vec2::zero(),
                (ctx.world.sim().get_size() * TerrainChunkSize::RECT_SIZE).as_(),
            );
            match npc.body {
                Body::Ship(comp::ship::Body::DefaultAirship | comp::ship::Body::AirBalloon) => {
                    npc.wpos = clamped_wpos.with_z(
                        ctx.world
                            .sim()
                            .get_surface_alt_approx(clamped_wpos.as_())
                            .max(npc.wpos.z),
                    );
                },
                _ => {
                    npc.wpos = clamped_wpos.with_z(
                        ctx.world.sim().get_surface_alt_approx(clamped_wpos.as_())
                            + npc.body.flying_height(),
                    );
                },
            }
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
