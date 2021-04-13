use crate::{persistence::character_updater, presence::Presence, sys::SysScheduler};
use common::comp::{Inventory, Stats, Waypoint};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::PresenceKind;
use specs::{Join, ReadStorage, Write, WriteExpect};

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadStorage<'a, Presence>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Inventory>,
        ReadStorage<'a, Waypoint>,
        WriteExpect<'a, character_updater::CharacterUpdater>,
        Write<'a, SysScheduler<Self>>,
    );

    const NAME: &'static str = "persistence";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            presences,
            player_stats,
            player_inventories,
            player_waypoint,
            mut updater,
            mut scheduler,
        ): Self::SystemData,
    ) {
        if scheduler.should_run() {
            updater.batch_update(
                (
                    &presences,
                    &player_stats,
                    &player_inventories,
                    player_waypoint.maybe(),
                )
                    .join()
                    .filter_map(
                        |(presence, stats, inventory, waypoint)| match presence.kind {
                            PresenceKind::Character(id) => Some((id, stats, inventory, waypoint)),
                            PresenceKind::Spectator => None,
                        },
                    ),
            );
        }
    }
}
