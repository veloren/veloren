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
            let updates: Vec<(i32, &Stats)> = (&players, &player_stats)
                .join()
                .filter_map(|(player, stats)| {
                    if let Some(character_id) = player.character_id {
                        Some((character_id, stats))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            if !updates.is_empty() {
                stats::update(updates);
            }
        }
    }
}
