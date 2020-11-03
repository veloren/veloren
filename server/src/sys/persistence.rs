use crate::{
    persistence::character_updater,
    presence::Presence,
    sys::{SysScheduler, SysTimer},
};
use common::{
    comp::{Inventory, Loadout, Stats, Waypoint},
    msg::PresenceKind,
    span,
};
use specs::{Join, ReadExpect, ReadStorage, System, Write};

pub struct Sys;

impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)] // TODO: Pending review in #587
    type SystemData = (
        ReadStorage<'a, Presence>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Inventory>,
        ReadStorage<'a, Loadout>,
        ReadStorage<'a, Waypoint>,
        ReadExpect<'a, character_updater::CharacterUpdater>,
        Write<'a, SysScheduler<Self>>,
        Write<'a, SysTimer<Self>>,
    );

    fn run(
        &mut self,
        (
            presences,
            player_stats,
            player_inventories,
            player_loadouts,
            player_waypoint,
            updater,
            mut scheduler,
            mut timer,
        ): Self::SystemData,
    ) {
        span!(_guard, "run", "persistence::Sys::run");
        if scheduler.should_run() {
            timer.start();
            updater.batch_update(
                (
                    &presences,
                    &player_stats,
                    &player_inventories,
                    &player_loadouts,
                    player_waypoint.maybe(),
                )
                    .join()
                    .filter_map(
                        |(presence, stats, inventory, loadout, waypoint)| match presence.kind {
                            PresenceKind::Character(id) => {
                                Some((id, stats, inventory, loadout, waypoint))
                            },
                            PresenceKind::Spectator => None,
                        },
                    ),
            );
            timer.end();
        }
    }
}
