use crate::{
    persistence::stats,
    settings::PersistenceDBDir,
    sys::{SysScheduler, SysTimer},
};
use common::comp::{Player, Stats};
use specs::{Join, ReadExpect, ReadStorage, System, Write};

pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadStorage<'a, Player>,
        ReadStorage<'a, Stats>,
        ReadExpect<'a, PersistenceDBDir>,
        Write<'a, SysScheduler<Self>>,
        Write<'a, SysTimer<Self>>,
    );

    fn run(
        &mut self,
        (players, player_stats, persistence_db_dir, mut scheduler, mut timer): Self::SystemData,
    ) {
        if scheduler.should_run() {
            timer.start();

            stats::batch_update(
                (&players, &player_stats)
                    .join()
                    .filter_map(|(player, stats)| player.character_id.map(|id| (id, stats))),
                &persistence_db_dir.0,
            );

            timer.end();
        }
    }
}
