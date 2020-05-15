use crate::{
    persistence::stats,
    sys::{SysScheduler, SysTimer},
};
use common::comp::{Player, Stats};
use specs::{Join, ReadExpect, ReadStorage, System, Write};

pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadStorage<'a, Player>,
        ReadStorage<'a, Stats>,
        ReadExpect<'a, stats::Updater>,
        Write<'a, SysScheduler<Self>>,
        Write<'a, SysTimer<Self>>,
    );

    fn run(
        &mut self,
        (players, player_stats, updater, mut scheduler, mut timer): Self::SystemData,
    ) {
        if scheduler.should_run() {
            timer.start();
            updater.batch_update(
                (&players, &player_stats)
                    .join()
                    .filter_map(|(player, stats)| player.character_id.map(|id| (id, stats))),
            );
            timer.end();
        }
    }
}
