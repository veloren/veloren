use crate::{persistence::stats, sys::SysScheduler};
use common::comp::{Player, Stats};
use specs::{Join, ReadStorage, System, Write};

pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadStorage<'a, Player>,
        ReadStorage<'a, Stats>,
        Write<'a, SysScheduler<Self>>,
    );

    fn run(&mut self, (players, player_stats, mut scheduler): Self::SystemData) {
        if scheduler.should_run() {
            for (player, stats) in (&players, &player_stats).join() {
                if let Some(character_id) = player.character_id {
                    stats::update(
                        character_id,
                        Some(stats.level.level() as i32),
                        Some(stats.exp.current() as i32),
                        None,
                        None,
                        None,
                    );
                }
            }
        }
    }
}
