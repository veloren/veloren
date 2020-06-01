use crate::{
    persistence::character,
    sys::{SysScheduler, SysTimer},
};
use common::comp::{Inventory, Player, Stats};
use specs::{Join, ReadExpect, ReadStorage, System, Write};

pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadStorage<'a, Player>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Inventory>,
        ReadExpect<'a, character::CharacterUpdater>,
        Write<'a, SysScheduler<Self>>,
        Write<'a, SysTimer<Self>>,
    );

    fn run(
        &mut self,
        (players, player_stats, player_inventories, updater, mut scheduler, mut timer): Self::SystemData,
    ) {
        if scheduler.should_run() {
            timer.start();
            updater.batch_update(
                (&players, &player_stats, &player_inventories)
                    .join()
                    .filter_map(|(player, stats, inventory)| {
                        player.character_id.map(|id| (id, stats, inventory))
                    }),
            );
            timer.end();
        }
    }
}
