use crate::{
    RtState, Rule, RuleError,
    data::{Sentiment, npc::SimulationMode},
    event::{EventCtx, OnHealthChange, OnHelped, OnMountVolume, OnTick},
};
use common::{
    comp::{self, Body, agent::FlightMode},
    mounting::{Volume, VolumePos},
    rtsim::{Actor, NpcAction, NpcActivity, NpcInput},
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

    let mut npc_inputs = Vec::new();

    for (npc_id, npc) in data.npcs.npcs.iter_mut().filter(|(_, npc)| !npc.is_dead()) {
        npc.controller.actions.retain(|action| match action {
            // NPC-to-NPC messages never leave rtsim
            NpcAction::Msg { to, msg } => {
                if let Actor::Npc(to) = to {
                    npc_inputs.push((*to, NpcInput::Msg {
                        from: npc_id.into(),
                        msg: msg.clone(),
                    }));
                } else {
                    // TODO: Send to players?
                }
                false
            },
            // All other cases are handled by the game when loaded
            NpcAction::Say(_, _) | NpcAction::Attack(_) | NpcAction::Dialogue(_, _) => {
                matches!(npc.mode, SimulationMode::Loaded)
            },
        });

        if matches!(npc.mode, SimulationMode::Simulated) {
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
                Some(NpcActivity::GotoFlying(target, speed_factor, height, dir, mode)) => {
                    let diff = target - npc.wpos;
                    let dist2 = diff.magnitude_squared();

                    if dist2 > 0.5f32.powi(2) {
                        match npc.body {
                            Body::Ship(comp::ship::Body::DefaultAirship) => {
                                // RTSim NPCs don't interract with terrain, and their position is
                                // independent of ground level.
                                // While movement is simulated, airships will happily stay at ground
                                // level or fly through mountains.
                                // The code at the end of this block "Make sure NPCs remain in a
                                // valid location" just forces
                                // airships to be at least above ground (on the ground actually).
                                // The reason is that when docking, airships need to descend much
                                // closer to the terrain
                                // than when cruising between sites, so airships cannot be forced to
                                // stay at a fixed height above
                                // terrain (i.e. flying_height()). Instead, when mode is
                                // FlightMode::FlyThrough, set the airship altitude directly to
                                // terrain height + height (if Some)
                                // or terrain height + default height (npc.body.flying_height()).
                                // When mode is FlightMode::Braking, the airship is allowed to
                                // descend below flying height
                                // because it is near or at the dock. In this mode, if height is
                                // Some, set the airship altitude to
                                // the maximum of target.z or terrain height + height. If height is
                                // None, set the airship altitude to
                                // target.z. By forcing the airship altitude to be at a specific
                                // value, when the airship is
                                // suddenly in a loaded chunk it will not be below or at the ground
                                // and will not get stuck.

                                // Move in x,y
                                let diffxy = target.xy() - npc.wpos.xy();
                                let distxy2 = diffxy.magnitude_squared();
                                if distxy2 > 0.5f32.powi(2) {
                                    let offsetxy = diffxy
                                        * (npc.body.max_speed_approx()
                                            * speed_factor
                                            * ctx.event.dt
                                            / distxy2.sqrt());
                                    npc.wpos.x += offsetxy.x;
                                    npc.wpos.y += offsetxy.y;
                                }
                                // The diff is not computed for z like x,y. Rather, the altitude is
                                // set directly so that when the
                                // simulated ship is suddenly in a loaded chunk it will not be below
                                // or at the ground level and risk getting stuck.
                                let base_height =
                                    if mode == FlightMode::FlyThrough || height.is_some() {
                                        ctx.world.sim().get_surface_alt_approx(npc.wpos.xy().as_())
                                    } else {
                                        0.0
                                    };
                                let ship_z = match mode {
                                    FlightMode::FlyThrough => {
                                        base_height + height.unwrap_or(npc.body.flying_height())
                                    },
                                    FlightMode::Braking(_) => {
                                        (base_height + height.unwrap_or(0.0)).max(target.z)
                                    },
                                };
                                npc.wpos.z = ship_z;
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
                // Don't force air ships to be at flying_height, else they can't land at docks.
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
        for (id, quest) in core::mem::take(&mut npc.controller.quests_to_create) {
            data.quests.create(id, quest);
        }

        // Set job status
        npc.job = npc.controller.job.clone();
    }

    for (npc_id, input) in npc_inputs {
        if let Some(npc) = data.npcs.get_mut(npc_id) {
            npc.inbox.push_back(input);
        }
    }
}
