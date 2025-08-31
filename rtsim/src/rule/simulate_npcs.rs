use crate::{
    RtState, Rule, RuleError,
    data::{Sentiment, npc::SimulationMode},
    event::{EventCtx, OnHealthChange, OnHelped, OnMountVolume, OnTick},
};
use common::{
    comp::{self, Body},
    mounting::{Volume, VolumePos},
    rtsim::{Actor, NpcAction, NpcActivity},
    terrain::{CoordinateConversions, TerrainChunkSize},
    vol::RectVolSize,
};
use slotmap::SecondaryMap;
use vek::{Clamp, Vec2};

pub struct SimulateNpcs;

impl Rule for SimulateNpcs {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind(on_helped);
        rtstate.bind(on_health_changed);
        rtstate.bind(on_mount_volume);
        rtstate.bind(on_tick);

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

fn on_health_changed(ctx: EventCtx<SimulateNpcs, OnHealthChange>) {
    let data = &mut *ctx.state.data_mut();

    if let Some(cause) = ctx.event.cause
        && let Actor::Npc(npc) = ctx.event.actor
        && let Some(npc) = data.npcs.get_mut(npc)
    {
        if ctx.event.change < 0.0 {
            npc.sentiments
                .toward_mut(cause)
                .change_by(-0.1, Sentiment::ENEMY);
        } else if ctx.event.change > 0.0 {
            npc.sentiments
                .toward_mut(cause)
                .change_by(0.05, Sentiment::POSITIVE);
        }
    }
}

fn on_helped(ctx: EventCtx<SimulateNpcs, OnHelped>) {
    let data = &mut *ctx.state.data_mut();

    if let Some(saver) = ctx.event.saver
        && let Actor::Npc(npc) = ctx.event.actor
        && let Some(npc) = data.npcs.get_mut(npc)
    {
        npc.controller.actions.push(NpcAction::Say(
            Some(ctx.event.actor),
            comp::Content::localized("npc-speech-thank_you"),
        ));
        npc.sentiments
            .toward_mut(saver)
            .change_by(0.3, Sentiment::FRIEND);
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
                .filter(|mount| !mount.is_dead())
            {
                let wpos = mount.wpos;
                if let Actor::Npc(rider) = link.rider {
                    if let Some(rider) = data
                        .npcs
                        .npcs
                        .get_mut(rider)
                        .filter(|rider| !rider.is_dead())
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

    for (npc_id, npc) in data.npcs.npcs.iter_mut().filter(|(_, npc)| !npc.is_dead()) {
        if matches!(npc.mode, SimulationMode::Simulated) {
            // Consume NPC actions
            for action in std::mem::take(&mut npc.controller.actions) {
                match action {
                    NpcAction::Say(_, _) => {}, // Currently, just swallow interactions
                    NpcAction::Attack(_) => {}, // TODO: Implement simulated combat
                    NpcAction::Dialogue(_, _) => {},
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
                                    .is_none_or(|f| f.river.river_kind.is_some())
                            },
                            Body::Ship(comp::ship::Body::DefaultAirship) => false,
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
                // Move Flying NPCs like airships if they have a target destination
                Some(NpcActivity::GotoFlying(target, speed_factor, _, dir, _)) => {
                    let diff = target - npc.wpos;
                    let dist2 = diff.magnitude_squared();

                    if dist2 > 0.5f32.powi(2) {
                        match npc.body {
                            Body::Ship(comp::ship::Body::DefaultAirship) => {
                                // Don't limit airship movement to 1.0 per axis
                                // SimulationMode::Simulated treats the Npc dimensions differently
                                // somehow from when the Npc is
                                // loaded. The calculation below results in a position
                                // that is roughly the offset from the ship centerline to the
                                // docking position and is also off
                                // by the offset from the ship fore/aft centerline to the docking
                                // position. The result is that if the player spawns in at a dock
                                // where an airship is docked, the
                                // airship will bounce around while seeking the docking position
                                // in loaded mode.
                                let offset = diff
                                    * (npc.body.max_speed_approx() * speed_factor * ctx.event.dt
                                        / dist2.sqrt());
                                npc.wpos += offset;
                            },
                            _ => {
                                let offset = diff
                                    * (npc.body.max_speed_approx() * speed_factor * ctx.event.dt
                                        / dist2.sqrt())
                                    .min(1.0);
                                let new_wpos = npc.wpos + offset;

                                let is_valid = match npc.body {
                                    // Don't move water bound bodies outside of water.
                                    Body::Ship(
                                        comp::ship::Body::SailBoat | comp::ship::Body::Galleon,
                                    )
                                    | Body::FishMedium(_)
                                    | Body::FishSmall(_) => {
                                        let chunk_pos = new_wpos.xy().as_().wpos_to_cpos();
                                        ctx.world
                                            .sim()
                                            .get(chunk_pos)
                                            .is_none_or(|f| f.river.river_kind.is_some())
                                    },
                                    _ => true,
                                };

                                if is_valid {
                                    npc.wpos = new_wpos;
                                }
                            },
                        }

                        if let Some(dir_override) = dir {
                            npc.dir = dir_override.xy().try_normalized().unwrap_or(npc.dir);
                        } else {
                            npc.dir = (target.xy() - npc.wpos.xy())
                                .try_normalized()
                                .unwrap_or(npc.dir);
                        }
                    }
                },
                Some(
                    NpcActivity::Gather(_)
                    | NpcActivity::HuntAnimals
                    | NpcActivity::Dance(_)
                    | NpcActivity::Cheer(_)
                    | NpcActivity::Sit(..)
                    | NpcActivity::Talk(..),
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
            if let Some(old_home) = npc.home
                && let Some(old_home) = data.sites.get_mut(old_home)
            {
                old_home.population.remove(&npc_id);
            }
            // Add the NPC to their new home population
            if let Some(new_home) = new_home
                && let Some(new_home) = data.sites.get_mut(new_home)
            {
                new_home.population.insert(npc_id);
            }
            npc.home = new_home;
        }

        // Create registered quests
        for (id, quest) in core::mem::take(&mut npc.controller.created_quests) {
            data.quests.create(id, quest);
        }

        // Set job status
        npc.job = npc.controller.job.clone();
    }
}
